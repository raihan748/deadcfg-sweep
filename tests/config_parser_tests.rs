use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

use deadcfg_sweep::config::{ConfigFormat, ConfigParser};

#[test]
fn test_parse_yaml_nested_structure() {
    let yaml_content = r#"
server:
  host: "127.0.0.1"
  port: 8080
  features:
    metrics: true
    logging: null
"#;
    let entries = ConfigParser::parse_yaml_str(yaml_content, Path::new("config.yaml"))
        .expect("Failed to parse YAML");

    assert_eq!(entries.len(), 4);

    let host_entry = entries.iter().find(|e| e.dot_path == "server.host").unwrap();
    assert_eq!(host_entry.value_repr, "127.0.0.1");
    assert_eq!(host_entry.env_path, "SERVER_HOST");
    assert_eq!(host_entry.depth, 2);

    let port_entry = entries.iter().find(|e| e.dot_path == "server.port").unwrap();
    assert_eq!(port_entry.value_repr, "8080");

    let metrics_entry = entries.iter().find(|e| e.dot_path == "server.features.metrics").unwrap();
    assert_eq!(metrics_entry.value_repr, "true");
    assert_eq!(metrics_entry.depth, 3);
}

#[test]
fn test_parse_toml_tables_and_scalars() {
    let toml_content = r#"
title = "Microservice Config"

[database]
host = "postgres.internal"
max_connections = 50

[database.credentials]
user = "app_svc"
"#;
    let entries = ConfigParser::parse_toml_str(toml_content, Path::new("config.toml"))
        .expect("Failed to parse TOML");

    assert_eq!(entries.len(), 4);

    let title_entry = entries.iter().find(|e| e.dot_path == "title").unwrap();
    assert_eq!(title_entry.depth, 1);
    assert_eq!(title_entry.leaf_key, "title");

    let max_conn = entries
        .iter()
        .find(|e| e.dot_path == "database.max_connections")
        .unwrap();
    assert_eq!(max_conn.env_path, "DATABASE_MAX_CONNECTIONS");
    assert_eq!(max_conn.value_repr, "50");

    let user_entry = entries
        .iter()
        .find(|e| e.dot_path == "database.credentials.user")
        .unwrap();
    assert_eq!(user_entry.double_underscore_path, "DATABASE__CREDENTIALS__USER");
}

#[test]
fn test_parse_json_objects_and_arrays() {
    let json_content = r#"{
  "app": {
    "name": "deadcfg-sweep",
    "version": "1.0.0",
    "retries": 3,
    "enabled": true
  }
}"#;
    let entries = ConfigParser::parse_json_str(json_content, Path::new("config.json"))
        .expect("Failed to parse JSON");

    assert_eq!(entries.len(), 4);

    let name_entry = entries.iter().find(|e| e.dot_path == "app.name").unwrap();
    assert_eq!(name_entry.value_repr, "deadcfg-sweep");
    assert_eq!(name_entry.format, ConfigFormat::Json);

    let retries_entry = entries.iter().find(|e| e.dot_path == "app.retries").unwrap();
    assert_eq!(retries_entry.value_repr, "3");
}

#[test]
fn test_parse_dotenv_with_comments_and_exports() {
    let dotenv_content = r#"
# Server settings
HOST=0.0.0.0
PORT=3000 # inline comment

# Auth secrets
export JWT_SECRET="super-secret-key"
export API_KEY='raw-api-key'

# Blank lines and comments should be skipped
"#;
    let entries = ConfigParser::parse_dotenv_str(dotenv_content, Path::new(".env"))
        .expect("Failed to parse Dotenv");

    assert_eq!(entries.len(), 4);

    let host = entries.iter().find(|e| e.dot_path == "HOST").unwrap();
    assert_eq!(host.value_repr, "0.0.0.0");
    assert_eq!(host.line, 3);

    let port = entries.iter().find(|e| e.dot_path == "PORT").unwrap();
    assert_eq!(port.value_repr, "3000");

    let jwt = entries.iter().find(|e| e.dot_path == "JWT_SECRET").unwrap();
    assert_eq!(jwt.value_repr, "super-secret-key");

    let api_key = entries.iter().find(|e| e.dot_path == "API_KEY").unwrap();
    assert_eq!(api_key.value_repr, "raw-api-key");
}

#[test]
fn test_parse_file_auto_detection() {
    let mut temp = NamedTempFile::with_suffix(".yaml").expect("Failed to create temp file");
    writeln!(temp, "cache:\n  ttl_seconds: 3600").expect("Failed to write to temp file");

    let entries = ConfigParser::parse_file(temp.path()).expect("Failed to parse file");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].dot_path, "cache.ttl_seconds");
    assert_eq!(entries[0].env_path, "CACHE_TTL_SECONDS");
    assert_eq!(entries[0].format, ConfigFormat::Yaml);
}
