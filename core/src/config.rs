use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripMode {
    All,
    Safe,
    None,
}

impl fmt::Display for StripMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Safe => write!(f, "safe"),
            Self::None => write!(f, "none"),
        }
    }
}

impl FromStr for StripMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "all" => Ok(Self::All),
            "safe" => Ok(Self::Safe),
            "none" => Ok(Self::None),
            _ => Err(format!("unknown strip mode: {s}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    /// Quantization quality 0-100 (lower = smaller file, worse quality)
    pub quality: u8,
    /// Speed 1-10 (1 = slowest/best, 10 = fastest/worst)
    pub speed: i32,
    /// Whether to skip lossy quantization (lossless only + strip)
    pub no_lossy: bool,
    /// Metadata strip mode
    pub strip: StripMode,
    /// Dry run - don't write anything
    pub dry_run: bool,
    /// Create .bak backup before overwriting
    pub backup: bool,
    /// Extract frames from MP4 to PNG
    pub extract_frames: bool,
    /// Frames per second to extract (0 = all frames)
    pub fps: f32,
    /// Target width for resize (None = no resize)
    pub resize_width: Option<u32>,
    /// Target height for resize (None = no resize)
    pub resize_height: Option<u32>,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            quality: 80,
            speed: 3,
            no_lossy: false,
            strip: StripMode::All,
            dry_run: false,
            backup: false,
            extract_frames: false,
            fps: 1.0,
            resize_width: None,
            resize_height: None,
        }
    }
}
