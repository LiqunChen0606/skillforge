mod attrs;
mod block;
pub mod inline;
pub mod lexer;

use aif_core::ast::Document;
use aif_core::error::ParseError;

/// Parse an AIF document from source text
pub fn parse(input: &str) -> Result<Document, Vec<ParseError>> {
    let mut parser = block::BlockParser::new(input);
    parser.parse()
}
