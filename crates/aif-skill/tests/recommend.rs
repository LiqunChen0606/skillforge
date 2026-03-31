use aif_core::ast::*;
use aif_core::span::Span;
use aif_skill::recommend::*;

fn span() -> Span {
    Span::new(0, 0)
}

fn paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: text.into() }],
        },
        span: span(),
    }
}

fn code_block(code: &str) -> Block {
    Block {
        kind: BlockKind::CodeBlock {
            lang: Some("rust".into()),
            attrs: Attrs::new(),
            code: code.into(),
        },
        span: span(),
    }
}

fn skill_block() -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("name".into(), "debugging".into());
                a
            },
            title: None,
            content: vec![],
            children: vec![
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Step,
                        attrs: Attrs::new(),
                        title: None,
                        content: vec![Inline::Text { text: "Step 1".into() }],
                        children: vec![],
                    },
                    span: span(),
                },
            ],
        },
        span: span(),
    }
}

fn semantic_block() -> Block {
    Block {
        kind: BlockKind::SemanticBlock {
            block_type: SemanticBlockType::Claim,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "A claim".into() }],
        },
        span: span(),
    }
}

#[test]
fn skill_heavy_doc_recommends_lml_aggressive() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![paragraph("intro"), skill_block()],
    };
    let rec = recommend_format(&doc);
    assert_eq!(rec.format, "lml-aggressive");
    assert!(rec.reason.contains("skill"));
}

#[test]
fn code_heavy_doc_recommends_markdown() {
    // 3 code blocks out of 5 total = 60% > 40%
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            paragraph("intro"),
            code_block("fn main() {}"),
            code_block("let x = 1;"),
            code_block("println!(\"hi\");"),
            paragraph("end"),
        ],
    };
    let rec = recommend_format(&doc);
    assert_eq!(rec.format, "markdown");
    assert!(rec.reason.contains("code"));
}

#[test]
fn semantic_doc_recommends_lml_conservative() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![paragraph("intro"), semantic_block(), paragraph("conclusion")],
    };
    let rec = recommend_format(&doc);
    assert_eq!(rec.format, "lml-conservative");
    assert!(rec.reason.contains("semantic"));
}

#[test]
fn plain_doc_recommends_markdown() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![paragraph("Hello world"), paragraph("Another paragraph")],
    };
    let rec = recommend_format(&doc);
    assert_eq!(rec.format, "markdown");
    assert!(rec.reason.contains("General"));
}

#[test]
fn wire_purpose_recommends_binary() {
    let doc = Document::new();
    let rec = recommend_format_for_purpose(&doc, "wire");
    assert_eq!(rec.format, "binary-wire");
    assert!(rec.reason.contains("wire") || rec.reason.contains("Wire"));
}

#[test]
fn storage_purpose_recommends_json() {
    let doc = Document::new();
    let rec = recommend_format_for_purpose(&doc, "storage");
    assert_eq!(rec.format, "json");
    assert!(rec.reason.contains("Storage") || rec.reason.contains("storage"));
}
