use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use image_preparer::cli::{Cli, Command};
use image_preparer::io::{collect_files, create_backup, read_file, resolve_output, write_file};
use image_preparer::report::{FileResult, Report};
use image_preparer_core::config::{ProcessingConfig, StripMode};
use image_preparer_core::converter::{ConvertFormat, convert_image};
use image_preparer_core::format::ImageFormat;
use image_preparer_core::pipeline::Pipeline;
use image_preparer_core::processor::png::{PngProcessor, inspect_png};
use image_preparer_core::processor::jpg::{JpgProcessor, inspect_jpg};
use image_preparer_core::processor::mp3::{Mp3Processor, inspect_mp3};
use image_preparer_core::processor::wav::{WavProcessor, inspect_wav};
use image_preparer_core::processor::webp::{WebpProcessor, inspect_webp};
use image_preparer_core::processor::mp4::{Mp4Processor, inspect_mp4, extract_frames_to_png};

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
            recursive,
            backup,
        } => {
            let config = ProcessingConfig {
                quality: *quality,
                speed: 3,
                no_lossy: *no_lossy,
                strip: StripMode::All,
                dry_run: false,
                backup: *backup,
                extract_frames: false,
                fps: 0.0,
                resize_width: None,
                resize_height: None,
            };
            handle_convert(input, output.as_deref(), to, *recursive, &config)
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
    config: &ProcessingConfig,
) -> Result<()> {
    let target_format = ConvertFormat::from_str(target_format_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid target format: {}. Use: png, jpg, jpeg, webp", target_format_str))?;

    let files = collect_files(input, recursive)
        .context("Failed to collect input files")?;

    if files.is_empty() {
        println!("No supported files found.");
        return Ok(());
    }

    println!("Converting {} file(s) to {}...", files.len(), target_format.as_str());

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let report = Mutex::new(Report::new());

    files.par_iter().for_each(|input_path| {
        let result = (|| -> std::result::Result<FileResult, anyhow::Error> {
            let data = read_file(input_path)?;
            let original_size = data.len() as u64;

            let converted = convert_image(&data, target_format, config)?;
            let converted_size = converted.len() as u64;

            // Determine output path with new extension
            let output_path = if let Some(output_dir) = output {
                if output_dir.is_dir() {
                    let file_name = input_path.file_stem().unwrap();
                    output_dir.join(format!("{}.{}", file_name.to_string_lossy(), target_format.extension()))
                } else {
                    output_dir.to_path_buf()
                }
            } else {
                input_path.with_extension(target_format.extension())
            };

            if config.backup && output_path.exists() {
                create_backup(&output_path)?;
            }
            write_file(&output_path, &converted)?;

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
                    target_format.as_str()
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
    });

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
