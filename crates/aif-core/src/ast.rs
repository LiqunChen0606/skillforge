use crate::span::Span;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level document
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub metadata: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            metadata: BTreeMap::new(),
            blocks: Vec::new(),
        }
    }
}

/// A block-level element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BlockKind {
    Section {
        attrs: Attrs,
        title: Vec<Inline>,
        children: Vec<Block>,
    },
    Paragraph {
        content: Vec<Inline>,
    },
    SemanticBlock {
        block_type: SemanticBlockType,
        attrs: Attrs,
        title: Option<Vec<Inline>>,
        content: Vec<Inline>,
    },
    Callout {
        callout_type: CalloutType,
        attrs: Attrs,
        content: Vec<Inline>,
    },
    Table {
        attrs: Attrs,
        caption: Option<Vec<Inline>>,
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Figure {
        attrs: Attrs,
        caption: Option<Vec<Inline>>,
        src: String,
    },
    CodeBlock {
        lang: Option<String>,
        attrs: Attrs,
        code: String,
    },
    BlockQuote {
        content: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListItem {
    pub content: Vec<Inline>,
    pub children: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SemanticBlockType {
    Claim,
    Evidence,
    Definition,
    Theorem,
    Assumption,
    Result,
    Conclusion,
    Requirement,
    Recommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CalloutType {
    Note,
    Warning,
    Info,
    Tip,
}

/// Attributes on a block
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Attrs {
    pub id: Option<String>,
    pub pairs: BTreeMap<String, String>,
}

impl Attrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        if key == "id" {
            self.id.as_deref()
        } else {
            self.pairs.get(key).map(|s| s.as_str())
        }
    }
}

/// Inline-level element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Inline {
    Text { text: String },
    Emphasis { content: Vec<Inline> },
    Strong { content: Vec<Inline> },
    InlineCode { code: String },
    Link { text: Vec<Inline>, url: String },
    Reference { target: String },
    Footnote { content: Vec<Inline> },
    SoftBreak,
    HardBreak,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document() {
        let doc = Document::new();
        assert!(doc.metadata.is_empty());
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn attrs_get() {
        let mut attrs = Attrs::new();
        attrs.id = Some("test".into());
        attrs.pairs.insert("src".into(), "data.csv".into());
        assert_eq!(attrs.get("id"), Some("test"));
        assert_eq!(attrs.get("src"), Some("data.csv"));
        assert_eq!(attrs.get("missing"), None);
    }

    #[test]
    fn document_serializes_to_json() {
        let mut doc = Document::new();
        doc.metadata.insert("title".into(), "Test".into());
        doc.blocks.push(Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text { text: "Hello".into() }],
            },
            span: Span::new(0, 5),
        });
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("\"title\":\"Test\""));
        assert!(json.contains("\"type\":\"Paragraph\""));
    }
}
