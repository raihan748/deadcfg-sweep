use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::ConfigFormat;
use super::patterns::SourceLanguage;

/// Recursively traverses directory to collect all relevant source code files.
pub struct CodeWalker;

impl CodeWalker {
    /// Walk directory and collect all candidate source code files.
    pub fn collect_source_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_ignored_dir(e.path()))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if path.is_file() {
                // Skip configuration files themselves
                if ConfigFormat::detect(path).is_some() {
                    continue;
                }

                // Check if file is recognized source code
                if SourceLanguage::detect(path) != SourceLanguage::Unknown {
                    files.push(path.to_path_buf());
                }
            }
        }

        files
    }
}

/// Identifies common build artifact and repository metadata directories to ignore.
fn is_ignored_dir(path: &Path) -> bool {
    let name = match path.file_name().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };

    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | "vendor"
            | ".idea"
            | ".vscode"
            | "coverage"
            | ".venv"
            | "__pycache__"
            | ".next"
            | ".turbo"
    )
}
