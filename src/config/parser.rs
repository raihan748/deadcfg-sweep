use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

use super::flattener::{flatten_json, flatten_toml, flatten_yaml};
use super::key_entry::{ConfigFormat, KeyEntry};

/// Errors encountered while loading or parsing configuration files.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    Toml(toml::de::Error),
    Json(serde_json::Error),
    UnsupportedFormat(PathBuf),
    InvalidDotenv(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "I/O error: {}", e),
            ConfigError::Yaml(e) => write!(f, "YAML parse error: {}", e),
            ConfigError::Toml(e) => write!(f, "TOML parse error: {}", e),
            ConfigError::Json(e) => write!(f, "JSON parse error: {}", e),
            ConfigError::UnsupportedFormat(p) => {
                write!(f, "Unsupported configuration format for: {}", p.display())
            }
            ConfigError::InvalidDotenv(msg) => write!(f, "Invalid dotenv format: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(e: serde_yaml::Error) -> Self {
        ConfigError::Yaml(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Toml(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::Json(e)
    }
}

/// Primary configuration parser supporting YAML, TOML, JSON, and Dotenv.
pub struct ConfigParser;

impl ConfigParser {
    /// Parse a configuration file by automatically detecting its format from extension.
    pub fn parse_file(path: &Path) -> Result<Vec<KeyEntry>, ConfigError> {
        let format = ConfigFormat::detect(path)
            .ok_or_else(|| ConfigError::UnsupportedFormat(path.to_path_buf()))?;
        let content = fs::read_to_string(path)?;
        Self::parse_str(&content, path, format)
    }

    /// Parse configuration content from string with explicit format.
    pub fn parse_str(
        content: &str,
        file_path: &Path,
        format: ConfigFormat,
    ) -> Result<Vec<KeyEntry>, ConfigError> {
        match format {
            ConfigFormat::Yaml => Self::parse_yaml_str(content, file_path),
            ConfigFormat::Toml => Self::parse_toml_str(content, file_path),
            ConfigFormat::Json => Self::parse_json_str(content, file_path),
            ConfigFormat::Dotenv => Self::parse_dotenv_str(content, file_path),
        }
    }

    /// Parse YAML formatted string.
    pub fn parse_yaml_str(content: &str, file_path: &Path) -> Result<Vec<KeyEntry>, ConfigError> {
        let val: YamlValue = serde_yaml::from_str(content)?;
        Ok(flatten_yaml(&val, file_path, content))
    }

    /// Parse TOML formatted string.
    pub fn parse_toml_str(content: &str, file_path: &Path) -> Result<Vec<KeyEntry>, ConfigError> {
        let val: TomlValue = toml::from_str(content)?;
        Ok(flatten_toml(&val, file_path, content))
    }

    /// Parse JSON formatted string.
    pub fn parse_json_str(content: &str, file_path: &Path) -> Result<Vec<KeyEntry>, ConfigError> {
        let val: JsonValue = serde_json::from_str(content)?;
        Ok(flatten_json(&val, file_path, content))
    }

    /// Parse Dotenv (`.env`) formatted string.
    pub fn parse_dotenv_str(
        content: &str,
        file_path: &Path,
    ) -> Result<Vec<KeyEntry>, ConfigError> {
        let mut entries = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            // Skip blank lines and full comment lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Remove optional 'export ' prefix
            let line_to_parse = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();

            if let Some((k, v)) = line_to_parse.split_once('=') {
                let key_name = k.trim().to_string();
                if key_name.is_empty() {
                    continue;
                }

                let value_raw = v.trim();
                let clean_val = strip_env_quotes(value_raw);

                entries.push(KeyEntry::from_segments(
                    file_path.to_path_buf(),
                    vec![key_name],
                    line_num,
                    clean_val,
                    ConfigFormat::Dotenv,
                ));
            }
        }

        Ok(entries)
    }

    /// Discover and parse all supported configuration files in a given directory (non-recursive for top-level configs).
    pub fn parse_dir(dir: &Path) -> Result<Vec<KeyEntry>, ConfigError> {
        let mut all_entries = Vec::new();
        if !dir.is_dir() {
            return Ok(all_entries);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(_) = ConfigFormat::detect(&path) {
                    if let Ok(entries) = Self::parse_file(&path) {
                        all_entries.extend(entries);
                    }
                }
            }
        }

        Ok(all_entries)
    }
}

/// Helper function to strip single or double quotes and inline comments from dotenv values.
fn strip_env_quotes(raw: &str) -> String {
    let mut s = raw.trim();

    // Check for matching double quotes
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        return s[1..s.len() - 1].to_string();
    }

    // Check for matching single quotes
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        return s[1..s.len() - 1].to_string();
    }

    // If unquoted, strip trailing inline comments starting with '#'
    if let Some((val_part, _)) = s.split_once(" #") {
        s = val_part.trim();
    }

    s.to_string()
}
