use image::GenericImageView;

use crate::config::{ProcessingConfig, StripMode};
use crate::error::ProcessingError;
use crate::format::ImageFormat;
use crate::processor::{ImageProcessor, maybe_resize};

pub struct PngProcessor;

/// Display all metadata from a PNG file
pub fn inspect_png(input: &[u8]) -> Result<(), ProcessingError> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("                  PNG Metadata Inspection");
    println!("═══════════════════════════════════════════════════════\n");

    let file_size = input.len();
    println!("File size: {} bytes ({:.2} KB)\n", file_size, file_size as f64 / 1024.0);

    // Load image to get dimensions and color info
    match image::load_from_memory_with_format(input, image::ImageFormat::Png) {
        Ok(img) => {
            let (width, height) = img.dimensions();
            let color_type = img.color();
            println!("Image dimensions: {} x {} pixels", width, height);
            println!("Color type: {:?}", color_type);
            println!("Total pixels: {}\n", width * height);
        }
        Err(e) => {
            println!("Could not decode PNG image: {}\n", e);
        }
    }

    // Parse PNG chunks
    println!("PNG Chunks:");
    println!("───────────────────────────────────────────────────────");

    if input.len() < 8 || &input[0..8] != b"\x89PNG\r\n\x1a\n" {
        println!("  Invalid PNG signature");
        println!("\n═══════════════════════════════════════════════════════\n");
        return Ok(());
    }

    let mut pos = 8;
    let mut chunk_count = 0;
    let mut critical_chunks = 0;
    let mut ancillary_chunks = 0;

    while pos + 8 <= input.len() {
        let length = u32::from_be_bytes([input[pos], input[pos + 1], input[pos + 2], input[pos + 3]]) as usize;
        let chunk_type = &input[pos + 4..pos + 8];

        if let Ok(chunk_name) = std::str::from_utf8(chunk_type) {
            chunk_count += 1;

            // Check if critical or ancillary
            let is_critical = chunk_type[0] & 0x20 == 0;
            if is_critical {
                critical_chunks += 1;
            } else {
                ancillary_chunks += 1;
            }

            let chunk_info = get_chunk_info(chunk_name);
            let criticality = if is_critical { "[CRITICAL]" } else { "[ANCILLARY]" };

            println!("  {} {} - {}", criticality, chunk_name, chunk_info);
            println!("      Size: {} bytes", length);

            // Display some chunk contents
            if pos + 8 + length <= input.len() {
                display_chunk_content(chunk_name, &input[pos + 8..pos + 8 + length]);
            }

            println!();
        }

        // Move to next chunk: length (4) + type (4) + data (length) + crc (4)
        pos += 12 + length;

        if pos > input.len() {
            break;
        }
    }

    println!("───────────────────────────────────────────────────────");
    println!("Summary: {} total chunks ({} critical, {} ancillary)",
             chunk_count, critical_chunks, ancillary_chunks);
    println!("\n═══════════════════════════════════════════════════════\n");

    Ok(())
}

/// Get human-readable chunk information
fn get_chunk_info(chunk_type: &str) -> &str {
    match chunk_type {
        "IHDR" => "Image Header",
        "PLTE" => "Palette",
        "IDAT" => "Image Data",
        "IEND" => "Image End",
        "tRNS" => "Transparency",
        "gAMA" => "Gamma",
        "cHRM" => "Chromaticity",
        "sRGB" => "Standard RGB Color Space",
        "iCCP" => "ICC Color Profile",
        "tEXt" => "Textual Data",
        "zTXt" => "Compressed Textual Data",
        "iTXt" => "International Textual Data",
        "bKGD" => "Background Color",
        "pHYs" => "Physical Pixel Dimensions",
        "tIME" => "Last Modification Time",
        "sBIT" => "Significant Bits",
        "sPLT" => "Suggested Palette",
        "hIST" => "Histogram",
        "eXIf" => "EXIF Data",
        _ => "Unknown/Custom Chunk",
    }
}

