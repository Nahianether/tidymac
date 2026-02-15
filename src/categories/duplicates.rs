use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Minimum file size: 1 MB
const MIN_SIZE: u64 = 1_048_576;

/// Maximum file size for hashing: 500 MB (skip very large files)
const MAX_SIZE: u64 = 500_000_000;

/// Bytes to read for partial hash (first 4 KB)
const PARTIAL_READ: usize = 4096;

/// Directories/bundles to skip inside scanned folders
const SKIP_EXTENSIONS: &[&str] = &[
    ".photoslibrary",
    ".musiclibrary",
    ".tvlibrary",
    ".fcpbundle",
    ".vmwarevm",
    ".parallels",
    ".app",
];

pub struct DuplicateFinder;

fn should_skip_dir(name: &str) -> bool {
    let lower = name.to_lowercase();
    SKIP_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
        || lower == ".trash"
        || lower == "node_modules"
        || lower == ".git"
}

/// Compute blake3 hash of the first `n` bytes of a file.
fn partial_hash(path: &std::path::Path) -> Option<blake3::Hash> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; PARTIAL_READ];
    let bytes_read = file.read(&mut buf).ok()?;
    buf.truncate(bytes_read);
    Some(blake3::hash(&buf))
}

/// Compute blake3 hash of an entire file.
fn full_hash(path: &std::path::Path) -> Option<blake3::Hash> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize())
}

impl Cleaner for DuplicateFinder {
    fn name(&self) -> &'static str {
        "duplicates"
    }

    fn label(&self) -> &'static str {
        "Duplicate Files"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let errors = Vec::new();

        let home = utils::home_dir();
        let dirs_to_scan = [
            home.join("Documents"),
            home.join("Downloads"),
            home.join("Desktop"),
            home.join("Pictures"),
        ];

        // Pass 1: Group all files by size
        let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();

        for dir in &dirs_to_scan {
            if !dir.exists() {
                continue;
            }
            for entry in WalkDir::new(dir)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    if e.file_type().is_dir() {
                        let name = e.file_name().to_string_lossy();
                        return !should_skip_dir(&name);
                    }
                    true
                })
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let size = match path.metadata() {
                    Ok(m) => m.len(),
                    Err(_) => continue,
                };
                if size < MIN_SIZE || size > MAX_SIZE {
                    continue;
                }
                size_groups
                    .entry(size)
                    .or_default()
                    .push(path.to_path_buf());
            }
        }

        // Pass 2: For groups with 2+ files, compute partial hash
        for (_size, paths) in &size_groups {
            if paths.len() < 2 {
                continue;
            }

            let mut partial_groups: HashMap<blake3::Hash, Vec<&PathBuf>> = HashMap::new();
            for path in paths {
                if let Some(hash) = partial_hash(path) {
                    partial_groups.entry(hash).or_default().push(path);
                }
            }

            // Pass 3: For matching partial hashes, compute full hash
            for (_phash, partial_matches) in &partial_groups {
                if partial_matches.len() < 2 {
                    continue;
                }

                let mut full_groups: HashMap<blake3::Hash, Vec<&PathBuf>> = HashMap::new();
                for path in partial_matches {
                    if let Some(hash) = full_hash(path) {
                        full_groups.entry(hash).or_default().push(path);
                    }
                }

                // For each group of true duplicates, keep the first, mark rest as entries
                for (_fhash, dupes) in &full_groups {
                    if dupes.len() < 2 {
                        continue;
                    }
                    // Skip the first file (the "original"), mark the rest
                    for dup_path in &dupes[1..] {
                        let size = utils::entry_size(dup_path);
                        total_bytes += size;
                        entries.push(ScanEntry {
                            path: dup_path.to_path_buf(),
                            size_bytes: size,
                        });
                    }
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
