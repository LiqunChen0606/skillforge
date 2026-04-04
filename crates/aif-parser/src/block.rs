use aif_core::ast::*;
use aif_core::error::ParseError;
use aif_core::span::Span;

use crate::attrs::parse_attrs;
use crate::inline::parse_inline;

/// Containers hold other skill blocks as children and need an explicit
/// `@/name` closer. Leaves auto-close at the next `@` directive.
fn is_container(ty: &SkillBlockType) -> bool {
    matches!(
        ty,
        SkillBlockType::Skill | SkillBlockType::ArtifactSkill | SkillBlockType::Scenario
    )
}

/// Map a container SkillBlockType to its closer directive name.
fn container_closer_name(ty: &SkillBlockType) -> Option<&'static str> {
    match ty {
        SkillBlockType::Skill => Some("skill"),
        SkillBlockType::ArtifactSkill => Some("artifact_skill"),
        SkillBlockType::Scenario => Some("scenario"),
        _ => None,
    }
}

/// Extract known media metadata keys from attrs into a MediaMeta struct,
/// removing them from attrs.pairs so they don't remain as generic pairs.
fn extract_media_meta(attrs: &mut Attrs) -> MediaMeta {
    let mut meta = MediaMeta::default();
    if let Some(alt) = attrs.pairs.remove("alt") {
        meta.alt = Some(alt);
    }
    if let Some(w) = attrs.pairs.remove("width") {
        meta.width = w.parse().ok();
    }
    if let Some(h) = attrs.pairs.remove("height") {
        meta.height = h.parse().ok();
    }
    if let Some(d) = attrs.pairs.remove("duration") {
        meta.duration = d.parse().ok();
    }
    if let Some(m) = attrs.pairs.remove("mime") {
        meta.mime = Some(m);
    }
    if let Some(p) = attrs.pairs.remove("poster") {
        meta.poster = Some(p);
    }
    meta
}

fn is_skill_block_type(directive: &str) -> Option<SkillBlockType> {
    match directive {
        "skill" => Some(SkillBlockType::Skill),
        "step" => Some(SkillBlockType::Step),
        "verify" => Some(SkillBlockType::Verify),
        "precondition" => Some(SkillBlockType::Precondition),
        "output_contract" => Some(SkillBlockType::OutputContract),
        "decision" => Some(SkillBlockType::Decision),
        "tool" => Some(SkillBlockType::Tool),
        "fallback" => Some(SkillBlockType::Fallback),
        "red_flag" => Some(SkillBlockType::RedFlag),
        "example" => Some(SkillBlockType::Example),
        "scenario" => Some(SkillBlockType::Scenario),
        // Artifact skill block types
        "artifact_skill" => Some(SkillBlockType::ArtifactSkill),
        "input_schema" => Some(SkillBlockType::InputSchema),
        "template" => Some(SkillBlockType::Template),
        "binding" => Some(SkillBlockType::Binding),
        "generate" => Some(SkillBlockType::Generate),
        "export" => Some(SkillBlockType::Export),
        _ => None,
    }
}

/// A line with its byte offset in the original input
struct Line<'a> {
    text: &'a str,
    offset: usize,
}

pub(crate) struct BlockParser<'a> {
    lines: Vec<Line<'a>>,
    pos: usize,
}

