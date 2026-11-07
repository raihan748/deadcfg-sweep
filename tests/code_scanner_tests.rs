use std::path::{Path, PathBuf};

use deadcfg_sweep::config::{ConfigFormat, KeyEntry};
use deadcfg_sweep::scanner::SourceMatcher;

#[test]
fn test_scanner_rust_env_var() {
    let rust_code = r#"
fn main() {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    println!("{}:{}", db_url, port);
}
"#;
    let key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["DATABASE_URL".to_string()],
        1,
        "postgres://localhost".to_string(),
        ConfigFormat::Dotenv,
    );

    let refs = SourceMatcher::scan_for_key(rust_code, Path::new("main.rs"), &key);
    assert!(!refs.is_empty());
    assert!(refs.iter().any(|r| r.is_programmatic_api));
    assert!(refs[0].matched_form == "dot_path" || refs[0].matched_form == "leaf_key");
}

#[test]
fn test_scanner_go_os_getenv_and_viper() {
    let go_code = r#"
package main

import (
    "os"
    "github.com/spf13/viper"
)

func initConfig() {
    secretKey := os.Getenv("SECRET_KEY")
    dbHost := viper.GetString("database.host")
    _ = secretKey
    _ = dbHost
}
"#;
    let secret_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["SECRET_KEY".to_string()],
        2,
        "shh".to_string(),
        ConfigFormat::Dotenv,
    );
    let refs_secret = SourceMatcher::scan_for_key(go_code, Path::new("config.go"), &secret_key);
    assert!(!refs_secret.is_empty());
    assert!(refs_secret.iter().any(|r| r.is_programmatic_api));

    let db_host_key = KeyEntry::from_segments(
        PathBuf::from("config.yaml"),
        vec!["database".to_string(), "host".to_string()],
        12,
        "localhost".to_string(),
        ConfigFormat::Yaml,
    );
    let refs_db = SourceMatcher::scan_for_key(go_code, Path::new("config.go"), &db_host_key);
    assert!(!refs_db.is_empty());
    assert!(refs_db.iter().any(|r| r.matched_form == "dot_path"));
}

#[test]
fn test_scanner_python_environ_and_settings() {
    let py_code = r#"
import os
from django.conf import settings

def load_settings():
    api_token = os.environ.get("API_TOKEN", "default")
    db_url = settings.DATABASE_URL
    return api_token, db_url
"#;
    let api_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["API_TOKEN".to_string()],
        5,
        "token123".to_string(),
        ConfigFormat::Dotenv,
    );
    let refs_api = SourceMatcher::scan_for_key(py_code, Path::new("service.py"), &api_key);
    assert!(!refs_api.is_empty());
    assert!(refs_api.iter().any(|r| r.is_programmatic_api));

    let db_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["DATABASE_URL".to_string()],
        6,
        "pg://".to_string(),
        ConfigFormat::Dotenv,
    );
    let refs_db = SourceMatcher::scan_for_key(py_code, Path::new("service.py"), &db_key);
    assert!(!refs_db.is_empty());
}

#[test]
fn test_scanner_js_process_env_and_meta() {
    let ts_code = r#"
export function getRuntimeConfig() {
    const nodeEnv = process.env.NODE_ENV;
    const dbUrl = process.env["DATABASE_URL"];
    const title = import.meta.env.VITE_APP_TITLE;
    return { nodeEnv, dbUrl, title };
}
"#;
    let node_env_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["NODE_ENV".to_string()],
        1,
        "production".to_string(),
        ConfigFormat::Dotenv,
    );
    let refs = SourceMatcher::scan_for_key(ts_code, Path::new("env.ts"), &node_env_key);
    assert!(!refs.is_empty());
    assert!(refs.iter().any(|r| r.is_programmatic_api));

    let title_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["VITE_APP_TITLE".to_string()],
        3,
        "My App".to_string(),
        ConfigFormat::Dotenv,
    );
    let refs_title = SourceMatcher::scan_for_key(ts_code, Path::new("env.ts"), &title_key);
    assert!(!refs_title.is_empty());
}

#[test]
fn test_scanner_shell_variable_references() {
    let sh_code = r#"
#!/usr/bin/env bash
echo "Connecting to $DATABASE_URL on port ${CACHE_TTL}"
"#;
    let db_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["DATABASE_URL".to_string()],
        1,
        "postgres://".to_string(),
        ConfigFormat::Dotenv,
    );
    let refs_db = SourceMatcher::scan_for_key(sh_code, Path::new("deploy.sh"), &db_key);
    assert!(!refs_db.is_empty());

    let cache_key = KeyEntry::from_segments(
        PathBuf::from(".env"),
        vec!["CACHE_TTL".to_string()],
        2,
        "300".to_string(),
        ConfigFormat::Dotenv,
    );
    let refs_cache = SourceMatcher::scan_for_key(sh_code, Path::new("deploy.sh"), &cache_key);
    assert!(!refs_cache.is_empty());
}
