use aif_core::ast::*;
use aif_core::span::Span;
use std::collections::BTreeMap;

fn sample_doc() -> Document {
    Document {
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
    }
}

#[test]
fn wire_roundtrip_paragraph() {
    let doc = sample_doc();
    let bytes = aif_binary::wire::encode(&doc);
    let decoded = aif_binary::wire::decode(&bytes).unwrap();
    assert_eq!(doc, decoded);
}

#[test]
fn wire_is_smaller_than_json() {
    let doc = sample_doc();
    let wire_bytes = aif_binary::wire::encode(&doc);
    let json_bytes = serde_json::to_string(&doc).unwrap();
    assert!(
        wire_bytes.len() < json_bytes.len(),
        "wire ({}) should be smaller than JSON ({})",
        wire_bytes.len(),
        json_bytes.len()
    );
}
