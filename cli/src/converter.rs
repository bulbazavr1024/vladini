use image::{GenericImageView, ImageFormat as ImgFormat, DynamicImage};
use std::io::Cursor;
use std::path::Path;
use std::process::Command;

use crate::config::{ProcessingConfig, StripMode};
use crate::error::ProcessingError;
use crate::processor::maybe_resize;
use crate::processor::mp4::is_ffmpeg_available;

/// Input extensions `convert` picks up when walking a directory
pub const MEDIA_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "avif",
    "mp3", "wav", "flac", "ogg", "oga", "opus", "m4a", "aac", "aiff", "wma", "ac3", "mka",
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "flv", "wmv", "mpg", "mpeg", "3gp",
];

/// Audio-only targets: the video stream is dropped instead of being muxed as cover art
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "ogg", "oga", "opus", "m4a", "aac", "aiff", "wma", "ac3", "mka",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertFormat {
    Png,
    Jpg,
    Webp,
}

impl ConvertFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "png" => Some(ConvertFormat::Png),
            "jpg" | "jpeg" => Some(ConvertFormat::Jpg),
            "webp" => Some(ConvertFormat::Webp),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ConvertFormat::Png => "png",
            ConvertFormat::Jpg => "jpg",
            ConvertFormat::Webp => "webp",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ConvertFormat::Png => "PNG",
            ConvertFormat::Jpg => "JPEG",
            ConvertFormat::Webp => "WebP",
        }
    }
}

/// Convert image from one format to another
pub fn convert_image(
    input: &[u8],
    target_format: ConvertFormat,
    config: &ProcessingConfig,
) -> Result<Vec<u8>, ProcessingError> {
    // Load image (supports PNG, JPG, WebP automatically)
    let img = image::load_from_memory(input)
        .map_err(|e| ProcessingError::Decode(format!("Failed to load image: {}", e)))?;
    let img = maybe_resize(img, config);

    log::debug!(
        "Converting image: {}x{} pixels to {}",
        img.width(),
        img.height(),
        target_format.as_str()
    );

    // Convert based on target format
    let output = match target_format {
        ConvertFormat::Png => convert_to_png(&img, config)?,
        ConvertFormat::Jpg => convert_to_jpg(&img, config)?,
        ConvertFormat::Webp => convert_to_webp(&img, config)?,
    };

    log::debug!(
        "Conversion complete: {} bytes ({})",
        output.len(),
        target_format.as_str()
    );

    Ok(output)
}

/// Convert to PNG format
fn convert_to_png(img: &DynamicImage, config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);

    if config.no_lossy {
        // Lossless PNG
        img.write_to(&mut cursor, ImgFormat::Png)
            .map_err(|e| ProcessingError::Encode(format!("Failed to encode PNG: {}", e)))?;
    } else {
        // For lossy PNG, we could use imagequant here
        // For now, just save as regular PNG and let PNG processor optimize it later
        img.write_to(&mut cursor, ImgFormat::Png)
            .map_err(|e| ProcessingError::Encode(format!("Failed to encode PNG: {}", e)))?;
    }

    Ok(output)
}

/// Convert to JPEG format
fn convert_to_jpg(img: &DynamicImage, config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);

    // Convert to RGB (JPEG doesn't support alpha)
    let rgb_img = img.to_rgb8();

    // Create JPEG encoder with quality
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
        &mut cursor,
        config.quality,
    );

    encoder
        .encode(
            rgb_img.as_raw(),
            rgb_img.width(),
            rgb_img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| ProcessingError::Encode(format!("Failed to encode JPEG: {}", e)))?;

    Ok(output)
}

/// Convert to WebP format
fn convert_to_webp(img: &DynamicImage, config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError> {
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();

    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), width, height);

    let encoded = if config.no_lossy {
        encoder.encode_lossless()
    } else {
        encoder.encode(config.quality as f32)
    };

    Ok(encoded.to_vec())
}

