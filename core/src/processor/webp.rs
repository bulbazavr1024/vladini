use image::GenericImageView;

use crate::config::{ProcessingConfig, StripMode};
use crate::error::ProcessingError;
use crate::format::ImageFormat;
use crate::processor::{ImageProcessor, maybe_resize};

pub struct WebpProcessor;

/// Display all metadata from a WebP file
pub fn inspect_webp(input: &[u8]) -> Result<(), ProcessingError> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("                 WebP Metadata Inspection");
    println!("═══════════════════════════════════════════════════════\n");

    let file_size = input.len();
    println!("File size: {} bytes ({:.2} KB)\n", file_size, file_size as f64 / 1024.0);

    // Decode WebP to get image info
    match webp::Decoder::new(input).decode() {
        Some(decoded) => {
            let (width, height) = (decoded.width(), decoded.height());
            println!("Image dimensions: {} x {} pixels", width, height);
            println!("Total pixels: {}\n", width * height);
        }
        None => {
            println!("Could not decode WebP image\n");
        }
    }

    // Parse WebP structure (RIFF container)
    if input.len() < 12 {
        println!("File too small to be a valid WebP");
        println!("\n═══════════════════════════════════════════════════════\n");
        return Ok(());
    }

    if &input[0..4] != b"RIFF" || &input[8..12] != b"WEBP" {
        println!("Invalid WebP signature");
        println!("\n═══════════════════════════════════════════════════════\n");
        return Ok(());
    }

    let file_size_riff = u32::from_le_bytes([input[4], input[5], input[6], input[7]]);
    println!("RIFF container size: {} bytes\n", file_size_riff);

    println!("WebP Chunks:");
    println!("───────────────────────────────────────────────────────");

    let mut pos = 12;
    let mut chunk_count = 0;

    while pos + 8 <= input.len() {
        let chunk_type = &input[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            input[pos + 4],
            input[pos + 5],
            input[pos + 6],
            input[pos + 7],
        ]) as usize;

        if let Ok(chunk_name) = std::str::from_utf8(chunk_type) {
            chunk_count += 1;
            let chunk_info = get_webp_chunk_info(chunk_name);

            println!("  {} - {}", chunk_name, chunk_info);
            println!("      Size: {} bytes", chunk_size);

            // Display some chunk contents
            if pos + 8 + chunk_size <= input.len() {
                display_webp_chunk_content(chunk_name, &input[pos + 8..pos + 8 + chunk_size]);
            }

            println!();
        }

        // WebP chunks are padded to even size
        let padded_size = (chunk_size + 1) & !1;
        pos += 8 + padded_size;

        if pos > input.len() {
            break;
        }
    }

    println!("───────────────────────────────────────────────────────");
    println!("Summary: {} total chunks", chunk_count);
    println!("\n═══════════════════════════════════════════════════════\n");

    Ok(())
}

/// Get human-readable chunk information
fn get_webp_chunk_info(chunk_type: &str) -> &str {
    match chunk_type {
        "VP8 " => "Lossy VP8 bitstream",
        "VP8L" => "Lossless VP8L bitstream",
        "VP8X" => "Extended file format",
        "ANIM" => "Animation parameters",
        "ANMF" => "Animation frame",
        "ALPH" => "Alpha channel",
        "ICCP" => "ICC Color Profile",
        "EXIF" => "EXIF metadata",
        "XMP " => "XMP metadata",
        _ => "Unknown chunk",
    }
}

