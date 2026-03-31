use aif_core::ast::*;
use aif_core::span::Span;
use aif_lml::render_lml_hybrid;

#[test]
fn hybrid_long_content_gets_base64() {
    let long_text =
        "This is a long piece of text that exceeds the fifty character threshold for encoding";
    let mut doc = Document::new();
    doc.blocks.push(Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text {
                text: long_text.into(),
            }],
            children: vec![],
        },
        span: Span::new(0, 30),
    });
    let output = render_lml_hybrid(&doc);
    assert!(output.contains("@step"));
    assert!(output.contains("~b64:"));
    assert!(!output.contains(long_text)); // original text should be encoded
}

#[test]
fn hybrid_short_content_stays_plain() {
    let mut doc = Document::new();
    doc.blocks.push(Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text {
                text: "Hi".into(),
            }],
        },
        span: Span::new(0, 2),
    });
    let output = render_lml_hybrid(&doc);
    assert!(output.contains("Hi"));
    assert!(!output.contains("~b64:"));
}

#[test]
fn hybrid_metadata_preserved() {
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Test".into());
    doc.blocks.push(Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text {
                text: "Hello".into(),
            }],
        },
        span: Span::new(0, 5),
    });
    let output = render_lml_hybrid(&doc);
    assert!(output.contains("#title: Test"));
}

#[test]
fn hybrid_code_block_not_encoded() {
    let mut doc = Document::new();
    doc.blocks.push(Block {
        kind: BlockKind::CodeBlock {
            lang: Some("rust".into()),
            attrs: Attrs::new(),
            code: "fn main() { println!(\"this is a long line of code that exceeds the threshold\"); }".into(),
        },
        span: Span::new(0, 50),
    });
    let output = render_lml_hybrid(&doc);
    assert!(output.contains("```rust"));
    assert!(!output.contains("~b64:")); // code blocks should not be base64-encoded
}
