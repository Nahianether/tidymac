use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct AppInfo {
    pub name: String,
    pub path: PathBuf,
    pub total_size: u64,
    pub binary_size: u64,
    pub resources_size: u64,
    pub frameworks_size: u64,
    pub plugins_size: u64,
    pub other_size: u64,
}

/// Scan `/Applications/` for `.app` bundles and compute size breakdowns.
/// Uses parallel analysis via rayon for speed.
/// Calls `progress_fn(completed, total, current_app_name)` for UI updates.
pub fn scan_applications(
    progress_fn: impl Fn(usize, usize, &str) + Send + Sync,
) -> Vec<AppInfo> {
    let apps_dir = Path::new("/Applications");
    if !apps_dir.exists() {
        return vec![];
    }

    // Collect all .app bundles first (fast — just readdir)
    let app_paths: Vec<(PathBuf, String)> = match std::fs::read_dir(apps_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_dir() {
                    return None;
                }
                let name = path.file_name()?.to_string_lossy().to_string();
                if !name.ends_with(".app") {
                    return None;
                }
                let display_name = name.trim_end_matches(".app").to_string();
                Some((path, display_name))
            })
            .collect(),
        Err(_) => return vec![],
    };

    let total = app_paths.len();
    let completed = Arc::new(AtomicUsize::new(0));

    // Parallel analysis of all app bundles
    let mut apps: Vec<AppInfo> = app_paths
        .into_par_iter()
        .map(|(path, name)| {
            progress_fn(completed.load(Ordering::Relaxed), total, &name);
            let info = analyze_app_bundle(&path, name);
            completed.fetch_add(1, Ordering::Relaxed);
            info
        })
        .collect();

    // Sort by total size descending
    apps.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    apps
}

fn analyze_app_bundle(app_path: &Path, name: String) -> AppInfo {
    let contents = app_path.join("Contents");

    let mut binary_size = 0u64;
    let mut resources_size = 0u64;
    let mut frameworks_size = 0u64;
    let mut plugins_size = 0u64;
    let mut total_size = 0u64;

    for entry in WalkDir::new(app_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        total_size += size;

        // Classify based on location within the bundle
        let p = entry.path();
        if let Ok(rel) = p.strip_prefix(&contents) {
            // Use components for reliable cross-platform matching
            let mut comps = rel.components();
            if let Some(first) = comps.next() {
                let dir = first.as_os_str().to_string_lossy();
                match dir.as_ref() {
                    "Frameworks" => frameworks_size += size,
                    "Resources" => resources_size += size,
                    "MacOS" => binary_size += size,
                    "PlugIns" | "Plugins" | "Extensions" | "Helpers"
                    | "XPCServices" | "Library" => plugins_size += size,
                    _ => {}
                }
            }
        }
    }

    let other_size = total_size
        .saturating_sub(binary_size)
        .saturating_sub(resources_size)
        .saturating_sub(frameworks_size)
        .saturating_sub(plugins_size);

    AppInfo {
        name,
        path: app_path.to_path_buf(),
        total_size,
        binary_size,
        resources_size,
        frameworks_size,
        plugins_size,
        other_size,
    }
}
