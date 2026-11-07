use crate::analyzer::AuditReport;

/// Serializes an `AuditReport` into a pretty-printed JSON string.
pub fn report_to_json(report: &AuditReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}
