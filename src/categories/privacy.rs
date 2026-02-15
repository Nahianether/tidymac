use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::path::PathBuf;

pub struct PrivacyCleaner;

impl PrivacyCleaner {
    fn safari_privacy_files() -> Vec<PathBuf> {
        let home = utils::home_dir();
        let candidates = [
            home.join("Library/Safari/History.db"),
            home.join("Library/Safari/History.db-lock"),
            home.join("Library/Safari/History.db-shm"),
            home.join("Library/Safari/History.db-wal"),
            home.join("Library/Safari/Downloads.plist"),
            home.join("Library/Safari/LastSession.plist"),
            home.join("Library/Safari/TopSites.plist"),
            home.join("Library/Safari/CloudTabs.db"),
            home.join("Library/Safari/LocalStorage"),
            home.join("Library/Safari/Databases"),
            home.join("Library/Cookies/Cookies.binarycookies"),
        ];
        candidates.into_iter().filter(|p| p.exists()).collect()
    }

    fn chrome_privacy_files() -> Vec<PathBuf> {
        let base = utils::home_dir().join("Library/Application Support/Google/Chrome");
        if !base.exists() {
            return vec![];
        }

        let mut files = Vec::new();

        // Check Default profile and numbered profiles
        let mut profiles: Vec<PathBuf> = vec![base.join("Default")];
        if let Ok(read_dir) = std::fs::read_dir(&base) {
            for entry in read_dir.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("Profile ") {
                    profiles.push(entry.path());
                }
            }
        }

        let targets = [
            "Cookies",
            "History",
            "History-journal",
            "Login Data",
            "Login Data-journal",
            "Web Data",
            "Web Data-journal",
            "Top Sites",
            "Top Sites-journal",
            "Visited Links",
        ];

        for profile in &profiles {
            if !profile.exists() {
                continue;
            }
            for target in &targets {
                let path = profile.join(target);
                if path.exists() {
                    files.push(path);
                }
            }
        }

        files
    }

    fn firefox_privacy_files() -> Vec<PathBuf> {
        let profiles_dir =
            utils::home_dir().join("Library/Application Support/Firefox/Profiles");
        if !profiles_dir.exists() {
            return vec![];
        }

        let mut files = Vec::new();

        let targets = [
            "cookies.sqlite",
            "cookies.sqlite-wal",
            "cookies.sqlite-shm",
            "places.sqlite",
            "places.sqlite-wal",
            "places.sqlite-shm",
            "formhistory.sqlite",
            "webappsstore.sqlite",
        ];

        if let Ok(read_dir) = std::fs::read_dir(&profiles_dir) {
            for entry in read_dir.flatten() {
                let profile_path = entry.path();
                if !profile_path.is_dir() {
                    continue;
                }
                for target in &targets {
                    let path = profile_path.join(target);
                    if path.exists() {
                        files.push(path);
                    }
                }
            }
        }

        files
    }

    fn system_privacy_files() -> Vec<PathBuf> {
        let home = utils::home_dir();
        let candidates = [
            home.join("Library/Application Support/com.apple.sharedfilelist"),
            home.join("Library/Preferences/com.apple.recentitems.plist"),
        ];
        candidates.into_iter().filter(|p| p.exists()).collect()
    }
}

impl Cleaner for PrivacyCleaner {
    fn name(&self) -> &'static str {
        "privacy"
    }

    fn label(&self) -> &'static str {
        "Privacy Data"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut errors = Vec::new();

        errors.push(
            "Clearing cookies and history will log you out of websites.".to_string(),
        );

        let all_files: Vec<PathBuf> = [
            Self::safari_privacy_files(),
            Self::chrome_privacy_files(),
            Self::firefox_privacy_files(),
            Self::system_privacy_files(),
        ]
        .into_iter()
        .flatten()
        .collect();

        for path in all_files {
            let size = utils::entry_size(&path);
            if size > 0 {
                total_bytes += size;
                entries.push(ScanEntry {
                    path,
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
