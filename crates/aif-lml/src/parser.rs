use aif_core::ast::*;
use aif_core::span::Span;

/// Parse LML aggressive-mode text back into a Document AST.
pub fn parse_lml(input: &str) -> Result<Document, String> {
    let mut parser = LmlParser::new(input);
    parser.parse()
}

struct LmlParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> LmlParser<'a> {
    fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self { lines, pos: 0 }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.lines.len()
    }

    fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<&'a str> {
        let line = self.lines.get(self.pos).copied();
        if line.is_some() {
            self.pos += 1;
        }
        line
    }

    fn skip_blank_lines(&mut self) {
        while let Some(line) = self.peek() {
            if line.trim().is_empty() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<Document, String> {
        let mut doc = Document::new();

        // Parse metadata lines at the start (#key: value)
        while let Some(line) = self.peek() {
            if let Some(rest) = line.strip_prefix('#') {
                // Make sure it's metadata (not a heading like ## )
                if rest.starts_with('#') {
                    break;
                }
                if let Some((key, value)) = rest.split_once(':') {
                    let key = key.trim().to_string();
                    let value = value.trim().to_string();
                    doc.metadata.insert(key, value);
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        self.skip_blank_lines();

        // Parse blocks
        while !self.at_end() {
            self.skip_blank_lines();
            if self.at_end() {
                break;
            }
            let block = self.parse_block()?;
            doc.blocks.push(block);
        }

        Ok(doc)
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        let line = self.peek().unwrap();

        // Code fence
        if line.starts_with("```") {
            return self.parse_code_block();
        }

        // Thematic break
        if line.trim() == "---" {
            self.advance();
            self.skip_blank_lines();
            return Ok(Block {
                kind: BlockKind::ThematicBreak,
                span: Span::empty(),
            });
        }

        // Heading (## Title)
        if line.starts_with('#') && !line.starts_with("#(") {
            // Count the # characters
            let trimmed = line.trim_start_matches('#');
            let depth = line.len() - trimmed.len();
            if depth > 0 && trimmed.starts_with(' ') {
                return self.parse_section(depth);
            }
        }

        // Skill directive (@step, @verify, etc.)
        if line.starts_with('@') {
            return self.parse_skill_directive();
        }

        // Blockquote
        if line.starts_with("> ") || line == ">" {
            return self.parse_blockquote();
        }

        // Unordered list
        if line.starts_with("- ") {
            return self.parse_list(false);
        }

        // Ordered list
        if is_ordered_list_line(line) {
            return self.parse_list(true);
        }

        // Paragraph (default)
        self.parse_paragraph()
    }

    fn parse_code_block(&mut self) -> Result<Block, String> {
        let first_line = self.advance().unwrap();
        let lang_str = first_line.trim_start_matches('`');
        let lang = if lang_str.is_empty() {
            None
        } else {
            Some(lang_str.to_string())
        };

        let mut code = String::new();
        loop {
            match self.advance() {
                Some(line) if line.starts_with("```") => break,
                Some(line) => {
                    code.push_str(line);
                    code.push('\n');
                }
                None => return Err("Unterminated code block".to_string()),
            }
        }

        self.skip_blank_lines();

        Ok(Block {
            kind: BlockKind::CodeBlock {
                lang,
                attrs: Attrs::new(),
                code,
            },
            span: Span::empty(),
        })
    }

    fn parse_section(&mut self, depth: usize) -> Result<Block, String> {
        let line = self.advance().unwrap();
        let title_text = line[depth..].trim().to_string();
        let title = vec![Inline::Text { text: title_text }];

        let mut children = Vec::new();

        // Collect children until we hit a heading at same or lower depth, or end
        while let Some(next_line) = self.peek() {
            if next_line.trim().is_empty() {
                self.skip_blank_lines();
                continue;
            }

            // Check if next line is a heading at same or shallower depth
            if next_line.starts_with('#') && !next_line.starts_with("#(") {
                let trimmed = next_line.trim_start_matches('#');
                let next_depth = next_line.len() - trimmed.len();
                if next_depth > 0 && trimmed.starts_with(' ') && next_depth <= depth {
                    break;
                }
            }

            let child = self.parse_block()?;
            children.push(child);
        }

        Ok(Block {
            kind: BlockKind::Section {
                attrs: Attrs::new(),
                title,
                children,
            },
            span: Span::empty(),
        })
    }

    fn parse_skill_directive(&mut self) -> Result<Block, String> {
        let line = self.advance().unwrap();

        // Parse: @name(attrs): content
        // or:   @name(attrs):\n followed by children
        // or:   @name: content
        let rest = &line[1..]; // skip '@'

        // Find the directive name
        let name_end = rest
            .find(['(', ':'])
            .unwrap_or(rest.len());
        let name = &rest[..name_end];

        let skill_type = match_skill_type(name)
            .ok_or_else(|| format!("Unknown skill directive: @{}", name))?;

        let after_name = &rest[name_end..];

        // Parse optional attrs
        let (attrs, after_attrs) = if after_name.starts_with('(') {
            parse_attrs_parens(after_name)?
        } else {
            (Attrs::new(), after_name)
        };

        // Parse optional content after ':'
        let content_str = if let Some(rest) = after_attrs.strip_prefix(':') {
            rest.trim()
        } else {
            ""
        };

        let is_container = matches!(skill_type, SkillBlockType::Skill);

        if is_container || content_str.is_empty() {
            // Container: collect children until next directive at same or lower level
            let content = if content_str.is_empty() {
                vec![]
            } else {
                vec![Inline::Text {
                    text: content_str.to_string(),
                }]
            };

            let mut children = Vec::new();

            // For @skill containers, collect child directives
            if is_container {
                while let Some(next_line) = self.peek() {
                    if next_line.trim().is_empty() {
                        self.skip_blank_lines();
                        continue;
                    }
                    // Stop at another @skill or non-@ block
                    if next_line.starts_with("@skill") && !next_line.starts_with("@skill(") {
                        // Another @skill block, stop
                        break;
                    }
                    if next_line.starts_with('@') {
                        let child = self.parse_skill_directive()?;
                        children.push(child);
                    } else if next_line.starts_with('#') || next_line.starts_with("```") {
                        break;
                    } else {
                        // Paragraph inside skill
                        let child = self.parse_block()?;
                        children.push(child);
                    }
                }
            } else if content_str.is_empty() {
                // Leaf directive with no inline content: collect paragraph content
                // until blank line or next directive
                let mut text_lines = Vec::new();
                while let Some(next_line) = self.peek() {
                    if next_line.trim().is_empty() {
                        self.skip_blank_lines();
                        break;
                    }
                    if next_line.starts_with('@') || next_line.starts_with('#') || next_line.starts_with("```") {
                        break;
                    }
                    text_lines.push(self.advance().unwrap().to_string());
                }
                if !text_lines.is_empty() {
                    return Ok(Block {
                        kind: BlockKind::SkillBlock {
                            skill_type,
                            attrs,
                            title: None,
                            content: vec![Inline::Text {
                                text: text_lines.join("\n"),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    });
                }
            }

            Ok(Block {
                kind: BlockKind::SkillBlock {
                    skill_type,
                    attrs,
                    title: None,
                    content,
                    children,
                },
                span: Span::empty(),
            })
        } else {
            // Leaf with inline content on same line
            let content = vec![Inline::Text {
                text: content_str.to_string(),
            }];

            Ok(Block {
                kind: BlockKind::SkillBlock {
                    skill_type,
                    attrs,
                    title: None,
                    content,
                    children: vec![],
                },
                span: Span::empty(),
            })
        }
    }

    fn parse_blockquote(&mut self) -> Result<Block, String> {
        let mut content_blocks = Vec::new();
        let mut para_lines = Vec::new();

        while let Some(line) = self.peek() {
            if let Some(rest) = line.strip_prefix("> ") {
                para_lines.push(rest.to_string());
                self.advance();
            } else if line == ">" {
                // Empty blockquote line — paragraph break
                if !para_lines.is_empty() {
                    content_blocks.push(Block {
                        kind: BlockKind::Paragraph {
                            content: vec![Inline::Text {
                                text: para_lines.join(" "),
                            }],
                        },
                        span: Span::empty(),
                    });
                    para_lines.clear();
                }
                self.advance();
            } else {
                break;
            }
        }

        if !para_lines.is_empty() {
            content_blocks.push(Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text {
                        text: para_lines.join(" "),
                    }],
                },
                span: Span::empty(),
            });
        }

        self.skip_blank_lines();

        Ok(Block {
            kind: BlockKind::BlockQuote {
                content: content_blocks,
            },
            span: Span::empty(),
        })
    }

    fn parse_list(&mut self, ordered: bool) -> Result<Block, String> {
        let mut items = Vec::new();

        while let Some(line) = self.peek() {
            if ordered {
                if let Some(text) = strip_ordered_prefix(line) {
                    items.push(ListItem {
                        content: vec![Inline::Text {
                            text: text.to_string(),
                        }],
                        children: vec![],
                    });
                    self.advance();
                } else {
                    break;
                }
            } else if let Some(text) = line.strip_prefix("- ") {
                items.push(ListItem {
                    content: vec![Inline::Text {
                        text: text.to_string(),
                    }],
                    children: vec![],
                });
                self.advance();
            } else {
                break;
            }
        }

        self.skip_blank_lines();

        Ok(Block {
            kind: BlockKind::List { ordered, items },
            span: Span::empty(),
        })
    }

    fn parse_paragraph(&mut self) -> Result<Block, String> {
        let mut text_lines = Vec::new();

        while let Some(line) = self.peek() {
            if line.trim().is_empty() {
                break;
            }
            // Stop at block-level constructs
            if line.starts_with('#')
                || line.starts_with('@')
                || line.starts_with("```")
                || line.starts_with("> ")
                || line.starts_with("- ")
                || line.trim() == "---"
                || is_ordered_list_line(line)
            {
                break;
            }
            text_lines.push(self.advance().unwrap());
        }

        self.skip_blank_lines();

        let text = text_lines.join(" ");
        Ok(Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text { text }],
            },
            span: Span::empty(),
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn match_skill_type(name: &str) -> Option<SkillBlockType> {
    match name {
        "skill" => Some(SkillBlockType::Skill),
        "step" => Some(SkillBlockType::Step),
        "verify" => Some(SkillBlockType::Verify),
        "pre" => Some(SkillBlockType::Precondition),
        "output" => Some(SkillBlockType::OutputContract),
        "decision" => Some(SkillBlockType::Decision),
        "tool" => Some(SkillBlockType::Tool),
        "fallback" => Some(SkillBlockType::Fallback),
        "redflag" => Some(SkillBlockType::RedFlag),
        "example" => Some(SkillBlockType::Example),
        _ => None,
    }
}

fn parse_attrs_parens(input: &str) -> Result<(Attrs, &str), String> {
    // input starts with '('
    let close = input
        .find(')')
        .ok_or_else(|| "Unclosed attribute parenthesis".to_string())?;
    let inner = &input[1..close];
    let rest = &input[close + 1..];

    let mut attrs = Attrs::new();
    for pair in inner.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((key, value)) = pair.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if key == "id" {
                attrs.id = Some(value.to_string());
            } else {
                attrs.pairs.insert(key.to_string(), value.to_string());
            }
        }
    }

    Ok((attrs, rest))
}

fn is_ordered_list_line(line: &str) -> bool {
    // Match "N. " pattern
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i > 0 && bytes.get(i) == Some(&b'.') && bytes.get(i + 1) == Some(&b' ')
}

fn strip_ordered_prefix(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && bytes.get(i) == Some(&b'.') && bytes.get(i + 1) == Some(&b' ') {
        Some(&line[i + 2..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let doc = parse_lml("").unwrap();
        assert!(doc.blocks.is_empty());
        assert!(doc.metadata.is_empty());
    }

    #[test]
    fn parse_metadata() {
        let input = "#title: My Document\n#author: Alice\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.metadata.get("title").unwrap(), "My Document");
        assert_eq!(doc.metadata.get("author").unwrap(), "Alice");
    }

    #[test]
    fn parse_paragraph() {
        let input = "Hello world\n\nSecond paragraph\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);
        match &doc.blocks[0].kind {
            BlockKind::Paragraph { content } => {
                assert_eq!(content[0], Inline::Text { text: "Hello world".into() });
            }
            _ => panic!("Expected paragraph"),
        }
    }

    #[test]
    fn parse_skill_step_with_attrs() {
        let input = "@step(order=1): Reproduce the bug\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::SkillBlock { skill_type, attrs, content, .. } => {
                assert_eq!(*skill_type, SkillBlockType::Step);
                assert_eq!(attrs.get("order"), Some("1"));
                assert_eq!(content[0], Inline::Text { text: "Reproduce the bug".into() });
            }
            _ => panic!("Expected SkillBlock"),
        }
    }

    #[test]
    fn parse_code_block() {
        let input = "```rust\nfn main() {}\n```\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::CodeBlock { lang, code, .. } => {
                assert_eq!(lang.as_deref(), Some("rust"));
                assert_eq!(code, "fn main() {}\n");
            }
            _ => panic!("Expected CodeBlock"),
        }
    }

    #[test]
    fn parse_list_unordered() {
        let input = "- first\n- second\n- third\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::List { ordered, items } => {
                assert!(!ordered);
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn parse_list_ordered() {
        let input = "1. first\n2. second\n";
        let doc = parse_lml(input).unwrap();
        match &doc.blocks[0].kind {
            BlockKind::List { ordered, items } => {
                assert!(ordered);
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected List"),
        }
    }
}
