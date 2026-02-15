use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

/// Minimum file size: 10 MB
const MIN_SIZE: u64 = 10_485_760;

/// Minimum age: 180 days (6 months)
const MIN_AGE_DAYS: u64 = 180;

pub struct OldFiles;

impl Cleaner for OldFiles {
    fn name(&self) -> &'static str {
        "old-files"
    }

    fn label(&self) -> &'static str {
        "Old & Unused Files"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut errors = Vec::new();

        let home = utils::home_dir();
        let dirs_to_scan = [
            home.join("Downloads"),
            home.join("Documents"),
            home.join("Desktop"),
        ];

        let threshold = SystemTime::now()
            .checked_sub(Duration::from_secs(MIN_AGE_DAYS * 86400))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        for dir in &dirs_to_scan {
            if !dir.exists() {
                continue;
            }

            for entry in WalkDir::new(dir)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let meta = match path.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        errors.push(format!("Cannot read {}: {e}", path.display()));
                        continue;
                    }
                };

                let size = meta.len();
                if size < MIN_SIZE {
                    continue;
                }

                // Check last accessed time, fall back to modified time
                let last_used = meta
                    .accessed()
                    .or_else(|_| meta.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                if last_used > threshold {
                    continue;
                }

                total_bytes += size;
                entries.push(ScanEntry {
                    path: path.to_path_buf(),
                    size_bytes: size,
                });
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
