use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Supported configuration file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    Yaml,
    Toml,
    Json,
    Dotenv,
}

impl ConfigFormat {
    /// Detect format from file path or extension.
    pub fn detect(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_string_lossy();
        if file_name == ".env" || file_name.starts_with(".env.") {
            return Some(ConfigFormat::Dotenv);
        }

        let ext = path.extension()?.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "yaml" | "yml" => Some(ConfigFormat::Yaml),
            "toml" => Some(ConfigFormat::Toml),
            "json" => Some(ConfigFormat::Json),
            "env" => Some(ConfigFormat::Dotenv),
            _ => None,
        }
    }

    /// Canonical string identifier for the format.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigFormat::Yaml => "yaml",
            ConfigFormat::Toml => "toml",
            ConfigFormat::Json => "json",
            ConfigFormat::Dotenv => "dotenv",
        }
    }
}

/// Represents a flattened configuration key with its various naming conventions and source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEntry {
    /// Source config file path where the key was defined.
    pub file_path: PathBuf,
    /// Dot-delimited canonical path (e.g. `database.pool.max_connections`).
    pub dot_path: String,
    /// Screaming snake case environment variable notation (e.g. `DATABASE_POOL_MAX_CONNECTIONS`).
    pub env_path: String,
    /// Double-underscore notation used by Viper, Spring Boot, etc. (e.g. `DATABASE__POOL__MAX_CONNECTIONS`).
    pub double_underscore_path: String,
    /// Final leaf key name (e.g. `max_connections`).
    pub leaf_key: String,
    /// List of hierarchy segments from root to leaf.
    pub path_segments: Vec<String>,
    /// 1-indexed line number in the source file.
    pub line: usize,
    /// Stringified value representation.
    pub value_repr: String,
    /// Nesting depth (1 for root keys).
    pub depth: usize,
    /// Source format of the config file.
    pub format: ConfigFormat,
}

impl KeyEntry {
    /// Create a new `KeyEntry` from path segments, line number, value, and file metadata.
    pub fn from_segments(
        file_path: PathBuf,
        segments: Vec<String>,
        line: usize,
        value_repr: String,
        format: ConfigFormat,
    ) -> Self {
        assert!(!segments.is_empty(), "Key segments must not be empty");

        let dot_path = segments.join(".");
        let env_path = segments
            .iter()
            .map(|s| sanitize_for_env(s))
            .collect::<Vec<_>>()
            .join("_")
            .to_uppercase();

        let double_underscore_path = segments
            .iter()
            .map(|s| sanitize_for_env(s))
            .collect::<Vec<_>>()
            .join("__")
            .to_uppercase();

        let leaf_key = segments.last().cloned().unwrap_or_default();
        let depth = segments.len();

        Self {
            file_path,
            dot_path,
            env_path,
            double_underscore_path,
            leaf_key,
            path_segments: segments,
            line,
            value_repr,
            depth,
            format,
        }
    }

    /// Return all distinct canonical lookup forms for scanning across codebases.
    pub fn all_lookup_forms(&self) -> Vec<String> {
        let mut forms = Vec::with_capacity(4);
        forms.push(self.dot_path.clone());

        if !forms.contains(&self.env_path) {
            forms.push(self.env_path.clone());
        }

        if !forms.contains(&self.double_underscore_path) {
            forms.push(self.double_underscore_path.clone());
        }

        if !forms.contains(&self.leaf_key) {
            forms.push(self.leaf_key.clone());
        }

        forms
    }

    /// Check if a given string matches any of the canonical lookup forms.
    pub fn matches_token(&self, token: &str) -> bool {
        let trimmed = token.trim();
        trimmed == self.dot_path
            || trimmed.eq_ignore_ascii_case(&self.env_path)
            || trimmed.eq_ignore_ascii_case(&self.double_underscore_path)
            || trimmed == self.leaf_key
    }
}

/// Helper function to convert segment characters into valid env variable tokens.
fn sanitize_for_env(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}
