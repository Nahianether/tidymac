use std::path::PathBuf;

/// One item found during a scan.
pub struct ScanEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
}

/// Result of scanning a single category.
pub struct ScanResult {
    pub entries: Vec<ScanEntry>,
    pub total_bytes: u64,
    pub errors: Vec<String>,
}

/// The trait every cleaner module implements.
pub trait Cleaner {
    /// Machine-readable name used in --category flag (e.g. "system-caches").
    fn name(&self) -> &'static str;

    /// Human-readable label for display (e.g. "System Caches").
    fn label(&self) -> &'static str;

    /// Scan and return what would be cleaned. Never deletes anything.
    fn scan(&self) -> ScanResult;

    /// Actually delete the entries when dry_run is false.
    /// When dry_run is true, behaves like scan().
    fn clean(&self, dry_run: bool) -> ScanResult;
}
