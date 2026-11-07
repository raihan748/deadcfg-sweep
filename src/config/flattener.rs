use std::path::Path;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

use super::key_entry::{ConfigFormat, KeyEntry};

/// Flattens a `serde_json::Value` tree into a list of leaf `KeyEntry` items.
pub fn flatten_json(
    root: &JsonValue,
    file_path: &Path,
    raw_content: &str,
) -> Vec<KeyEntry> {
    let mut entries = Vec::new();
    let mut current_path = Vec::new();
    walk_json(root, &mut current_path, file_path, raw_content, &mut entries);
    entries
}

fn walk_json(
    val: &JsonValue,
    current_path: &mut Vec<String>,
    file_path: &Path,
    raw_content: &str,
    entries: &mut Vec<KeyEntry>,
) {
    match val {
        JsonValue::Object(map) => {
            for (k, v) in map {
                current_path.push(k.clone());
                walk_json(v, current_path, file_path, raw_content, entries);
                current_path.pop();
            }
        }
        JsonValue::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                current_path.push(idx.to_string());
                walk_json(item, current_path, file_path, raw_content, entries);
                current_path.pop();
            }
        }
        _ => {
            if !current_path.is_empty() {
                let line = find_key_line(raw_content, current_path, ConfigFormat::Json);
                let value_repr = match val {
                    JsonValue::String(s) => s.clone(),
                    other => other.to_string(),
                };
                entries.push(KeyEntry::from_segments(
                    file_path.to_path_buf(),
                    current_path.clone(),
                    line,
                    value_repr,
                    ConfigFormat::Json,
                ));
            }
        }
    }
}

/// Flattens a `serde_yaml::Value` tree into a list of leaf `KeyEntry` items.
pub fn flatten_yaml(
    root: &YamlValue,
    file_path: &Path,
    raw_content: &str,
) -> Vec<KeyEntry> {
    let mut entries = Vec::new();
    let mut current_path = Vec::new();
    walk_yaml(root, &mut current_path, file_path, raw_content, &mut entries);
    entries
}

fn walk_yaml(
    val: &YamlValue,
    current_path: &mut Vec<String>,
    file_path: &Path,
    raw_content: &str,
    entries: &mut Vec<KeyEntry>,
) {
    match val {
        YamlValue::Mapping(map) => {
            for (k, v) in map {
                let key_str = match k {
                    YamlValue::String(s) => s.clone(),
                    YamlValue::Number(n) => n.to_string(),
                    YamlValue::Bool(b) => b.to_string(),
                    _ => format!("{:?}", k),
                };
                current_path.push(key_str);
                walk_yaml(v, current_path, file_path, raw_content, entries);
                current_path.pop();
            }
        }
        YamlValue::Sequence(seq) => {
            for (idx, item) in seq.iter().enumerate() {
                current_path.push(idx.to_string());
                walk_yaml(item, current_path, file_path, raw_content, entries);
                current_path.pop();
            }
        }
        _ => {
            if !current_path.is_empty() {
                let line = find_key_line(raw_content, current_path, ConfigFormat::Yaml);
                let value_repr = match val {
                    YamlValue::String(s) => s.clone(),
                    YamlValue::Null => "null".to_string(),
                    YamlValue::Bool(b) => b.to_string(),
                    YamlValue::Number(n) => n.to_string(),
                    _ => format!("{:?}", val),
                };
                entries.push(KeyEntry::from_segments(
                    file_path.to_path_buf(),
                    current_path.clone(),
                    line,
                    value_repr,
                    ConfigFormat::Yaml,
                ));
            }
        }
    }
}

/// Flattens a `toml::Value` tree into a list of leaf `KeyEntry` items.
pub fn flatten_toml(
    root: &TomlValue,
    file_path: &Path,
    raw_content: &str,
) -> Vec<KeyEntry> {
    let mut entries = Vec::new();
    let mut current_path = Vec::new();
    walk_toml(root, &mut current_path, file_path, raw_content, &mut entries);
    entries
}

fn walk_toml(
    val: &TomlValue,
    current_path: &mut Vec<String>,
    file_path: &Path,
    raw_content: &str,
    entries: &mut Vec<KeyEntry>,
) {
    match val {
        TomlValue::Table(table) => {
            for (k, v) in table {
                current_path.push(k.clone());
                walk_toml(v, current_path, file_path, raw_content, entries);
                current_path.pop();
            }
        }
        TomlValue::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                current_path.push(idx.to_string());
                walk_toml(item, current_path, file_path, raw_content, entries);
                current_path.pop();
            }
        }
        _ => {
            if !current_path.is_empty() {
                let line = find_key_line(raw_content, current_path, ConfigFormat::Toml);
                let value_repr = match val {
                    TomlValue::String(s) => s.clone(),
                    TomlValue::Integer(i) => i.to_string(),
                    TomlValue::Float(f) => f.to_string(),
                    TomlValue::Boolean(b) => b.to_string(),
                    TomlValue::Datetime(d) => d.to_string(),
                    _ => val.to_string(),
                };
                entries.push(KeyEntry::from_segments(
                    file_path.to_path_buf(),
                    current_path.clone(),
                    line,
                    value_repr,
                    ConfigFormat::Toml,
                ));
            }
        }
    }
}

/// Locates the 1-indexed line number of a given key path in the raw config string.
pub fn find_key_line(raw: &str, segments: &[String], format: ConfigFormat) -> usize {
    if segments.is_empty() {
        return 1;
    }

    let leaf = match segments.last() {
        Some(s) => s,
        None => return 1,
    };

    let lines: Vec<&str> = raw.lines().collect();

    match format {
        ConfigFormat::Dotenv => {
            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }
                let without_export = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
                if let Some((k, _)) = without_export.split_once('=') {
                    if k.trim() == leaf {
                        return idx + 1;
                    }
                }
            }
        }
        ConfigFormat::Json => {
            let pattern = format!("\"{}\"", leaf);
            for (idx, line) in lines.iter().enumerate() {
                if line.contains(&pattern) {
                    return idx + 1;
                }
            }
        }
        ConfigFormat::Yaml => {
            // Find key with colon, e.g. "leaf:" or "leaf: value"
            let target = format!("{}:", leaf);
            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with(&target) {
                    return idx + 1;
                }
            }
            // Fallback check
            for (idx, line) in lines.iter().enumerate() {
                if line.contains(leaf) && line.contains(':') {
                    return idx + 1;
                }
            }
        }
        ConfigFormat::Toml => {
            // Check for key = value in current section
            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == leaf {
                        return idx + 1;
                    }
                }
            }
        }
    }

    1
}