/// Display relevant chunk content
fn display_chunk_content(chunk_type: &str, data: &[u8]) {
    match chunk_type {
        "IHDR" => {
            if data.len() >= 13 {
                let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                let bit_depth = data[8];
                let color_type = data[9];
                println!("      {}x{}, bit depth: {}, color type: {}",
                         width, height, bit_depth, color_type);
            }
        }
        "tEXt" | "zTXt" | "iTXt" => {
            if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                let keyword = String::from_utf8_lossy(&data[..null_pos]);
                let value_str = if chunk_type == "tEXt" && null_pos + 1 < data.len() {
                    String::from_utf8_lossy(&data[null_pos + 1..]).to_string()
                } else {
                    String::from("<compressed or binary>")
                };
                println!("      {}: {}", keyword,
                         if value_str.len() > 60 {
                             format!("{}...", &value_str[..60])
                         } else {
                             value_str
                         });
            }
        }
        "pHYs" => {
            if data.len() >= 9 {
                let x = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                let y = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                let unit = data[8];
                println!("      {}x{} pixels per {}", x, y,
                         if unit == 1 { "meter" } else { "unit" });
            }
        }
        "tIME" => {
            if data.len() >= 7 {
                let year = u16::from_be_bytes([data[0], data[1]]);
                let month = data[2];
                let day = data[3];
                let hour = data[4];
                let minute = data[5];
                let second = data[6];
                println!("      {}-{:02}-{:02} {:02}:{:02}:{:02}",
                         year, month, day, hour, minute, second);
            }
        }
        "gAMA" => {
            if data.len() >= 4 {
                let gamma = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                println!("      Gamma: {:.5}", gamma as f64 / 100000.0);
            }
        }
        _ => {}
    }
}

impl ImageProcessor for PngProcessor {
    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Png]
    }

    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
        let needs_resize = config.resize_width.is_some() || config.resize_height.is_some();

        if config.no_lossy {
            if needs_resize {
                let resized = resize_png(input, config)?;
                optimize_lossless(&resized, config)
            } else {
                optimize_lossless(input, config)
            }
        } else {
            let quantized = quantize_png(input, config)?;
            optimize_lossless(&quantized, config)
        }
    }
}

/// Resize PNG without lossy quantization — decode, resize, re-encode as truecolor PNG
fn resize_png(input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
    let img = image::load_from_memory_with_format(input, image::ImageFormat::Png)
        .map_err(|e| ProcessingError::Decode(e.to_string()))?;

    let img = maybe_resize(img, config);

    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| ProcessingError::Encode(e.to_string()))?;

    Ok(buf.into_inner())
}

/// Decode PNG -> quantize colors -> encode as indexed palette PNG
fn quantize_png(input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
    // Step 1: Decode to RGBA pixels
    let img = image::load_from_memory_with_format(input, image::ImageFormat::Png)
        .map_err(|e| ProcessingError::Decode(e.to_string()))?;

    // Resize if requested
    let img = maybe_resize(img, config);

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
    let raw_pixels = rgba.as_raw();

    // Convert &[u8] to &[imagequant::RGBA]
    let pixels: &[imagequant::RGBA] = unsafe {
        std::slice::from_raw_parts(
            raw_pixels.as_ptr() as *const imagequant::RGBA,
            (width * height) as usize,
        )
    };

    // Step 2: Quantize with imagequant
    let mut attr = imagequant::new();
    attr.set_quality(0, config.quality)
        .map_err(|e| ProcessingError::Quantize(e.to_string()))?;
    attr.set_speed(config.speed)
        .map_err(|e| ProcessingError::Quantize(e.to_string()))?;

    let mut image = attr
        .new_image_borrowed(pixels, width as usize, height as usize, 0.0)
        .map_err(|e| ProcessingError::Quantize(e.to_string()))?;

    let mut quantization = attr
        .quantize(&mut image)
        .map_err(|e| ProcessingError::Quantize(e.to_string()))?;

    let (palette, indices) = quantization
        .remapped(&mut image)
        .map_err(|e| ProcessingError::Quantize(e.to_string()))?;

    // Step 3: Encode as indexed PNG with lodepng
    let lodepng_palette: Vec<lodepng::RGBA> = palette
        .iter()
        .map(|c| lodepng::RGBA {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        })
        .collect();

    let mut encoder = lodepng::Encoder::new();
    encoder.set_auto_convert(false);
    encoder
        .set_palette(&lodepng_palette)
        .map_err(|e| ProcessingError::Encode(e.to_string()))?;

    {
        let raw = encoder.info_raw_mut();
        raw.set_colortype(lodepng::ColorType::PALETTE);
        raw.set_bitdepth(8);
        raw.palette_clear();
        for &color in &lodepng_palette {
            raw.palette_add(color)
                .map_err(|e| ProcessingError::Encode(e.to_string()))?;
        }
    }

    let png_data = encoder
        .encode(&indices, width as usize, height as usize)
        .map_err(|e| ProcessingError::Encode(e.to_string()))?;

    Ok(png_data)
}

/// Lossless DEFLATE re-compression + metadata stripping via oxipng
fn optimize_lossless(png_data: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
    let mut opts = oxipng::Options::from_preset(4);

    opts.strip = match config.strip {
        StripMode::All => oxipng::StripChunks::All,
        StripMode::Safe => oxipng::StripChunks::Safe,
        StripMode::None => oxipng::StripChunks::None,
    };

    oxipng::optimize_from_memory(png_data, &opts)
        .map_err(|e| ProcessingError::Optimize(e.to_string()))
}
