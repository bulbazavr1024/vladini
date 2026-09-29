use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::{ProcessingConfig, StripMode};

/// CLI tool for image/video compression, conversion, and metadata management
#[derive(Debug, Parser)]
#[command(name = "image_preparer", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compress images or videos
    Compress {
        /// Input file or directory
        input: PathBuf,

        /// Output file or directory (default: overwrite in-place)
        output: Option<PathBuf>,

        /// Quantization quality 0-100
        #[arg(short, long, default_value_t = 80, value_parser = clap::value_parser!(u8).range(0..=100))]
        quality: u8,

        /// Speed vs quality: 1 (slowest/best) to 10 (fastest/worst)
        #[arg(short, long, default_value_t = 3, value_parser = clap::value_parser!(i32).range(1..=10))]
        speed: i32,

        /// Skip lossy compression — only lossless optimization + strip metadata
        #[arg(long)]
        no_lossy: bool,

        /// Metadata strip mode
        #[arg(long, default_value = "all")]
        strip: StripMode,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,

        /// Create .bak backup before overwriting
        #[arg(long)]
        backup: bool,

        /// Show what would be done without writing files
        #[arg(long)]
        dry_run: bool,

        /// Resize width in pixels (preserves aspect ratio if height is omitted)
        #[arg(long)]
        width: Option<u32>,

        /// Resize height in pixels (preserves aspect ratio if width is omitted)
        #[arg(long)]
        height: Option<u32>,
    },

    /// Convert images, audio and video between formats (audio/video via ffmpeg)
    Convert {
        /// Input file or directory
        input: PathBuf,

        /// Output file or directory (default: next to the input)
        output: Option<PathBuf>,

        /// Target format: png, jpg, webp, or any ffmpeg extension (mp3, flac, ogg, m4a, mp4, webm, mkv, gif, ...).
        /// Defaults to the output file's extension
        #[arg(long, short = 't', value_name = "FORMAT")]
        to: Option<String>,

        /// Quality for lossy formats (0-100)
        #[arg(short, long, default_value_t = 80, value_parser = clap::value_parser!(u8).range(0..=100))]
        quality: u8,

        /// Use lossless compression where applicable
        #[arg(long)]
        no_lossy: bool,

        /// Metadata strip mode (audio/video tags are kept by default)
        #[arg(long, default_value = "none")]
        strip: StripMode,

        /// Audio bitrate, e.g. 192k (overrides --quality for audio)
        #[arg(long, short = 'b')]
        bitrate: Option<String>,

        /// Resize width in pixels (preserves aspect ratio if height is omitted)
        #[arg(long)]
        width: Option<u32>,

        /// Resize height in pixels (preserves aspect ratio if width is omitted)
        #[arg(long)]
        height: Option<u32>,

        /// Start position for audio/video, e.g. 90 or 1:30
        #[arg(long)]
        start: Option<String>,

        /// End position in the source for audio/video, e.g. 2:45
        #[arg(long)]
        end: Option<String>,

        /// Max files converted in parallel (default: CPU cores)
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,

        /// Create .bak backup before overwriting
        #[arg(long)]
        backup: bool,
    },

    /// Display file metadata without processing
    Inspect {
        /// Input file or directory
        input: PathBuf,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Extract frames from MP4 videos to PNG images
    Extract {
        /// Input MP4 file or directory
        input: PathBuf,

        /// Output directory for frames
        output: PathBuf,

        /// Frames per second to extract (default: 1). Use 0 to extract all frames
        #[arg(long, short = 'f', default_value_t = 1.0)]
        fps: f32,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,
    },
}

impl Cli {
    pub fn to_config(
        &self,
        cmd_quality: u8,
        cmd_speed: i32,
        cmd_no_lossy: bool,
        cmd_strip: StripMode,
        cmd_dry_run: bool,
        cmd_backup: bool,
        cmd_resize_width: Option<u32>,
        cmd_resize_height: Option<u32>,
    ) -> ProcessingConfig {
        ProcessingConfig {
            quality: cmd_quality,
            speed: cmd_speed,
            no_lossy: cmd_no_lossy,
            strip: cmd_strip,
            dry_run: cmd_dry_run,
            backup: cmd_backup,
            extract_frames: false,
            fps: 0.0,
            resize_width: cmd_resize_width,
            resize_height: cmd_resize_height,
            ..Default::default()
        }
    }
}
