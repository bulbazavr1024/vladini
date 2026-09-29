use crate::config::{ProcessingConfig, StripMode};
use crate::error::ProcessingError;
use crate::format::ImageFormat;
use crate::processor::ImageProcessor;

pub struct WavProcessor;

impl ImageProcessor for WavProcessor {
    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Wav]
    }

    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
        match config.strip {
            StripMode::None => return Ok(input.to_vec()),
            _ => {}
        }

        strip_wav_metadata(input, config)
    }
}

/// Rebuild WAV keeping only essential chunks based on strip mode.
fn strip_wav_metadata(input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
    if input.len() < 12 {
        return Err(ProcessingError::Decode("WAV file too small".into()));
    }

    if &input[0..4] != b"RIFF" || &input[8..12] != b"WAVE" {
        return Err(ProcessingError::Decode("Invalid WAV signature".into()));
    }

    let chunks = parse_riff_chunks(&input[12..])?;

    let mut output = Vec::with_capacity(input.len());
    // Placeholder RIFF header — will fix size at the end
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&[0u8; 4]); // placeholder
    output.extend_from_slice(b"WAVE");

    for chunk in &chunks {
        let keep = match config.strip {
            StripMode::All => is_essential_chunk(&chunk.id),
            StripMode::Safe => is_essential_chunk(&chunk.id) || is_safe_chunk(&chunk.id),
            StripMode::None => true,
        };

        if keep {
            output.extend_from_slice(&chunk.id);
            output.extend_from_slice(&(chunk.data.len() as u32).to_le_bytes());
            output.extend_from_slice(chunk.data);
            // RIFF chunks are word-aligned
            if chunk.data.len() % 2 != 0 {
                output.push(0);
            }
        }
    }

    // Fix RIFF size
    let riff_size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&riff_size.to_le_bytes());

    Ok(output)
}

struct RiffChunk<'a> {
    id: [u8; 4],
    data: &'a [u8],
}

fn parse_riff_chunks(data: &[u8]) -> Result<Vec<RiffChunk<'_>>, ProcessingError> {
    let mut chunks = Vec::new();
    let mut pos = 0;

    while pos + 8 <= data.len() {
        let mut id = [0u8; 4];
        id.copy_from_slice(&data[pos..pos + 4]);
        let size = u32::from_le_bytes([
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]) as usize;

        let chunk_end = pos + 8 + size;
        if chunk_end > data.len() {
            // Tolerate truncated final chunk
            let available = data.len() - (pos + 8);
            chunks.push(RiffChunk {
                id,
                data: &data[pos + 8..pos + 8 + available],
            });
            break;
        }

        chunks.push(RiffChunk {
            id,
            data: &data[pos + 8..chunk_end],
        });

        // Advance past chunk + word alignment padding
        pos = chunk_end;
        if pos % 2 != 0 {
            pos += 1;
        }
    }

    Ok(chunks)
}

/// Essential chunks that must always be kept
fn is_essential_chunk(id: &[u8; 4]) -> bool {
    matches!(id, b"fmt " | b"data" | b"fact")
}

/// Safe metadata chunks (non-sensitive)
fn is_safe_chunk(id: &[u8; 4]) -> bool {
    matches!(id, b"LIST" | b"cue " | b"smpl" | b"inst")
}

