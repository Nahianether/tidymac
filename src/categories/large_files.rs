use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Directories to skip during large file scan.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "Library",
    ".Trash",
    ".cargo",
    ".rustup",
];

pub struct LargeFiles {
    min_bytes: u64,
    root: PathBuf,
}

impl LargeFiles {
    pub fn new(min_bytes: u64, path: Option<&str>) -> Self {
        let root = path
            .map(PathBuf::from)
            .unwrap_or_else(utils::home_dir);
        Self { min_bytes, root }
    }
}

impl Cleaner for LargeFiles {
    fn name(&self) -> &'static str {
        "large-files"
    }

    fn label(&self) -> &'static str {
        "Large Files"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut errors = Vec::new();

        if !self.root.exists() {
            errors.push(format!("Path does not exist: {}", self.root.display()));
            return ScanResult {
    
                entries,
                total_bytes,
                errors,
            };
        }

        let walker = WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    return !SKIP_DIRS.iter().any(|&skip| name == skip);
                }
                true
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            if let Ok(metadata) = entry.metadata() {
                if metadata.len() >= self.min_bytes {
                    total_bytes += metadata.len();
                    entries.push(ScanEntry {
                        path: entry.path().to_path_buf(),
                        size_bytes: metadata.len(),
                    });
                }
            }
        }

        // Sort by size descending — biggest files first
        entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        ScanResult {

            entries,
            total_bytes,
            errors,
        }
    }

    fn clean(&self, _dry_run: bool) -> ScanResult {
        // Large files are report-only — never auto-delete
        self.scan()
    }
}
