//! Symbol table construction for go-to-definition support.
//!
//! Walks the AST to collect block IDs and their byte offsets,
//! enabling resolution of @ref{target} references to definitions.

use aif_core::ast::*;
use std::collections::HashMap;

/// A symbol definition: block ID → byte offset range in source.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SymbolLocation {
    pub id: String,
    pub start: usize,
    pub end: usize,
}

/// Collect all block IDs from a document into a lookup table.
#[allow(dead_code)]
pub fn collect_symbols(doc: &Document) -> HashMap<String, SymbolLocation> {
    let mut symbols = HashMap::new();
    for block in &doc.blocks {
        collect_block_symbols(block, &mut symbols);
    }
    symbols
}

/// Extract the attrs from a block kind, if it has one.
#[allow(dead_code)]
fn block_attrs(kind: &BlockKind) -> Option<&Attrs> {
    match kind {
        BlockKind::Section { attrs, .. }
        | BlockKind::SemanticBlock { attrs, .. }
        | BlockKind::Callout { attrs, .. }
        | BlockKind::Table { attrs, .. }
        | BlockKind::Figure { attrs, .. }
        | BlockKind::Audio { attrs, .. }
        | BlockKind::Video { attrs, .. }
        | BlockKind::CodeBlock { attrs, .. }
        | BlockKind::SkillBlock { attrs, .. } => Some(attrs),
        BlockKind::Paragraph { .. }
        | BlockKind::BlockQuote { .. }
        | BlockKind::List { .. }
        | BlockKind::ThematicBreak => None,
    }
}

