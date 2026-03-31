use aif_core::ast::*;
use aif_core::span::Span;
use std::collections::BTreeMap;

#[test]
fn wire_roundtrip_skill_block() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".to_string(), "test-skill".to_string());
                    a.pairs.insert("version".to_string(), "1.0.0".to_string());
                    a
                },
                title: Some(vec![Inline::Text {
                    text: "Test Skill".to_string(),
                }]),
                content: vec![Inline::Text {
                    text: "Description".to_string(),
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
                                text: "Do something".to_string(),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Verify,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text {
                                text: "Check it".to_string(),
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

    let bytes = aif_binary::wire::encode(&doc);
    let decoded = aif_binary::wire::decode(&bytes).unwrap();
    assert_eq!(doc, decoded);

    // Verify compactness
    let json = serde_json::to_string(&doc).unwrap();
    let ratio = bytes.len() as f64 / json.len() as f64;
    assert!(ratio < 0.6, "wire should be <60% of JSON size, got {:.1}%", ratio * 100.0);
}
