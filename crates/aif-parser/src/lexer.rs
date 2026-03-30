use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")]
pub enum Token {
    #[regex(r"#[a-zA-Z_][a-zA-Z0-9_]*:")]
    MetaKey,

    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*")]
    BlockDirective,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(":")]
    Colon,

    #[token("=")]
    Equals,

    #[token(",")]
    Comma,

    #[regex(r"```[`]*")]
    CodeFence,

    #[token(">")]
    BlockQuoteMarker,

    #[token("-")]
    Dash,

    #[regex(r"[0-9]+\.")]
    OrderedMarker,

    #[regex(r"---[-]*")]
    ThematicBreak,

    #[token("\n")]
    Newline,

    #[token("**")]
    DoubleStar,

    #[token("*")]
    Star,

    #[token("`")]
    Backtick,

    #[token("](")]
    LinkMiddle,

    #[token(")")]
    Paren,

    #[token("@ref")]
    RefMarker,

    #[token("~")]
    Tilde,

    #[regex(r"[^\n\[\]()@#`*~>=,: \t\\-]+")]
    Text,

    #[regex(r"\\.", priority = 10)]
    Escaped,
}

/// A token with its span in the source
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: std::ops::Range<usize>,
    pub slice: String,
}

pub fn lex(input: &str) -> Vec<SpannedToken> {
    let mut lexer = Token::lexer(input);
    let mut tokens = Vec::new();
    while let Some(result) = lexer.next() {
        if let Ok(token) = result {
            tokens.push(SpannedToken {
                token,
                span: lexer.span(),
                slice: lexer.slice().to_string(),
            });
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_metadata() {
        let tokens = lex("#title: Hello World\n");
        assert_eq!(tokens[0].token, Token::MetaKey);
        assert_eq!(tokens[0].slice, "#title:");
    }

    #[test]
    fn lex_block_directive() {
        let tokens = lex("@section[id=intro]: Title\n");
        assert_eq!(tokens[0].token, Token::BlockDirective);
        assert_eq!(tokens[0].slice, "@section");
    }

    #[test]
    fn lex_code_fence() {
        let tokens = lex("```rust\n");
        assert_eq!(tokens[0].token, Token::CodeFence);
    }
}