/// Convert any audio/video file with ffmpeg. Container and codecs are picked by ffmpeg
/// from the output extension, so every format the installed ffmpeg supports works.
pub fn convert_media(input: &Path, output: &Path, config: &ProcessingConfig) -> Result<(), ProcessingError> {
    if !is_ffmpeg_available() {
        return Err(ProcessingError::Encode(
            "ffmpeg not found - audio/video conversion requires ffmpeg".to_string(),
        ));
    }

    // ffmpeg truncates the output before reading the input, so same file = lost source
    if let (Ok(a), Ok(b)) = (input.canonicalize(), output.canonicalize()) {
        if a == b {
            return Err(ProcessingError::Encode(
                "input and output are the same file - pass a different output path".to_string(),
            ));
        }
    }

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ProcessingError::WriteFile {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let ext = output
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y"]);
    // As input options: fast seek, and --end is a position in the source rather than a duration
    if let Some(start) = &config.start {
        cmd.arg("-ss").arg(start);
    }
    if let Some(end) = &config.end {
        cmd.arg("-to").arg(end);
    }
    cmd.arg("-i").arg(input);

    if config.strip != StripMode::None {
        cmd.args(["-map_metadata", "-1"]);
    }

    if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        cmd.arg("-vn");
        match &config.audio_bitrate {
            Some(bitrate) => cmd.arg("-b:a").arg(bitrate),
            // Vorbis rejects fixed bitrates outside a per-channel range; VBR quality 0-10 always works
            None if matches!(ext.as_str(), "ogg" | "oga" | "mka") => {
                cmd.arg("-q:a").arg((config.quality / 10).to_string())
            }
            // quality 0-100 -> 32-256 kbps (opus caps mono at 256k), but never above the source:
            // re-encoding can't add quality, only size. Lossless codecs (wav, flac) ignore it
            None => {
                let kbps = (config.quality as u32 * 256 / 100).min(source_kbps(input).unwrap_or(u32::MAX));
                cmd.arg("-b:a").arg(format!("{}k", kbps.max(32)))
            }
        };
    } else {
        if let Some(bitrate) = &config.audio_bitrate {
            cmd.arg("-b:a").arg(bitrate);
        }

        // Same quality -> CRF mapping as MP4 compression; -b:v 0 puts VP9/AV1 into pure CRF mode
        let crf = (((100 - config.quality) as f32 * 0.33 + 18.0) as u32).clamp(18, 35);
        cmd.arg("-crf").arg(crf.to_string()).args(["-b:v", "0"]);

        // -2 keeps the aspect ratio with an even size, which h264 requires
        let scale = match (config.resize_width, config.resize_height) {
            (None, None) => None,
            (w, h) => Some(format!(
                "scale={}:{}",
                w.map_or(-2, |w| w as i64),
                h.map_or(-2, |h| h as i64)
            )),
        };
        let filter = if ext == "gif" {
            // Palette built from the clip itself; the default fixed palette looks washed out
            let prefix = scale.map(|s| s + ",").unwrap_or_default();
            Some(format!("{}split[a][b];[a]palettegen[p];[b][p]paletteuse", prefix))
        } else {
            scale
        };
        if let Some(filter) = filter {
            cmd.arg("-vf").arg(filter);
        }
    }

    cmd.arg(output);

    log::debug!("Executing: ffmpeg {:?}", cmd.get_args().collect::<Vec<_>>());

    let result = cmd
        .output()
        .map_err(|e| ProcessingError::Encode(format!("Failed to execute ffmpeg: {}", e)))?;

    if !result.status.success() {
        return Err(ProcessingError::Encode(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        )));
    }

    Ok(())
}

