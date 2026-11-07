use std::path::{Path, PathBuf};

use deadcfg_sweep::analyzer::{AuditStatus, ConfigAuditor};
use deadcfg_sweep::config::{ConfigFormat, KeyEntry};

#[test]
fn test_auditor_confirmed_dead_key() {
    let dead_key = KeyEntry::from_segments(
        PathBuf::from("config.yaml"),
        vec!["legacy_feature".to_string(), "enabled".to_string()],
        10,
        "false".to_string(),
        ConfigFormat::Yaml,
    );

    let sources = [(
        Path::new("app.rs"),
        "fn main() { println!(\"no matching tokens here\"); }",
    )];

    let report = ConfigAuditor::audit_keys_with_sources(&[dead_key], &sources);

    assert_eq!(report.dead_keys, 1);
    assert_eq!(report.active_keys, 0);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].status, AuditStatus::ConfirmedDead);
    assert_eq!(report.findings[0].confidence, 1.0);
}

#[test]
fn test_auditor_active_key_exact_match() {
    let active_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["STRIPE_WEBHOOK_SECRET".to_string()],
        15,
        "whsec_xxx".to_string(),
        ConfigFormat::Dotenv,
    );

    let sources = [(
        Path::new("handler.go"),
        "package main\nimport \"os\"\nvar s = os.Getenv(\"STRIPE_WEBHOOK_SECRET\")",
    )];

    let report = ConfigAuditor::audit_keys_with_sources(&[active_key], &sources);

    assert_eq!(report.active_keys, 1);
    assert_eq!(report.dead_keys, 0);
    assert_eq!(report.findings[0].status, AuditStatus::Active);
    assert_eq!(report.findings[0].confidence, 0.0);
}

#[test]
fn test_auditor_potential_shadow_common_leaf() {
    let key = KeyEntry::from_segments(
        PathBuf::from("config.yaml"),
        vec![
            "internal_metrics".to_string(),
            "collector".to_string(),
            "port".to_string(),
        ],
        22,
        "9090".to_string(),
        ConfigFormat::Yaml,
    );

    // Code contains word "port" in unrelated context, but not the full path
    let sources = [(
        Path::new("server.rs"),
        "struct NetworkAddress { port: u16, host: String }",
    )];

    let report = ConfigAuditor::audit_keys_with_sources(&[key], &sources);

    assert_eq!(report.shadow_keys, 1);
    assert_eq!(report.active_keys, 0);
    assert_eq!(report.dead_keys, 0);
    assert_eq!(report.findings[0].status, AuditStatus::PotentialShadow);
    assert!(report.findings[0].confidence >= 0.70);
}

#[test]
fn test_auditor_unsynchronized_env_example() {
    let primary_keys = vec![
        KeyEntry::from_segments(
            PathBuf::from(".env"),
            vec!["DATABASE_URL".to_string()],
            1,
            "pg://".to_string(),
            ConfigFormat::Dotenv,
        ),
        KeyEntry::from_segments(
            PathBuf::from(".env"),
            vec!["SECRET_TOKEN".to_string()],
            2,
            "secret".to_string(),
            ConfigFormat::Dotenv,
        ),
    ];

    let example_keys = vec![KeyEntry::from_segments(
        PathBuf::from(".env.example"),
        vec!["DATABASE_URL".to_string()],
        1,
        "pg://example".to_string(),
        ConfigFormat::Dotenv,
    )];

    let unsynced = ConfigAuditor::check_env_sync(&primary_keys, &example_keys);

    assert_eq!(unsynced.len(), 1);
    assert_eq!(unsynced[0].key.dot_path, "SECRET_TOKEN");
    assert_eq!(unsynced[0].status, AuditStatus::Unsynchronized);
}

#[test]
fn test_auditor_aggregate_report_metrics() {
    let k1 = KeyEntry::from_segments(
        PathBuf::from("cfg.json"),
        vec!["active_one".to_string()],
        1,
        "v1".to_string(),
        ConfigFormat::Json,
    );
    let k2 = KeyEntry::from_segments(
        PathBuf::from("cfg.json"),
        vec!["active_two".to_string()],
        2,
        "v2".to_string(),
        ConfigFormat::Json,
    );
    let k3 = KeyEntry::from_segments(
        PathBuf::from("cfg.json"),
        vec!["dead_one".to_string()],
        3,
        "v3".to_string(),
        ConfigFormat::Json,
    );
    let k4 = KeyEntry::from_segments(
        PathBuf::from("cfg.json"),
        vec!["dead_two".to_string()],
        4,
        "v4".to_string(),
        ConfigFormat::Json,
    );
    let k5 = KeyEntry::from_segments(
        PathBuf::from("cfg.json"),
        vec!["nested".to_string(), "port".to_string()],
        5,
        "v5".to_string(),
        ConfigFormat::Json,
    );

    let sources = [(
        Path::new("app.py"),
        "x = os.environ.get('ACTIVE_ONE')\ny = 'active_two'\nport = 80",
    )];

    let report = ConfigAuditor::audit_keys_with_sources(&[k1, k2, k3, k4, k5], &sources);

    assert_eq!(report.total_keys, 5);
    assert_eq!(report.active_keys, 2);
    assert_eq!(report.dead_keys, 2);
    assert_eq!(report.shadow_keys, 1);
    assert_eq!(report.confirmed_dead_keys().len(), 2);
}
