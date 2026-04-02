use crate::span::Span;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level document
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct Document {
    pub metadata: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }
}

/// A block-level element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct Block {
    pub kind: BlockKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
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
        meta: MediaMeta,
    },
    Audio {
        attrs: Attrs,
        caption: Option<Vec<Inline>>,
        src: String,
        meta: MediaMeta,
    },
    Video {
        attrs: Attrs,
        caption: Option<Vec<Inline>>,
        src: String,
        meta: MediaMeta,
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
    SkillBlock {
        skill_type: SkillBlockType,
        attrs: Attrs,
        title: Option<Vec<Inline>>,
        content: Vec<Inline>,
        children: Vec<Block>,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ListItem {
    pub content: Vec<Inline>,
    pub children: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub enum CalloutType {
    Note,
    Warning,
    Info,
    Tip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub enum SkillBlockType {
    Skill,
    Step,
    Verify,
    Precondition,
    OutputContract,
    Decision,
    Tool,
    Fallback,
    RedFlag,
    Example,
    Scenario,
}

/// Shared metadata for media blocks (Figure, Audio, Video).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(default)]
pub struct MediaMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,  // seconds, for audio/video
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>, // poster image URL for video
}

/// Attributes on a block
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(tag = "type")]
pub enum Inline {
    Text { text: String },
    Emphasis { content: Vec<Inline> },
    Strong { content: Vec<Inline> },
    InlineCode { code: String },
    Link { text: Vec<Inline>, url: String },
    Image { alt: String, src: String },
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

    #[test]
    fn skill_block_serializes_to_json() {
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text { text: "Debug process".into() }],
                children: vec![],
            },
            span: Span::new(0, 20),
        };
        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("\"type\":\"SkillBlock\""));
        assert!(json.contains("\"Skill\""));
    }

    #[test]
    fn skill_step_with_order_attr() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("order".into(), "1".into());
        let step = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text { text: "Reproduce the bug".into() }],
                children: vec![],
            },
            span: Span::new(0, 30),
        };
        if let BlockKind::SkillBlock { attrs, .. } = &step.kind {
            assert_eq!(attrs.get("order"), Some("1"));
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn skill_block_with_children() {
        let step = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text { text: "Step 1".into() }],
                children: vec![],
            },
            span: Span::new(10, 30),
        };
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "debugging".into());
                    a
                },
                title: None,
                content: vec![],
                children: vec![step],
            },
            span: Span::new(0, 50),
        };
        if let BlockKind::SkillBlock { children, attrs, .. } = &skill.kind {
            assert_eq!(children.len(), 1);
            assert_eq!(attrs.get("name"), Some("debugging"));
        } else {
            panic!("expected SkillBlock");
        }
    }
}
