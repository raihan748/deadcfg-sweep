//! deadcfg-sweep: AST-powered dead configuration key detector across codebase files with safe auto-prune.

pub mod config;

pub use config::{ConfigError, ConfigFormat, ConfigParser, KeyEntry};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &str = "deadcfg-sweep";
