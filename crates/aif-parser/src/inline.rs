use aif_core::ast::Inline;

/// Parse inline content from a string
pub fn parse_inline(input: &str) -> Vec<Inline> {
    let mut parser = InlineParser::new(input);
    parser.parse()
}

struct InlineParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> InlineParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse(&mut self) -> Vec<Inline> {
        let mut inlines = Vec::new();
        let mut text_start = self.pos;

        while self.pos < self.input.len() {
            let remaining = &self.input[self.pos..];

            // **bold**
            if remaining.starts_with("**") {
                self.flush_text(text_start, &mut inlines);
                if let Some(content) = self.parse_delimited("**", "**") {
                    let inner = InlineParser::new(&content).parse();
                    inlines.push(Inline::Strong { content: inner });
                    text_start = self.pos;
                    continue;
                }
                self.pos += 2;
                continue;
            }

            // *italic*
            if remaining.starts_with('*') && !remaining.starts_with("**") {
                self.flush_text(text_start, &mut inlines);
                if let Some(content) = self.parse_delimited("*", "*") {
                    let inner = InlineParser::new(&content).parse();
                    inlines.push(Inline::Emphasis { content: inner });
                    text_start = self.pos;
                    continue;
                }
                self.pos += 1;
                continue;
            }

            // `code`
            if remaining.starts_with('`') {
                self.flush_text(text_start, &mut inlines);
                if let Some(code) = self.parse_delimited("`", "`") {
                    inlines.push(Inline::InlineCode { code });
                    text_start = self.pos;
                    continue;
                }
                self.pos += 1;
                continue;
            }

            // ![alt](url)
            if remaining.starts_with("![") {
                self.flush_text(text_start, &mut inlines);
                if let Some((alt, src)) = self.parse_image() {
                    inlines.push(Inline::Image { alt, src });
                    text_start = self.pos;
                    continue;
                }
                self.pos += 2;
                continue;
            }

            // [text](url)
            if remaining.starts_with('[') {
                self.flush_text(text_start, &mut inlines);
                if let Some((text, url)) = self.parse_link() {
                    let text_inlines = InlineParser::new(&text).parse();
                    inlines.push(Inline::Link { text: text_inlines, url });
                    text_start = self.pos;
                    continue;
                }
                self.pos += 1;
                continue;
            }

            // @ref[id]
            if remaining.starts_with("@ref[") {
                self.flush_text(text_start, &mut inlines);
                if let Some(target) = self.parse_ref() {
                    inlines.push(Inline::Reference { target });
                    text_start = self.pos;
                    continue;
                }
                self.pos += 5;
                continue;
            }

            // ~footnote~
            if remaining.starts_with('~') {
                self.flush_text(text_start, &mut inlines);
                if let Some(content) = self.parse_delimited("~", "~") {
                    let inner = InlineParser::new(&content).parse();
                    inlines.push(Inline::Footnote { content: inner });
                    text_start = self.pos;
                    continue;
                }
                self.pos += 1;
                continue;
            }

            // Escape: \*
            if remaining.starts_with('\\') && self.pos + 1 < self.input.len() {
                self.flush_text(text_start, &mut inlines);
                self.pos += 1; // skip backslash
                text_start = self.pos;
                self.pos += remaining[1..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                continue;
            }

            self.pos += remaining.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }

        self.flush_text(text_start, &mut inlines);
        inlines
    }

    fn flush_text(&self, start: usize, inlines: &mut Vec<Inline>) {
        if start < self.pos {
            let text = self.input[start..self.pos].to_string();
            if !text.is_empty() {
                inlines.push(Inline::Text { text });
            }
        }
    }

    fn parse_delimited(&mut self, open: &str, close: &str) -> Option<String> {
        let start = self.pos + open.len();
        if start >= self.input.len() {
            return None;
        }
        let remaining = &self.input[start..];
        if let Some(mut end) = remaining.find(close) {
            // For multi-char delimiters like **, align to the end of a run of that char
            // e.g., in "bold *italic***" match the last ** so inner * is part of content
            if close.len() > 1 {
                let ch = close.as_bytes()[0];
                while end + close.len() < remaining.len() && remaining.as_bytes()[end + close.len()] == ch {
                    end += 1;
                }
            }
            let content = remaining[..end].to_string();
            self.pos = start + end + close.len();
            Some(content)
        } else {
            None
        }
    }

    fn parse_image(&mut self) -> Option<(String, String)> {
        // Starts at '!', we need ![alt](url)
        let start = self.pos + 2; // skip '!['
        let remaining = &self.input[start..];
        let close_bracket = remaining.find(']')?;
        let alt = remaining[..close_bracket].to_string();
        let after = &remaining[close_bracket + 1..];
        if !after.starts_with('(') {
            return None;
        }
        let url_start = close_bracket + 2;
        let url_remaining = &remaining[url_start..];
        let close_paren = url_remaining.find(')')?;
        let src = url_remaining[..close_paren].to_string();
        self.pos = start + url_start + close_paren + 1;
        Some((alt, src))
    }

    fn parse_link(&mut self) -> Option<(String, String)> {
        let start = self.pos + 1;
        let remaining = &self.input[start..];
        let close_bracket = remaining.find(']')?;
        let text = remaining[..close_bracket].to_string();
        let after = &remaining[close_bracket + 1..];
        if !after.starts_with('(') {
            return None;
        }
        let url_start = close_bracket + 2;
        let url_remaining = &remaining[url_start..];
        let close_paren = url_remaining.find(')')?;
        let url = url_remaining[..close_paren].to_string();
        self.pos = start + url_start + close_paren + 1;
        Some((text, url))
    }

    fn parse_ref(&mut self) -> Option<String> {
        let start = self.pos + 5;
        let remaining = &self.input[start..];
        let close = remaining.find(']')?;
        let target = remaining[..close].to_string();
        self.pos = start + close + 1;
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_text() {
        let result = parse_inline("hello world");
        assert_eq!(result, vec![Inline::Text { text: "hello world".into() }]);
    }

    #[test]
    fn parse_bold() {
        let result = parse_inline("hello **bold** world");
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[1], Inline::Strong { .. }));
    }

    #[test]
    fn parse_italic() {
        let result = parse_inline("hello *italic* world");
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[1], Inline::Emphasis { .. }));
    }

    #[test]
    fn parse_inline_code() {
        let result = parse_inline("use `println!` here");
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[1], Inline::InlineCode { code } if code == "println!"));
    }

    #[test]
    fn parse_link() {
        let result = parse_inline("see [docs](https://example.com) for more");
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[1], Inline::Link { url, .. } if url == "https://example.com"));
    }

    #[test]
    fn parse_reference() {
        let result = parse_inline("see @ref[intro] for details");
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[1], Inline::Reference { target } if target == "intro"));
    }

    #[test]
    fn parse_nested_bold_italic() {
        let result = parse_inline("**bold *and italic***");
        assert_eq!(result.len(), 1);
        if let Inline::Strong { content } = &result[0] {
            assert_eq!(content.len(), 2);
        } else {
            panic!("expected Strong");
        }
    }

    #[test]
    fn parse_inline_image() {
        let result = parse_inline("see ![alt text](https://img.png) here");
        assert_eq!(result.len(), 3);
        match &result[1] {
            Inline::Image { alt, src } => {
                assert_eq!(alt, "alt text");
                assert_eq!(src, "https://img.png");
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }
}
