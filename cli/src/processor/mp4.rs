use std::io::Cursor;
use std::process::Command;

use crate::config::{ProcessingConfig, StripMode};
use crate::error::ProcessingError;
use crate::format::ImageFormat;
use crate::processor::ImageProcessor;

pub struct Mp4Processor;

/// Extract frames from MP4 video to PNG images
pub fn extract_frames_to_png(
    input_path: &std::path::Path,
    output_dir: &std::path::Path,
    fps: f32,
) -> Result<usize, ProcessingError> {
    use std::fs;

    if !is_ffmpeg_available() {
        return Err(ProcessingError::Encode(
            "ffmpeg not found - frame extraction requires ffmpeg".to_string(),
        ));
    }

    // Create output directory for frames
    let video_name = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let frames_dir = output_dir.join(format!("{}_frames", video_name));

    fs::create_dir_all(&frames_dir)
        .map_err(|e| ProcessingError::Encode(format!("Failed to create frames directory: {}", e)))?;

    // Build ffmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(input_path);
    cmd.arg("-y"); // Overwrite output files

    // Frame extraction filter
    if fps > 0.0 {
        // Extract N frames per second
        cmd.arg("-vf").arg(format!("fps={}", fps));
    }
    // If fps == 0, extract all frames (no filter)

    // Output format
    let output_pattern = frames_dir.join("frame_%04d.png");
    cmd.arg(output_pattern);

    // Execute ffmpeg
    log::debug!("Extracting frames: ffmpeg {:?}", cmd.get_args().collect::<Vec<_>>());

    let output = cmd
        .output()
        .map_err(|e| ProcessingError::Encode(format!("Failed to execute ffmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("ffmpeg failed: {}", stderr);
        return Err(ProcessingError::Encode(format!("ffmpeg failed: {}", stderr)));
    }

    // Count extracted frames
    let frame_count = fs::read_dir(&frames_dir)
        .map_err(|e| ProcessingError::Encode(format!("Failed to read frames directory: {}", e)))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "png")
                .unwrap_or(false)
        })
        .count();

    log::info!(
        "Extracted {} frames to {}",
        frame_count,
        frames_dir.display()
    );

    Ok(frame_count)
}

/// Display all metadata from an MP4 file
pub fn inspect_mp4(input: &[u8]) -> Result<(), ProcessingError> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("                 MP4 Metadata Inspection");
    println!("═══════════════════════════════════════════════════════\n");

    let file_size = input.len();
    println!("File size: {} bytes ({:.2} MB)\n", file_size, file_size as f64 / 1024.0 / 1024.0);

    let mut reader = Cursor::new(input);

    match mp4::Mp4Reader::read_header(&mut reader, input.len() as u64) {
        Ok(mp4) => {
            // File type info
            println!("File Type:");
            println!("───────────────────────────────────────────────────────");
            println!("  Major brand: {}", mp4.ftyp.major_brand);
            println!("  Minor version: {}", mp4.ftyp.minor_version);
            println!("  Compatible brands: {:?}\n", mp4.ftyp.compatible_brands);

            // Movie header info
            println!("Movie Header:");
            println!("───────────────────────────────────────────────────────");
            println!("  Duration: {:.2} seconds", mp4.duration().as_secs_f64());
            println!("  Timescale: {}", mp4.timescale());
            println!("  Fragmented: {}", mp4.is_fragmented());
            println!("  Tracks: {}\n", mp4.tracks().len());

            // Tracks info
            println!("Tracks:");
            println!("───────────────────────────────────────────────────────");
            for track in mp4.tracks().values() {
                println!("  Track #{}", track.track_id());

                if let Ok(track_type) = track.track_type() {
                    println!("      Type: {:?}", track_type);
                    println!("      Codec: {:?}", track.media_type());
                    println!("      Duration: {:.2}s", track.duration().as_secs_f64());

                    let bitrate = track.bitrate();
                    println!("      Bitrate: {} kbps", bitrate / 1000);

                    if track_type == mp4::TrackType::Video {
                        println!("      Width: {}", track.width());
                        println!("      Height: {}", track.height());
                        let fps = track.frame_rate();
                        println!("      Frame rate: {:.2} fps", fps);
                    } else if track_type == mp4::TrackType::Audio {
                        // Audio-specific info
                        if let Ok(config) = track.channel_config() {
                            println!("      Channel config: {:?}", config);
                        }
                    }
                }

                println!();
            }

            // Metadata
            println!("Metadata:");
            println!("───────────────────────────────────────────────────────");
            println!("  Note: Detailed metadata inspection requires manual box parsing");
            println!("  The file may contain user data (udta) and metadata (meta) boxes\n");

            // File structure
            println!("File Structure:");
            println!("───────────────────────────────────────────────────────");
            println!("  Fast start optimized: {}",
                     check_fast_start(input).unwrap_or(false));

        }
        Err(e) => {
            println!("Could not parse MP4 file: {}", e);
        }
    }

    println!("\n═══════════════════════════════════════════════════════\n");

    Ok(())
}

/// Check if MP4 has moov box before mdat (fast start)
fn check_fast_start(input: &[u8]) -> Result<bool, ProcessingError> {
    let mut pos = 0usize;
    let mut found_moov = false;
    let mut found_mdat = false;

    while pos + 8 <= input.len() {
        let size = u32::from_be_bytes([input[pos], input[pos + 1], input[pos + 2], input[pos + 3]]) as usize;
        let box_type = &input[pos + 4..pos + 8];

        if size < 8 {
            break;
        }

        match box_type {
            b"moov" => {
                if found_mdat {
                    return Ok(false);
                }
                found_moov = true;
            }
            b"mdat" => {
                if found_moov {
                    return Ok(true);
                }
                found_mdat = true;
            }
            _ => {}
        }

        pos += size;
        if pos > input.len() {
            break;
        }
    }

    Ok(found_moov && !found_mdat)
}

