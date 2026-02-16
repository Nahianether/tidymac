use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use walkdir::WalkDir;

/// Directories to skip for performance and safety.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".Trash",
    ".cargo",
    ".rustup",
    ".npm",
];

pub struct BrokenSymlinks;

fn should_skip(name: &str) -> bool {
    SKIP_DIRS.iter().any(|&s| name == s)
}

impl Cleaner for BrokenSymlinks {
    fn name(&self) -> &'static str {
        "broken-symlinks"
    }

    fn label(&self) -> &'static str {
        "Broken Symlinks"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let errors = Vec::new();

        let home = utils::home_dir();

        let dirs_to_scan = [
            home.join("Library"),
            std::path::PathBuf::from("/usr/local/bin"),
            std::path::PathBuf::from("/usr/local/lib"),
            home.join("bin"),
        ];

        for dir in &dirs_to_scan {
            if !dir.exists() {
                continue;
            }

            let max_depth = if dir.starts_with("/usr/local") { 1 } else { 5 };

            for entry in WalkDir::new(dir)
                .max_depth(max_depth)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    if e.file_type().is_dir() {
                        let name = e.file_name().to_string_lossy();
                        return !should_skip(&name);
                    }
                    true
                })
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                // Check if this entry is a symlink
                let is_symlink = entry
                    .path()
                    .symlink_metadata()
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false);

                if !is_symlink {
                    continue;
                }

                // Check if the symlink target exists
                let target_exists = std::fs::metadata(path).is_ok();

                if !target_exists {
                    // Broken symlink — target is gone
                    let target = std::fs::read_link(path)
                        .map(|t| t.to_string_lossy().to_string())
                        .unwrap_or_default();

                    // Symlinks themselves are tiny, but report 0 since they don't use real space
                    let size = entry
                        .path()
                        .symlink_metadata()
                        .map(|m| m.len())
                        .unwrap_or(0);
                    total_bytes += size;

                    entries.push(ScanEntry {
                        path: path.to_path_buf(),
                        size_bytes: size,
                    });

                    let _ = target; // target info available if needed for display
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
