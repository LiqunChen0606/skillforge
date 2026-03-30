use aif_core::ast::*;
use aif_core::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct ImportMapping {
    pub heading: String,
    pub mapped_to: SkillBlockType,
    pub confidence: Confidence,
}

#[derive(Debug)]
pub struct SkillImportResult {
    pub block: Block,
    pub mappings: Vec<ImportMapping>,
}

/// Classify an H2 heading into a SkillBlockType with a confidence level.
fn classify_heading(heading: &str) -> Option<(SkillBlockType, Confidence)> {
    match heading.to_lowercase().trim() {
        "steps" | "procedure" | "instructions" | "checklist" => {
            Some((SkillBlockType::Step, Confidence::High))
        }
        "prerequisites" | "requirements" | "preconditions" => {
            Some((SkillBlockType::Precondition, Confidence::High))
        }
        "verification" | "testing" | "verify" => {
            Some((SkillBlockType::Verify, Confidence::High))
        }
        "examples" | "usage" => Some((SkillBlockType::Example, Confidence::High)),
        "tools" | "commands" => Some((SkillBlockType::Tool, Confidence::Medium)),
        "fallback" | "recovery" => Some((SkillBlockType::Fallback, Confidence::Medium)),
        "anti-patterns" | "red flags" | "common mistakes" => {
            Some((SkillBlockType::RedFlag, Confidence::Medium))
        }
        "output" | "expected output" => {
            Some((SkillBlockType::OutputContract, Confidence::Medium))
        }
        "decision" | "options" | "choose" => Some((SkillBlockType::Decision, Confidence::Low)),
        _ => None,
    }
}

/// Parse optional YAML frontmatter delimited by `---` lines.
/// Returns (frontmatter key-value pairs, remaining content).
fn parse_frontmatter(input: &str) -> (std::collections::BTreeMap<String, String>, &str) {
    let mut pairs = std::collections::BTreeMap::new();
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return (pairs, input);
    }
    // Find the closing ---
    let after_first = &trimmed[3..];
    let after_first = after_first.trim_start_matches(|c: char| c == '\r' || c == '\n');
    if let Some(end_idx) = after_first.find("\n---") {
        let frontmatter_text = &after_first[..end_idx];
        let rest_start = end_idx + 4; // skip \n---
        let rest = after_first[rest_start..].trim_start_matches(|c: char| c == '\r' || c == '\n');
        // Simple YAML parsing: key: value lines
        for line in frontmatter_text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                pairs.insert(key, value);
            }
        }
        // Calculate byte offset in original input
        let rest_ptr = rest.as_ptr() as usize;
        let input_ptr = input.as_ptr() as usize;
        let offset = rest_ptr - input_ptr;
        return (pairs, &input[offset..]);
    }
    (pairs, input)
}

/// Parse markdown content into sections based on headings.
struct MdSection {
    #[allow(dead_code)]
    level: usize,
    heading: String,
    body: String,
}

fn parse_md_sections(input: &str) -> (Option<String>, String, Vec<MdSection>) {
    let mut h1_title: Option<String> = None;
    let mut h1_body = String::new();
    let mut sections: Vec<MdSection> = Vec::new();

    let mut current_heading: Option<(usize, String)> = None;
    let mut current_body = String::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            // Flush previous section
            if let Some((level, heading)) = current_heading.take() {
                sections.push(MdSection {
                    level,
                    heading,
                    body: current_body.trim().to_string(),
                });
            } else {
                h1_body = current_body.trim().to_string();
            }
            current_body = String::new();
            current_heading = Some((2, trimmed[3..].trim().to_string()));
        } else if trimmed.starts_with("# ") && h1_title.is_none() && current_heading.is_none() {
            h1_title = Some(trimmed[2..].trim().to_string());
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    // Flush last section
    if let Some((level, heading)) = current_heading.take() {
        sections.push(MdSection {
            level,
            heading,
            body: current_body.trim().to_string(),
        });
    } else {
        h1_body = current_body.trim().to_string();
    }

    (h1_title, h1_body, sections)
}