impl ImageProcessor for Mp4Processor {
    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Mp4]
    }

    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
        // Parse MP4 to validate
        let mut reader = Cursor::new(input);
        let mp4 = mp4::Mp4Reader::read_header(&mut reader, input.len() as u64)
            .map_err(|e| ProcessingError::Decode(e.to_string()))?;

        log::debug!("Processing MP4: {} tracks, {:.2}s duration",
                   mp4.tracks().len(),
                   mp4.duration().as_secs_f64());

        // Check if ffmpeg is available
        if !is_ffmpeg_available() {
            log::warn!("ffmpeg not found - MP4 compression requires ffmpeg to be installed");
            log::warn!("Install: brew install ffmpeg (macOS) or apt install ffmpeg (Linux)");
            return Ok(input.to_vec());
        }

        if config.no_lossy {
            // Lossless mode: only strip metadata using ffmpeg
            log::debug!("MP4 lossless mode: stripping metadata only");
            compress_mp4_with_ffmpeg(input, config, true)
        } else {
            // Lossy mode: re-encode with compression
            log::debug!("MP4 lossy mode: re-encoding with quality {}", config.quality);
            compress_mp4_with_ffmpeg(input, config, false)
        }
    }
}

/// Check if ffmpeg is available in the system
pub(crate) fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Compress MP4 using ffmpeg
fn compress_mp4_with_ffmpeg(input: &[u8], config: &ProcessingConfig, lossless: bool) -> Result<Vec<u8>, ProcessingError> {
    use std::io::Write;

    // Create temporary files with unique names per invocation
    let temp_dir = std::env::temp_dir();
    let unique_id = format!("{}_{:?}", std::process::id(), std::thread::current().id());
    let input_path = temp_dir.join(format!("input_{}.mp4", unique_id));
    let output_path = temp_dir.join(format!("output_{}.mp4", unique_id));

    // Write input to temp file
    let mut input_file = std::fs::File::create(&input_path)
        .map_err(|e| ProcessingError::Encode(format!("Failed to create temp input: {}", e)))?;
    input_file.write_all(input)
        .map_err(|e| ProcessingError::Encode(format!("Failed to write temp input: {}", e)))?;
    drop(input_file);

    // Build ffmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(&input_path);
    cmd.arg("-y"); // Overwrite output file

    if lossless {
        // Lossless: copy video/audio streams, only strip metadata
        log::debug!("Using ffmpeg copy mode (no re-encoding)");
        cmd.arg("-c:v").arg("copy");
        cmd.arg("-c:a").arg("copy");

        // Strip metadata based on config
        match config.strip {
            StripMode::All | StripMode::Safe => {
                cmd.arg("-map_metadata").arg("-1"); // Remove all metadata
            }
            StripMode::None => {
                // Keep metadata
            }
        }

        // Fast start
        cmd.arg("-movflags").arg("+faststart");
    } else {
        // Lossy: re-encode with compression
        // Map quality (0-100) to CRF (0-51, lower is better)
        // quality 100 -> CRF 18 (very high quality)
        // quality 80 -> CRF 23 (good quality, default)
        // quality 50 -> CRF 28 (medium quality)
        // quality 0 -> CRF 35 (low quality)
        let crf = ((100 - config.quality) as f32 * 0.33 + 18.0) as u32;
        let crf = crf.min(35).max(18);

        log::debug!("Using ffmpeg with CRF {} (quality {})", crf, config.quality);

        // Video encoding
        cmd.arg("-c:v").arg("libx264");
        cmd.arg("-crf").arg(crf.to_string());

        // Map speed (1-10) to preset
        // speed 1 (slowest) -> veryslow
        // speed 3 (default) -> medium
        // speed 10 (fastest) -> ultrafast
        let preset = match config.speed {
            1 => "veryslow",
            2 => "slow",
            3 | 4 => "medium",
            5 | 6 => "fast",
            7 | 8 => "faster",
            _ => "ultrafast",
        };
        cmd.arg("-preset").arg(preset);

        // Audio encoding
        cmd.arg("-c:a").arg("aac");
        cmd.arg("-b:a").arg("128k");

        // Strip metadata
        if config.strip != StripMode::None {
            cmd.arg("-map_metadata").arg("-1");
        }

        // Fast start
        cmd.arg("-movflags").arg("+faststart");
    }

    cmd.arg(&output_path);

    // Execute ffmpeg
    log::debug!("Executing: ffmpeg {:?}", cmd.get_args().collect::<Vec<_>>());

    let output = cmd.output()
        .map_err(|e| ProcessingError::Encode(format!("Failed to execute ffmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("ffmpeg failed: {}", stderr);

        // Cleanup temp files
        let _ = std::fs::remove_file(&input_path);
        let _ = std::fs::remove_file(&output_path);

        return Err(ProcessingError::Encode(format!("ffmpeg failed: {}", stderr)));
    }

    // Read output
    let result = std::fs::read(&output_path)
        .map_err(|e| ProcessingError::Encode(format!("Failed to read ffmpeg output: {}", e)))?;

    // Cleanup temp files
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    log::debug!("ffmpeg completed: {} -> {} bytes ({:.1}% reduction)",
               input.len(),
               result.len(),
               (1.0 - result.len() as f64 / input.len() as f64) * 100.0);

    Ok(result)
}