/// Display relevant chunk content
fn display_webp_chunk_content(chunk_type: &str, data: &[u8]) {
    match chunk_type {
        "VP8X" => {
            if data.len() >= 10 {
                let flags = data[0];
                let has_icc = flags & 0x20 != 0;
                let has_alpha = flags & 0x10 != 0;
                let has_exif = flags & 0x08 != 0;
                let has_xmp = flags & 0x04 != 0;
                let has_anim = flags & 0x02 != 0;

                let width = u32::from_le_bytes([data[4], data[5], data[6], 0]) + 1;
                let height = u32::from_le_bytes([data[7], data[8], data[9], 0]) + 1;

                println!("      Canvas: {}x{}", width, height);
                println!("      Has ICC: {}, Alpha: {}, EXIF: {}, XMP: {}, Animation: {}",
                         has_icc, has_alpha, has_exif, has_xmp, has_anim);
            }
        }
        "VP8 " => {
            if data.len() >= 10 {
                // VP8 frame tag
                let frame_tag = data[0] as u32
                    | ((data[1] as u32) << 8)
                    | ((data[2] as u32) << 16);
                let key_frame = (frame_tag & 1) == 0;
                let version = (frame_tag >> 1) & 7;
                let show_frame = (frame_tag >> 4) & 1 == 1;

                println!("      Key frame: {}, Version: {}, Show: {}",
                         key_frame, version, show_frame);

                if data.len() >= 10 && data[3] == 0x9d && data[4] == 0x01 && data[5] == 0x2a {
                    let width = ((data[7] as u16) << 8) | (data[6] as u16);
                    let height = ((data[9] as u16) << 8) | (data[8] as u16);
                    println!("      Dimensions: {}x{}", width & 0x3fff, height & 0x3fff);
                }
            }
        }
        "EXIF" => {
            println!("      Contains EXIF metadata ({} bytes)", data.len());
        }
        "XMP " => {
            println!("      Contains XMP metadata ({} bytes)", data.len());
        }
        "ICCP" => {
            println!("      Contains ICC color profile ({} bytes)", data.len());
        }
        _ => {}
    }
}

impl ImageProcessor for WebpProcessor {
    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Webp]
    }

    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
        // Decode WebP
        let img = image::load_from_memory_with_format(input, image::ImageFormat::WebP)
            .map_err(|e| ProcessingError::Decode(e.to_string()))?;

        // Resize if requested
        let img = maybe_resize(img, config);

        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();

        // Encode with WebP
        let encoder = if config.no_lossy {
            // Lossless mode
            webp::Encoder::from_rgba(rgba.as_raw(), width, height)
        } else {
            // Lossy mode with quality setting
            webp::Encoder::from_rgba(rgba.as_raw(), width, height)
        };

        let encoded = if config.no_lossy {
            encoder.encode_lossless()
        } else {
            // Map quality 0-100 to WebP quality (0-100)
            encoder.encode(config.quality as f32)
        };

        let mut output = encoded.to_vec();

        // Strip metadata if requested
        if config.strip != StripMode::None {
            output = strip_webp_metadata(&output, config.strip)?;
        }

        Ok(output)
    }
}

/// Strip metadata chunks from WebP file
fn strip_webp_metadata(input: &[u8], strip_mode: StripMode) -> Result<Vec<u8>, ProcessingError> {
    if input.len() < 12 {
        return Ok(input.to_vec());
    }

    if &input[0..4] != b"RIFF" || &input[8..12] != b"WEBP" {
        return Ok(input.to_vec());
    }

    let mut output = Vec::new();

    // Copy RIFF header (we'll update size later)
    output.extend_from_slice(&input[0..12]);

    let mut pos = 12;
    let mut kept_size = 0u32;

    while pos + 8 <= input.len() {
        let chunk_type = &input[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            input[pos + 4],
            input[pos + 5],
            input[pos + 6],
            input[pos + 7],
        ]) as usize;

        let padded_size = (chunk_size + 1) & !1;

        if pos + 8 + chunk_size > input.len() {
            break;
        }

        let chunk_name = std::str::from_utf8(chunk_type).unwrap_or("");
        let should_keep = match strip_mode {
            StripMode::None => true,
            StripMode::Safe => {
                // Keep only essential chunks: VP8, VP8L, VP8X, ALPH, ANIM, ANMF
                matches!(chunk_name, "VP8 " | "VP8L" | "VP8X" | "ALPH" | "ANIM" | "ANMF")
            }
            StripMode::All => {
                // Keep only image data chunks
                matches!(chunk_name, "VP8 " | "VP8L" | "ALPH")
            }
        };

        if should_keep {
            // Copy chunk header and data
            output.extend_from_slice(&input[pos..pos + 8 + padded_size]);
            kept_size += 8 + padded_size as u32;
        } else {
            log::debug!("Stripping WebP chunk: {}", chunk_name);
        }

        pos += 8 + padded_size;
    }

    // Update RIFF size (total file size - 8)
    let total_size = 4 + kept_size; // "WEBP" fourcc + chunks
    output[4..8].copy_from_slice(&total_size.to_le_bytes());

    Ok(output)
}
