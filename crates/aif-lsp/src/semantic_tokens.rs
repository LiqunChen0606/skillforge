//! Semantic token definitions for AIF syntax highlighting.
//!
//! Maps AIF lexer token types to LSP semantic token types for
//! editor syntax highlighting of `.aif` files.

use tower_lsp::lsp_types::*;

/// AIF semantic token types, mapped to LSP standard types.
#[allow(dead_code)]
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,   // 0: block directives (@section, @skill, @claim, etc.)
    SemanticTokenType::PROPERTY,  // 1: metadata keys (#title:, #author:)
    SemanticTokenType::STRING,    // 2: text content
    SemanticTokenType::OPERATOR,  // 3: code fences (```)
    SemanticTokenType::VARIABLE,  // 4: attribute names in [key=value]
    SemanticTokenType::NUMBER,    // 5: ordered list markers
    SemanticTokenType::DECORATOR, // 6: @ref markers
    SemanticTokenType::COMMENT,   // 7: thematic breaks (---)
];

/// Token modifiers (none used currently, but required by the protocol).
#[allow(dead_code)]
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[];

/// Build the semantic token legend for the server capabilities.
#[allow(dead_code)]
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

/// Map a lexer token type to its semantic token type index.
/// Returns None for tokens that don't need highlighting (brackets, punctuation).
#[allow(dead_code)]
pub fn token_type_index(token: &aif_parser::lexer::Token) -> Option<u32> {
    use aif_parser::lexer::Token;
    match token {
        Token::BlockDirective => Some(0), // keyword
        Token::MetaKey => Some(1),        // property
        Token::Text => Some(2),           // string
        Token::CodeFence => Some(3),      // operator
        Token::OrderedMarker => Some(5),  // number
        Token::RefMarker => Some(6),      // decorator
        Token::ThematicBreak => Some(7),  // comment
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_parser::lexer::Token;

    #[test]
    fn block_directive_is_keyword() {
        assert_eq!(token_type_index(&Token::BlockDirective), Some(0));
    }

    #[test]
    fn meta_key_is_property() {
        assert_eq!(token_type_index(&Token::MetaKey), Some(1));
    }

    #[test]
    fn punctuation_has_no_token_type() {
        assert_eq!(token_type_index(&Token::BracketOpen), None);
        assert_eq!(token_type_index(&Token::Comma), None);
    }

    #[test]
    fn legend_has_correct_count() {
        let leg = legend();
        assert_eq!(leg.token_types.len(), TOKEN_TYPES.len());
        assert_eq!(leg.token_modifiers.len(), TOKEN_MODIFIERS.len());
    }
}