impl<'a> BlockParser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lines = Vec::new();
        let mut offset = 0;
        for line in input.split('\n') {
            lines.push(Line { text: line, offset });
            offset += line.len() + 1; // +1 for the newline
        }
        // If input ends with \n, the last split produces an empty trailing element.
        // Remove it so we don't generate a spurious empty paragraph.
        if input.ends_with('\n') && lines.last().map(|l| l.text.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        Self { lines, pos: 0 }
    }

    pub(crate) fn parse(&mut self) -> Result<Document, Vec<ParseError>> {
        let mut doc = Document::new();
        let errors: Vec<ParseError> = Vec::new();

        // Parse metadata at the top
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;
            if let Some(rest) = line.strip_prefix('#') {
                if let Some((key, value)) = rest.split_once(':') {
                    let key = key.trim().to_string();
                    let value = value.trim().to_string();
                    // Only treat as metadata if key looks like an identifier
                    if !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        doc.metadata.insert(key, value);
                        self.pos += 1;
                        continue;
                    }
                }
            }
            break;
        }

        // Skip blank lines after metadata
        self.skip_blank_lines();

        // Parse blocks
        while self.pos < self.lines.len() {
            self.skip_blank_lines();
            if self.pos >= self.lines.len() {
                break;
            }

            let block = self.parse_block();
            if let Some(b) = block {
                doc.blocks.push(b);
            }
        }

        if errors.is_empty() {
            Ok(doc)
        } else {
            Err(errors)
        }
    }

    fn skip_blank_lines(&mut self) {
        while self.pos < self.lines.len() && self.lines[self.pos].text.trim().is_empty() {
            self.pos += 1;
        }
    }

    fn current_line(&self) -> &str {
        self.lines[self.pos].text
    }

    fn current_offset(&self) -> usize {
        self.lines[self.pos].offset
    }

    fn parse_block(&mut self) -> Option<Block> {
        if self.pos >= self.lines.len() {
            return None;
        }

        let line = self.current_line();

        // Thematic break: --- or more dashes
        if line.trim().starts_with("---") && line.trim().chars().all(|c| c == '-') {
            let start = self.current_offset();
            let end = start + line.len();
            self.pos += 1;
            return Some(Block {
                kind: BlockKind::ThematicBreak,
                span: Span::new(start, end),
            });
        }

        // Code fence: ```
        if line.trim_start().starts_with("```") {
            return self.parse_code_fence();
        }

        // Block directive: @type[attrs]: title
        if line.starts_with('@') {
            return self.parse_directive();
        }

        // Block quote: > prefix
        if line.starts_with('>') {
            return self.parse_block_quote();
        }

        // Unordered list: - prefix
        if line.starts_with("- ") || line == "-" {
            return self.parse_unordered_list();
        }

        // Ordered list: N. prefix
        if self.is_ordered_list_line(line) {
            return self.parse_ordered_list();
        }

        // Default: paragraph
        self.parse_paragraph()
    }

    fn parse_code_fence(&mut self) -> Option<Block> {
        let start = self.current_offset();
        let opening_line = self.current_line().to_string();
        let trimmed = opening_line.trim_start();
        let after_fence = trimmed.trim_start_matches('`');
        let lang = if after_fence.trim().is_empty() {
            None
        } else {
            Some(after_fence.trim().to_string())
        };

        self.pos += 1;

        let mut code_lines: Vec<&str> = Vec::new();
        let mut end = start + opening_line.len();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;
            end = self.lines[self.pos].offset + line.len();
            if line.trim_start().starts_with("```") && line.trim().chars().all(|c| c == '`') {
                self.pos += 1;
                break;
            }
            code_lines.push(line);
            self.pos += 1;
        }

        let code = code_lines.join("\n");

        Some(Block {
            kind: BlockKind::CodeBlock {
                lang,
                attrs: Attrs::new(),
                code,
            },
            span: Span::new(start, end),
        })
    }

    fn parse_directive(&mut self) -> Option<Block> {
        let start = self.current_offset();
        let line = self.current_line().to_string();

        // Parse @type[attrs]: title
        let rest = &line[1..]; // skip '@'

        // Find directive type
        let type_end = rest
            .find(|c: char| c == '[' || c == ':' || c.is_whitespace())
            .unwrap_or(rest.len());
        let directive_type = &rest[..type_end];
        let after_type = &rest[type_end..];

        // Parse optional [attrs]
        let (attrs, after_attrs) = if after_type.starts_with('[') {
            if let Some(close) = after_type.find(']') {
                let attr_str = &after_type[1..close];
                (parse_attrs(attr_str), &after_type[close + 1..])
            } else {
                (Attrs::new(), after_type)
            }
        } else {
            (Attrs::new(), after_type)
        };

        // Parse optional : title
        let title_str = if let Some(rest) = after_attrs.strip_prefix(':') {
            rest.trim()
        } else {
            after_attrs.trim()
        };

        self.pos += 1;

        // Check for skill block types before collecting body lines,
        // since skill blocks use @end termination instead of blank-line termination.
        if let Some(skill_type) = is_skill_block_type(directive_type) {
            return self.parse_skill_block(skill_type, attrs, title_str, start);
        }

        // Collect body lines (stop at blank line, new directive, or metadata)
        let body_lines = self.collect_body_lines();
        let end = if body_lines.is_empty() {
            start + line.len()
        } else {
            body_lines.last().unwrap().0 + body_lines.last().unwrap().1.len()
        };

        let body_text: String = body_lines.iter().map(|(_, l)| *l).collect::<Vec<&str>>().join("\n");
        let span = Span::new(start, end);

        match directive_type {
            "section" => {
                let title = parse_inline(title_str);
                // Parse children recursively from body text
                let children = if body_text.is_empty() {
                    Vec::new()
                } else {
                    let mut child_parser = BlockParser::new(&body_text);
                    // Skip metadata parsing for child parser - just parse blocks
                    match child_parser.parse() {
                        Ok(child_doc) => child_doc.blocks,
                        Err(_) => Vec::new(),
                    }
                };
                Some(Block {
                    kind: BlockKind::Section {
                        attrs,
                        title,
                        children,
                    },
                    span,
                })
            }

            "callout" => {
                let callout_type = match attrs.get("type").unwrap_or("note") {
                    "warning" => CalloutType::Warning,
                    "info" => CalloutType::Info,
                    "tip" => CalloutType::Tip,
                    _ => CalloutType::Note,
                };
                let content = parse_inline(&body_text);
                Some(Block {
                    kind: BlockKind::Callout {
                        callout_type,
                        attrs,
                        content,
                    },
                    span,
                })
            }

            "table" => {
                let caption = if title_str.is_empty() {
                    None
                } else {
                    Some(parse_inline(title_str))
                };
                let (headers, rows) = self.parse_table_body(&body_text);
                Some(Block {
                    kind: BlockKind::Table {
                        attrs,
                        caption,
                        headers,
                        rows,
                    },
                    span,
                })
            }

            "figure" => {
                let caption = if title_str.is_empty() {
                    None
                } else {
                    Some(parse_inline(title_str))
                };
                let src = attrs
                    .get("src")
                    .unwrap_or("")
                    .to_string();
                let mut attrs = attrs;
                attrs.pairs.remove("src");
                let meta = extract_media_meta(&mut attrs);
                Some(Block {
                    kind: BlockKind::Figure {
                        attrs,
                        caption,
                        src,
                        meta,
                    },
                    span,
                })
            }

            "audio" => {
                let caption = if title_str.is_empty() {
                    None
                } else {
                    Some(parse_inline(title_str))
                };
                let src = attrs
                    .get("src")
                    .unwrap_or("")
                    .to_string();
                let mut attrs = attrs;
                attrs.pairs.remove("src");
                let meta = extract_media_meta(&mut attrs);
                Some(Block {
                    kind: BlockKind::Audio {
                        attrs,
                        caption,
                        src,
                        meta,
                    },
                    span,
                })
            }

            "video" => {
                let caption = if title_str.is_empty() {
                    None
                } else {
                    Some(parse_inline(title_str))
                };
                let src = attrs
                    .get("src")
                    .unwrap_or("")
                    .to_string();
                let mut attrs = attrs;
                attrs.pairs.remove("src");
                let meta = extract_media_meta(&mut attrs);
                Some(Block {
                    kind: BlockKind::Video {
                        attrs,
                        caption,
                        src,
                        meta,
                    },
                    span,
                })
            }

            "code" => {
                let lang = attrs.get("lang").map(|s| s.to_string());
                Some(Block {
                    kind: BlockKind::CodeBlock {
                        lang,
                        attrs,
                        code: body_text,
                    },
                    span,
                })
            }

            _ => {
                // Semantic block types
                let block_type = match directive_type {
                    "claim" => SemanticBlockType::Claim,
                    "evidence" => SemanticBlockType::Evidence,
                    "definition" => SemanticBlockType::Definition,
                    "theorem" => SemanticBlockType::Theorem,
                    "assumption" => SemanticBlockType::Assumption,
                    "result" => SemanticBlockType::Result,
                    "conclusion" => SemanticBlockType::Conclusion,
                    "requirement" => SemanticBlockType::Requirement,
                    "recommendation" => SemanticBlockType::Recommendation,
                    _ => {
                        // Unknown directive, treat as a paragraph
                        let content = parse_inline(&format!("@{}", &line[1..]));
                        return Some(Block {
                            kind: BlockKind::Paragraph { content },
                            span,
                        });
                    }
                };

                let title = if title_str.is_empty() {
                    None
                } else {
                    Some(parse_inline(title_str))
                };
                let content = parse_inline(&body_text);

                Some(Block {
                    kind: BlockKind::SemanticBlock {
                        block_type,
                        attrs,
                        title,
                        content,
                    },
                    span,
                })
            }
        }
    }

    /// Collect body lines for a directive. Stops at blank lines, new directives, or metadata lines.
    fn collect_body_lines(&mut self) -> Vec<(usize, &'a str)> {
        let mut lines = Vec::new();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;
            // Stop conditions
            if line.trim().is_empty() {
                break;
            }
            if line.starts_with('@') {
                break;
            }
            if line.starts_with('#') && line.contains(':') {
                // Check if it looks like metadata
                let rest = &line[1..];
                if let Some((key, _)) = rest.split_once(':') {
                    let key = key.trim();
                    if !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        break;
                    }
                }
            }
            lines.push((self.lines[self.pos].offset, line));
            self.pos += 1;
        }
        lines
    }

    fn parse_skill_block(
        &mut self,
        skill_type: SkillBlockType,
        attrs: Attrs,
        title_str: &str,
        start: usize,
    ) -> Option<Block> {
        let is_container = is_container(&skill_type);
        let title = if title_str.is_empty() {
            None
        } else {
            Some(parse_inline(title_str))
        };

        let mut content_lines: Vec<&str> = Vec::new();
        let mut children: Vec<Block> = Vec::new();
        let mut end = start;
        let expected_closer = container_closer_name(&skill_type);

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;
            let trimmed_line = line.trim();

            // `@/name` closer
            if let Some(rest) = trimmed_line.strip_prefix("@/") {
                let closer_name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphabetic() || *c == '_')
                    .collect();
                if is_container {
                    if let Some(expected) = expected_closer {
                        if closer_name == expected {
                            end = self.lines[self.pos].offset + line.len();
                            self.pos += 1;
                            break;
                        }
                    }
                    // Wrong closer — stop and let outer parser handle.
                    break;
                } else {
                    // Leaf: don't consume the closer, let outer break.
                    break;
                }
            }
            // Leaf auto-closes at any line starting with `@`.
            if !is_container && line.trim_start().starts_with('@') {
                break;
            }

            // Nested skill block directive inside a container
            if is_container && line.trim_start().starts_with('@') {
                let trimmed = line.trim_start();
                let inner_rest = &trimmed[1..]; // skip '@'
                let inner_type_end = inner_rest
                    .find(|c: char| c == '[' || c == ':' || c.is_whitespace())
                    .unwrap_or(inner_rest.len());
                let inner_directive = &inner_rest[..inner_type_end];
                let inner_after_type = &inner_rest[inner_type_end..];

                if let Some(inner_skill_type) = is_skill_block_type(inner_directive) {
                    // Parse optional [attrs]
                    let (inner_attrs, inner_after_attrs) = if inner_after_type.starts_with('[') {
                        if let Some(close) = inner_after_type.find(']') {
                            let attr_str = &inner_after_type[1..close];
                            (parse_attrs(attr_str), &inner_after_type[close + 1..])
                        } else {
                            (Attrs::new(), inner_after_type)
                        }
                    } else {
                        (Attrs::new(), inner_after_type)
                    };

                    // Parse optional : title
                    let inner_title_str = if let Some(rest) = inner_after_attrs.strip_prefix(':') {
                        rest.trim()
                    } else {
                        inner_after_attrs.trim()
                    };

                    let inner_start = self.lines[self.pos].offset;
                    self.pos += 1;

                    if let Some(child) =
                        self.parse_skill_block(inner_skill_type, inner_attrs, inner_title_str, inner_start)
                    {
                        children.push(child);
                    }
                    continue;
                }
            }

            // Regular content line
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                content_lines.push(trimmed);
            }
            end = self.lines[self.pos].offset + line.len();
            self.pos += 1;
        }

        let content_text = content_lines.join("\n");
        let content = if content_text.is_empty() {
            vec![]
        } else {
            parse_inline(&content_text)
        };

        Some(Block {
            kind: BlockKind::SkillBlock {
                skill_type,
                attrs,
                title,
                content,
                children,
            },
            span: Span::new(start, end),
        })
    }

    fn parse_table_body(&self, body: &str) -> (Vec<Vec<Inline>>, Vec<Vec<Vec<Inline>>>) {
        let mut headers = Vec::new();
        let mut rows = Vec::new();
        let mut is_header = true;

        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Separator line (e.g. |---|---|)
            if line.contains("---") && line.starts_with('|') {
                is_header = false;
                continue;
            }
            let cells: Vec<&str> = line
                .trim_start_matches('|')
                .trim_end_matches('|')
                .split('|')
                .map(|s| s.trim())
                .collect();

            if is_header {
                headers = cells.iter().map(|c| parse_inline(c)).collect();
                is_header = false;
            } else {
                let row: Vec<Vec<Inline>> = cells.iter().map(|c| parse_inline(c)).collect();
                rows.push(row);
            }
        }

        (headers, rows)
    }

    fn parse_block_quote(&mut self) -> Option<Block> {
        let start = self.current_offset();
        let mut quote_lines: Vec<String> = Vec::new();
        let mut end = start;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;
            if let Some(rest) = line.strip_prefix("> ") {
                quote_lines.push(rest.to_string());
                end = self.lines[self.pos].offset + line.len();
                self.pos += 1;
            } else if line == ">" {
                quote_lines.push(String::new());
                end = self.lines[self.pos].offset + line.len();
                self.pos += 1;
            } else {
                break;
            }
        }

        let inner_text = quote_lines.join("\n");
        let mut inner_parser = BlockParser::new(&inner_text);
        let content = match inner_parser.parse() {
            Ok(doc) => doc.blocks,
            Err(_) => Vec::new(),
        };

        Some(Block {
            kind: BlockKind::BlockQuote { content },
            span: Span::new(start, end),
        })
    }

    fn parse_unordered_list(&mut self) -> Option<Block> {
        let start = self.current_offset();
        let mut items: Vec<ListItem> = Vec::new();
        let mut end = start;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;
            if let Some(rest) = line.strip_prefix("- ") {
                end = self.lines[self.pos].offset + line.len();
                self.pos += 1;

                // Collect nested items
                let mut nested_lines: Vec<&str> = Vec::new();
                while self.pos < self.lines.len() {
                    let inner = self.lines[self.pos].text;
                    if inner.starts_with("  - ") || inner.starts_with("  ") && !inner.starts_with("  - ") && !inner.trim().is_empty() {
                        // Part of nested content under this item
                        if inner.starts_with("  ") {
                            nested_lines.push(inner.trim_start());
                        }
                        end = self.lines[self.pos].offset + inner.len();
                        self.pos += 1;
                    } else {
                        break;
                    }
                }

                let content = parse_inline(rest.trim());
                let children = if nested_lines.is_empty() {
                    Vec::new()
                } else {
                    // Parse nested content as sub-list or blocks
                    let nested_text = nested_lines.join("\n");
                    let mut nested_parser = BlockParser::new(&nested_text);
                    match nested_parser.parse() {
                        Ok(doc) => doc.blocks,
                        Err(_) => Vec::new(),
                    }
                };

                items.push(ListItem { content, children });
            } else if line == "-" {
                end = self.lines[self.pos].offset + line.len();
                self.pos += 1;
                items.push(ListItem {
                    content: Vec::new(),
                    children: Vec::new(),
                });
            } else {
                break;
            }
        }

        Some(Block {
            kind: BlockKind::List {
                ordered: false,
                items,
            },
            span: Span::new(start, end),
        })
    }

    fn is_ordered_list_line(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        if let Some(dot_pos) = trimmed.find(". ") {
            trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) && dot_pos > 0
        } else {
            false
        }
    }

    fn parse_ordered_list(&mut self) -> Option<Block> {
        let start = self.current_offset();
        let mut items: Vec<ListItem> = Vec::new();
        let mut end = start;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;
            if self.is_ordered_list_line(line) {
                end = self.lines[self.pos].offset + line.len();
                if let Some(dot_pos) = line.find(". ") {
                    let text = &line[dot_pos + 2..];
                    let content = parse_inline(text.trim());
                    items.push(ListItem {
                        content,
                        children: Vec::new(),
                    });
                }
                self.pos += 1;
            } else {
                break;
            }
        }

        Some(Block {
            kind: BlockKind::List {
                ordered: true,
                items,
            },
            span: Span::new(start, end),
        })
    }

    fn parse_paragraph(&mut self) -> Option<Block> {
        let start = self.current_offset();
        let mut text_parts: Vec<&str> = Vec::new();
        let mut end = start;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].text;

            // Stop at blank lines
            if line.trim().is_empty() {
                break;
            }
            // Stop at special block starters
            if line.starts_with('@')
                || line.starts_with('>')
                || line.starts_with("- ")
                || line == "-"
                || line.starts_with("```")
                || (line.starts_with("---") && line.trim().chars().all(|c| c == '-'))
                || self.is_ordered_list_line(line)
            {
                break;
            }
            // Stop at metadata lines
            if line.starts_with('#') && line.contains(':') {
                let rest = &line[1..];
                if let Some((key, _)) = rest.split_once(':') {
                    let key = key.trim();
                    if !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        break;
                    }
                }
            }

            end = self.lines[self.pos].offset + line.len();
            text_parts.push(line);
            self.pos += 1;
        }

        if text_parts.is_empty() {
            // Consume the line to avoid infinite loops on unrecognized content
            if self.pos < self.lines.len() {
                self.pos += 1;
            }
            return None;
        }

        let joined = text_parts.join(" ");
        let content = parse_inline(&joined);

        Some(Block {
            kind: BlockKind::Paragraph { content },
            span: Span::new(start, end),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let mut parser = BlockParser::new("");
        let doc = parser.parse().unwrap();
        assert!(doc.metadata.is_empty());
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn parse_metadata_only() {
        let mut parser = BlockParser::new("#title: Hello\n#author: Test\n");
        let doc = parser.parse().unwrap();
        assert_eq!(doc.metadata.get("title").unwrap(), "Hello");
        assert_eq!(doc.metadata.get("author").unwrap(), "Test");
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn parse_simple_paragraph() {
        let mut parser = BlockParser::new("Hello world.\n");
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(&doc.blocks[0].kind, BlockKind::Paragraph { .. }));
    }

    #[test]
    fn parse_multi_line_paragraph() {
        let mut parser = BlockParser::new("Line one\nLine two\n");
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::Paragraph { content } = &doc.blocks[0].kind {
            // Lines joined with space
            let text: String = content.iter().map(|i| match i {
                Inline::Text { text } => text.as_str(),
                _ => "",
            }).collect();
            assert!(text.contains("Line one Line two"));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn parse_thematic_break() {
        let mut parser = BlockParser::new("---\n");
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(&doc.blocks[0].kind, BlockKind::ThematicBreak));
    }

    #[test]
    fn parse_code_fence() {
        let input = "```rust\nfn main() {}\n```\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::CodeBlock { lang, code, .. } = &doc.blocks[0].kind {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn parse_section_directive() {
        let input = "@section[id=intro]: Introduction\nSome content here.\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::Section { attrs, title, children } = &doc.blocks[0].kind {
            assert_eq!(attrs.id, Some("intro".to_string()));
            assert!(!title.is_empty());
            assert_eq!(children.len(), 1);
        } else {
            panic!("expected section");
        }
    }

    #[test]
    fn parse_block_quote() {
        let input = "> This is a quote\n> Second line\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(&doc.blocks[0].kind, BlockKind::BlockQuote { .. }));
    }

    #[test]
    fn parse_unordered_list() {
        let input = "- Item one\n- Item two\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::List { ordered, items } = &doc.blocks[0].kind {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn parse_ordered_list() {
        let input = "1. First\n2. Second\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::List { ordered, items } = &doc.blocks[0].kind {
            assert!(ordered);
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn parse_semantic_block() {
        let input = "@claim[id=c1]: Main Claim\nThe evidence supports this.\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::SemanticBlock { block_type, attrs, title, .. } = &doc.blocks[0].kind {
            assert!(matches!(block_type, SemanticBlockType::Claim));
            assert_eq!(attrs.id, Some("c1".to_string()));
            assert!(title.is_some());
        } else {
            panic!("expected semantic block");
        }
    }

    #[test]
    fn parse_skill_container_with_closer() {
        let input = "@skill[name=debugging]\n  Some intro text.\n@/skill\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::SkillBlock { skill_type, attrs, content, children, .. } = &doc.blocks[0].kind {
            assert!(matches!(skill_type, SkillBlockType::Skill));
            assert_eq!(attrs.get("name"), Some("debugging"));
            assert!(!content.is_empty());
            assert!(children.is_empty());
        } else {
            panic!("expected SkillBlock, got {:?}", doc.blocks[0].kind);
        }
    }

    #[test]
    fn parse_skill_with_inner_blocks() {
        let input = "\
@skill[name=debugging version=1.0]
@precondition
  User has reported a bug.
@step[order=1]
  Reproduce the issue.
@verify
  Fix resolves issue without regressions.
@/skill
";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::SkillBlock { children, .. } = &doc.blocks[0].kind {
            assert_eq!(children.len(), 3);
            if let BlockKind::SkillBlock { skill_type, .. } = &children[0].kind {
                assert!(matches!(skill_type, SkillBlockType::Precondition));
            } else {
                panic!("expected precondition");
            }
            if let BlockKind::SkillBlock { skill_type, attrs, .. } = &children[1].kind {
                assert!(matches!(skill_type, SkillBlockType::Step));
                assert_eq!(attrs.get("order"), Some("1"));
            } else {
                panic!("expected step");
            }
            if let BlockKind::SkillBlock { skill_type, .. } = &children[2].kind {
                assert!(matches!(skill_type, SkillBlockType::Verify));
            } else {
                panic!("expected verify");
            }
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn parse_skill_with_free_text_and_blocks() {
        let input = "\
@skill[name=test]
Some free text intro.

@step[order=1]
  Do something.
@/skill
";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::SkillBlock { content, children, .. } = &doc.blocks[0].kind {
            assert!(!content.is_empty());
            assert_eq!(children.len(), 1);
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn parse_audio_directive() {
        let input = "@audio[src=file.mp3]: My Audio Caption\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::Audio { attrs: _, caption, src, .. } = &doc.blocks[0].kind {
            assert_eq!(src, "file.mp3");
            assert!(caption.is_some());
        } else {
            panic!("expected Audio, got {:?}", doc.blocks[0].kind);
        }
    }

    #[test]
    fn parse_video_directive() {
        let input = "@video[src=clip.mp4]: Video Title\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::Video { attrs: _, caption, src, .. } = &doc.blocks[0].kind {
            assert_eq!(src, "clip.mp4");
            assert!(caption.is_some());
        } else {
            panic!("expected Video, got {:?}", doc.blocks[0].kind);
        }
    }

    #[test]
    fn parse_figure_with_media_meta() {
        let input = "@figure[src=photo.jpg, alt=A nice photo, width=800, height=600]: Caption\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::Figure { src, meta, attrs, .. } = &doc.blocks[0].kind {
            assert_eq!(src, "photo.jpg");
            assert_eq!(meta.alt.as_deref(), Some("A nice photo"));
            assert_eq!(meta.width, Some(800));
            assert_eq!(meta.height, Some(600));
            // These should have been removed from attrs.pairs
            assert!(attrs.pairs.get("alt").is_none());
            assert!(attrs.pairs.get("width").is_none());
        } else {
            panic!("expected Figure, got {:?}", doc.blocks[0].kind);
        }
    }

    #[test]
    fn parse_video_with_poster_and_duration() {
        let input = "@video[src=clip.mp4, poster=thumb.jpg, duration=120.5]: Video\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::Video { meta, .. } = &doc.blocks[0].kind {
            assert_eq!(meta.poster.as_deref(), Some("thumb.jpg"));
            assert_eq!(meta.duration, Some(120.5));
        } else {
            panic!("expected Video");
        }
    }

    #[test]
    fn parse_audio_with_mime() {
        let input = "@audio[src=song.ogg, mime=audio/ogg]: Song\n";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::Audio { meta, .. } = &doc.blocks[0].kind {
            assert_eq!(meta.mime.as_deref(), Some("audio/ogg"));
        } else {
            panic!("expected Audio");
        }
    }

    // ======================================================================
    // V2 syntax tests (container closers, leaf auto-close)
    // ======================================================================

    #[test]
    fn v2_skill_with_leaf_children() {
        let input = "\
@skill[name=\"t\"]
@precondition
Ready.
@step[order=1]
Do it.
@verify
Passed.
@/skill
";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        assert_eq!(doc.blocks.len(), 1);
        if let BlockKind::SkillBlock { children, skill_type, .. } = &doc.blocks[0].kind {
            assert!(matches!(skill_type, SkillBlockType::Skill));
            assert_eq!(children.len(), 3);
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn v2_leaf_auto_closes_at_next_directive() {
        let input = "\
@skill[name=\"t\"]
@step[order=1]
Line one.
Line two.
@step[order=2]
Other body.
@/skill
";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        if let BlockKind::SkillBlock { children, .. } = &doc.blocks[0].kind {
            assert_eq!(children.len(), 2);
            // First step should have "Line one." and "Line two." joined.
            if let BlockKind::SkillBlock { content, .. } = &children[0].kind {
                let text: String = content.iter().filter_map(|i| match i {
                    Inline::Text { text } => Some(text.clone()),
                    _ => None,
                }).collect();
                assert!(text.contains("Line one"));
                assert!(text.contains("Line two"));
            }
        }
    }

    #[test]
    fn v2_artifact_skill_container() {
        let input = "\
@artifact_skill[name=\"gen\"]
@template
A template.
@/artifact_skill
";
        let mut parser = BlockParser::new(input);
        let doc = parser.parse().unwrap();
        if let BlockKind::SkillBlock { skill_type, children, .. } = &doc.blocks[0].kind {
            assert!(matches!(skill_type, SkillBlockType::ArtifactSkill));
            assert_eq!(children.len(), 1);
        } else {
            panic!("expected ArtifactSkill");
        }
    }

    #[test]
    fn legacy_v1_input_is_rejected() {
        let err = crate::parse("@skill[name=a]\n@step\nBody.\n@end\n@end\n").unwrap_err();
        assert!(err[0].message.contains("legacy v1"));
        assert!(err[0].message.contains("migrate-syntax"));
    }
}
