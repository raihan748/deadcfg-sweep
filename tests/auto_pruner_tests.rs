use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;

use deadcfg_sweep::config::{ConfigFormat, KeyEntry};
use deadcfg_sweep::pruner::{generate_unified_diff, AutoPruner};

#[test]
fn test_prune_dotenv_removes_dead_keys_preserves_comments() {
    let dotenv_raw = r#"# Production environment
DATABASE_URL=postgres://localhost/db
DEAD_SECRET_KEY=obsolete-token
# Cache config
CACHE_TTL=3600
"#;

    let dead_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["DEAD_SECRET_KEY".to_string()],
        3,
        "obsolete-token".to_string(),
        ConfigFormat::Dotenv,
    );

    let pruned = AutoPruner::prune_content(dotenv_raw, ConfigFormat::Dotenv, &[&dead_key])
        .expect("Failed to prune dotenv");

    assert!(!pruned.contains("DEAD_SECRET_KEY"));
    assert!(pruned.contains("DATABASE_URL=postgres://localhost/db"));
    assert!(pruned.contains("# Production environment"));
    assert!(pruned.contains("CACHE_TTL=3600"));
}

#[test]
fn test_prune_json_removes_dead_key_and_cleans_empty_parent() {
    let json_raw = r#"{
  "service": {
    "name": "auth-service"
  },
  "deprecated_section": {
    "dead_token": "legacy"
  }
}"#;

    let dead_key = KeyEntry::from_segments(
        PathBuf::from("config.json"),
        vec!["deprecated_section".to_string(), "dead_token".to_string()],
        6,
        "legacy".to_string(),
        ConfigFormat::Json,
    );

    let pruned = AutoPruner::prune_content(json_raw, ConfigFormat::Json, &[&dead_key])
        .expect("Failed to prune json");

    assert!(!pruned.contains("dead_token"));
    assert!(!pruned.contains("deprecated_section"));
    assert!(pruned.contains("auth-service"));

    // Verify valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&pruned).expect("Pruned JSON must be valid");
    assert!(parsed.get("service").is_some());
    assert!(parsed.get("deprecated_section").is_none());
}

#[test]
fn test_prune_yaml_preserves_valid_syntax() {
    let yaml_raw = r#"
server:
  host: "127.0.0.1"
  unused_flag: false
"#;

    let dead_key = KeyEntry::from_segments(
        PathBuf::from("config.yaml"),
        vec!["server".to_string(), "unused_flag".to_string()],
        4,
        "false".to_string(),
        ConfigFormat::Yaml,
    );

    let pruned = AutoPruner::prune_content(yaml_raw, ConfigFormat::Yaml, &[&dead_key])
        .expect("Failed to prune yaml");

    assert!(!pruned.contains("unused_flag"));
    assert!(pruned.contains("host: 127.0.0.1") || pruned.contains("host: \"127.0.0.1\""));

    // Verify valid YAML
    let parsed: serde_yaml::Value = serde_yaml::from_str(&pruned).expect("Pruned YAML must be valid");
    assert!(parsed.get("server").is_some());
}

#[test]
fn test_prune_toml_table_key() {
    let toml_raw = r#"
[server]
host = "127.0.0.1"
dead_timeout = 90
"#;

    let dead_key = KeyEntry::from_segments(
        PathBuf::from("config.toml"),
        vec!["server".to_string(), "dead_timeout".to_string()],
        4,
        "90".to_string(),
        ConfigFormat::Toml,
    );

    let pruned = AutoPruner::prune_content(toml_raw, ConfigFormat::Toml, &[&dead_key])
        .expect("Failed to prune toml");

    assert!(!pruned.contains("dead_timeout"));
    assert!(pruned.contains("host"));

    // Verify valid TOML
    let parsed: toml::Value = toml::from_str(&pruned).expect("Pruned TOML must be valid");
    assert!(parsed.get("server").is_some());
}

#[test]
fn test_atomic_backup_and_rollback() {
    let temp = NamedTempFile::new().expect("Failed to create temp file");
    let original_text = "INITIAL_DATA=123\n";
    fs::write(temp.path(), original_text).expect("Failed to write initial text");

    let backup_path = AutoPruner::create_atomic_backup(temp.path()).expect("Failed to create backup");
    assert!(backup_path.exists());

    // Tamper with the file
    fs::write(temp.path(), "CORRUPTED_DATA=000\n").expect("Failed to write corrupted text");

    // Perform rollback
    AutoPruner::rollback(&backup_path).expect("Failed to rollback");
    let restored_text = fs::read_to_string(temp.path()).expect("Failed to read restored file");
    assert_eq!(restored_text, original_text);

    // Clean up backup file
    let _ = fs::remove_file(backup_path);

    // Test unified diff output
    let diff = generate_unified_diff("test.txt", "test.txt", "line1\nline2\n", "line1\n");
    assert!(diff.contains("-line2"));
}
