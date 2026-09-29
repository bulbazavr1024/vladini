use std::path::Path;

use crate::config::ProcessingConfig;
use crate::error::ProcessingError;
use crate::format::ImageFormat;
use crate::processor::ImageProcessor;

pub struct Pipeline {
    processors: Vec<Box<dyn ImageProcessor>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn register(&mut self, processor: Box<dyn ImageProcessor>) {
        self.processors.push(processor);
    }

    /// Find a processor that supports the given format.
    fn find_processor(&self, format: ImageFormat) -> Option<&dyn ImageProcessor> {
        self.processors
            .iter()
            .find(|p| p.supported_formats().contains(&format))
            .map(|p| p.as_ref())
    }

    /// Process a single file's bytes, given its path (for format detection).
    pub fn process_file(
        &self,
        path: &Path,
        data: &[u8],
        config: &ProcessingConfig,
    ) -> Result<Vec<u8>, ProcessingError> {
        let format = ImageFormat::from_path(path).ok_or_else(|| {
            ProcessingError::UnsupportedFormat(
                path.extension()
                    .map(|e| e.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".into()),
            )
        })?;

        let processor = self.find_processor(format).ok_or_else(|| {
            ProcessingError::UnsupportedFormat(format.as_str().to_string())
        })?;

        processor.process(data, config)
    }
}
