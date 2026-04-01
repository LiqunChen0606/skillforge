use aif_core::ast::*;
use aif_core::span::Span;

fn make_paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text {
                text: text.to_string(),
            }],
        },
        span: Span::new(0, 0),
    }
}

fn make_section(title: &str, children: Vec<Block>) -> Block {
    Block {
        kind: BlockKind::Section {
            attrs: Attrs::new(),
            title: vec![Inline::Text {
                text: title.to_string(),
            }],
            children,
        },
        span: Span::new(0, 0),
    }
}

#[test]
fn export_simple_document() {
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert("title".to_string(), "Test Document".to_string());

    let doc = Document {
        metadata,
        blocks: vec![
            make_section(
                "Introduction",
                vec![make_paragraph("This is the introduction.")],
            ),
            make_section("Body", vec![make_paragraph("This is the body text.")]),
        ],
    };

    let pdf_bytes = aif_pdf::export::export_pdf(&doc).expect("export should succeed");
    // PDF files start with %PDF
    assert!(pdf_bytes.starts_with(b"%PDF"));
    assert!(pdf_bytes.len() > 100);
}

#[test]
fn export_empty_document() {
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![],
    };

    let result = aif_pdf::export::export_pdf(&doc);
    // Empty doc should still produce a valid PDF (krilla adds an empty page)
    assert!(result.is_ok());
}

#[test]
fn export_with_code_block() {
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::CodeBlock {
                lang: Some("rust".to_string()),
                attrs: Attrs::new(),
                code: "fn main() {\n    println!(\"hello\");\n}".to_string(),
            },
            span: Span::new(0, 0),
        }],
    };

    let pdf_bytes = aif_pdf::export::export_pdf(&doc).expect("export should succeed");
    assert!(pdf_bytes.starts_with(b"%PDF"));
}

#[test]
fn export_with_list() {
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::List {
                ordered: true,
                items: vec![
                    ListItem {
                        content: vec![Inline::Text {
                            text: "First item".to_string(),
                        }],
                        children: vec![],
                    },
                    ListItem {
                        content: vec![Inline::Text {
                            text: "Second item".to_string(),
                        }],
                        children: vec![],
                    },
                ],
            },
            span: Span::new(0, 0),
        }],
    };

    let pdf_bytes = aif_pdf::export::export_pdf(&doc).expect("export should succeed");
    assert!(pdf_bytes.starts_with(b"%PDF"));
}
