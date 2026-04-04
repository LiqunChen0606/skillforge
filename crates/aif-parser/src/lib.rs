mod attrs;
mod block;
pub mod inline;
pub mod lexer;
pub mod migrate;

use aif_core::ast::Document;
use aif_core::error::ParseError;

/// Parse an AIF v2 document.
///
/// AIF v2 syntax: `@skill` / `@artifact_skill` are containers and close with
/// `@/skill` / `@/artifact_skill`. All other blocks auto-close at the next
/// `@`-directive. Legacy v1 files (using `@end`) are rejected — run
/// `aif migrate-syntax` to convert.
pub fn parse(input: &str) -> Result<Document, Vec<ParseError>> {
    // Reject v1 (`@end`) files with a clear migration hint.
    for line in input.lines() {
        if line.trim() == "@end" {
            return Err(vec![ParseError::new(
                "legacy v1 syntax (@end) is no longer supported; run 'aif migrate-syntax <path>' to convert to v2",
                aif_core::span::Span::new(0, 0),
            )]);
        }
    }
    let mut parser = block::BlockParser::new(input);
    parser.parse()
}
