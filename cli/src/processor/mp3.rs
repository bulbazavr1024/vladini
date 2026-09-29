use std::collections::HashSet;
use std::io::Cursor;

use id3::{Tag, TagLike, Content};

use crate::config::{ProcessingConfig, StripMode};
use crate::error::ProcessingError;
use crate::format::ImageFormat;
use crate::processor::ImageProcessor;

pub struct Mp3Processor;

/// Display all metadata from an MP3 file
pub fn inspect_mp3(input: &[u8]) -> Result<(), ProcessingError> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("                  MP3 Metadata Inspection");
    println!("═══════════════════════════════════════════════════════\n");

    let file_size = input.len();
    println!("File size: {} bytes ({:.2} KB)", file_size, file_size as f64 / 1024.0);

    // Check ID3v2
    let id3v2_size = detect_id3v2_size(input);
    if id3v2_size > 0 {
        println!("ID3v2 tag: {} bytes ({:.2} KB)", id3v2_size, id3v2_size as f64 / 1024.0);
    } else {
        println!("ID3v2 tag: Not found");
    }

    // Check ID3v1
    let has_v1 = has_id3v1(input);
    if has_v1 {
        println!("ID3v1 tag: Present (128 bytes)");
    } else {
        println!("ID3v1 tag: Not found");
    }

    let audio_start = id3v2_size;
    let audio_end = if has_v1 {
        input.len().saturating_sub(128)
    } else {
        input.len()
    };
    let audio_size = audio_end - audio_start;
    println!("Audio data: {} bytes ({:.2} KB)\n", audio_size, audio_size as f64 / 1024.0);

    // Parse and display ID3v2 frames
    match Tag::read_from2(&mut Cursor::new(input)) {
        Ok(tag) => {
            let version = tag.version();
            let version_str = match version {
                id3::Version::Id3v22 => "2.2",
                id3::Version::Id3v23 => "2.3",
                id3::Version::Id3v24 => "2.4",
            };

            println!("ID3v{} Tag Contents:", version_str);
            println!("───────────────────────────────────────────────────────");

            let frames: Vec<_> = tag.frames().collect();
            if frames.is_empty() {
                println!("  (no frames found)");
            } else {
                println!("  Total frames: {}\n", frames.len());

                let safe_frames = get_safe_frame_ids();

                for frame in &frames {
                    let frame_id = frame.id();
                    let is_safe = safe_frames.contains(frame_id);
                    let safety_marker = if is_safe { "[SAFE]" } else { "[UNSAFE]" };

                    let frame_name = get_frame_name(frame_id);
                    let value = format_frame_content(frame.content());

                    println!("  {} {}", safety_marker, frame_name);
                    println!("      ID: {}", frame_id);

                    // Special handling for PRIV frames - display owner separately
                    if frame_id == "PRIV" {
                        if let Content::Private(priv_data) = frame.content() {
                            println!("      Owner: {}", priv_data.owner_identifier);
                            println!("      Data: {}", format_unknown_data(&priv_data.private_data));

                            // Extract and list all file paths found
                            let paths = extract_file_paths(&priv_data.private_data);
                            if !paths.is_empty() {
                                println!("      Found {} file path(s):", paths.len());
                                for path in paths {
                                    println!("        • {}", path);
                                }
                            }
                        } else {
                            println!("      Value: {}", value);
                        }
                    } else {
                        println!("      Value: {}", value);
                    }
                    println!();
                }

                // Summary
                let safe_count = frames.iter().filter(|f| safe_frames.contains(f.id())).count();
                let unsafe_count = frames.len() - safe_count;
                println!("───────────────────────────────────────────────────────");
                println!("Summary: {} safe frames, {} unsafe frames", safe_count, unsafe_count);
            }
        }
        Err(e) => {
            if id3v2_size > 0 {
                println!("Could not parse ID3v2 tag: {}", e);
            } else {
                println!("No ID3v2 tag found");
            }
        }
    }

    // Display ID3v1 if present
    if has_v1 {
        println!("\nID3v1 Tag Contents:");
        println!("───────────────────────────────────────────────────────");
        display_id3v1(input);
    }

    println!("\n═══════════════════════════════════════════════════════\n");

    Ok(())
}

