use std::io::Cursor;

use image::GenericImageView;
use image::codecs::jpeg::JpegEncoder;

use crate::config::ProcessingConfig;
use crate::error::ProcessingError;
use crate::format::ImageFormat;
use crate::processor::{ImageProcessor, maybe_resize};

pub struct JpgProcessor;

impl ImageProcessor for JpgProcessor {
    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Jpg]
    }

    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
        let img = image::load_from_memory_with_format(input, image::ImageFormat::Jpeg)
            .map_err(|e| ProcessingError::Decode(e.to_string()))?;

        // Resize if requested
        let img = maybe_resize(img, config);

        let rgb = img.to_rgb8();
        let (width, height) = img.dimensions();

        let quality = if config.no_lossy { 100 } else { config.quality };

        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);
        let mut encoder = JpegEncoder::new_with_quality(&mut cursor, quality);

        encoder
            .encode(rgb.as_raw(), width, height, image::ExtendedColorType::Rgb8)
            .map_err(|e| ProcessingError::Encode(e.to_string()))?;

        Ok(output)
    }
}

/// Display metadata from a JPEG file
pub fn inspect_jpg(input: &[u8]) -> Result<(), ProcessingError> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("                 JPEG Metadata Inspection");
    println!("═══════════════════════════════════════════════════════\n");

    let file_size = input.len();
    println!("File size: {} bytes ({:.2} KB)\n", file_size, file_size as f64 / 1024.0);

    match image::load_from_memory_with_format(input, image::ImageFormat::Jpeg) {
        Ok(img) => {
            let (width, height) = img.dimensions();
            let color_type = img.color();
            println!("Image dimensions: {} x {} pixels", width, height);
            println!("Color type: {:?}", color_type);
            println!("Total pixels: {}\n", width * height);
        }
        Err(e) => {
            println!("Could not decode JPEG image: {}\n", e);
        }
    }

    // Parse JPEG segment markers
    println!("JPEG Segments:");
    println!("───────────────────────────────────────────────────────");

    if input.len() < 2 || input[0] != 0xFF || input[1] != 0xD8 {
        println!("  Invalid JPEG signature");
        println!("\n═══════════════════════════════════════════════════════\n");
        return Ok(());
    }

    let mut pos = 2;
    let mut segment_count = 0;

    while pos + 1 < input.len() {
        if input[pos] != 0xFF {
            pos += 1;
            continue;
        }

        let marker = input[pos + 1];

        // Skip padding bytes (0xFF followed by 0xFF)
        if marker == 0xFF {
            pos += 1;
            continue;
        }

        // Skip standalone markers (RST0-RST7, SOI, EOI)
        if marker == 0x00 {
            pos += 2;
            continue;
        }

        let (name, description) = marker_info(marker);
        segment_count += 1;

        // SOS (Start of Scan) — image data follows, stop parsing
        if marker == 0xDA {
            println!("  [0xFF{:02X}] {} - {}", marker, name, description);
            println!("      (image data follows)");
            break;
        }

        // EOI
        if marker == 0xD9 {
            println!("  [0xFF{:02X}] {} - {}", marker, name, description);
            break;
        }

        // Markers without length (RST0-RST7)
        if (0xD0..=0xD7).contains(&marker) {
            println!("  [0xFF{:02X}] {} - {}", marker, name, description);
            pos += 2;
            continue;
        }

        // Read segment length
        if pos + 3 >= input.len() {
            break;
        }
        let length = u16::from_be_bytes([input[pos + 2], input[pos + 3]]) as usize;

        println!("  [0xFF{:02X}] {} - {}", marker, name, description);
        println!("      Size: {} bytes", length);

        // Show EXIF identifier for APP1
        if marker == 0xE1 && length > 2 && pos + 4 + 6 <= input.len() {
            let id = &input[pos + 4..pos + 4 + 6.min(length - 2)];
            if id.starts_with(b"Exif\x00") {
                println!("      Contains: EXIF data");
            } else if id.starts_with(b"http:") {
                println!("      Contains: XMP data");
            }
        }

        // Show JFIF identifier for APP0
        if marker == 0xE0 && length > 2 && pos + 4 + 5 <= input.len() {
            let id = &input[pos + 4..pos + 4 + 5.min(length - 2)];
            if id.starts_with(b"JFIF\x00") {
                println!("      Contains: JFIF header");
            }
        }

        // Show comment content
        if marker == 0xFE && length > 2 {
            let end = (pos + 4 + length - 2).min(input.len());
            if let Ok(comment) = std::str::from_utf8(&input[pos + 4..end]) {
                let display = if comment.len() > 60 {
                    format!("{}...", &comment[..60])
                } else {
                    comment.to_string()
                };
                println!("      Comment: {}", display);
            }
        }

        println!();
        pos += 2 + length;
    }

    println!("───────────────────────────────────────────────────────");
    println!("Summary: {} segments found", segment_count);
    println!("\n═══════════════════════════════════════════════════════\n");

    Ok(())
}

fn marker_info(marker: u8) -> (&'static str, &'static str) {
    match marker {
        0xC0 => ("SOF0", "Baseline DCT"),
        0xC1 => ("SOF1", "Extended Sequential DCT"),
        0xC2 => ("SOF2", "Progressive DCT"),
        0xC3 => ("SOF3", "Lossless (Sequential)"),
        0xC4 => ("DHT", "Define Huffman Table"),
        0xC8 => ("JPG", "JPEG Extensions"),
        0xC9 => ("SOF9", "Extended Sequential DCT (Arithmetic)"),
        0xCA => ("SOF10", "Progressive DCT (Arithmetic)"),
        0xCB => ("SOF11", "Lossless (Arithmetic)"),
        0xCC => ("DAC", "Define Arithmetic Coding"),
        0xD0..=0xD7 => ("RST", "Restart Marker"),
        0xD8 => ("SOI", "Start of Image"),
        0xD9 => ("EOI", "End of Image"),
        0xDA => ("SOS", "Start of Scan"),
        0xDB => ("DQT", "Define Quantization Table"),
        0xDC => ("DNL", "Define Number of Lines"),
        0xDD => ("DRI", "Define Restart Interval"),
        0xDE => ("DHP", "Define Hierarchical Progression"),
        0xDF => ("EXP", "Expand Reference Component"),
        0xE0 => ("APP0", "Application Segment (JFIF)"),
        0xE1 => ("APP1", "Application Segment (EXIF/XMP)"),
        0xE2 => ("APP2", "Application Segment (ICC Profile)"),
        0xE3..=0xEF => ("APPn", "Application Segment"),
        0xFE => ("COM", "Comment"),
        _ => ("???", "Unknown Marker"),
    }
}
