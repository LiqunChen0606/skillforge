use aif_parser::parse;
use aif_core::ast::BlockKind;

#[test]
fn parse_all_semantic_block_types() {
    let types = [
        "claim", "evidence", "definition", "theorem",
        "assumption", "result", "conclusion", "requirement", "recommendation",
    ];
    for block_type in &types {
        let input = format!("@{}[id=test]\nContent for {}.\n", block_type, block_type);
        let doc = parse(&input).unwrap();
        assert_eq!(doc.blocks.len(), 1, "Expected 1 block for @{}", block_type);
        match &doc.blocks[0].kind {
            BlockKind::SemanticBlock { .. } => {}
            other => panic!("Expected SemanticBlock for @{}, got {:?}", block_type, other),
        }
    }
}

#[test]
fn parse_all_callout_types() {
    for callout_type in &["note", "warning", "info", "tip"] {
        let input = format!("@callout[type={}]\nCallout content.\n", callout_type);
        let doc = parse(&input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::Callout { .. } => {}
            other => panic!("Expected Callout for type={}, got {:?}", callout_type, other),
        }
    }
}

#[test]
fn parse_section_with_paragraph_child() {
    let input = "\
@section[id=main]: Main Section
A paragraph in the section.
";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::Section { children, .. } => {
            assert_eq!(children.len(), 1, "Expected 1 paragraph child in section");
        }
        _ => panic!("Expected Section"),
    }
}

#[test]
fn parse_section_followed_by_claim() {
    // Directives at the same level are siblings, not nested
    let input = "\
@section[id=main]: Main Section
A paragraph in the section.

@claim[id=c1]
A claim after the section.
";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 2, "Section and claim should be siblings");
    assert!(matches!(&doc.blocks[0].kind, BlockKind::Section { .. }));
    assert!(matches!(&doc.blocks[1].kind, BlockKind::SemanticBlock { .. }));
}

#[test]
fn parse_code_block_with_language() {
    let input = "```python\ndef hello():\n    print(\"hi\")\n```\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), Some("python"));
            assert!(code.contains("def hello()"));
        }
        _ => panic!("Expected CodeBlock"),
    }
}

#[test]
fn parse_code_block_without_language() {
    let input = "```\nplain code\n```\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert!(lang.is_none());
            assert!(code.contains("plain code"));
        }
        _ => panic!("Expected CodeBlock"),
    }
}

#[test]
fn parse_unordered_list() {
    let input = "- Alpha\n- Beta\n- Gamma\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn parse_ordered_list() {
    let input = "1. First\n2. Second\n3. Third\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(ordered);
            assert_eq!(items.len(), 3);
        }
        _ => panic!("Expected ordered List"),
    }
}

#[test]
fn parse_table() {
    let input = "@table[id=t1]: My Table\n| Header A | Header B |\n| Cell 1 | Cell 2 |\n| Cell 3 | Cell 4 |\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::Table { headers, rows, caption, .. } => {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 2);
            assert!(caption.is_some());
        }
        _ => panic!("Expected Table"),
    }
}

#[test]
fn parse_figure() {
    let input = "@figure[id=fig1, src=photo.jpg]: A nice photo\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::Figure { src, caption, attrs } => {
            assert_eq!(src, "photo.jpg");
            assert!(caption.is_some());
            assert_eq!(attrs.id.as_deref(), Some("fig1"));
        }
        _ => panic!("Expected Figure"),
    }
}

#[test]
fn parse_metadata_fields() {
    let input = "#title: My Title\n#summary: My Summary\n#author: Test Author\n\nBody text.\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.metadata.get("title").unwrap(), "My Title");
    assert_eq!(doc.metadata.get("summary").unwrap(), "My Summary");
    assert_eq!(doc.metadata.get("author").unwrap(), "Test Author");
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn parse_semantic_block_with_title() {
    let input = "@claim[id=c1]: Main Claim\nThe body of the claim.\n";
    let doc = parse(input).unwrap();
    match &doc.blocks[0].kind {
        BlockKind::SemanticBlock { title, content, .. } => {
            assert!(title.is_some(), "Expected title on semantic block");
            assert!(!content.is_empty(), "Expected content on semantic block");
        }
        _ => panic!("Expected SemanticBlock"),
    }
}

#[test]
fn parse_empty_document() {
    let input = "";
    let doc = parse(input).unwrap();
    assert!(doc.blocks.is_empty());
    assert!(doc.metadata.is_empty());
}

#[test]
fn parse_metadata_only() {
    let input = "#title: Just Metadata\n#summary: No content\n";
    let doc = parse(input).unwrap();
    assert_eq!(doc.metadata.get("title").unwrap(), "Just Metadata");
    assert!(doc.blocks.is_empty());
}

#[test]
fn parse_fixture_all_blocks() {
    let input = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/blocks/all_blocks.aif")
    ).unwrap();
    let doc = parse(&input).unwrap();
    assert_eq!(doc.metadata.get("title").unwrap(), "All Block Types");
    assert!(!doc.blocks.is_empty(), "Expected blocks from all_blocks.aif");
}

#[test]
fn parse_fixture_inline_formatting() {
    let input = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/inline/formatting.aif")
    ).unwrap();
    let doc = parse(&input).unwrap();
    assert_eq!(doc.metadata.get("title").unwrap(), "Inline Formatting Test");
    assert!(!doc.blocks.is_empty());
}
