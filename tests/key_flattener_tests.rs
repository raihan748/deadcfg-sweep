use std::path::PathBuf;

use deadcfg_sweep::config::{ConfigFormat, KeyEntry};

#[test]
fn test_flattener_dot_notation() {
    let entry = KeyEntry::from_segments(
        PathBuf::from("config.yaml"),
        vec![
            "cloud".to_string(),
            "aws".to_string(),
            "s3".to_string(),
            "bucket_name".to_string(),
        ],
        10,
        "my-assets".to_string(),
        ConfigFormat::Yaml,
    );

    assert_eq!(entry.dot_path, "cloud.aws.s3.bucket_name");
    assert_eq!(entry.depth, 4);
}

#[test]
fn test_flattener_screaming_snake_case_env() {
    let entry = KeyEntry::from_segments(
        PathBuf::from("config.json"),
        vec![
            "database-cluster".to_string(),
            "read-replica-pool".to_string(),
            "max-idle-connections".to_string(),
        ],
        15,
        "25".to_string(),
        ConfigFormat::Json,
    );

    assert_eq!(
        entry.env_path,
        "DATABASE_CLUSTER_READ_REPLICA_POOL_MAX_IDLE_CONNECTIONS"
    );
}

#[test]
fn test_flattener_double_underscore_notation() {
    let entry = KeyEntry::from_segments(
        PathBuf::from("config.toml"),
        vec![
            "auth".to_string(),
            "oauth2".to_string(),
            "client_secret".to_string(),
        ],
        8,
        "secret-val".to_string(),
        ConfigFormat::Toml,
    );

    assert_eq!(
        entry.double_underscore_path,
        "AUTH__OAUTH2__CLIENT_SECRET"
    );
}

#[test]
fn test_flattener_leaf_key_extraction() {
    let entry = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["LOG_LEVEL".to_string()],
        1,
        "DEBUG".to_string(),
        ConfigFormat::Dotenv,
    );

    assert_eq!(entry.leaf_key, "LOG_LEVEL");
    assert_eq!(entry.depth, 1);
}

#[test]
fn test_all_lookup_forms_deduplication() {
    let entry = KeyEntry::from_segments(
        PathBuf::from("config.yaml"),
        vec!["database".to_string(), "port".to_string()],
        5,
        "5432".to_string(),
        ConfigFormat::Yaml,
    );

    let forms = entry.all_lookup_forms();
    assert!(forms.contains(&"database.port".to_string()));
    assert!(forms.contains(&"DATABASE_PORT".to_string()));
    assert!(forms.contains(&"DATABASE__PORT".to_string()));
    assert!(forms.contains(&"port".to_string()));

    // Ensure no duplicates exist in returned lookup forms
    let mut deduped = forms.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(forms.len(), deduped.len());
}
