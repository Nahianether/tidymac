use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::path::PathBuf;

pub struct BrowserCaches;

impl BrowserCaches {
    fn chrome_cache_dirs() -> Vec<PathBuf> {
        let base = utils::home_dir().join("Library/Caches/Google/Chrome");
        if !base.exists() {
            return vec![];
        }

        let mut dirs = Vec::new();
        // Chrome stores caches per profile: Default, Profile 1, etc.
        if let Ok(read_dir) = std::fs::read_dir(&base) {
            for entry in read_dir.flatten() {
                let profile_path = entry.path();
                if !profile_path.is_dir() {
                    continue;
                }
                // Each profile has Cache/Cache_Data and Code Cache
                let cache_data = profile_path.join("Cache");
                if cache_data.exists() {
                    dirs.push(cache_data);
                }
                let code_cache = profile_path.join("Code Cache");
                if code_cache.exists() {
                    dirs.push(code_cache);
                }
            }
        }
        dirs
    }

    fn safari_cache_dirs() -> Vec<PathBuf> {
        let path = utils::home_dir().join("Library/Caches/com.apple.Safari");
        if path.exists() {
            vec![path]
        } else {
            vec![]
        }
    }

    fn firefox_cache_dirs() -> Vec<PathBuf> {
        let profiles_dir = utils::home_dir().join("Library/Caches/Firefox/Profiles");
        if !profiles_dir.exists() {
            return vec![];
        }

        let mut dirs = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(&profiles_dir) {
            for entry in read_dir.flatten() {
                let profile_path = entry.path();
                if !profile_path.is_dir() {
                    continue;
                }
                let cache2 = profile_path.join("cache2");
                if cache2.exists() {
                    dirs.push(cache2);
                }
            }
        }
        dirs
    }
}

impl Cleaner for BrowserCaches {
    fn name(&self) -> &'static str {
        "browser-caches"
    }

    fn label(&self) -> &'static str {
        "Browser Caches"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let errors = Vec::new();

        let all_dirs: Vec<PathBuf> = [
            Self::chrome_cache_dirs(),
            Self::safari_cache_dirs(),
            Self::firefox_cache_dirs(),
        ]
        .into_iter()
        .flatten()
        .collect();

        for dir in all_dirs {
            let size = utils::entry_size(&dir);
            if size > 0 {
                total_bytes += size;
                entries.push(ScanEntry {
                    path: dir,
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
