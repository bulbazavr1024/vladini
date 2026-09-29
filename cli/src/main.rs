use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use vladini::cli::{Cli, Command};
use vladini::io::{collect_files, collect_files_matching, create_backup, read_file, resolve_output, write_file};
use vladini::report::{FileResult, Report};
use vladini::config::{ProcessingConfig, StripMode};
use vladini::converter::{AUDIO_EXTENSIONS, ConvertFormat, MEDIA_EXTENSIONS, convert_image, convert_media};
use vladini::format::ImageFormat;
use vladini::pipeline::Pipeline;
use vladini::processor::png::{PngProcessor, inspect_png};
use vladini::processor::jpg::{JpgProcessor, inspect_jpg};
use vladini::processor::mp3::{Mp3Processor, inspect_mp3};
use vladini::processor::wav::{WavProcessor, inspect_wav};
use vladini::processor::webp::{WebpProcessor, inspect_webp};
use vladini::processor::mp4::{Mp4Processor, inspect_mp4, extract_frames_to_png};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Init logging
    let log_level = if cli.verbose { "debug" } else { "warn" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    match &cli.command {
        Command::Compress {
            input,
            output,
            quality,
            speed,
            no_lossy,
            strip,
            recursive,
            backup,
            dry_run,
            width,
            height,
        } => {
            let config = cli.to_config(*quality, *speed, *no_lossy, *strip, *dry_run, *backup, *width, *height);
            handle_compress(input, output.as_deref(), *recursive, &config)
        }
        Command::Convert {
            input,
            output,
            to,
            quality,
            no_lossy,
            strip,
            bitrate,
            width,
            height,
            start,
            end,
            jobs,
            recursive,
            backup,
        } => {
            let config = ProcessingConfig {
                quality: *quality,
                no_lossy: *no_lossy,
                strip: *strip,
                backup: *backup,
                resize_width: *width,
                resize_height: *height,
                audio_bitrate: bitrate.clone(),
                start: start.clone(),
                end: end.clone(),
                ..Default::default()
            };
            // `convert clip.webm clip.mp3` works without --to
            let to = to
                .clone()
                .or_else(|| {
                    let out = output.as_ref().filter(|o| !o.is_dir())?;
                    Some(out.extension()?.to_string_lossy().into_owned())
                })
                .context("Pass --to FORMAT or an output file name with an extension")?;
            handle_convert(input, output.as_deref(), &to, *recursive, *jobs, &config)
        }
        Command::Inspect { input, recursive } => {
            handle_inspect(input, *recursive)
        }
        Command::Extract { input, output, fps, recursive } => {
            handle_extract(input, output, *fps, *recursive)
        }
    }
}