/// Display metadata from a WAV file
pub fn inspect_wav(input: &[u8]) -> Result<(), ProcessingError> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("                  WAV Metadata Inspection");
    println!("═══════════════════════════════════════════════════════\n");

    let file_size = input.len();
    println!("File size: {} bytes ({:.2} KB)\n", file_size, file_size as f64 / 1024.0);

    if input.len() < 12 || &input[0..4] != b"RIFF" || &input[8..12] != b"WAVE" {
        println!("  Invalid WAV file");
        println!("\n═══════════════════════════════════════════════════════\n");
        return Ok(());
    }

    let chunks = parse_riff_chunks(&input[12..])?;

    // Display fmt chunk info
    for chunk in &chunks {
        if &chunk.id == b"fmt " && chunk.data.len() >= 16 {
            let audio_format = u16::from_le_bytes([chunk.data[0], chunk.data[1]]);
            let channels = u16::from_le_bytes([chunk.data[2], chunk.data[3]]);
            let sample_rate = u32::from_le_bytes([
                chunk.data[4], chunk.data[5], chunk.data[6], chunk.data[7],
            ]);
            let byte_rate = u32::from_le_bytes([
                chunk.data[8], chunk.data[9], chunk.data[10], chunk.data[11],
            ]);
            let bits_per_sample = u16::from_le_bytes([chunk.data[14], chunk.data[15]]);

            let format_name = match audio_format {
                1 => "PCM (uncompressed)",
                3 => "IEEE Float",
                6 => "A-law",
                7 => "mu-law",
                0xFFFE => "Extensible",
                _ => "Unknown",
            };

            println!("Audio Format: {} ({})", format_name, audio_format);
            println!("Channels: {}", channels);
            println!("Sample Rate: {} Hz", sample_rate);
            println!("Byte Rate: {} bytes/sec", byte_rate);
            println!("Bits Per Sample: {}", bits_per_sample);
            println!("Bitrate: {} kbps", byte_rate * 8 / 1000);

            // Calculate duration from data chunk
            for data_chunk in &chunks {
                if &data_chunk.id == b"data" {
                    let data_size = data_chunk.data.len();
                    if byte_rate > 0 {
                        let duration_secs = data_size as f64 / byte_rate as f64;
                        let minutes = duration_secs as u64 / 60;
                        let seconds = duration_secs % 60.0;
                        println!("Duration: {}:{:05.2}", minutes, seconds);
                    }
                    println!("Audio Data Size: {} bytes ({:.2} MB)",
                        data_size, data_size as f64 / (1024.0 * 1024.0));
                    break;
                }
            }
            println!();
            break;
        }
    }

    // Display all chunks
    println!("RIFF Chunks:");
    println!("───────────────────────────────────────────────────────");

    for chunk in &chunks {
        let id_str = String::from_utf8_lossy(&chunk.id);
        let description = chunk_description(&chunk.id);
        let essential = if is_essential_chunk(&chunk.id) { "[ESSENTIAL]" } else { "[METADATA]" };

        println!("  {} {} - {}", essential, id_str, description);
        println!("      Size: {} bytes", chunk.data.len());

        // Show LIST sub-type
        if &chunk.id == b"LIST" && chunk.data.len() >= 4 {
            let list_type = String::from_utf8_lossy(&chunk.data[0..4]);
            println!("      List type: {}", list_type);

            if &chunk.data[0..4] == b"INFO" {
                display_info_chunks(&chunk.data[4..]);
            }
        }

        println!();
    }

    let metadata_size: usize = chunks.iter()
        .filter(|c| !is_essential_chunk(&c.id))
        .map(|c| c.data.len() + 8)
        .sum();

    println!("───────────────────────────────────────────────────────");
    println!("Summary: {} chunks, {} bytes strippable metadata",
        chunks.len(), metadata_size);
    println!("\n═══════════════════════════════════════════════════════\n");

    Ok(())
}

fn chunk_description(id: &[u8; 4]) -> &'static str {
    match id {
        b"fmt " => "Format",
        b"data" => "Audio Data",
        b"fact" => "Fact (sample count)",
        b"LIST" => "List Container",
        b"cue " => "Cue Points",
        b"smpl" => "Sampler Info",
        b"inst" => "Instrument",
        b"bext" => "Broadcast Extension (BWF)",
        b"iXML" => "iXML Metadata",
        b"JUNK" | b"junk" => "Padding/Junk",
        b"PAD " | b"pad " => "Padding",
        b"PEAK" => "Peak Envelope",
        b"DISP" => "Display/Title",
        b"acid" => "Acid Loop Info",
        b"strc" => "Structure",
        b"afsp" => "AFsp Info",
        b"cart" => "Cart Chunk (AES46)",
        b"labl" => "Label",
        b"note" => "Note",
        b"ltxt" => "Labeled Text",
        b"plst" => "Playlist",
        b"ID3 " => "ID3 Tag",
        _ => "Unknown",
    }
}

fn display_info_chunks(data: &[u8]) {
    let mut pos = 0;
    while pos + 8 <= data.len() {
        let id = String::from_utf8_lossy(&data[pos..pos + 4]);
        let size = u32::from_le_bytes([
            data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7],
        ]) as usize;

        let end = (pos + 8 + size).min(data.len());
        let value = String::from_utf8_lossy(&data[pos + 8..end])
            .trim_end_matches('\0')
            .to_string();

        let info_name = match &data[pos..pos + 4] {
            b"IART" => "Artist",
            b"INAM" => "Title",
            b"IPRD" => "Product/Album",
            b"ICMT" => "Comment",
            b"ICRD" => "Creation Date",
            b"IGNR" => "Genre",
            b"ISFT" => "Software",
            b"ITRK" => "Track Number",
            b"ICOP" => "Copyright",
            b"IENG" => "Engineer",
            b"ITCH" => "Technician",
            b"ISRC" => "Source",
            _ => &id,
        };

        if !value.is_empty() {
            println!("      {}: {}", info_name, value);
        }

        pos += 8 + size;
        if pos % 2 != 0 {
            pos += 1;
        }
    }
}