/// Display ID3v1 tag contents
fn display_id3v1(input: &[u8]) {
    if input.len() < 128 {
        return;
    }

    let tag_start = input.len() - 128;
    let tag_data = &input[tag_start..];

    let title_str = String::from_utf8_lossy(&tag_data[3..33]);
    let title = title_str.trim_end_matches('\0').trim();
    let artist_str = String::from_utf8_lossy(&tag_data[33..63]);
    let artist = artist_str.trim_end_matches('\0').trim();
    let album_str = String::from_utf8_lossy(&tag_data[63..93]);
    let album = album_str.trim_end_matches('\0').trim();
    let year_str = String::from_utf8_lossy(&tag_data[93..97]);
    let year = year_str.trim_end_matches('\0').trim();
    let comment_str = String::from_utf8_lossy(&tag_data[97..127]);
    let comment = comment_str.trim_end_matches('\0').trim();
    let genre = tag_data[127];

    println!("  Title:   {}", if title.is_empty() { "(empty)" } else { title });
    println!("  Artist:  {}", if artist.is_empty() { "(empty)" } else { artist });
    println!("  Album:   {}", if album.is_empty() { "(empty)" } else { album });
    println!("  Year:    {}", if year.is_empty() { "(empty)" } else { year });
    println!("  Comment: {}", if comment.is_empty() { "(empty)" } else { comment });
    println!("  Genre:   {} ({})", genre, get_genre_name(genre));
}

/// Get human-readable frame name
fn get_frame_name(frame_id: &str) -> &str {
    match frame_id {
        "TIT2" => "Title",
        "TPE1" => "Artist",
        "TALB" => "Album",
        "TYER" => "Year",
        "TDRC" => "Recording Time",
        "TCON" => "Genre",
        "TRCK" => "Track Number",
        "TPOS" => "Part Of Set",
        "COMM" => "Comment",
        "APIC" => "Attached Picture",
        "USLT" => "Unsynchronized Lyrics",
        "TXXX" => "User Defined Text",
        "WXXX" => "User Defined URL",
        "PRIV" => "Private Data",
        "POPM" => "Popularimeter",
        "TBPM" => "BPM",
        "TCOM" => "Composer",
        "TLEN" => "Length",
        "TPUB" => "Publisher",
        "TPE2" => "Band/Orchestra/Accompaniment",
        "TPE3" => "Conductor",
        "TPE4" => "Interpreted/Remixed By",
        "TEXT" => "Lyricist",
        "TCOP" => "Copyright",
        "TENC" => "Encoded By",
        "TSRC" => "ISRC",
        _ => "Unknown Frame",
    }
}

/// Format frame content for display
fn format_frame_content(content: &Content) -> String {
    use id3::Content::*;

    match content {
        Text(text) => text.clone(),
        Link(link) => link.clone(),
        Lyrics(lyrics) => format!("[{}] {}", lyrics.lang, lyrics.text),
        Comment(comment) => format!("[{}] {}: {}", comment.lang, comment.description, comment.text),
        Picture(pic) => {
            format!(
                "Image ({}), {} bytes, description: '{}'",
                pic.mime_type,
                pic.data.len(),
                pic.description
            )
        }
        ExtendedText(ext) => format!("{}: {}", ext.description, ext.value),
        ExtendedLink(ext) => format!("{}: {}", ext.description, ext.link),
        Unknown(data) => {
            format_unknown_data(&data.data)
        }
        Private(priv_data) => {
            format!("Owner: {}, Data: {}",
                    priv_data.owner_identifier,
                    format_unknown_data(&priv_data.private_data))
        }
        _ => format!("<other content type>"),
    }
}

/// Format unknown/binary data, attempting to extract readable text
fn format_unknown_data(data: &[u8]) -> String {
    if data.is_empty() {
        return String::from("<empty>");
    }

    // Try to parse as UTF-8 or Latin-1 text
    let text_data = String::from_utf8_lossy(data);

    // Check if it contains printable characters and might be text
    let printable_count = text_data.chars()
        .filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
        .count();
    let total_chars = text_data.chars().count();

    // If more than 60% is printable, treat as text
    if total_chars > 0 && (printable_count * 100 / total_chars) > 60 {
        // Check for potentially sensitive paths
        let has_paths = text_data.contains(":\\") ||
                       text_data.contains(":/") ||
                       text_data.contains("/Users/") ||
                       text_data.contains("/home/") ||
                       text_data.contains("C:\\") ||
                       text_data.contains("D:\\") ||
                       text_data.contains(".prproj") ||
                       text_data.contains(".aep") ||
                       text_data.contains("\\AppData\\");

        let warning = if has_paths {
            " ⚠️  CONTAINS FILE PATHS"
        } else {
            ""
        };

        // Show full data if it contains paths, otherwise limit to 500 chars
        let display_text = if has_paths {
            text_data.replace('\0', "\\0")
        } else if text_data.len() > 500 {
            format!("{}... (truncated, total {} bytes)",
                   &text_data[..500].replace('\0', "\\0"),
                   data.len())
        } else {
            text_data.replace('\0', "\\0")
        };

        format!("\"{}\"{}",  display_text, warning)
    } else {
        // Binary data - show hex preview
        let hex_preview: String = data.iter()
            .take(16)
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        if data.len() > 16 {
            format!("<binary: {} ... ({} bytes total)>", hex_preview, data.len())
        } else {
            format!("<binary: {} ({} bytes)>", hex_preview, data.len())
        }
    }
}

