use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use clap::Parser;

use deadcfg_sweep::{
    print_prune_results, print_terminal_report, report_to_json, AuditStatus, AutoPruner,
    CodeWalker, ConfigAuditor, ConfigParser, KeyEntry,
};

#[derive(Parser, Debug)]
#[command(
    name = "deadcfg-sweep",
    author = "SMK Telkom Developer <dev@smktelkom-mks.sch.id>",
    version = env!("CARGO_PKG_VERSION"),
    about = "AST-powered dead configuration key detector across codebase files with safe auto-prune",
    long_about = "deadcfg-sweep parses hierarchical configuration files (YAML, TOML, JSON, Dotenv),\nscans source code files across multiple languages for active configuration key usages,\nidentifies unreferenced dead keys, and provides automated safe pruning with atomic backup rollback."
)]
struct Cli {
    /// Configuration file or directory to audit (default: auto-detects .env, config.yaml, config.toml, config.json in working dir)
    #[arg(short, long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Source code directory to scan for configuration references
    #[arg(short = 's', long, default_value = ".", value_name = "DIR")]
    code: PathBuf,

    /// Automatically prune confirmed dead configuration keys
    #[arg(long)]
    prune: bool,

    /// Show simulated pruning diff without modifying any files on disk
    #[arg(long)]
    dry_run: bool,

    /// Minimum confidence threshold for pruning [0.0 - 1.0] (default: 0.8)
    #[arg(long, default_value_t = 0.8)]
    min_confidence: f64,

    /// Output report in structured JSON format to stdout
    #[arg(long)]
    json: bool,

    /// Save report to a specified file path
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Check key synchronization between .env and .env.example
    #[arg(long)]
    check_env_example: bool,

    /// Disable atomic backup copy generation before pruning
    #[arg(long)]
    no_backup: bool,
}

fn main() -> ExitCode {
    let args = Cli::parse();

    // 1. Discover configuration keys
    let config_keys = match discover_config_keys(args.config.as_deref()) {
        Ok(keys) => keys,
        Err(e) => {
            eprintln!("Error discovering configuration keys: {}", e);
            return ExitCode::from(1);
        }
    };

    if config_keys.is_empty() {
        eprintln!("No configuration keys found to audit. Please specify a config file via --config <PATH>");
        return ExitCode::from(1);
    }

    // 2. Discover source code files
    let source_files = CodeWalker::collect_source_files(&args.code);

    // 3. Execute configuration audit
    let mut report = ConfigAuditor::audit_keys(&config_keys, &source_files);

    // 4. Check .env.example synchronization if requested
    if args.check_env_example {
        let env_path = Path::new(".env");
        let example_path = Path::new(".env.example");

        if env_path.exists() && example_path.exists() {
            if let (Ok(primaries), Ok(examples)) = (
                ConfigParser::parse_file(env_path),
                ConfigParser::parse_file(example_path),
            ) {
                let unsynced = ConfigAuditor::check_env_sync(&primaries, &examples);
                report.unsynced_keys = unsynced.len();
                report.findings.extend(unsynced);
            }
        }
    }

    // 5. Handle output formatting
    if args.json {
        match report_to_json(&report) {
            Ok(json_str) => println!("{}", json_str),
            Err(e) => {
                eprintln!("Failed to format JSON report: {}", e);
                return ExitCode::from(1);
            }
        }
    } else {
        print_terminal_report(&report);
    }

    if let Some(out_path) = &args.output {
        if let Ok(json_str) = report_to_json(&report) {
            if let Err(e) = fs::write(out_path, json_str) {
                eprintln!("Failed to write report to {}: {}", out_path.display(), e);
            }
        }
    }

    // 6. Handle pruning if requested
    if args.prune || args.dry_run {
        let mut grouped_by_file: HashMap<PathBuf, Vec<&KeyEntry>> = HashMap::new();

        for finding in &report.findings {
            if finding.status == AuditStatus::ConfirmedDead && finding.confidence >= args.min_confidence {
                grouped_by_file
                    .entry(finding.key.file_path.clone())
                    .or_default()
                    .push(&finding.key);
            }
        }

        let mut prune_results = Vec::new();
        let create_backup = !args.no_backup;

        for (file_path, dead_keys) in grouped_by_file {
            match AutoPruner::prune_file(&file_path, &dead_keys, args.dry_run, create_backup) {
                Ok(result) => prune_results.push(result),
                Err(e) => eprintln!("Error pruning {}: {}", file_path.display(), e),
            }
        }

        if !args.json {
            print_prune_results(&prune_results);
        }
    }

    if report.dead_keys > 0 && !args.prune && !args.dry_run {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Helper function to discover configuration keys from explicit path or common locations.
fn discover_config_keys(config_path: Option<&Path>) -> Result<Vec<KeyEntry>, Box<dyn std::error::Error>> {
    if let Some(path) = config_path {
        if path.is_file() {
            let entries = ConfigParser::parse_file(path)?;
            return Ok(entries);
        } else if path.is_dir() {
            let entries = ConfigParser::parse_dir(path)?;
            return Ok(entries);
        } else {
            return Err(format!("Specified config path does not exist: {}", path.display()).into());
        }
    }

    // Look for default candidates in current directory
    let candidates = [
        "config.yaml",
        "config.yml",
        "config.toml",
        "config.json",
        ".env",
        "application.yml",
        "application.yaml",
    ];

    let mut all_entries = Vec::new();
    for candidate in candidates {
        let p = Path::new(candidate);
        if p.is_file() {
            if let Ok(entries) = ConfigParser::parse_file(p) {
                all_entries.extend(entries);
            }
        }
    }

    Ok(all_entries)
}
