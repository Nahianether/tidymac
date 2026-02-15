use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::ffi::OsStr;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Directories to skip during .DS_Store scan for performance.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".Trash",
    "Library",
    ".cargo",
    ".rustup",
    ".npm",
];

pub struct DsStore {
    root: PathBuf,
}

impl DsStore {
    pub fn new(path: Option<&str>) -> Self {
        let root = path
            .map(PathBuf::from)
            .unwrap_or_else(utils::home_dir);
        Self { root }
    }
}

impl Cleaner for DsStore {
    fn name(&self) -> &'static str {
        "ds-store"
    }

    fn label(&self) -> &'static str {
        ".DS_Store Files"
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
                // Skip certain directories for performance
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    return !SKIP_DIRS.iter().any(|&skip| name == skip);
                }
                true
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && entry.file_name() == OsStr::new(".DS_Store") {
                let path = entry.path().to_path_buf();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                total_bytes += size;
                entries.push(ScanEntry {
                    path,
                    size_bytes: size,
                });
            }
        }

        ScanResult {

            entries,
            total_bytes,
            errors,
        }
    }

    fn clean(&self, dry_run: bool) -> ScanResult {
        let mut result = self.scan();
        if dry_run {
            return result;
        }

        let mut cleaned_entries = Vec::new();
        let mut total_freed = 0u64;

        for entry in result.entries.drain(..) {
            match std::fs::remove_file(&entry.path) {
                Ok(()) => {
                    total_freed += entry.size_bytes;
                    cleaned_entries.push(entry);
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("Failed to remove {}: {e}", entry.path.display()));
                }
            }
        }

        result.entries = cleaned_entries;
        result.total_bytes = total_freed;
        result
    }
}