/// Extract file paths from binary data
fn extract_file_paths(data: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(data);
    let mut paths = Vec::new();

    for line in text.lines() {
        // Windows paths (C:\, D:\, etc.)
        for cap in line.match_indices(":\\").filter(|(i, _)| {
            *i > 0 && line.as_bytes()[i - 1].is_ascii_alphabetic()
        }) {
            let start = cap.0 - 1;
            let rest = &line[start..];

            // Extract until we hit invalid characters or whitespace
            let end = rest.find(|c: char| {
                c == '\0' || c == '\n' || c == '\r' || c == '<' || c == '>' ||
                c == '"' || c == '|' || c == '?' || c == '*'
            }).unwrap_or(rest.len());

            if end > 3 {
                let path = rest[..end].trim();
                if !path.is_empty() && path.len() > 3 {
                    paths.push(path.to_string());
                }
            }
        }

        // Unix/Mac paths
        if line.contains("/Users/") || line.contains("/home/") || line.contains("/mnt/") {
            for (i, _) in line.match_indices('/') {
                let rest = &line[i..];
                let end = rest.find(|c: char| {
                    c == '\0' || c == '\n' || c == '\r' || c == '<' || c == '>' ||
                    c == '"' || c == ' ' || c == '\t'
                }).unwrap_or(rest.len());

                let path = rest[..end].trim();
                // Only include if it looks like a real path (has / and extension or is a directory)
                if path.len() > 5 && (path.contains('.') || path.ends_with('/')) {
                    if path.starts_with("/Users/") || path.starts_with("/home/") ||
                       path.starts_with("/mnt/") || path.starts_with("/Volumes/") {
                        paths.push(path.to_string());
                        break;
                    }
                }
            }
        }

        // Project file extensions in quotes or tags
        for ext in &[".prproj", ".aep", ".fcp", ".fcpx", ".avp", ".psd", ".ai"] {
            if let Some(pos) = line.find(ext) {
                // Try to find the start of the path
                let before = &line[..pos + ext.len()];

                // Look backwards for path start
                let start = before.rfind(|c: char| {
                    c == '"' || c == '>' || c == '\0' || c == '\n'
                }).map(|i| i + 1).unwrap_or(0);

                let path = before[start..].trim();
                if path.len() > ext.len() + 2 {
                    paths.push(path.to_string());
                }
            }
        }
    }

    // Deduplicate and sort
    paths.sort();
    paths.dedup();
    paths
}

/// Get genre name from ID3v1 genre code
fn get_genre_name(code: u8) -> &'static str {
    match code {
        0 => "Blues",
        1 => "Classic Rock",
        2 => "Country",
        3 => "Dance",
        4 => "Disco",
        5 => "Funk",
        6 => "Grunge",
        7 => "Hip-Hop",
        8 => "Jazz",
        9 => "Metal",
        10 => "New Age",
        11 => "Oldies",
        12 => "Other",
        13 => "Pop",
        14 => "R&B",
        15 => "Rap",
        16 => "Reggae",
        17 => "Rock",
        18 => "Techno",
        19 => "Industrial",
        20 => "Alternative",
        21 => "Ska",
        22 => "Death Metal",
        23 => "Pranks",
        24 => "Soundtrack",
        25 => "Euro-Techno",
        26 => "Ambient",
        27 => "Trip-Hop",
        28 => "Vocal",
        29 => "Jazz+Funk",
        30 => "Fusion",
        31 => "Trance",
        32 => "Classical",
        33 => "Instrumental",
        34 => "Acid",
        35 => "House",
        36 => "Game",
        37 => "Sound Clip",
        38 => "Gospel",
        39 => "Noise",
        40 => "AlternRock",
        41 => "Bass",
        _ => "Unknown",
    }
}

