use aif_core::ast::*;
use aif_core::span::Span;
use aif_lml::render_lml_compressed;

#[test]
fn repeated_text_gets_deduplicated() {
    let repeated = "This is a long repeated phrase that appears multiple times in the document";
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: repeated.into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: repeated.into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: repeated.into() }],
                },
                span: Span::new(0, 0),
            },
        ],
    };
    let output = render_lml_compressed(&doc);
    assert!(output.contains("~dict:"));
    let count = output.matches(repeated).count();
    assert_eq!(count, 1, "repeated text should appear exactly once (in dict)");
    assert!(output.contains("~ref:"));
}

#[test]
fn short_text_not_deduplicated() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "short".into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "short".into() }],
                },
                span: Span::new(0, 0),
            },
        ],
    };
    let output = render_lml_compressed(&doc);
    assert!(!output.contains("~dict:"));
}

#[test]
fn unique_long_text_not_deduplicated() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "unique text one that is definitely long enough to pass".into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "unique text two that is also long enough to pass".into() }],
                },
                span: Span::new(0, 0),
            },
        ],
    };
    let output = render_lml_compressed(&doc);
    assert!(!output.contains("~dict:"));
}
