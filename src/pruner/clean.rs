use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

use crate::config::{ConfigFormat, KeyEntry};
use super::diff::generate_unified_diff;

/// Errors that can occur during configuration key pruning.
#[derive(Debug)]
pub enum PruneError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Yaml(serde_yaml::Error),
    Toml(toml::de::Error),
    TomlSer(toml::ser::Error),
    UnsupportedFormat,
}

impl std::fmt::Display for PruneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PruneError::Io(e) => write!(f, "I/O error: {}", e),
            PruneError::Json(e) => write!(f, "JSON error: {}", e),
            PruneError::Yaml(e) => write!(f, "YAML error: {}", e),
            PruneError::Toml(e) => write!(f, "TOML de error: {}", e),
            PruneError::TomlSer(e) => write!(f, "TOML ser error: {}", e),
            PruneError::UnsupportedFormat => write!(f, "Unsupported format for pruning"),
        }
    }
}

impl std::error::Error for PruneError {}

impl From<std::io::Error> for PruneError {
    fn from(e: std::io::Error) -> Self {
        PruneError::Io(e)
    }
}

impl From<serde_json::Error> for PruneError {
    fn from(e: serde_json::Error) -> Self {
        PruneError::Json(e)
    }
}

impl From<serde_yaml::Error> for PruneError {
    fn from(e: serde_yaml::Error) -> Self {
        PruneError::Yaml(e)
    }
}

impl From<toml::de::Error> for PruneError {
    fn from(e: toml::de::Error) -> Self {
        PruneError::Toml(e)
    }
}

impl From<toml::ser::Error> for PruneError {
    fn from(e: toml::ser::Error) -> Self {
        PruneError::TomlSer(e)
    }
}

/// Summary result of a file pruning operation.
#[derive(Debug, Clone, PartialEq)]
pub struct PruneResult {
    pub file_path: PathBuf,
    pub keys_pruned: usize,
    pub diff: String,
    pub backup_path: Option<PathBuf>,
    pub dry_run: bool,
}

/// Automatic safe pruner for removing unreferenced configuration keys.
pub struct AutoPruner;

impl AutoPruner {
    /// Prune dead keys from a configuration file on disk.
    pub fn prune_file(
        file_path: &Path,
        dead_keys: &[&KeyEntry],
        dry_run: bool,
        create_backup: bool,
    ) -> Result<PruneResult, PruneError> {
        let format = ConfigFormat::detect(file_path).ok_or(PruneError::UnsupportedFormat)?;
        let original_content = fs::read_to_string(file_path)?;

        let pruned_content = Self::prune_content(&original_content, format, dead_keys)?;
        let diff = generate_unified_diff(
            &file_path.display().to_string(),
            &file_path.display().to_string(),
            &original_content,
            &pruned_content,
        );

        let mut backup_path = None;

        if !dry_run && original_content != pruned_content {
            if create_backup {
                backup_path = Some(Self::create_atomic_backup(file_path)?);
            }
            fs::write(file_path, &pruned_content)?;
        }

        Ok(PruneResult {
            file_path: file_path.to_path_buf(),
            keys_pruned: dead_keys.len(),
            diff,
            backup_path,
            dry_run,
        })
    }

    /// Prune dead keys from an in-memory configuration string.
    pub fn prune_content(
        raw: &str,
        format: ConfigFormat,
        dead_keys: &[&KeyEntry],
    ) -> Result<String, PruneError> {
        if dead_keys.is_empty() {
            return Ok(raw.to_string());
        }

        match format {
            ConfigFormat::Dotenv => Ok(Self::prune_dotenv(raw, dead_keys)),
            ConfigFormat::Json => Self::prune_json(raw, dead_keys),
            ConfigFormat::Yaml => Self::prune_yaml(raw, dead_keys),
            ConfigFormat::Toml => Self::prune_toml(raw, dead_keys),
        }
    }