impl ImageProcessor for Mp3Processor {
    fn supported_formats(&self) -> &[ImageFormat] {
        &[ImageFormat::Mp3]
    }

    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
        match config.strip {
            StripMode::None => {
                log::debug!("Strip mode: None - returning original MP3 unchanged");
                Ok(input.to_vec())
            }
            StripMode::Safe => strip_unsafe_tags(input),
            StripMode::All => strip_all_tags(input),
        }
    }
}

/// Remove all ID3 tags (v1 and v2), returning only raw MPEG audio frames
fn strip_all_tags(input: &[u8]) -> Result<Vec<u8>, ProcessingError> {
    let id3v2_size = detect_id3v2_size(input);
    let has_v1 = has_id3v1(input);

    let audio_start = id3v2_size;
    let audio_end = if has_v1 {
        input.len().saturating_sub(128)
    } else {
        input.len()
    };

    if audio_start >= audio_end {
        return Err(ProcessingError::Decode(
            "Invalid MP3 structure: no audio data found".to_string(),
        ));
    }

    let audio_only = input[audio_start..audio_end].to_vec();

    // Logging
    let mut removed_tags = Vec::new();
    if id3v2_size > 0 {
        removed_tags.push(format!("ID3v2 ({} bytes)", id3v2_size));
    }
    if has_v1 {
        removed_tags.push("ID3v1 (128 bytes)".to_string());
    }

    if !removed_tags.is_empty() {
        log::info!("Strip mode: All - removing all ID3 tags");
        log::info!("Removed: {}", removed_tags.join(", "));
        let saved = (id3v2_size + if has_v1 { 128 } else { 0 }) as f64 / 1024.0;
        log::info!("Stripped all tags ({:.2} KB saved from metadata)", saved);
    } else {
        log::debug!("No ID3 tags found in file");
    }

    Ok(audio_only)
}

/// Remove unsafe metadata, keeping only basic tags (title, artist, album, year, genre, track)
fn strip_unsafe_tags(input: &[u8]) -> Result<Vec<u8>, ProcessingError> {
    // Try to parse ID3v2 tag
    let tag = match Tag::read_from2(&mut Cursor::new(input)) {
        Ok(tag) => tag,
        Err(e) => {
            // No tag or parse error - check if there's ID3v1
            if has_id3v1(input) {
                log::info!("Found ID3v1 tag (Safe mode removes ID3v1, keeping ID3v2 safe frames only)");
                // Remove ID3v1, return rest
                let without_v1 = input[..input.len().saturating_sub(128)].to_vec();
                return Ok(without_v1);
            }
            log::debug!("No ID3 tags found or parse error: {}", e);
            return Ok(input.to_vec());
        }
    };

    let total_frames = tag.frames().count();
    let version = tag.version();
    log::info!(
        "Processing MP3: Found ID3v{} tag with {} frames",
        match version {
            id3::Version::Id3v22 => "2.2",
            id3::Version::Id3v23 => "2.3",
            id3::Version::Id3v24 => "2.4",
        },
        total_frames
    );

    let safe_frame_ids = get_safe_frame_ids();
    let mut kept_frames = Vec::new();
    let mut removed_frames = Vec::new();

    // Categorize frames
    for frame in tag.frames() {
        let frame_id = frame.id();
        if safe_frame_ids.contains(frame_id) {
            kept_frames.push(frame_id.to_string());
        } else {
            removed_frames.push(frame_id.to_string());
        }
    }

    log::info!(
        "Strip mode: Safe - keeping {} safe frames, removing {} unsafe frames",
        kept_frames.len(),
        removed_frames.len()
    );

    if !removed_frames.is_empty() {
        log::debug!("Removing frames: {}", removed_frames.join(", "));
    }
    if !kept_frames.is_empty() {
        log::debug!("Keeping safe frames: {}", kept_frames.join(", "));
    }

    // If no frames to remove, return original
    if removed_frames.is_empty() && !has_id3v1(input) {
        log::info!("No unsafe frames to remove");
        return Ok(input.to_vec());
    }

    // Create new tag with only safe frames
    let mut new_tag = Tag::new();

    for frame in tag.frames() {
        if safe_frame_ids.contains(frame.id()) {
            new_tag.add_frame(frame.clone());
        }
    }

    // Get audio data (skip old ID3v2, exclude ID3v1)
    let id3v2_size = detect_id3v2_size(input);
    let audio_start = id3v2_size;
    let audio_end = if has_id3v1(input) {
        input.len().saturating_sub(128)
    } else {
        input.len()
    };

    if audio_start >= audio_end {
        return Err(ProcessingError::Decode(
            "Invalid MP3 structure: no audio data found".to_string(),
        ));
    }

    let audio_data = &input[audio_start..audio_end];

    // Write new tag + audio to buffer
    let mut output = Vec::new();
    new_tag
        .write_to(&mut output, id3::Version::Id3v24)
        .map_err(|e| ProcessingError::Encode(format!("Failed to write ID3 tag: {}", e)))?;

    output.extend_from_slice(audio_data);

    let original_metadata_size = id3v2_size + if has_id3v1(input) { 128 } else { 0 };
    let new_tag_size = output.len() - audio_data.len();
    let saved = (original_metadata_size as isize - new_tag_size as isize) as f64 / 1024.0;

    if saved > 0.0 {
        log::info!(
            "Stripped {} unsafe frames ({:.2} KB saved from metadata)",
            removed_frames.len(),
            saved
        );
    } else {
        log::info!(
            "Stripped {} unsafe frames (new tag: {:.2} KB)",
            removed_frames.len(),
            new_tag_size as f64 / 1024.0
        );
    }

    Ok(output)
}