/// Parse a numbered list from body text, returning the list items.
fn parse_numbered_list(body: &str) -> Vec<String> {
    let mut items = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        // Match lines like "1. First step"
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
                items.push(trimmed[dot_pos + 2..].to_string());
            }
        }
    }
    items
}

/// Parse a bullet list from body text, returning the list items.
fn parse_bullet_list(body: &str) -> Vec<String> {
    let mut items = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("- ") {
            items.push(rest.to_string());
        } else if let Some(rest) = trimmed.strip_prefix("* ") {
            items.push(rest.to_string());
        }
    }
    items
}

pub fn import_skill_md(input: &str) -> SkillImportResult {
    let (frontmatter, content) = parse_frontmatter(input);
    let (h1_title, h1_body, sections) = parse_md_sections(content);

    // Determine skill name
    let name = frontmatter
        .get("name")
        .cloned()
        .or_else(|| h1_title.clone())
        .unwrap_or_else(|| "Untitled Skill".to_string());

    // Build attrs
    let mut attrs = Attrs::new();
    attrs.pairs.insert("name".to_string(), name);
    for (key, value) in &frontmatter {
        if !attrs.pairs.contains_key(key) {
            attrs.pairs.insert(key.clone(), value.clone());
        }
    }

    // Build content from H1 body
    let skill_content = if h1_body.is_empty() {
        vec![]
    } else {
        vec![Inline::Text {
            text: h1_body.clone(),
        }]
    };

    // Build children from sections
    let mut children = Vec::new();
    let mut mappings = Vec::new();

    for section in &sections {
        if let Some((skill_type, confidence)) = classify_heading(&section.heading) {
            mappings.push(ImportMapping {
                heading: section.heading.clone(),
                mapped_to: skill_type.clone(),
                confidence: confidence.clone(),
            });

            match &skill_type {
                SkillBlockType::Step => {
                    // Parse numbered list into individual Step blocks
                    let items = parse_numbered_list(&section.body);
                    if items.is_empty() {
                        // Single step block with whole body
                        let mut step_attrs = Attrs::new();
                        step_attrs
                            .pairs
                            .insert("order".to_string(), "1".to_string());
                        children.push(Block {
                            kind: BlockKind::SkillBlock {
                                skill_type: SkillBlockType::Step,
                                attrs: step_attrs,
                                title: None,
                                content: vec![Inline::Text {
                                    text: section.body.clone(),
                                }],
                                children: vec![],
                            },
                            span: Span::empty(),
                        });
                    } else {
                        for (i, item) in items.iter().enumerate() {
                            let mut step_attrs = Attrs::new();
                            step_attrs
                                .pairs
                                .insert("order".to_string(), (i + 1).to_string());
                            children.push(Block {
                                kind: BlockKind::SkillBlock {
                                    skill_type: SkillBlockType::Step,
                                    attrs: step_attrs,
                                    title: None,
                                    content: vec![Inline::Text {
                                        text: item.clone(),
                                    }],
                                    children: vec![],
                                },
                                span: Span::empty(),
                            });
                        }
                    }
                }
                _ => {
                    // Create a single child block of the classified type
                    let body_content = if section.body.is_empty() {
                        vec![]
                    } else {
                        // For list-based sections, join bullet items
                        let bullets = parse_bullet_list(&section.body);
                        if bullets.is_empty() {
                            vec![Inline::Text {
                                text: section.body.clone(),
                            }]
                        } else {
                            vec![Inline::Text {
                                text: bullets.join("\n"),
                            }]
                        }
                    };
                    children.push(Block {
                        kind: BlockKind::SkillBlock {
                            skill_type,
                            attrs: Attrs::new(),
                            title: Some(vec![Inline::Text {
                                text: section.heading.clone(),
                            }]),
                            content: body_content,
                            children: vec![],
                        },
                        span: Span::empty(),
                    });
                }
            }
        } else {
            // Unclassified heading → Section block
            let body_content = if section.body.is_empty() {
                vec![]
            } else {
                vec![Inline::Text {
                    text: section.body.clone(),
                }]
            };
            children.push(Block {
                kind: BlockKind::Section {
                    attrs: Attrs::new(),
                    title: vec![Inline::Text {
                        text: section.heading.clone(),
                    }],
                    children: if body_content.is_empty() {
                        vec![]
                    } else {
                        vec![Block {
                            kind: BlockKind::Paragraph {
                                content: body_content,
                            },
                            span: Span::empty(),
                        }]
                    },
                },
                span: Span::empty(),
            });
        }
    }

    let block = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            title: h1_title.map(|t| vec![Inline::Text { text: t }]),
            content: skill_content,
            children,
        },
        span: Span::empty(),
    };

    SkillImportResult { block, mappings }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_minimal_skill() {
        let input = "# My Skill\n\nSome description.\n";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock {
            skill_type,
            attrs,
            content,
            ..
        } = &result.block.kind
        {
            assert!(matches!(skill_type, SkillBlockType::Skill));
            assert_eq!(attrs.get("name"), Some("My Skill"));
            assert!(!content.is_empty());
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_skill_with_frontmatter() {
        let input = "---\nname: debugging\ndescription: Use when encountering bugs\n---\n\n# Debugging\n\nDebug stuff.\n";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { attrs, .. } = &result.block.kind {
            assert_eq!(attrs.get("name"), Some("debugging"));
            assert_eq!(
                attrs.get("description"),
                Some("Use when encountering bugs")
            );
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_skill_with_steps_heading() {
        let input =
            "# Test Skill\n\n## Steps\n\n1. First step\n2. Second step\n3. Third step\n";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { children, .. } = &result.block.kind {
            let steps: Vec<_> = children
                .iter()
                .filter(|c| {
                    matches!(
                        &c.kind,
                        BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            ..
                        }
                    )
                })
                .collect();
            assert_eq!(steps.len(), 3);
            if let BlockKind::SkillBlock { attrs, .. } = &steps[0].kind {
                assert_eq!(attrs.get("order"), Some("1"));
            }
        } else {
            panic!("expected SkillBlock");
        }
        let step_mapping = result
            .mappings
            .iter()
            .find(|m| m.heading == "Steps")
            .unwrap();
        assert_eq!(step_mapping.confidence, Confidence::High);
    }

    #[test]
    fn import_skill_with_prerequisites() {
        let input = "# Test Skill\n\n## Prerequisites\n\n- Must have access to logs\n- Must have a reproduction case\n";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { children, .. } = &result.block.kind {
            let preconds: Vec<_> = children
                .iter()
                .filter(|c| {
                    matches!(
                        &c.kind,
                        BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Precondition,
                            ..
                        }
                    )
                })
                .collect();
            assert_eq!(preconds.len(), 1);
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_skill_with_verification() {
        let input =
            "# Test Skill\n\n## Verification\n\n- All tests pass\n- No regressions\n";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { children, .. } = &result.block.kind {
            let verifies: Vec<_> = children
                .iter()
                .filter(|c| {
                    matches!(
                        &c.kind,
                        BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Verify,
                            ..
                        }
                    )
                })
                .collect();
            assert_eq!(verifies.len(), 1);
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_reports_confidence_levels() {
        let input = "# Test Skill\n\n## Steps\n\n1. Do something\n\n## Commands\n\nUse these tools.\n\n## Options\n\nPick one.\n";
        let result = import_skill_md(input);
        assert!(result
            .mappings
            .iter()
            .any(|m| m.confidence == Confidence::High));
        assert!(result
            .mappings
            .iter()
            .any(|m| m.confidence == Confidence::Medium));
        assert!(result
            .mappings
            .iter()
            .any(|m| m.confidence == Confidence::Low));
    }
}
