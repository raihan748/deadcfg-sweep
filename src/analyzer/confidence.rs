use serde::{Deserialize, Serialize};

/// Audit classification status of a configuration key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    /// Confirmed active configuration key referenced in codebase.
    Active,
    /// Confirmed dead configuration key: 0 references found across any lookup form.
    ConfirmedDead,
    /// Potential shadow/collision: leaf key exists in generic code, but exact path has 0 references.
    PotentialShadow,
    /// Present in primary configuration (e.g. `.env`) but absent from template (e.g. `.env.example`).
    Unsynchronized,
}

impl AuditStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditStatus::Active => "ACTIVE",
            AuditStatus::ConfirmedDead => "CONFIRMED_DEAD",
            AuditStatus::PotentialShadow => "POTENTIAL_SHADOW",
            AuditStatus::Unsynchronized => "UNSYNCHRONIZED",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            AuditStatus::Active => "[ACTIVE]",
            AuditStatus::ConfirmedDead => "[DEAD]",
            AuditStatus::PotentialShadow => "[SHADOW]",
            AuditStatus::Unsynchronized => "[UNSYNC]",
        }
    }
}
