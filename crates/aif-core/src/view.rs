//! Multi-view compilation: filter AST blocks based on target audience.
//!
//! Three view modes:
//! - **Author**: Full document, no filtering.
//! - **Llm**: Strips verbose content (@example, @scenario blocks, collapses code blocks).
//! - **Api**: Only @tool, @output_contract, @precondition blocks + metadata.

use crate::ast::*;

/// Target audience for document compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Full document — no filtering.
    Author,
    /// LLM-optimized — strips @example, @scenario blocks and collapses large code blocks.
    Llm,
    /// API reference — only @tool, @output_contract, @precondition blocks + metadata.
    Api,
}

impl ViewMode {
    /// Parse a view mode from a string. Returns None for invalid input.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "author" => Some(ViewMode::Author),
            "llm" => Some(ViewMode::Llm),
            "api" => Some(ViewMode::Api),
            _ => None,
        }
    }
}

/// Filter a document for the given view mode. Returns a new document with
/// blocks removed or transformed according to the view.
pub fn filter_for_view(doc: &Document, mode: ViewMode) -> Document {
    match mode {
        ViewMode::Author => doc.clone(),
        ViewMode::Llm => filter_llm(doc),
        ViewMode::Api => filter_api(doc),
    }
}

/// LLM view: strip @example and @scenario skill blocks, collapse large code blocks.
fn filter_llm(doc: &Document) -> Document {
    Document {
        metadata: doc.metadata.clone(),
        blocks: filter_blocks_llm(&doc.blocks),
    }
}

fn filter_blocks_llm(blocks: &[Block]) -> Vec<Block> {
    blocks
        .iter()
        .filter_map(filter_block_llm)
        .collect()
}

fn filter_block_llm(block: &Block) -> Option<Block> {
    match &block.kind {
        BlockKind::SkillBlock {
            skill_type: SkillBlockType::Example | SkillBlockType::Scenario,
            ..
        } => None,
        BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        } => {
            let filtered_children = filter_blocks_llm(children);
            Some(Block {
                kind: BlockKind::SkillBlock {
                    skill_type: skill_type.clone(),
                    attrs: attrs.clone(),
                    title: title.clone(),
                    content: content.clone(),
                    children: filtered_children,
                },
                span: block.span,
            })
        }
        BlockKind::CodeBlock { lang, attrs, code } => {
            // Collapse large code blocks (>20 lines) to first 5 lines + truncation notice
            let lines: Vec<&str> = code.lines().collect();
            if lines.len() > 20 {
                let truncated = format!(
                    "{}\n... ({} lines truncated)",
                    lines[..5].join("\n"),
                    lines.len() - 5
                );
                Some(Block {
                    kind: BlockKind::CodeBlock {
                        lang: lang.clone(),
                        attrs: attrs.clone(),
                        code: truncated,
                    },
                    span: block.span,
                })
            } else {
                Some(block.clone())
            }
        }
        BlockKind::Section {
            attrs,
            title,
            children,
        } => {
            let filtered_children = filter_blocks_llm(children);
            Some(Block {
                kind: BlockKind::Section {
                    attrs: attrs.clone(),
                    title: title.clone(),
                    children: filtered_children,
                },
                span: block.span,
            })
        }
        BlockKind::BlockQuote { content } => {
            let filtered = filter_blocks_llm(content);
            Some(Block {
                kind: BlockKind::BlockQuote { content: filtered },
                span: block.span,
            })
        }
        _ => Some(block.clone()),
    }
}

/// API view: keep only @tool, @output_contract, @precondition skill blocks + metadata.
fn filter_api(doc: &Document) -> Document {
    Document {
        metadata: doc.metadata.clone(),
        blocks: filter_blocks_api(&doc.blocks),
    }
}

fn filter_blocks_api(blocks: &[Block]) -> Vec<Block> {
    blocks
        .iter()
        .filter_map(filter_block_api)
        .collect()
}

