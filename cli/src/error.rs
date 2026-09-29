use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("failed to read file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to decode image: {0}")]
    Decode(String),

    #[error("quantization failed: {0}")]
    Quantize(String),

    #[error("encoding failed: {0}")]
    Encode(String),

    #[error("optimization failed: {0}")]
    Optimize(String),
}
