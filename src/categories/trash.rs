use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;

pub struct Trash;

impl Cleaner for Trash {
    fn name(&self) -> &'static str {
        "trash"
    }

    fn label(&self) -> &'static str {
        "Trash"
    }

    fn scan(&self) -> ScanResult {
        let trash_dir = utils::home_dir().join(".Trash");
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut errors = Vec::new();

        match std::fs::read_dir(&trash_dir) {
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
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                errors.push(
                    "Trash access denied. Grant Full Disk Access: System Settings → Privacy & Security → Full Disk Access → enable your terminal/TidyMac.".to_string()
                );
            }
            Err(e) => {
                errors.push(format!("Cannot read {}: {e}", trash_dir.display()));
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
