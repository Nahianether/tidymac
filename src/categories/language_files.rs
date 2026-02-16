use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;
use std::collections::HashSet;
use std::sync::OnceLock;
use walkdir::WalkDir;

/// Languages to always keep.
const KEEP_LPROJ: &[&str] = &["en.lproj", "Base.lproj", "en_US.lproj"];

/// Max depth inside /Applications (app bundle is ~3 levels deep for Resources).
const MAX_DEPTH: usize = 6;

pub struct LanguageFiles;

/// Cached system language detection — only runs `defaults read` once per process.
static SYSTEM_LANGS: OnceLock<HashSet<String>> = OnceLock::new();

fn system_languages() -> &'static HashSet<String> {
    SYSTEM_LANGS.get_or_init(|| {
        let mut langs = HashSet::new();
        // Always keep English
        langs.insert("en".to_string());

        // Read macOS preferred languages
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "NSGlobalDomain", "AppleLanguages"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    let trimmed = line
                        .trim()
                        .trim_matches(|c| c == '"' || c == ',' || c == '(' || c == ')');
                    if !trimmed.is_empty() {
                        if let Some(lang) = trimmed.split('-').next() {
                            langs.insert(lang.to_string());
                        }
                        langs.insert(trimmed.replace('-', "_"));
                    }
                }
            }
        }

        langs
    })
}

impl Cleaner for LanguageFiles {
    fn name(&self) -> &'static str {
        "language-files"
    }

    fn label(&self) -> &'static str {
        "Language Files"
    }

    fn scan(&self) -> ScanResult {
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut errors = Vec::new();

        let keep_langs = system_languages();

        let apps_dir = std::path::Path::new("/Applications");
        if !apps_dir.exists() {
            return ScanResult {
                entries,
                total_bytes,
                errors,
            };
        }

        // Walk /Applications with depth limit
        for entry in WalkDir::new(apps_dir)
            .max_depth(MAX_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only look at directories ending in .lproj
            if !entry.file_type().is_dir() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) if n.ends_with(".lproj") => n,
                _ => continue,
            };

            // Must be inside a .app bundle's Resources directory
            let parent = match path.parent() {
                Some(p) => p,
                None => continue,
            };
            if parent.file_name().and_then(|n| n.to_str()) != Some("Resources") {
                continue;
            }

            // Skip if it's a language we want to keep
            if KEEP_LPROJ.iter().any(|&k| k == name) {
                continue;
            }

            // Extract language code: "ja.lproj" -> "ja", "pt_BR.lproj" -> "pt_BR"
            let lang_code = name.strip_suffix(".lproj").unwrap_or(name);
            let base_lang = lang_code.split('_').next().unwrap_or(lang_code);

            if keep_langs.contains(lang_code) || keep_langs.contains(base_lang) {
                continue;
            }

            match utils::entry_size(path) {
                size if size > 0 => {
                    total_bytes += size;
                    entries.push(ScanEntry {
                        path: path.to_path_buf(),
                        size_bytes: size,
                    });
                }
                _ => {}
            }
        }

        if let Err(e) = std::fs::read_dir(apps_dir) {
            errors.push(format!("Cannot read /Applications: {e}"));
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
                    result.errors.push(format!(
                        "Failed to remove {}: {e}",
                        entry.path.display()
                    ));
                }
            }
        }

        result.entries = cleaned_entries;
        result.total_bytes = total_freed;
        result
    }
}
