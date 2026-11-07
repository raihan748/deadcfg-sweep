use std::fmt::Write;

/// Generates a unified diff format string comparing original and updated text.
pub fn generate_unified_diff(
    original_name: &str,
    modified_name: &str,
    original: &str,
    modified: &str,
) -> String {
    let orig_lines: Vec<&str> = original.lines().collect();
    let mod_lines: Vec<&str> = modified.lines().collect();

    let mut output = String::new();
    let _ = writeln!(output, "--- a/{}", original_name);
    let _ = writeln!(output, "+++ b/{}", modified_name);

    if original == modified {
        return output;
    }

    let mut orig_idx = 0;
    let mut mod_idx = 0;

    while orig_idx < orig_lines.len() || mod_idx < mod_lines.len() {
        if orig_idx < orig_lines.len() && mod_idx < mod_lines.len() {
            if orig_lines[orig_idx] == mod_lines[mod_idx] {
                let _ = writeln!(output, " {}", orig_lines[orig_idx]);
                orig_idx += 1;
                mod_idx += 1;
            } else {
                // Line changed or removed
                let _ = writeln!(output, "-{}", orig_lines[orig_idx]);
                orig_idx += 1;
            }
        } else if orig_idx < orig_lines.len() {
            let _ = writeln!(output, "-{}", orig_lines[orig_idx]);
            orig_idx += 1;
        } else {
            let _ = writeln!(output, "+{}", mod_lines[mod_idx]);
            mod_idx += 1;
        }
    }

    output
}
