use std::path::PathBuf;

/// Result of processing a single file.
pub struct FileResult {
    pub path: PathBuf,
    pub original_size: u64,
    pub compressed_size: u64,
    pub skipped: bool,
    pub error: Option<String>,
}

impl FileResult {
    pub fn savings_pct(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        (1.0 - self.compressed_size as f64 / self.original_size as f64) * 100.0
    }
}

/// Aggregate report for all processed files.
pub struct Report {
    pub results: Vec<FileResult>,
}

impl Report {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn add(&mut self, result: FileResult) {
        self.results.push(result);
    }

    pub fn total_original(&self) -> u64 {
        self.results.iter().map(|r| r.original_size).sum()
    }

    pub fn total_compressed(&self) -> u64 {
        self.results.iter().map(|r| r.compressed_size).sum()
    }

    pub fn total_savings_pct(&self) -> f64 {
        let orig = self.total_original();
        if orig == 0 {
            return 0.0;
        }
        (1.0 - self.total_compressed() as f64 / orig as f64) * 100.0
    }

    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.error.is_none() && !r.skipped).count()
    }

    pub fn error_count(&self) -> usize {
        self.results.iter().filter(|r| r.error.is_some()).count()
    }

    pub fn print_summary(&self) {
        println!("\n--- Summary ---");
        println!(
            "Files processed: {} | Skipped: {} | Errors: {}",
            self.success_count(),
            self.results.iter().filter(|r| r.skipped).count(),
            self.error_count()
        );

        if self.success_count() > 0 {
            println!(
                "Total: {} → {} ({:.1}% reduction)",
                format_size(self.total_original()),
                format_size(self.total_compressed()),
                self.total_savings_pct()
            );
        }

        for r in &self.results {
            if let Some(ref err) = r.error {
                println!("  ERROR {}: {}", r.path.display(), err);
            }
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