#[allow(dead_code)]
fn collect_block_symbols(block: &Block, symbols: &mut HashMap<String, SymbolLocation>) {
    if let Some(attrs) = block_attrs(&block.kind) {
        if let Some(ref id) = attrs.id {
            symbols.insert(
                id.clone(),
                SymbolLocation {
                    id: id.clone(),
                    start: block.span.start,
                    end: block.span.end,
                },
            );
        }
    }

    // Recurse into child blocks
    match &block.kind {
        BlockKind::Section { children, .. } => {
            for child in children {
                collect_block_symbols(child, symbols);
            }
        }
        BlockKind::SkillBlock { children, .. } => {
            for child in children {
                collect_block_symbols(child, symbols);
            }
        }
        BlockKind::BlockQuote { content } => {
            for child in content {
                collect_block_symbols(child, symbols);
            }
        }
        BlockKind::List { items, .. } => {
            for item in items {
                for child in &item.children {
                    collect_block_symbols(child, symbols);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;
    use std::collections::BTreeMap;

    fn make_attrs(id: Option<&str>) -> Attrs {
        Attrs {
            id: id.map(|s| s.to_string()),
            pairs: BTreeMap::new(),
        }
    }

    #[test]
    fn collect_symbols_from_empty_doc() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![],
        };
        let symbols = collect_symbols(&doc);
        assert!(symbols.is_empty());
    }

    #[test]
    fn collect_symbols_finds_section_id() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::Section {
                    attrs: make_attrs(Some("intro")),
                    title: vec![],
                    children: vec![],
                },
                span: Span::new(0, 50),
            }],
        };
        let symbols = collect_symbols(&doc);
        assert!(symbols.contains_key("intro"));
        assert_eq!(symbols["intro"].start, 0);
        assert_eq!(symbols["intro"].end, 50);
    }

    #[test]
    fn collect_symbols_finds_nested_ids() {
        let inner = Block {
            kind: BlockKind::SemanticBlock {
                attrs: make_attrs(Some("claim1")),
                block_type: SemanticBlockType::Claim,
                title: None,
                content: vec![],
            },
            span: Span::new(20, 40),
        };
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::Section {
                    attrs: make_attrs(Some("sec1")),
                    title: vec![],
                    children: vec![inner],
                },
                span: Span::new(0, 50),
            }],
        };
        let symbols = collect_symbols(&doc);
        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains_key("sec1"));
        assert!(symbols.contains_key("claim1"));
    }

    #[test]
    fn blocks_without_id_are_skipped() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::Section {
                    attrs: make_attrs(None),
                    title: vec![],
                    children: vec![],
                },
                span: Span::new(0, 10),
            }],
        };
        let symbols = collect_symbols(&doc);
        assert!(symbols.is_empty());
    }

    #[test]
    fn paragraph_blocks_have_no_attrs() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "hello".to_string() }],
                },
                span: Span::new(0, 5),
            }],
        };
        let symbols = collect_symbols(&doc);
        assert!(symbols.is_empty());
    }

    #[test]
    fn collect_symbols_from_skill_block() {
        let step = Block {
            kind: BlockKind::SkillBlock {
                attrs: make_attrs(Some("step1")),
                skill_type: SkillBlockType::Step,
                title: None,
                content: vec![],
                children: vec![],
            },
            span: Span::new(30, 60),
        };
        let skill = Block {
            kind: BlockKind::SkillBlock {
                attrs: make_attrs(Some("my_skill")),
                skill_type: SkillBlockType::ArtifactSkill,
                title: None,
                content: vec![],
                children: vec![step],
            },
            span: Span::new(0, 100),
        };
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![skill],
        };
        let symbols = collect_symbols(&doc);
        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains_key("my_skill"));
        assert!(symbols.contains_key("step1"));
    }

    #[test]
    fn collect_symbols_from_blockquote() {
        let inner = Block {
            kind: BlockKind::Section {
                attrs: make_attrs(Some("inner_sec")),
                title: vec![],
                children: vec![],
            },
            span: Span::new(5, 25),
        };
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::BlockQuote {
                    content: vec![inner],
                },
                span: Span::new(0, 30),
            }],
        };
        let symbols = collect_symbols(&doc);
        assert_eq!(symbols.len(), 1);
        assert!(symbols.contains_key("inner_sec"));
    }

    #[test]
    fn collect_symbols_multiple_top_level_blocks() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![
                Block {
                    kind: BlockKind::Section {
                        attrs: make_attrs(Some("s1")),
                        title: vec![],
                        children: vec![],
                    },
                    span: Span::new(0, 20),
                },
                Block {
                    kind: BlockKind::Table {
                        attrs: make_attrs(Some("t1")),
                        caption: None,
                        headers: vec![],
                        rows: vec![],
                    },
                    span: Span::new(20, 50),
                },
                Block {
                    kind: BlockKind::CodeBlock {
                        attrs: make_attrs(Some("code1")),
                        lang: Some("rust".to_string()),
                        code: "fn main() {}".to_string(),
                    },
                    span: Span::new(50, 80),
                },
            ],
        };
        let symbols = collect_symbols(&doc);
        assert_eq!(symbols.len(), 3);
        assert!(symbols.contains_key("s1"));
        assert!(symbols.contains_key("t1"));
        assert!(symbols.contains_key("code1"));
    }

    #[test]
    fn symbol_location_preserves_span() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::Figure {
                    attrs: make_attrs(Some("fig1")),
                    src: "image.png".to_string(),
                    caption: None,
                    meta: MediaMeta::default(),
                },
                span: Span::new(42, 99),
            }],
        };
        let symbols = collect_symbols(&doc);
        let loc = &symbols["fig1"];
        assert_eq!(loc.id, "fig1");
        assert_eq!(loc.start, 42);
        assert_eq!(loc.end, 99);
    }

    #[test]
    fn collect_from_parsed_document() {
        // Integration-style: parse real AIF text, then collect symbols
        let text = "\
#title: Test

@section[id=intro]: Introduction
A paragraph.

@section[id=body]: Body
Another paragraph.
";
        let doc = aif_parser::parse(text).expect("should parse");
        let symbols = collect_symbols(&doc);
        assert!(
            symbols.contains_key("intro"),
            "Should find 'intro' section, got: {:?}",
            symbols.keys().collect::<Vec<_>>()
        );
        assert!(
            symbols.contains_key("body"),
            "Should find 'body' section"
        );
        assert_eq!(symbols.len(), 2);
    }
}
