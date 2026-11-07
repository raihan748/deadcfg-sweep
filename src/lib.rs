//! deadcfg-sweep: AST-powered dead configuration key detector across codebase files with safe auto-prune.

pub mod analyzer;
pub mod config;
pub mod pruner;
pub mod reporter;
pub mod scanner;

pub use analyzer::{AuditFinding, AuditReport, AuditStatus, ConfigAuditor};
pub use config::{ConfigError, ConfigFormat, ConfigParser, KeyEntry};
pub use pruner::{AutoPruner, PruneError, PruneResult};
pub use reporter::{print_prune_results, print_terminal_report, report_to_json};
pub use scanner::{CodeReference, CodeWalker, LanguagePatterns, SourceLanguage, SourceMatcher};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = "deadcfg-sweep";
