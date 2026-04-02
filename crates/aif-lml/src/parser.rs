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

        // Media blocks (@fig, @audio, @vid)
        if line.starts_with("@fig(") || line.starts_with("@audio(") || line.starts_with("@vid(") {
            return self.parse_media_block();
        }

        // Table block (@table:)
        if line.starts_with("@table:") {
            return self.parse_table_block();
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

    fn parse_media_block(&mut self) -> Result<Block, String> {
        let line = self.advance().unwrap();

        // Determine media type from prefix
        let (media_type, rest) = if let Some(r) = line.strip_prefix("@fig(") {
            ("fig", r)
        } else if let Some(r) = line.strip_prefix("@audio(") {
            ("audio", r)
        } else if let Some(r) = line.strip_prefix("@vid(") {
            ("vid", r)
        } else {
            return Err(format!("Not a media block: {}", line));
        };

        // Find the closing ')' — may have nested attrs group after
        // Format: @fig(src=path, alt="desc", w=800)(id=x, k=v): Caption
        // First, find the close of the media params paren
        let close = find_closing_paren(rest)
            .ok_or_else(|| "Unclosed media attribute parenthesis".to_string())?;
        let inner = &rest[..close];
        let after_first_paren = &rest[close + 1..];

        // Parse key=value pairs from inner
        let pairs = parse_media_kv_pairs(inner);

        // Extract src
        let src = pairs.iter()
            .find(|(k, _)| k == "src")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        // Build MediaMeta
        let meta = MediaMeta {
            alt: pairs.iter().find(|(k, _)| k == "alt").map(|(_, v)| v.clone()),
            width: pairs.iter()
                .find(|(k, _)| k == "w" || k == "width")
                .and_then(|(_, v)| v.parse::<u32>().ok()),
            height: pairs.iter()
                .find(|(k, _)| k == "h" || k == "height")
                .and_then(|(_, v)| v.parse::<u32>().ok()),
            duration: pairs.iter()
                .find(|(k, _)| k == "dur" || k == "duration")
                .and_then(|(_, v)| v.parse::<f64>().ok()),
            mime: pairs.iter().find(|(k, _)| k == "mime").map(|(_, v)| v.clone()),
            poster: pairs.iter().find(|(k, _)| k == "poster").map(|(_, v)| v.clone()),
        };

        // Parse optional second attrs group
        let (attrs, after_attrs) = if after_first_paren.starts_with('(') {
            parse_attrs_parens(after_first_paren)?
        } else {
            (Attrs::new(), after_first_paren)
        };

        // Parse optional caption after ':'
        let caption = if let Some(rest) = after_attrs.strip_prefix(':') {
            let cap = rest.trim();
            if cap.is_empty() {
                None
            } else {
                Some(vec![Inline::Text { text: cap.to_string() }])
            }
        } else {
            None
        };

        self.skip_blank_lines();

        let kind = match media_type {
            "fig" => BlockKind::Figure { attrs, caption, src, meta },
            "audio" => BlockKind::Audio { attrs, caption, src, meta },
            "vid" => BlockKind::Video { attrs, caption, src, meta },
            _ => unreachable!(),
        };

        Ok(Block {
            kind,
            span: Span::empty(),
        })
    }

    fn parse_table_block(&mut self) -> Result<Block, String> {
        let line = self.advance().unwrap();
        // Parse "@table: Optional caption" or "@table:"
        let rest = line.strip_prefix("@table:").unwrap();
        let caption_text = rest.trim();
        let caption = if caption_text.is_empty() {
            None
        } else {
            Some(vec![Inline::Text { text: caption_text.to_string() }])
        };

        let mut headers = Vec::new();
        let mut rows = Vec::new();
        let mut is_header = true;

        while let Some(peeked) = self.peek() {
            let trimmed = peeked.trim();
            if trimmed.is_empty() {
                break;
            }
            if !trimmed.starts_with('|') {
                break;
            }
            self.advance();

            // Separator line (e.g. | --- | --- |)
            if trimmed.contains("---") {
                is_header = false;
                continue;
            }

            let cells: Vec<&str> = trimmed
                .trim_start_matches('|')
                .trim_end_matches('|')
                .split('|')
                .map(|s| s.trim())
                .collect();

            if is_header {
                headers = cells.iter().map(|c| vec![Inline::Text { text: c.to_string() }]).collect();
                is_header = false;
            } else {
                let row: Vec<Vec<Inline>> = cells.iter().map(|c| vec![Inline::Text { text: c.to_string() }]).collect();
                rows.push(row);
            }
        }

        self.skip_blank_lines();

        Ok(Block {
            kind: BlockKind::Table {
                attrs: Attrs::new(),
                caption,
                headers,
                rows,
            },
            span: Span::empty(),
        })
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
        "scenario" => Some(SkillBlockType::Scenario),
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

/// Find the index of the closing ')' that matches the opening, handling quoted strings.
fn find_closing_paren(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_quote = false;
    for (i, ch) in s.char_indices() {
        if in_quote {
            if ch == '"' {
                in_quote = false;
            }
            continue;
        }
        match ch {
            '"' => in_quote = true,
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Parse comma-separated key=value pairs from inside parentheses.
/// Handles quoted values like alt="A description".
fn parse_media_kv_pairs(input: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut rest = input;

    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        // Skip leading comma
        if rest.starts_with(',') {
            rest = &rest[1..];
            continue;
        }

        // Find '='
        let eq_pos = match rest.find('=') {
            Some(p) => p,
            None => break,
        };
        let key = rest[..eq_pos].trim().to_string();
        rest = &rest[eq_pos + 1..];

        // Parse value — may be quoted
        rest = rest.trim_start();
        let value;
        if rest.starts_with('"') {
            // Quoted value — find closing quote
            rest = &rest[1..]; // skip opening quote
            let end_quote = rest.find('"').unwrap_or(rest.len());
            value = rest[..end_quote].to_string();
            rest = if end_quote < rest.len() {
                &rest[end_quote + 1..]
            } else {
                ""
            };
        } else {
            // Unquoted value — until comma or end
            let end = rest.find(',').unwrap_or(rest.len());
            value = rest[..end].trim().to_string();
            rest = &rest[end..];
        }

        pairs.push((key, value));
    }

    pairs
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
    fn parse_figure_basic() {
        let input = "@fig(src=image.png, alt=\"A photo\", w=800, h=600): My caption\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::Figure { src, meta, caption, attrs } => {
                assert_eq!(src, "image.png");
                assert_eq!(meta.alt.as_deref(), Some("A photo"));
                assert_eq!(meta.width, Some(800));
                assert_eq!(meta.height, Some(600));
                assert!(meta.duration.is_none());
                assert!(meta.poster.is_none());
                assert!(attrs.id.is_none());
                let cap_text = caption.as_ref().unwrap();
                assert_eq!(cap_text[0], Inline::Text { text: "My caption".into() });
            }
            other => panic!("Expected Figure, got {:?}", other),
        }
    }

    #[test]
    fn parse_audio_basic() {
        let input = "@audio(src=track.mp3, alt=\"Song\", dur=120.5)\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::Audio { src, meta, caption, .. } => {
                assert_eq!(src, "track.mp3");
                assert_eq!(meta.alt.as_deref(), Some("Song"));
                assert_eq!(meta.duration, Some(120.5));
                assert!(meta.width.is_none());
                assert!(caption.is_none());
            }
            other => panic!("Expected Audio, got {:?}", other),
        }
    }

    #[test]
    fn parse_video_with_poster() {
        let input = "@vid(src=movie.mp4, alt=\"A movie\", w=1920, h=1080, dur=300, poster=thumb.jpg): Movie title\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::Video { src, meta, caption, .. } => {
                assert_eq!(src, "movie.mp4");
                assert_eq!(meta.alt.as_deref(), Some("A movie"));
                assert_eq!(meta.width, Some(1920));
                assert_eq!(meta.height, Some(1080));
                assert_eq!(meta.duration, Some(300.0));
                assert_eq!(meta.poster.as_deref(), Some("thumb.jpg"));
                let cap = caption.as_ref().unwrap();
                assert_eq!(cap[0], Inline::Text { text: "Movie title".into() });
            }
            other => panic!("Expected Video, got {:?}", other),
        }
    }

    #[test]
    fn parse_figure_no_caption() {
        let input = "@fig(src=diagram.svg)\n";
        let doc = parse_lml(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::Figure { src, caption, meta, .. } => {
                assert_eq!(src, "diagram.svg");
                assert!(caption.is_none());
                assert!(meta.alt.is_none());
            }
            other => panic!("Expected Figure, got {:?}", other),
        }
    }

    #[test]
    fn parse_figure_with_attrs() {
        let input = "@fig(src=img.png, alt=\"Photo\")(id=fig1): Caption here\n";
        let doc = parse_lml(input).unwrap();
        match &doc.blocks[0].kind {
            BlockKind::Figure { src, attrs, meta, caption } => {
                assert_eq!(src, "img.png");
                assert_eq!(attrs.id.as_deref(), Some("fig1"));
                assert_eq!(meta.alt.as_deref(), Some("Photo"));
                assert!(caption.is_some());
            }
            other => panic!("Expected Figure, got {:?}", other),
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
