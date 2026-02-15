use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct AppInfo {
    pub name: String,
    pub path: PathBuf,
    pub total_size: u64,
    pub binary_size: u64,
    pub resources_size: u64,
    pub frameworks_size: u64,
    pub other_size: u64,
}

/// Scan `/Applications/` for `.app` bundles and compute size breakdowns.
pub fn scan_applications(progress_fn: &dyn Fn(&str)) -> Vec<AppInfo> {
    let apps_dir = Path::new("/Applications");
    if !apps_dir.exists() {
        return vec![];
    }

    let mut apps: Vec<AppInfo> = Vec::new();

    let entries: Vec<_> = match std::fs::read_dir(apps_dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return vec![],
    };

    for entry in entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if !name.ends_with(".app") {
            continue;
        }

        progress_fn(&format!("Analyzing: {}", name));

        let display_name = name.trim_end_matches(".app").to_string();
        let info = analyze_app_bundle(&path, display_name);
        apps.push(info);
    }

    // Sort by total size descending
    apps.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    apps
}

fn analyze_app_bundle(app_path: &Path, name: String) -> AppInfo {
    let contents = app_path.join("Contents");

    let mut binary_size = 0u64;
    let mut resources_size = 0u64;
    let mut frameworks_size = 0u64;
    let mut total_size = 0u64;

    for entry in WalkDir::new(app_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        total_size += size;

        // Classify the file based on its location within the bundle
        if let Ok(rel) = p.strip_prefix(&contents) {
            let rel_str = rel.to_string_lossy();

            if rel_str.starts_with("Frameworks/") || rel_str.starts_with("Frameworks\\") {
                frameworks_size += size;
            } else if rel_str.starts_with("Resources/") || rel_str.starts_with("Resources\\") {
                resources_size += size;
            } else if rel_str.starts_with("MacOS/") || rel_str.starts_with("MacOS\\") {
                binary_size += size;
            } else {
                // Other contents (plugins, helpers, etc.)
            }
        }
    }

    let other_size = total_size
        .saturating_sub(binary_size)
        .saturating_sub(resources_size)
        .saturating_sub(frameworks_size);

    AppInfo {
        name,
        path: app_path.to_path_buf(),
        total_size,
        binary_size,
        resources_size,
        frameworks_size,
        other_size,
    }
}
