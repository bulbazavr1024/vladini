use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Png,
    Jpg,
    Mp3,
    Wav,
    Webp,
    Mp4,
}

impl ImageFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some(ImageFormat::Png),
            "jpg" | "jpeg" => Some(ImageFormat::Jpg),
            "mp3" => Some(ImageFormat::Mp3),
            "wav" => Some(ImageFormat::Wav),
            "webp" => Some(ImageFormat::Webp),
            "mp4" | "m4v" | "m4a" => Some(ImageFormat::Mp4),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ImageFormat::Png => "PNG",
            ImageFormat::Jpg => "JPEG",
            ImageFormat::Mp3 => "MP3",
            ImageFormat::Wav => "WAV",
            ImageFormat::Webp => "WebP",
            ImageFormat::Mp4 => "MP4",
        }
    }
}