/// Returns the set of safe frame IDs to keep in Safe mode
fn get_safe_frame_ids() -> HashSet<&'static str> {
    [
        "TIT2", // Title
        "TPE1", // Artist
        "TALB", // Album
        "TYER", // Year (ID3v2.3)
        "TDRC", // Recording time (ID3v2.4)
        "TCON", // Genre
        "TRCK", // Track number
    ]
    .iter()
    .copied()
    .collect()
}

/// Detect ID3v2 tag size at the start of the file
/// Returns the total size including the 10-byte header, or 0 if no ID3v2 tag
fn detect_id3v2_size(input: &[u8]) -> usize {
    if input.len() < 10 {
        return 0;
    }

    // Check for "ID3" signature
    if &input[0..3] != b"ID3" {
        return 0;
    }

    // Parse synchsafe integer from bytes 6-9
    // Synchsafe: only 7 bits per byte are used (MSB is always 0)
    let size = ((input[6] as usize) << 21)
        | ((input[7] as usize) << 14)
        | ((input[8] as usize) << 7)
        | (input[9] as usize);

    // Total size = header (10 bytes) + tag size
    size + 10
}

/// Check if the file has an ID3v1 tag at the end (last 128 bytes start with "TAG")
fn has_id3v1(input: &[u8]) -> bool {
    input.len() >= 128 && &input[input.len() - 128..input.len() - 125] == b"TAG"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_id3v2_size_no_tag() {
        let data = vec![0xFF, 0xFB, 0x90, 0x00]; // MPEG frame header
        assert_eq!(detect_id3v2_size(&data), 0);
    }

    #[test]
    fn test_detect_id3v2_size_with_tag() {
        // ID3v2.3 header with synchsafe size = 100
        // Synchsafe 100 = 0x64 = 0b0000000_0000000_0000001_1100100
        let mut data = vec![
            b'I', b'D', b'3', // Signature
            0x03, 0x00, // Version 2.3
            0x00, // Flags
            0x00, 0x00, 0x00, 0x64, // Size (synchsafe 100)
        ];
        data.extend(vec![0; 100]); // Tag data
        assert_eq!(detect_id3v2_size(&data), 110); // 10 + 100
    }

    #[test]
    fn test_has_id3v1_no_tag() {
        let data = vec![0xFF; 200];
        assert!(!has_id3v1(&data));
    }

    #[test]
    fn test_has_id3v1_with_tag() {
        let mut data = vec![0xFF; 200];
        let tag_start = data.len() - 128;
        data[tag_start] = b'T';
        data[tag_start + 1] = b'A';
        data[tag_start + 2] = b'G';
        assert!(has_id3v1(&data));
    }

    #[test]
    fn test_get_safe_frame_ids() {
        let safe = get_safe_frame_ids();
        assert!(safe.contains("TIT2"));
        assert!(safe.contains("TPE1"));
        assert!(safe.contains("TALB"));
        assert!(!safe.contains("APIC"));
        assert!(!safe.contains("COMM"));
    }
}