/// Overall bitrate of a media file in kbps, if ffprobe can tell
fn source_kbps(input: &Path) -> Option<u32> {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=bit_rate", "-of", "csv=p=0"])
        .arg(input)
        .output()
        .ok()?;
    let bps: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    u32::try_from(bps.div_ceil(1000)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn convert_media_webm_to_audio() {
        if !is_ffmpeg_available() {
            eprintln!("skipped: ffmpeg not installed");
            return;
        }
        let dir = std::env::temp_dir().join(format!("vladini_convert_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let webm = dir.join("in.webm");
        // Clip with both video and audio, so the mp3 target has to drop the video stream
        let status = Command::new("ffmpeg")
            .args(["-nostdin", "-y", "-loglevel", "error",
                   "-f", "lavfi", "-i", "color=c=red:s=64x64:d=1",
                   "-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
            .arg(&webm)
            .status()
            .unwrap();
        assert!(status.success());

        // m4a/ogg would silently keep the video stream without -vn; output dir doesn't exist yet.
        // Mono source at both quality extremes: vorbis and opus reject out-of-range bitrates.
        for (ext, quality) in [("mp3", 0), ("m4a", 100), ("ogg", 0), ("ogg", 100), ("opus", 100)] {
            let out = dir.join(format!("out/in.{}", ext));
            let config = ProcessingConfig { quality, ..Default::default() };
            convert_media(&webm, &out, &config).unwrap_or_else(|e| panic!("{} q{}: {}", ext, quality, e));

            let probe = Command::new("ffprobe")
                .args(["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0"])
                .arg(&out)
                .output()
                .unwrap();
            assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "audio", "{}", ext);
        }
        let mp3 = dir.join("out/in.mp3");

        // Converting a file onto itself must be refused, not truncate the source
        let size = std::fs::metadata(&mp3).unwrap().len();
        assert!(convert_media(&mp3, &mp3, &ProcessingConfig::default()).is_err());
        assert_eq!(std::fs::metadata(&mp3).unwrap().len(), size);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn convert_media_never_inflates_audio() {
        if !is_ffmpeg_available() {
            eprintln!("skipped: ffmpeg not installed");
            return;
        }
        let dir = std::env::temp_dir().join(format!("vladini_inflate_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Typical YouTube-style audio webm: opus well below the default mp3 bitrate
        let webm = dir.join("talk.webm");
        let status = Command::new("ffmpeg")
            .args(["-nostdin", "-y", "-loglevel", "error",
                   "-f", "lavfi", "-i", "anoisesrc=d=5:a=0.1", "-ac", "2", "-c:a", "libopus", "-b:a", "64k"])
            .arg(&webm)
            .status()
            .unwrap();
        assert!(status.success());

        let mp3 = dir.join("talk.mp3");
        convert_media(&webm, &mp3, &ProcessingConfig::default()).unwrap();
        let (src, out) = (std::fs::metadata(&webm).unwrap().len(), std::fs::metadata(&mp3).unwrap().len());
        // mp3 bitrates come in fixed steps, so allow landing on the next one up
        assert!(out as f64 <= src as f64 * 1.15, "{} -> {} bytes", src, out);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn probe(path: &Path, entries: &str) -> String {
        let out = Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", entries, "-of", "csv=p=0"])
            .arg(path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn convert_media_options() {
        if !is_ffmpeg_available() {
            eprintln!("skipped: ffmpeg not installed");
            return;
        }
        let dir = std::env::temp_dir().join(format!("vladini_options_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src.mkv");
        let status = Command::new("ffmpeg")
            .args(["-nostdin", "-y", "-loglevel", "error",
                   "-f", "lavfi", "-i", "testsrc=s=64x48:d=2",
                   "-f", "lavfi", "-i", "sine=duration=2",
                   "-metadata", "title=Song"])
            .arg(&src)
            .status()
            .unwrap();
        assert!(status.success());

        // Trim + resize + palette gif
        let gif = dir.join("clip.gif");
        let config = ProcessingConfig {
            resize_width: Some(32),
            start: Some("0.5".into()),
            end: Some("1.5".into()),
            ..Default::default()
        };
        convert_media(&src, &gif, &config).unwrap();
        assert_eq!(probe(&gif, "stream=codec_name,width,height"), "gif,32,24");
        let secs: f64 = probe(&gif, "format=duration").parse().unwrap();
        assert!((0.8..1.2).contains(&secs), "duration {}", secs);

        // Explicit bitrate wins over quality; tags survive unless stripped
        let mp3 = dir.join("song.mp3");
        let config = ProcessingConfig {
            strip: StripMode::None,
            audio_bitrate: Some("96k".into()),
            ..Default::default()
        };
        convert_media(&src, &mp3, &config).unwrap();
        assert_eq!(probe(&mp3, "stream=bit_rate"), "96000");
        assert_eq!(probe(&mp3, "format_tags=title"), "Song");

        convert_media(&src, &mp3, &ProcessingConfig::default()).unwrap();
        assert_eq!(probe(&mp3, "format_tags=title"), "");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
