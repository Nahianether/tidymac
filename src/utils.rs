use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Get home directory or panic with a clear message.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().expect("Could not determine home directory")
}

/// Compute total size of a directory recursively.
pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Get size of a file or directory.
pub fn entry_size(path: &Path) -> u64 {
    if path.is_dir() {
        dir_size(path)
    } else {
        path.metadata().map(|m| m.len()).unwrap_or(0)
    }
}

/// Safely remove a file or directory. Returns bytes freed on success.
pub fn safe_remove(path: &Path) -> Result<u64, std::io::Error> {
    let size = entry_size(path);
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(size)
}

/// Parse human-readable size string ("100MB") into bytes.
pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("GB") {
        (n, 1_073_741_824u64)
    } else if let Some(n) = s.strip_suffix("gb") {
        (n, 1_073_741_824)
    } else if let Some(n) = s.strip_suffix("MB") {
        (n, 1_048_576)
    } else if let Some(n) = s.strip_suffix("mb") {
        (n, 1_048_576)
    } else if let Some(n) = s.strip_suffix("KB") {
        (n, 1_024)
    } else if let Some(n) = s.strip_suffix("kb") {
        (n, 1_024)
    } else if let Some(n) = s.strip_suffix("B") {
        (n, 1)
    } else if let Some(n) = s.strip_suffix("b") {
        (n, 1)
    } else {
        // assume bytes if no suffix
        (s, 1)
    };

    let num: f64 = num_str
        .trim()
        .parse()
        .map_err(|_| format!("Invalid number: '{num_str}'"))?;

    if num < 0.0 {
        return Err("Size cannot be negative".to_string());
    }

    Ok((num * multiplier as f64) as u64)
}

/// Format byte count as human-readable string.
pub fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.2} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Shorten a path for display by replacing home dir with ~.
pub fn display_path(path: &Path) -> String {
    let home = home_dir();
    if let Ok(relative) = path.strip_prefix(&home) {
        format!("~/{}", relative.display())
    } else {
        path.display().to_string()
    }
}