fn handle_compress(
    input: &Path,
    output: Option<&Path>,
    recursive: bool,
    config: &ProcessingConfig,
) -> Result<()> {
    // Build pipeline
    let mut pipeline = Pipeline::new();
    pipeline.register(Box::new(PngProcessor));
    pipeline.register(Box::new(JpgProcessor));
    pipeline.register(Box::new(Mp3Processor));
    pipeline.register(Box::new(WavProcessor));
    pipeline.register(Box::new(WebpProcessor));
    pipeline.register(Box::new(Mp4Processor));

    // Collect files
    let files = collect_files(input, recursive)
        .context("Failed to collect input files")?;

    if files.is_empty() {
        println!("No supported files found.");
        return Ok(());
    }

    println!("Found {} file(s) to process.", files.len());

    if config.dry_run {
        println!("[dry-run] Would process:");
        for f in &files {
            let out = resolve_output(f, input, output);
            println!("  {} → {}", f.display(), out.display());
        }
        return Ok(());
    }

    // Progress bar
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let report = Mutex::new(Report::new());

    // Process files in parallel
    files.par_iter().for_each(|input_path| {
        let output_path = resolve_output(input_path, input, output);

        let result = (|| -> std::result::Result<FileResult, anyhow::Error> {
            let data = read_file(input_path)?;
            let original_size = data.len() as u64;

            let compressed = pipeline.process_file(input_path, &data, config)?;
            let compressed_size = compressed.len() as u64;

            // For audio formats with metadata stripping, always write output
            // (the goal is metadata removal, not size reduction)
            let format = ImageFormat::from_path(input_path);
            let is_metadata_strip = matches!(
                format,
                Some(ImageFormat::Mp3) | Some(ImageFormat::Wav)
            ) && !matches!(config.strip, StripMode::None);

            // Skip if compressed is larger (but not for audio metadata stripping)
            if compressed_size >= original_size && !is_metadata_strip {
                log::debug!(
                    "Skipping {} — compressed ({}) >= original ({})",
                    input_path.display(),
                    compressed_size,
                    original_size
                );
                return Ok(FileResult {
                    path: input_path.clone(),
                    original_size,
                    compressed_size: original_size,
                    skipped: true,
                    error: None,
                });
            }

            if config.backup {
                create_backup(&output_path)?;
            }
            write_file(&output_path, &compressed)?;

            Ok(FileResult {
                path: input_path.clone(),
                original_size,
                compressed_size,
                skipped: false,
                error: None,
            })
        })();

        match result {
            Ok(file_result) => {
                if !file_result.skipped {
                    pb.set_message(format!(
                        "{} ({:.1}%)",
                        input_path.file_name().unwrap().to_string_lossy(),
                        file_result.savings_pct()
                    ));
                }
                report.lock().unwrap().add(file_result);
            }
            Err(e) => {
                log::error!("Error processing {}: {}", input_path.display(), e);
                report.lock().unwrap().add(FileResult {
                    path: input_path.clone(),
                    original_size: 0,
                    compressed_size: 0,
                    skipped: false,
                    error: Some(e.to_string()),
                });
            }
        }

        pb.inc(1);
    });

    pb.finish_with_message("Done!");
    report.lock().unwrap().print_summary();

    Ok(())
}

fn handle_convert(
    input: &Path,
    output: Option<&Path>,
    target_format_str: &str,
    recursive: bool,
    jobs: Option<usize>,
    config: &ProcessingConfig,
) -> Result<()> {
    // png/jpg/webp go through the image encoders, everything else through ffmpeg
    let image_format = ConvertFormat::from_str(target_format_str);
    if image_format.is_some() && (config.start.is_some() || config.end.is_some()) {
        anyhow::bail!("--start/--end only apply to audio/video targets");
    }
    let target_ext = match image_format {
        Some(format) => format.extension().to_string(),
        None => target_format_str.to_ascii_lowercase(),
    };

    // In a mixed folder, only pick up files that can become the target:
    // no cover art -> mp3, no phone clips -> jpg, no songs -> gif
    let is_image = |ext: &str| image::ImageFormat::from_extension(ext).is_some();
    let audio_target = AUDIO_EXTENSIONS.contains(&target_ext.as_str());
    let image_target = target_ext != "gif" && is_image(&target_ext);
    let files = collect_files_matching(input, recursive, |p| {
        let Some(ext) = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()) else {
            return false;
        };
        MEDIA_EXTENSIONS.contains(&ext.as_str())
            && if audio_target {
                !is_image(&ext)
            } else if image_target {
                is_image(&ext)
            } else {
                !(target_ext == "gif" && AUDIO_EXTENSIONS.contains(&ext.as_str()))
            }
    })
    .context("Failed to collect input files")?;

    if files.is_empty() {
        println!("No supported files found.");
        return Ok(());
    }

    println!("Converting {} file(s) to {}...", files.len(), target_ext);

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let report = Mutex::new(Report::new());

    // 0 = rayon default (one thread per core)
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs.unwrap_or(0)).build()?;

    // Mirrors the input tree under the output dir, same as compress
    let outputs: Vec<PathBuf> = files
        .iter()
        .map(|f| resolve_output(f, input, output).with_extension(&target_ext))
        .collect();
    let mut output_uses: HashMap<&PathBuf, usize> = HashMap::new();
    for out in &outputs {
        *output_uses.entry(out).or_default() += 1;
    }

    pool.install(|| files.par_iter().zip(&outputs).for_each(|(input_path, output_path)| {
        let result = (|| -> std::result::Result<FileResult, anyhow::Error> {
            // clip.mkv + clip.mp3 -> clip.mp4 would race on (or overwrite a source) one file
            if output_uses[output_path] > 1 {
                anyhow::bail!(
                    "several inputs convert to {} - convert them separately",
                    output_path.display()
                );
            }

            // Already in the target format (no output given) — nothing to do
            if output_path == input_path {
                return Ok(FileResult {
                    path: input_path.clone(),
                    original_size: 0,
                    compressed_size: 0,
                    skipped: true,
                    error: None,
                });
            }

            let original_size = std::fs::metadata(input_path)?.len();

            if config.backup && output_path.exists() {
                create_backup(output_path)?;
            }
            match image_format {
                Some(format) => write_file(output_path, &convert_image(&read_file(input_path)?, format, config)?)?,
                None => convert_media(input_path, output_path, config)?,
            }
            let converted_size = std::fs::metadata(output_path)?.len();

            Ok(FileResult {
                path: input_path.clone(),
                original_size,
                compressed_size: converted_size,
                skipped: false,
                error: None,
            })
        })();

        match result {
            Ok(file_result) => {
                pb.set_message(format!(
                    "{} → {}",
                    input_path.file_name().unwrap().to_string_lossy(),
                    target_ext
                ));
                report.lock().unwrap().add(file_result);
            }
            Err(e) => {
                log::error!("Error converting {}: {}", input_path.display(), e);
                report.lock().unwrap().add(FileResult {
                    path: input_path.clone(),
                    original_size: 0,
                    compressed_size: 0,
                    skipped: false,
                    error: Some(e.to_string()),
                });
            }
        }

        pb.inc(1);
    }));

    pb.finish_with_message("Done!");
    report.lock().unwrap().print_summary();

    Ok(())
}

