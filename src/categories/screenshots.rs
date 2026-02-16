use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Screenshots older than 30 days are marked for cleanup.
const MAX_AGE_DAYS: u64 = 30;

/// Screenshot filename prefixes used by macOS.
const SCREENSHOT_PREFIXES: &[&str] = &["Screenshot ", "Screen Recording "];

/// Valid screenshot/recording extensions.
const VALID_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "tiff", "gif", "mov", "mp4"];

pub struct Screenshots;

fn get_screenshot_dir() -> PathBuf {
    // Check if user has a custom screenshot location
    if let Ok(output) = std::process::Command::new("defaults")
        .args(["read", "com.apple.screencapture", "location"])
        .output()
    {
        if output.status.success() {
            let location = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !location.is_empty() {
                let path = PathBuf::from(&location);
                if path.exists() {
                    return path;
                }
            }
        }
    }
    // Default: ~/Desktop
    utils::home_dir().join("Desktop")
}

fn is_screenshot(name: &str) -> bool {
    SCREENSHOT_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

fn has_valid_extension(name: &str) -> bool {
    let lower = name.to_lowercase();
    VALID_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

fn is_older_than(metadata: &std::fs::Metadata, max_age: Duration) -> bool {
    metadata
        .modified()
        .ok()
        .and_then(|mtime| SystemTime::now().duration_since(mtime).ok())
        .map(|age| age > max_age)
        .unwrap_or(false)
}

impl Cleaner for Screenshots {
    fn name(&self) -> &'static str {
        "screenshots"
    }

    fn label(&self) -> &'static str {
        "Old Screenshots"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let errors = Vec::new();

        let screenshot_dir = get_screenshot_dir();
        if !screenshot_dir.exists() {
            return ScanResult {
                entries,
                total_bytes,
                errors,
            };
        }

        let max_age = Duration::from_secs(MAX_AGE_DAYS * 24 * 60 * 60);

        let dir_entries = match std::fs::read_dir(&screenshot_dir) {
            Ok(rd) => rd,
            Err(_) => {
                return ScanResult {
                    entries,
                    total_bytes,
                    errors,
                }
            }
        };

        for entry in dir_entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if !is_screenshot(&name_str) || !has_valid_extension(&name_str) {
                continue;
            }

            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if !is_older_than(&metadata, max_age) {
                continue;
            }

            let size = metadata.len();
            total_bytes += size;
            entries.push(ScanEntry {
                path,
                size_bytes: size,
            });
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
