use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Top-level user directories that should never be removed even if empty.
const PROTECTED_DIRS: &[&str] = &[
    "Desktop",
    "Documents",
    "Downloads",
    "Pictures",
    "Music",
    "Movies",
    "Public",
    "Library",
    "Applications",
];

/// Directories to skip entirely.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".Trash",
    ".cargo",
    ".rustup",
    ".npm",
];

pub struct EmptyFolders;

fn is_protected(path: &std::path::Path, home: &std::path::Path) -> bool {
    PROTECTED_DIRS.iter().any(|name| path == home.join(name))
}

fn should_skip(name: &str) -> bool {
    SKIP_DIRS.iter().any(|&s| name == s) || name.starts_with('.')
}

/// Check if a directory is empty or only contains .DS_Store files.
fn is_effectively_empty(path: &std::path::Path) -> bool {
    let entries = match std::fs::read_dir(path) {
        Ok(rd) => rd,
        Err(_) => return false,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str != ".DS_Store" {
            return false;
        }
    }
    true
}

impl Cleaner for EmptyFolders {
    fn name(&self) -> &'static str {
        "empty-folders"
    }

    fn label(&self) -> &'static str {
        "Empty Folders"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let total_bytes = 0u64;
        let errors = Vec::new();

        let home = utils::home_dir();

        let dirs_to_scan = [
            home.join("Library/Application Support"),
            home.join("Library/Caches"),
            home.join("Library/Containers"),
            home.join("Library/Preferences"),
        ];

        for dir in &dirs_to_scan {
            if !dir.exists() {
                continue;
            }

            // Walk bottom-up so we find the deepest empty dirs first
            let all_dirs: Vec<PathBuf> = WalkDir::new(dir)
                .follow_links(false)
                .max_depth(5)
                .into_iter()
                .filter_entry(|e| {
                    if e.file_type().is_dir() {
                        let name = e.file_name().to_string_lossy();
                        return !should_skip(&name);
                    }
                    true
                })
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
                .map(|e| e.path().to_path_buf())
                .collect();

            // Check each directory (skip the root scan dir itself)
            for path in all_dirs {
                if path == *dir {
                    continue;
                }
                if is_protected(&path, &home) {
                    continue;
                }
                if is_effectively_empty(&path) {
                    entries.push(ScanEntry {
                        path,
                        size_bytes: 0,
                    });
                }
            }
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));

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

        for entry in result.entries.drain(..) {
            // Remove .DS_Store inside first if present
            let ds = entry.path.join(".DS_Store");
            if ds.exists() {
                let _ = std::fs::remove_file(&ds);
            }
            // Now remove the empty directory
            match std::fs::remove_dir(&entry.path) {
                Ok(()) => {
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
        result
    }
}
