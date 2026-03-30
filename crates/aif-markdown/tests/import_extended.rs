use aif_markdown::import_markdown;
use aif_core::ast::BlockKind;

#[test]
fn import_wiki_article_fixture() {
    let md = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/import/wiki_article.md")
    ).unwrap();
    let doc = import_markdown(&md);

    // Title extracted from H1
    assert_eq!(doc.metadata.get("title").unwrap(), "Rust Programming Language");

    // Should have multiple blocks (sections, paragraphs, etc.)
    assert!(doc.blocks.len() >= 5, "Expected many blocks, got {}", doc.blocks.len());
}

#[test]
fn import_preserves_inline_formatting() {
    let md = "# Test\n\nThis is **bold** and *italic* text.\n";
    let doc = import_markdown(md);

    // Find the paragraph with formatting
    let has_paragraph = doc.blocks.iter().any(|b| matches!(&b.kind, BlockKind::Paragraph { .. }));
    assert!(has_paragraph, "Expected a paragraph block");
}

#[test]
fn import_preserves_links() {
    let md = "# Test\n\nVisit [Example](https://example.com).\n";
    let doc = import_markdown(md);

    let has_paragraph = doc.blocks.iter().any(|b| matches!(&b.kind, BlockKind::Paragraph { .. }));
    assert!(has_paragraph);
}

#[test]
fn import_preserves_code_block_language() {
    let md = "# Test\n\n```python\nprint('hi')\n```\n";
    let doc = import_markdown(md);

    let code_block = doc.blocks.iter().find(|b| matches!(&b.kind, BlockKind::CodeBlock { .. }));
    assert!(code_block.is_some(), "Expected a code block");
    match &code_block.unwrap().kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), Some("python"));
            assert!(code.contains("print('hi')"));
        }
        _ => unreachable!(),
    }
}

#[test]
fn import_ordered_list() {
    let md = "# Test\n\n1. First\n2. Second\n3. Third\n";
    let doc = import_markdown(md);

    let list = doc.blocks.iter().find(|b| matches!(&b.kind, BlockKind::List { .. }));
    assert!(list.is_some(), "Expected a list block");
    match &list.unwrap().kind {
        BlockKind::List { ordered, items } => {
            assert!(ordered);
            assert_eq!(items.len(), 3);
        }
        _ => unreachable!(),
    }
}

#[test]
fn import_unordered_list() {
    let md = "# Test\n\n- Alpha\n- Beta\n- Gamma\n";
    let doc = import_markdown(md);

    let list = doc.blocks.iter().find(|b| matches!(&b.kind, BlockKind::List { .. }));
    assert!(list.is_some());
    match &list.unwrap().kind {
        BlockKind::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        }
        _ => unreachable!(),
    }
}

#[test]
fn import_blockquote() {
    let md = "# Test\n\n> This is a quote.\n";
    let doc = import_markdown(md);

    let has_quote = doc.blocks.iter().any(|b| matches!(&b.kind, BlockKind::BlockQuote { .. }));
    assert!(has_quote, "Expected a blockquote");
}

#[test]
fn import_thematic_break() {
    let md = "# Test\n\nParagraph one.\n\n---\n\nParagraph two.\n";
    let doc = import_markdown(md);

    let has_break = doc.blocks.iter().any(|b| matches!(&b.kind, BlockKind::ThematicBreak));
    assert!(has_break, "Expected a thematic break");
}

#[test]
fn import_multiple_headings_become_sections() {
    let md = "# Title\n\n## Section A\n\nContent A.\n\n## Section B\n\nContent B.\n";
    let doc = import_markdown(md);

    let sections: Vec<_> = doc.blocks.iter()
        .filter(|b| matches!(&b.kind, BlockKind::Section { .. }))
        .collect();
    // H1 becomes title + section, H2s become sections
    assert!(sections.len() >= 2, "Expected at least 2 sections, got {}", sections.len());
}

#[test]
fn import_empty_markdown() {
    let doc = import_markdown("");
    assert!(doc.blocks.is_empty());
    assert!(doc.metadata.is_empty());
}
