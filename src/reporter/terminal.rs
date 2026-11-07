use colored::Colorize;

use crate::analyzer::{AuditFinding, AuditReport, AuditStatus};
use crate::pruner::PruneResult;

/// Renders the audit report to terminal stdout with ANSI colors.
pub fn print_terminal_report(report: &AuditReport) {
    println!();
    println!("{}", "================================================================================".cyan());
    println!("  {}", "deadcfg-sweep: Configuration Key Audit Report".bold().white());
    println!("{}", "================================================================================".cyan());
    println!();

    println!("  {:<24} : {}", "Total Keys Inspected".bold(), report.total_keys);
    println!("  {:<24} : {}", "Active Keys in Code".bold().green(), report.active_keys);
    println!("  {:<24} : {}", "Confirmed Dead Keys".bold().red(), report.dead_keys);
    println!("  {:<24} : {}", "Potential Shadow Keys".bold().yellow(), report.shadow_keys);
    if report.unsynced_keys > 0 {
        println!("  {:<24} : {}", "Unsynchronized Keys".bold().magenta(), report.unsynced_keys);
    }
    println!("  {:<24} : {}", "Source Files Scanned".bold(), report.scanned_source_files);
    println!();

    if report.findings.is_empty() {
        println!("  {}", "No configuration keys were detected.".yellow());
        println!();
        return;
    }

    println!("{}", "  Detailed Findings:".bold().underline());
    println!();

    for finding in &report.findings {
        print_finding_item(finding);
    }

    println!("{}", "--------------------------------------------------------------------------------".cyan());
    if report.dead_keys > 0 {
        println!(
            "  Run with {} to safely prune unreferenced configuration keys.",
            "--prune".bold().yellow()
        );
    } else {
        println!("  {}", "All configuration keys are actively referenced in codebase!".bold().green());
    }
    println!();
}

fn print_finding_item(finding: &AuditFinding) {
    let badge = match finding.status {
        AuditStatus::Active => "[ACTIVE]".bold().green(),
        AuditStatus::ConfirmedDead => "[DEAD]".bold().red(),
        AuditStatus::PotentialShadow => "[SHADOW]".bold().yellow(),
        AuditStatus::Unsynchronized => "[UNSYNC]".bold().magenta(),
    };

    println!(
        "  {} {} {}:{}",
        badge,
        finding.key.dot_path.bold().white(),
        finding.key.file_path.display(),
        finding.key.line
    );
    println!("         Confidence : {:.0}%", finding.confidence * 100.0);
    println!("         Suggestion : {}", finding.suggestion);

    if !finding.references.is_empty() {
        println!("         References ({}):", finding.references.len());
        for r in finding.references.iter().take(3) {
            println!(
                "           - {}:{} [{}] \"{}\"",
                r.file_path.display(),
                r.line,
                r.matched_form.cyan(),
                r.context_line.trim()
            );
        }
        if finding.references.len() > 3 {
            println!("           ... and {} more reference(s)", finding.references.len() - 3);
        }
    }
    println!();
}

/// Renders the pruning operation results to terminal stdout.
pub fn print_prune_results(results: &[PruneResult]) {
    println!();
    println!("{}", "================================================================================".yellow());
    println!("  {}", "deadcfg-sweep: Automated Pruning Summary".bold().white());
    println!("{}", "================================================================================".yellow());
    println!();

    for res in results {
        let mode_label = if res.dry_run {
            "[DRY-RUN]".yellow()
        } else {
            "[APPLIED]".green()
        };

        println!(
            "  {} Pruned {} dead key(s) from {}",
            mode_label,
            res.keys_pruned,
            res.file_path.display().to_string().bold()
        );

        if let Some(bak) = &res.backup_path {
            println!("         Backup created at: {}", bak.display().to_string().cyan());
        }

        if !res.diff.is_empty() {
            println!("         Diff:");
            for line in res.diff.lines().take(12) {
                if line.starts_with('+') {
                    println!("           {}", line.green());
                } else if line.starts_with('-') {
                    println!("           {}", line.red());
                } else {
                    println!("           {}", line);
                }
            }
        }
        println!();
    }
}
