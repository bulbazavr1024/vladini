pub mod png;
pub mod jpg;
pub mod mp3;
pub mod wav;
pub mod webp;
pub mod mp4;

use image::DynamicImage;

use crate::config::ProcessingConfig;
use crate::error::ProcessingError;
use crate::format::ImageFormat;

pub trait ImageProcessor: Send + Sync {
    fn supported_formats(&self) -> &[ImageFormat];
    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError>;
}

/// Resize image according to config. Returns the image unchanged if no resize is requested.
/// If only width or height is set, the other dimension is calculated to preserve aspect ratio.
pub fn maybe_resize(img: DynamicImage, config: &ProcessingConfig) -> DynamicImage {
    match (config.resize_width, config.resize_height) {
        (None, None) => img,
        (Some(w), Some(h)) => img.resize_exact(w, h, image::imageops::FilterType::Lanczos3),
        (Some(w), None) => img.resize(w, u32::MAX, image::imageops::FilterType::Lanczos3),
        (None, Some(h)) => img.resize(u32::MAX, h, image::imageops::FilterType::Lanczos3),
    }
}
