//! Symbol table construction for go-to-definition support.
//!
//! Walks the AST to collect block IDs and their byte offsets,
//! enabling resolution of @ref{target} references to definitions.

use aif_core::ast::*;
use std::collections::HashMap;

/// A symbol definition: block ID → byte offset range in source.
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    pub id: String,
    pub start: usize,
    pub end: usize,
}

/// Collect all block IDs from a document into a lookup table.
pub fn collect_symbols(doc: &Document) -> HashMap<String, SymbolLocation> {
    let mut symbols = HashMap::new();
    for block in &doc.blocks {
        collect_block_symbols(block, &mut symbols);
    }
    symbols
}

/// Extract the attrs from a block kind, if it has one.
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
                    attrs: Attrs {
                        id: Some("intro".to_string()),
                        pairs: BTreeMap::new(),
                    },
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
}
