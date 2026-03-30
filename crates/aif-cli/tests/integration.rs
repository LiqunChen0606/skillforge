use aif_parser::parse;
use aif_html::render_html;
use aif_markdown::{render_markdown, import_markdown};
use aif_lml::render_lml;

#[test]
fn roundtrip_parse_and_compile_html() {
    let input = "#title: Roundtrip Test\n\n@section[id=s1]: First\nHello **world**.\n\n@claim[id=c1]\nThis is important.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<title>Roundtrip Test</title>"));
    assert!(html.contains("<strong>world</strong>"));
    assert!(html.contains("aif-claim"));
}

#[test]
fn roundtrip_parse_and_compile_markdown() {
    let input = "#title: MD Test\n\nHello **world**.\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("# MD Test"));
    assert!(md.contains("**world**"));
}

#[test]
fn roundtrip_parse_and_compile_lml() {
    let input = "#title: LML Test\n#summary: Test summary\n\n@claim[id=c1]\nImportant claim.\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert!(lml.contains("[DOC"));
    assert!(lml.contains("[CLAIM id=c1]"));
    assert!(lml.contains("[/DOC]"));
}

#[test]
fn markdown_import_produces_valid_ir() {
    let md = "# Test\n\nHello world.\n\n- item 1\n- item 2\n";
    let doc = import_markdown(md);
    assert_eq!(doc.metadata.get("title"), Some(&"Test".to_string()));
    assert!(!doc.blocks.is_empty());
}

#[test]
fn all_semantic_block_types_parse() {
    for block_type in &["claim", "evidence", "definition", "theorem", "assumption", "result", "conclusion", "requirement", "recommendation"] {
        let input = format!("@{}[id=test]\nTest content.\n", block_type);
        let doc = parse(&input).unwrap();
        assert_eq!(doc.blocks.len(), 1, "Failed to parse @{}", block_type);
    }
}

#[test]
fn callout_types_parse() {
    for callout_type in &["note", "warning", "info", "tip"] {
        let input = format!("@callout[type={}]\nTest content.\n", callout_type);
        let doc = parse(&input).unwrap();
        assert_eq!(doc.blocks.len(), 1, "Failed to parse @callout[type={}]", callout_type);
    }
}