fn handle_inspect(input: &Path, recursive: bool) -> Result<()> {
    let files = collect_files(input, recursive)
        .context("Failed to collect input files")?;

    if files.is_empty() {
        println!("No supported files found.");
        return Ok(());
    }

    for file_path in &files {
        println!("\nFile: {}", file_path.display());
        let data = read_file(file_path)?;

        match ImageFormat::from_path(file_path) {
            Some(ImageFormat::Mp3) => {
                inspect_mp3(&data)?;
            }
            Some(ImageFormat::Png) => {
                inspect_png(&data)?;
            }
            Some(ImageFormat::Jpg) => {
                inspect_jpg(&data)?;
            }
            Some(ImageFormat::Webp) => {
                inspect_webp(&data)?;
            }
            Some(ImageFormat::Mp4) => {
                inspect_mp4(&data)?;
            }
            Some(ImageFormat::Wav) => {
                inspect_wav(&data)?;
            }
            None => {
                println!("  Unsupported file format");
            }
        }
    }

    Ok(())
}

fn handle_extract(input: &Path, output: &Path, fps: f32, recursive: bool) -> Result<()> {
    // Collect MP4 files
    let mp4_files = if input.is_file() {
        if !matches!(ImageFormat::from_path(input), Some(ImageFormat::Mp4)) {
            anyhow::bail!("Frame extraction only supports MP4 files");
        }
        vec![input.to_path_buf()]
    } else if input.is_dir() {
        let files = collect_files(input, recursive)
            .context("Failed to collect input files")?;
        let mp4s: Vec<_> = files
            .into_iter()
            .filter(|f| matches!(ImageFormat::from_path(f), Some(ImageFormat::Mp4)))
            .collect();
        if mp4s.is_empty() {
            println!("No MP4 files found.");
            return Ok(());
        }
        mp4s
    } else {
        anyhow::bail!("Input path does not exist: {}", input.display());
    };

    let total = mp4_files.len();
    let mut total_frames = 0usize;
    let mut errors = 0usize;

    for (i, mp4_path) in mp4_files.iter().enumerate() {
        println!(
            "[{}/{}] Extracting frames from {} at {} fps...",
            i + 1,
            total,
            mp4_path.display(),
            fps
        );
        match extract_frames_to_png(mp4_path, output, fps) {
            Ok(count) => {
                println!("  ✓ Extracted {} frames", count);
                total_frames += count;
            }
            Err(e) => {
                log::error!("Error extracting {}: {}", mp4_path.display(), e);
                eprintln!("  ✗ Error: {}", e);
                errors += 1;
            }
        }
    }

    println!("--- Summary ---");
    println!(
        "Videos: {} | Frames extracted: {} | Errors: {}",
        total, total_frames, errors
    );

    if errors > 0 && errors == total {
        anyhow::bail!("All extractions failed");
    }

    Ok(())
}
