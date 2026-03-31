use aif_core::ast::*;
use aif_core::span::Span;
use std::collections::BTreeMap;

#[test]
fn token_opt_produces_bytes() {
    let doc = Document {
        metadata: {
            let mut m = BTreeMap::new();
            m.insert("title".to_string(), "Test".to_string());
            m
        },
        blocks: vec![Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text {
                    text: "Hello world".to_string(),
                }],
            },
            span: Span::empty(),
        }],
    };

    let bytes = aif_binary::token_opt::encode(&doc);
    assert!(!bytes.is_empty());
    // Should be smaller than JSON
    let json = serde_json::to_string(&doc).unwrap();
    assert!(bytes.len() < json.len());
}

#[test]
fn token_opt_smaller_than_wire() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".to_string(), "test".to_string());
                    a
                },
                title: None,
                content: vec![Inline::Text {
                    text: "A skill".to_string(),
                }],
                children: vec![
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            attrs: {
                                let mut a = Attrs::new();
                                a.pairs.insert("order".to_string(), "1".to_string());
                                a
                            },
                            title: None,
                            content: vec![Inline::Text {
                                text: "Step one".to_string(),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            attrs: {
                                let mut a = Attrs::new();
                                a.pairs.insert("order".to_string(), "2".to_string());
                                a
                            },
                            title: None,
                            content: vec![Inline::Text {
                                text: "Step two".to_string(),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                ],
            },
            span: Span::empty(),
        }],
    };

    let wire = aif_binary::wire::encode(&doc);
    let token = aif_binary::token_opt::encode(&doc);
    assert!(
        token.len() <= wire.len(),
        "token-opt ({}) should be <= wire ({})",
        token.len(),
        wire.len()
    );
}
