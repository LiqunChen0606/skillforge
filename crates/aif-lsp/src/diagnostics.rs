//! Compute LSP diagnostics from AIF parse errors and lint results.

use aif_core::lint::{lint_document, DocLintSeverity};
use tower_lsp::lsp_types::*;

/// Convert a byte offset to an LSP Position (line, character).
fn offset_to_position(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position::new(line, col)
}

/// Compute diagnostics by parsing the document and running lint checks.
pub fn compute_diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    match aif_parser::parse(text) {
        Ok(doc) => {
            // Run lint checks on successfully parsed document
            let lint_results = lint_document(&doc);
            for result in lint_results {
                if result.passed {
                    continue;
                }
                let severity = match result.severity {
                    DocLintSeverity::Error => DiagnosticSeverity::ERROR,
                    DocLintSeverity::Warning => DiagnosticSeverity::WARNING,
                };
                // Lint results don't carry spans, so place at document start
                // with the block_id in the message for context.
                let message = if let Some(ref id) = result.block_id {
                    format!("[{}] {}", id, result.message)
                } else {
                    result.message.clone()
                };
                diags.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    severity: Some(severity),
                    source: Some("aif-lint".to_string()),
                    message,
                    ..Default::default()
                });
            }
        }
        Err(errors) => {
            for err in errors {
                let start = offset_to_position(text, err.span.start);
                let end = offset_to_position(text, err.span.end);
                diags.push(Diagnostic {
                    range: Range::new(start, end),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("aif-parser".to_string()),
                    message: err.message,
                    ..Default::default()
                });
            }
        }
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_to_position_first_line() {
        let text = "hello world";
        assert_eq!(offset_to_position(text, 0), Position::new(0, 0));
        assert_eq!(offset_to_position(text, 5), Position::new(0, 5));
    }

    #[test]
    fn offset_to_position_multi_line() {
        let text = "line1\nline2\nline3";
        assert_eq!(offset_to_position(text, 6), Position::new(1, 0));
        assert_eq!(offset_to_position(text, 8), Position::new(1, 2));
        assert_eq!(offset_to_position(text, 12), Position::new(2, 0));
    }

    #[test]
    fn valid_document_produces_no_parse_errors() {
        let text = "#title: Test\n\nHello world.\n";
        let diags = compute_diagnostics(text);
        // Should have no parse errors (may have lint warnings like MissingMetadata)
        assert!(
            diags.iter().all(|d| d.source.as_deref() != Some("aif-parser")),
            "Expected no parse errors"
        );
    }
}
