use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::config::KeyEntry;
use crate::scanner::{CodeReference, SourceMatcher};
use super::confidence::AuditStatus;

/// An individual audit finding for a configuration key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditFinding {
    pub key: KeyEntry,
    pub status: AuditStatus,
    pub confidence: f64,
    pub references: Vec<CodeReference>,
    pub suggestion: String,
}

/// Aggregated report of an entire configuration audit run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditReport {
    pub findings: Vec<AuditFinding>,
    pub total_keys: usize,
    pub active_keys: usize,
    pub dead_keys: usize,
    pub shadow_keys: usize,
    pub unsynced_keys: usize,
    pub scanned_source_files: usize,
}

impl AuditReport {
    /// Filter for all keys classified as confirmed dead.
    pub fn confirmed_dead_keys(&self) -> Vec<&KeyEntry> {
        self.findings
            .iter()
            .filter(|f| f.status == AuditStatus::ConfirmedDead)
            .map(|f| &f.key)
            .collect()
    }

    /// Filter findings with confidence score greater than or equal to threshold.
    pub fn findings_above_confidence(&self, threshold: f64) -> Vec<&AuditFinding> {
        self.findings
            .iter()
            .filter(|f| f.confidence >= threshold)
            .collect()
    }
}

/// Engine to audit configuration keys against codebase references.
pub struct ConfigAuditor;

impl ConfigAuditor {
    /// Audit a set of configuration keys across a list of source code files on disk.
    pub fn audit_keys(keys: &[KeyEntry], code_files: &[PathBuf]) -> AuditReport {
        let mut loaded_sources: Vec<(PathBuf, String)> = Vec::with_capacity(code_files.len());
        for p in code_files {
            if let Ok(content) = fs::read_to_string(p) {
                loaded_sources.push((p.clone(), content));
            }
        }

        let slice_ref: Vec<(&Path, &str)> = loaded_sources
            .iter()
            .map(|(p, c)| (p.as_path(), c.as_str()))
            .collect();

        Self::audit_keys_with_sources(keys, &slice_ref)
    }

    /// Audit configuration keys against preloaded in-memory source file representations.
    pub fn audit_keys_with_sources(
        keys: &[KeyEntry],
        sources: &[(&Path, &str)],
    ) -> AuditReport {
        let mut findings = Vec::new();
        let mut active_count = 0;
        let mut dead_count = 0;
        let mut shadow_count = 0;

        for key in keys {
            let mut key_refs = Vec::new();

            for (path, content) in sources {
                let refs = SourceMatcher::scan_for_key(content, path, key);
                key_refs.extend(refs);
            }

            // Determine whether matches are exact path / programmatic vs generic leaf
            let has_programmatic = key_refs.iter().any(|r| r.is_programmatic_api);
            let has_exact_path = key_refs
                .iter()
                .any(|r| r.matched_form == "dot_path" || r.matched_form == "env_path" || r.matched_form == "double_underscore_path");

            let (status, confidence, suggestion) = if has_programmatic || has_exact_path {
                active_count += 1;
                (
                    AuditStatus::Active,
                    0.0,
                    format!("Key is actively referenced across {} location(s)", key_refs.len()),
                )
            } else if key_refs.is_empty() {
                dead_count += 1;
                (
                    AuditStatus::ConfirmedDead,
                    1.0,
                    format!("Safely prune unused configuration key '{}'", key.dot_path),
                )
            } else {
                // Only leaf key references matched without exact hierarchy
                shadow_count += 1;
                let conf = if is_common_leaf_identifier(&key.leaf_key) {
                    0.75
                } else {
                    0.60
                };
                (
                    AuditStatus::PotentialShadow,
                    conf,
                    format!(
                        "Leaf key '{}' appeared in {} generic code identifier(s), but exact path is unreferenced",
                        key.leaf_key,
                        key_refs.len()
                    ),
                )
            };

            findings.push(AuditFinding {
                key: key.clone(),
                status,
                confidence,
                references: key_refs,
                suggestion,
            });
        }

        AuditReport {
            findings,
            total_keys: keys.len(),
            active_keys: active_count,
            dead_keys: dead_count,
            shadow_keys: shadow_count,
            unsynced_keys: 0,
            scanned_source_files: sources.len(),
        }
    }

    /// Check synchronization between primary configuration (e.g. `.env`) and template (e.g. `.env.example`).
    pub fn check_env_sync(
        primary_keys: &[KeyEntry],
        example_keys: &[KeyEntry],
    ) -> Vec<AuditFinding> {
        let mut unsynced = Vec::new();
        let example_set: std::collections::HashSet<&str> =
            example_keys.iter().map(|k| k.dot_path.as_str()).collect();

        for key in primary_keys {
            if !example_set.contains(key.dot_path.as_str()) {
                unsynced.push(AuditFinding {
                    key: key.clone(),
                    status: AuditStatus::Unsynchronized,
                    confidence: 1.0,
                    references: Vec::new(),
                    suggestion: format!(
                        "Add missing key '{}' to template/example configuration",
                        key.dot_path
                    ),
                });
            }
        }

        unsynced
    }
}

/// Identifies if a leaf key is a ubiquitous name frequently used in unrelated variables.
fn is_common_leaf_identifier(leaf: &str) -> bool {
    matches!(
        leaf.to_lowercase().as_str(),
        "id" | "name"
            | "type"
            | "port"
            | "host"
            | "path"
            | "url"
            | "enabled"
            | "disabled"
            | "key"
            | "value"
            | "mode"
            | "data"
            | "count"
            | "timeout"
            | "max"
            | "min"
    )
}