    /// Create an atomic timestamped backup copy: `<file>.bak.<timestamp>`.
    pub fn create_atomic_backup(file_path: &Path) -> Result<PathBuf, std::io::Error> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let backup_name = format!(
            "{}.bak.{}",
            file_path.file_name().unwrap_or_default().to_string_lossy(),
            ts
        );
        let backup_path = file_path.with_file_name(backup_name);
        fs::copy(file_path, &backup_path)?;
        Ok(backup_path)
    }

    /// Restore file from a backup copy.
    pub fn rollback(backup_path: &Path) -> Result<(), std::io::Error> {
        let original_name = backup_path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.split(".bak.").next())
            .unwrap_or_default();

        let original_path = backup_path.with_file_name(original_name);
        fs::copy(backup_path, &original_path)?;
        Ok(())
    }

    fn prune_dotenv(raw: &str, dead_keys: &[&KeyEntry]) -> String {
        let dead_names: Vec<&str> = dead_keys.iter().map(|k| k.leaf_key.as_str()).collect();
        let mut result_lines = Vec::new();

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                result_lines.push(line);
                continue;
            }

            let without_export = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
            if let Some((k, _)) = without_export.split_once('=') {
                if dead_names.contains(&k.trim()) {
                    // Skip dead key line
                    continue;
                }
            }

            result_lines.push(line);
        }

        let mut output = result_lines.join("\n");
        if raw.ends_with('\n') {
            output.push('\n');
        }
        output
    }

    fn prune_json(raw: &str, dead_keys: &[&KeyEntry]) -> Result<String, PruneError> {
        let mut root: JsonValue = serde_json::from_str(raw)?;

        for key in dead_keys {
            remove_json_path(&mut root, &key.path_segments);
        }

        clean_empty_json(&mut root);
        let mut formatted = serde_json::to_string_pretty(&root)?;
        if raw.ends_with('\n') {
            formatted.push('\n');
        }
        Ok(formatted)
    }

    fn prune_yaml(raw: &str, dead_keys: &[&KeyEntry]) -> Result<String, PruneError> {
        let mut root: YamlValue = serde_yaml::from_str(raw)?;

        for key in dead_keys {
            remove_yaml_path(&mut root, &key.path_segments);
        }

        clean_empty_yaml(&mut root);
        let mut formatted = serde_yaml::to_string(&root)?;
        if !raw.ends_with('\n') && formatted.ends_with('\n') {
            formatted.pop();
        }
        Ok(formatted)
    }

    fn prune_toml(raw: &str, dead_keys: &[&KeyEntry]) -> Result<String, PruneError> {
        let mut root: TomlValue = toml::from_str(raw)?;

        for key in dead_keys {
            remove_toml_path(&mut root, &key.path_segments);
        }

        clean_empty_toml(&mut root);
        let formatted = toml::to_string_pretty(&root)?;
        Ok(formatted)
    }
}

// JSON removal helpers
fn remove_json_path(val: &mut JsonValue, segments: &[String]) {
    if segments.is_empty() {
        return;
    }

    if segments.len() == 1 {
        if let JsonValue::Object(map) = val {
            map.remove(&segments[0]);
        }
        return;
    }

    if let JsonValue::Object(map) = val {
        if let Some(child) = map.get_mut(&segments[0]) {
            remove_json_path(child, &segments[1..]);
        }
    }
}

fn clean_empty_json(val: &mut JsonValue) {
    if let JsonValue::Object(map) = val {
        let keys: Vec<String> = map.keys().cloned().collect();
        for k in keys {
            if let Some(child) = map.get_mut(&k) {
                clean_empty_json(child);
                let is_empty = match child {
                    JsonValue::Object(m) => m.is_empty(),
                    JsonValue::Array(a) => a.is_empty(),
                    _ => false,
                };
                if is_empty {
                    map.remove(&k);
                }
            }
        }
    }
}

// YAML removal helpers
fn remove_yaml_path(val: &mut YamlValue, segments: &[String]) {
    if segments.is_empty() {
        return;
    }

    let leaf_key = YamlValue::String(segments[0].clone());

    if segments.len() == 1 {
        if let YamlValue::Mapping(map) = val {
            map.remove(&leaf_key);
        }
        return;
    }

    if let YamlValue::Mapping(map) = val {
        if let Some(child) = map.get_mut(&leaf_key) {
            remove_yaml_path(child, &segments[1..]);
        }
    }
}

fn clean_empty_yaml(val: &mut YamlValue) {
    if let YamlValue::Mapping(map) = val {
        let keys: Vec<YamlValue> = map.keys().cloned().collect();
        for k in keys {
            if let Some(child) = map.get_mut(&k) {
                clean_empty_yaml(child);
                let is_empty = match child {
                    YamlValue::Mapping(m) => m.is_empty(),
                    YamlValue::Sequence(s) => s.is_empty(),
                    _ => false,
                };
                if is_empty {
                    map.remove(&k);
                }
            }
        }
    }
}

// TOML removal helpers
fn remove_toml_path(val: &mut TomlValue, segments: &[String]) {
    if segments.is_empty() {
        return;
    }

    if segments.len() == 1 {
        if let TomlValue::Table(map) = val {
            map.remove(&segments[0]);
        }
        return;
    }

    if let TomlValue::Table(map) = val {
        if let Some(child) = map.get_mut(&segments[0]) {
            remove_toml_path(child, &segments[1..]);
        }
    }
}

fn clean_empty_toml(val: &mut TomlValue) {
    if let TomlValue::Table(map) = val {
        let keys: Vec<String> = map.keys().cloned().collect();
        for k in keys {
            if let Some(child) = map.get_mut(&k) {
                clean_empty_toml(child);
                let is_empty = match child {
                    TomlValue::Table(m) => m.is_empty(),
                    TomlValue::Array(a) => a.is_empty(),
                    _ => false,
                };
                if is_empty {
                    map.remove(&k);
                }
            }
        }
    }
}
