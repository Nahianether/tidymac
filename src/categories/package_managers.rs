use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;

struct PmCache {
    path: Vec<&'static str>, // path components relative to home
}

pub struct PackageManagerCaches;

impl PackageManagerCaches {
    fn cache_dirs() -> Vec<PmCache> {
        vec![
            PmCache {
                path: vec![".npm", "_cacache"],
            },
            PmCache {
                path: vec!["Library", "Caches", "Yarn"],
            },
            PmCache {
                path: vec!["Library", "Caches", "pip"],
            },
            PmCache {
                path: vec![".cargo", "registry", "cache"],
            },
        ]
    }
}

impl Cleaner for PackageManagerCaches {
    fn name(&self) -> &'static str {
        "package-managers"
    }

    fn label(&self) -> &'static str {
        "Package Manager Caches"
    }

    fn scan(&self) -> ScanResult {
        let home = utils::home_dir();
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let errors = Vec::new();

        for pm in Self::cache_dirs() {
            let mut cache_path = home.clone();
            for component in &pm.path {
                cache_path = cache_path.join(component);
            }

            if !cache_path.exists() {
                continue;
            }

            let size = utils::entry_size(&cache_path);
            if size > 0 {
                total_bytes += size;
                entries.push(ScanEntry {
                    path: cache_path,
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
