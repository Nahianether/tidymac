use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::path::PathBuf;

pub struct AppLogs;

impl Cleaner for AppLogs {
    fn name(&self) -> &'static str {
        "app-logs"
    }

    fn label(&self) -> &'static str {
        "Application Logs"
    }

    fn scan(&self) -> ScanResult {
        let log_dirs = vec![
            utils::home_dir().join("Library/Logs"),
            PathBuf::from("/Library/Logs"),
        ];

        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut errors = Vec::new();

        for log_dir in &log_dirs {
            if !log_dir.exists() {
                continue;
            }

            match std::fs::read_dir(log_dir) {
                Ok(read_dir) => {
                    for entry in read_dir.flatten() {
                        let path = entry.path();
                        let size = utils::entry_size(&path);
                        total_bytes += size;
                        entries.push(ScanEntry {
                            path,
                            size_bytes: size,
                        });
                    }
                }
                Err(e) => {
                    errors.push(format!("Cannot read {}: {e}", log_dir.display()));
                }
            }
        }

        entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

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
            match utils::safe_remove(&entry.path) {
                Ok(freed) => {
                    total_freed += freed;
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