fn filter_block_api(block: &Block) -> Option<Block> {
    match &block.kind {
        // Keep top-level skill containers, but filter their children
        BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            title,
            content,
            children,
        } => {
            let filtered_children = filter_blocks_api(children);
            if filtered_children.is_empty() {
                None
            } else {
                Some(Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Skill,
                        attrs: attrs.clone(),
                        title: title.clone(),
                        content: content.clone(),
                        children: filtered_children,
                    },
                    span: block.span,
                })
            }
        }
        // Keep @tool, @output_contract, @precondition
        BlockKind::SkillBlock {
            skill_type: SkillBlockType::Tool | SkillBlockType::OutputContract | SkillBlockType::Precondition,
            ..
        } => Some(block.clone()),
        // Drop all other skill blocks
        BlockKind::SkillBlock { .. } => None,
        // Keep sections but filter children
        BlockKind::Section {
            attrs,
            title,
            children,
        } => {
            let filtered_children = filter_blocks_api(children);
            if filtered_children.is_empty() {
                None
            } else {
                Some(Block {
                    kind: BlockKind::Section {
                        attrs: attrs.clone(),
                        title: title.clone(),
                        children: filtered_children,
                    },
                    span: block.span,
                })
            }
        }
        // Keep semantic blocks that define API contracts (Definition, Requirement)
        BlockKind::SemanticBlock {
            block_type: SemanticBlockType::Definition | SemanticBlockType::Requirement,
            ..
        } => Some(block.clone()),
        // Drop other block types in API view
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn author_view_returns_clone() {
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text {
                        text: "hello".into(),
                    }],
                },
                span: span(),
            }],
        };
        let filtered = filter_for_view(&doc, ViewMode::Author);
        assert_eq!(filtered, doc);
    }

    #[test]
    fn llm_view_strips_example_blocks() {
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Skill,
                        attrs: Attrs::default(),
                        title: None,
                        content: vec![],
                        children: vec![
                            Block {
                                kind: BlockKind::SkillBlock {
                                    skill_type: SkillBlockType::Step,
                                    attrs: Attrs::default(),
                                    title: None,
                                    content: vec![Inline::Text {
                                        text: "step".into(),
                                    }],
                                    children: vec![],
                                },
                                span: span(),
                            },
                            Block {
                                kind: BlockKind::SkillBlock {
                                    skill_type: SkillBlockType::Example,
                                    attrs: Attrs::default(),
                                    title: None,
                                    content: vec![Inline::Text {
                                        text: "example".into(),
                                    }],
                                    children: vec![],
                                },
                                span: span(),
                            },
                            Block {
                                kind: BlockKind::SkillBlock {
                                    skill_type: SkillBlockType::Scenario,
                                    attrs: Attrs::default(),
                                    title: None,
                                    content: vec![Inline::Text {
                                        text: "scenario".into(),
                                    }],
                                    children: vec![],
                                },
                                span: span(),
                            },
                        ],
                    },
                    span: span(),
                },
            ],
        };
        let filtered = filter_for_view(&doc, ViewMode::Llm);
        // Should have skill container with only the step child
        assert_eq!(filtered.blocks.len(), 1);
        if let BlockKind::SkillBlock { children, .. } = &filtered.blocks[0].kind {
            assert_eq!(children.len(), 1);
            if let BlockKind::SkillBlock { skill_type, .. } = &children[0].kind {
                assert_eq!(*skill_type, SkillBlockType::Step);
            } else {
                panic!("expected SkillBlock");
            }
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn llm_view_collapses_large_code() {
        let long_code = (0..30).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::CodeBlock {
                    lang: Some("rust".into()),
                    attrs: Attrs::default(),
                    code: long_code,
                },
                span: span(),
            }],
        };
        let filtered = filter_for_view(&doc, ViewMode::Llm);
        if let BlockKind::CodeBlock { code, .. } = &filtered.blocks[0].kind {
            assert!(code.contains("truncated"));
            assert!(code.contains("line 0"));
            assert!(!code.contains("line 29"));
        } else {
            panic!("expected CodeBlock");
        }
    }

    #[test]
    fn llm_view_keeps_small_code() {
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::CodeBlock {
                    lang: None,
                    attrs: Attrs::default(),
                    code: "small code".into(),
                },
                span: span(),
            }],
        };
        let filtered = filter_for_view(&doc, ViewMode::Llm);
        if let BlockKind::CodeBlock { code, .. } = &filtered.blocks[0].kind {
            assert_eq!(code, "small code");
        }
    }

    #[test]
    fn api_view_keeps_tool_and_contract() {
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Skill,
                    attrs: Attrs::default(),
                    title: None,
                    content: vec![],
                    children: vec![
                        Block {
                            kind: BlockKind::SkillBlock {
                                skill_type: SkillBlockType::Tool,
                                attrs: Attrs::default(),
                                title: None,
                                content: vec![Inline::Text { text: "tool".into() }],
                                children: vec![],
                            },
                            span: span(),
                        },
                        Block {
                            kind: BlockKind::SkillBlock {
                                skill_type: SkillBlockType::OutputContract,
                                attrs: Attrs::default(),
                                title: None,
                                content: vec![Inline::Text {
                                    text: "contract".into(),
                                }],
                                children: vec![],
                            },
                            span: span(),
                        },
                        Block {
                            kind: BlockKind::SkillBlock {
                                skill_type: SkillBlockType::Precondition,
                                attrs: Attrs::default(),
                                title: None,
                                content: vec![Inline::Text {
                                    text: "precondition".into(),
                                }],
                                children: vec![],
                            },
                            span: span(),
                        },
                        Block {
                            kind: BlockKind::SkillBlock {
                                skill_type: SkillBlockType::Step,
                                attrs: Attrs::default(),
                                title: None,
                                content: vec![Inline::Text { text: "step".into() }],
                                children: vec![],
                            },
                            span: span(),
                        },
                        Block {
                            kind: BlockKind::SkillBlock {
                                skill_type: SkillBlockType::Example,
                                attrs: Attrs::default(),
                                title: None,
                                content: vec![Inline::Text {
                                    text: "example".into(),
                                }],
                                children: vec![],
                            },
                            span: span(),
                        },
                    ],
                },
                span: span(),
            }],
        };
        let filtered = filter_for_view(&doc, ViewMode::Api);
        assert_eq!(filtered.blocks.len(), 1);
        if let BlockKind::SkillBlock { children, .. } = &filtered.blocks[0].kind {
            assert_eq!(children.len(), 3); // tool, output_contract, precondition
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn api_view_drops_paragraphs() {
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text {
                        text: "hello".into(),
                    }],
                },
                span: span(),
            }],
        };
        let filtered = filter_for_view(&doc, ViewMode::Api);
        assert!(filtered.blocks.is_empty());
    }

    #[test]
    fn api_view_keeps_definition_blocks() {
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::SemanticBlock {
                    block_type: SemanticBlockType::Definition,
                    attrs: Attrs::default(),
                    title: None,
                    content: vec![Inline::Text {
                        text: "A term defined".into(),
                    }],
                },
                span: span(),
            }],
        };
        let filtered = filter_for_view(&doc, ViewMode::Api);
        assert_eq!(filtered.blocks.len(), 1);
    }

    #[test]
    fn api_view_empty_section_removed() {
        let doc = Document {
            metadata: [("title".into(), "Test".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::Section {
                    attrs: Attrs::default(),
                    title: vec![Inline::Text { text: "Section".into() }],
                    children: vec![Block {
                        kind: BlockKind::Paragraph {
                            content: vec![Inline::Text {
                                text: "text".into(),
                            }],
                        },
                        span: span(),
                    }],
                },
                span: span(),
            }],
        };
        let filtered = filter_for_view(&doc, ViewMode::Api);
        // Section had only a paragraph, which is dropped, so section is empty and dropped
        assert!(filtered.blocks.is_empty());
    }

    #[test]
    fn view_mode_from_str() {
        assert_eq!(ViewMode::from_str("author"), Some(ViewMode::Author));
        assert_eq!(ViewMode::from_str("llm"), Some(ViewMode::Llm));
        assert_eq!(ViewMode::from_str("api"), Some(ViewMode::Api));
        assert_eq!(ViewMode::from_str("invalid"), None);
    }
}
