mod app_logs;
mod browser_caches;
mod ds_store;
mod homebrew;
mod large_files;
mod package_managers;
mod system_caches;
mod trash;
mod xcode;

use crate::cleaner::Cleaner;

pub fn all_cleaners(min_size_bytes: u64, scan_path: Option<&str>) -> Vec<Box<dyn Cleaner>> {
    vec![
        Box::new(system_caches::SystemCaches),
        Box::new(app_logs::AppLogs),
        Box::new(browser_caches::BrowserCaches),
        Box::new(xcode::XcodeDerivedData),
        Box::new(xcode::XcodeDeviceSupport),
        Box::new(xcode::XcodeArchives),
        Box::new(xcode::CoreSimulator),
        Box::new(homebrew::HomebrewCache),
        Box::new(package_managers::PackageManagerCaches),
        Box::new(trash::Trash),
        Box::new(ds_store::DsStore::new(scan_path)),
        Box::new(large_files::LargeFiles::new(min_size_bytes, scan_path)),
    ]
}

pub fn find_cleaner(
    name: &str,
    min_size_bytes: u64,
    scan_path: Option<&str>,
) -> Option<Box<dyn Cleaner>> {
    all_cleaners(min_size_bytes, scan_path)
        .into_iter()
        .find(|c| c.name() == name)
}

pub fn all_cleaner_names() -> Vec<&'static str> {
    vec![
        "system-caches",
        "app-logs",
        "browser-caches",
        "xcode",
        "xcode-device-support",
        "xcode-archives",
        "core-simulator",
        "homebrew",
        "package-managers",
        "trash",
        "ds-store",
        "large-files",
    ]
}
