use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::config::KeyEntry;
use super::patterns::{LanguagePatterns, SourceLanguage};

/// Describes a concrete reference to a configuration key detected in source code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeReference {
    /// Source file path where reference was found.
    pub file_path: PathBuf,
    /// 1-indexed line number in source file.
    pub line: usize,
    /// Exact text fragment that triggered the match.
    pub matched_text: String,
    /// Canonical form that was matched (e.g. "dot_path", "env_path", "leaf_key").
    pub matched_form: String,
    /// Full line context around the match.
    pub context_line: String,
    /// True if match was extracted via a language-specific programmatic API (e.g. `env::var`).
    pub is_programmatic_api: bool,
}

/// Matches configuration keys against source code content.
pub struct SourceMatcher;

impl SourceMatcher {
    /// Scan source content for references to a given configuration key.
    pub fn scan_for_key(
        content: &str,
        file_path: &Path,
        key: &KeyEntry,
    ) -> Vec<CodeReference> {
        let lang = SourceLanguage::detect(file_path);
        let patterns = LanguagePatterns::for_language(lang);
        let mut references = Vec::new();

        let dot_target = &key.dot_path;
        let env_target = &key.env_path;
        let dunder_target = &key.double_underscore_path;
        let leaf_target = &key.leaf_key;

        for (idx, line) in content.lines().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            // 1. Check programmatic API patterns for language
            for pattern in &patterns.api_patterns {
                for cap in pattern.captures_iter(line) {
                    if let Some(matched) = cap.get(1) {
                        let token = matched.as_str().trim();
                        if key.matches_token(token) {
                            references.push(CodeReference {
                                file_path: file_path.to_path_buf(),
                                line: line_num,
                                matched_text: matched.as_str().to_string(),
                                matched_form: determine_matched_form(token, key),
                                context_line: trimmed.to_string(),
                                is_programmatic_api: true,
                            });
                        }
                    }
                }
            }

            // 2. Direct string / token search for dot_path
            if line.contains(dot_target) {
                references.push(CodeReference {
                    file_path: file_path.to_path_buf(),
                    line: line_num,
                    matched_text: dot_target.clone(),
                    matched_form: "dot_path".to_string(),
                    context_line: trimmed.to_string(),
                    is_programmatic_api: false,
                });
                continue;
            }

            // 3. Direct string search for env_target
            if line.contains(env_target) {
                references.push(CodeReference {
                    file_path: file_path.to_path_buf(),
                    line: line_num,
                    matched_text: env_target.clone(),
                    matched_form: "env_path".to_string(),
                    context_line: trimmed.to_string(),
                    is_programmatic_api: false,
                });
                continue;
            }

            // 4. Direct string search for double underscore target
            if line.contains(dunder_target) {
                references.push(CodeReference {
                    file_path: file_path.to_path_buf(),
                    line: line_num,
                    matched_text: dunder_target.clone(),
                    matched_form: "double_underscore_path".to_string(),
                    context_line: trimmed.to_string(),
                    is_programmatic_api: false,
                });
                continue;
            }

            // 5. Leaf key reference (check with word boundaries)
            if contains_word_or_literal(line, leaf_target) {
                references.push(CodeReference {
                    file_path: file_path.to_path_buf(),
                    line: line_num,
                    matched_text: leaf_target.clone(),
                    matched_form: "leaf_key".to_string(),
                    context_line: trimmed.to_string(),
                    is_programmatic_api: false,
                });
            }
        }

        // Deduplicate references on (line, matched_form)
        references.sort_by(|a, b| a.line.cmp(&b.line));
        references.dedup_by(|a, b| a.line == b.line && a.matched_form == b.matched_form);

        references
    }
}

/// Identifies which canonical form a token corresponds to.
fn determine_matched_form(token: &str, key: &KeyEntry) -> String {
    if token == key.dot_path {
        "dot_path".to_string()
    } else if token.eq_ignore_ascii_case(&key.env_path) {
        "env_path".to_string()
    } else if token.eq_ignore_ascii_case(&key.double_underscore_path) {
        "double_underscore_path".to_string()
    } else if token == key.leaf_key {
        "leaf_key".to_string()
    } else {
        "alias".to_string()
    }
}

/// Check if a line contains the target as a standalone word or inside quotes.
fn contains_word_or_literal(line: &str, word: &str) -> bool {
    if word.is_empty() || !line.contains(word) {
        return false;
    }

    // Check quoted occurrence: "word" or 'word'
    let quoted_double = format!("\"{}\"", word);
    let quoted_single = format!("'{}'", word);
    if line.contains(&quoted_double) || line.contains(&quoted_single) {
        return true;
    }

    // Word boundary check
    let bytes = line.as_bytes();
    let word_bytes = word.as_bytes();
    let word_len = word_bytes.len();

    let mut start = 0;
    while let Some(pos) = line[start..].find(word) {
        let abs_pos = start + pos;
        let end_pos = abs_pos + word_len;

        let char_before_ok = abs_pos == 0 || !is_ident_char(bytes[abs_pos - 1]);
        let char_after_ok = end_pos >= bytes.len() || !is_ident_char(bytes[end_pos]);

        if char_before_ok && char_after_ok {
            return true;
        }

        start = abs_pos + 1;
    }

    false
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
