mod attrs;
mod block;
pub mod inline;
pub mod lexer;
pub mod migrate;

use aif_core::ast::Document;
use aif_core::error::ParseError;

/// AIF surface-syntax version.
///
/// - **V1** — legacy: all skill blocks are terminated by `@end`.
/// - **V2** — current: containers (`@skill`, `@artifact_skill`) close with
///   `@/skill` / `@/artifact_skill`; leaf blocks auto-close at the next
///   `@`-directive.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SyntaxVersion {
    V1,
    V2,
}

/// Auto-detect the syntax version of an AIF source string.
///
/// Rules:
/// - any line like `@end` → V1
/// - any line like `@/name` → V2
/// - neither → V2 (new default)
/// - both → error (mixed-syntax file)
pub fn detect_syntax_version(input: &str) -> Result<SyntaxVersion, String> {
    let mut has_end = false;
    let mut has_slash = false;
    for line in input.lines() {
        let t = line.trim();
        if t == "@end" {
            has_end = true;
        } else if let Some(rest) = t.strip_prefix("@/") {
            if rest.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
                has_slash = true;
            }
        }
        if has_end && has_slash {
            return Err(
                "file mixes v1 (@end) and v2 (@/name) syntax; run 'aif migrate-syntax' to convert".into(),
            );
        }
    }
    match (has_end, has_slash) {
        (true, false) => Ok(SyntaxVersion::V1),
        _ => Ok(SyntaxVersion::V2),
    }
}

/// Parse an AIF document from source text.
///
/// Auto-detects the syntax version (V1 = `@end`, V2 = `@/name`).
pub fn parse(input: &str) -> Result<Document, Vec<ParseError>> {
    let version = detect_syntax_version(input)
        .map_err(|e| vec![ParseError::new(e, aif_core::span::Span::new(0, 0))])?;
    let mut parser = block::BlockParser::new(input, version);
    parser.parse()
}

/// Parse an AIF document with an explicit syntax version (for tests / migration).
pub fn parse_with_version(input: &str, version: SyntaxVersion) -> Result<Document, Vec<ParseError>> {
    let mut parser = block::BlockParser::new(input, version);
    parser.parse()
}
