use aif_parser::parse;
use aif_html::render_html;
use aif_markdown::{render_markdown, import_markdown};
use aif_lml::render_lml;

/// Helper to resolve paths relative to workspace root.
macro_rules! workspace_path {
    ($path:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $path)
    };
}

#[test]
fn roundtrip_wiki_article_to_all_formats() {
    let input = std::fs::read_to_string(workspace_path!("examples/documents/wiki_article.aif")).unwrap();
    let doc = parse(&input).unwrap();

    let html = render_html(&doc);
    assert!(html.contains("<title>Photosynthesis</title>"));
    assert!(html.contains("aif-claim"));
    assert!(html.contains("aif-evidence"));
    assert!(html.contains("aif-definition"));

    let md = render_markdown(&doc);
    assert!(md.contains("# Photosynthesis"));
    assert!(md.contains("**Claim:**"));

    let lml = render_lml(&doc);
    assert!(lml.contains("[DOC"));
    assert!(lml.contains("[CLAIM id=c1]"));
    assert!(lml.contains("[/DOC]"));

    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Photosynthesis"));
    assert!(json.contains("Claim"));
}

#[test]
fn roundtrip_simple_example_to_all_formats() {
    let input = std::fs::read_to_string(workspace_path!("examples/documents/simple.aif")).unwrap();
    let doc = parse(&input).unwrap();

    let html = render_html(&doc);
    assert!(html.contains("Getting Started with AIF"));
    assert!(html.contains("aif-claim"));

    let md = render_markdown(&doc);
    assert!(md.contains("# Getting Started with AIF"));

    let lml = render_lml(&doc);
    assert!(lml.contains("[DOC"));
}

#[test]
fn roundtrip_markdown_import_then_export() {
    let md_input = std::fs::read_to_string(workspace_path!("examples/documents/wiki_source.md")).unwrap();
    let doc = import_markdown(&md_input);

    assert_eq!(doc.metadata.get("title").unwrap(), "Photosynthesis");

    // Re-export to markdown — should be structurally similar
    let md_output = render_markdown(&doc);
    assert!(md_output.contains("# Photosynthesis"));
    assert!(md_output.contains("## Overview"));
    assert!(md_output.contains("## Chemical Equation"));

    // Export imported doc to HTML
    let html = render_html(&doc);
    assert!(html.contains("<title>Photosynthesis</title>"));

    // Export imported doc to LML
    let lml = render_lml(&doc);
    assert!(lml.contains("[DOC"));
}

#[test]
fn roundtrip_wiki_fixture_import() {
    let md = std::fs::read_to_string(workspace_path!("tests/fixtures/import/wiki_article.md")).unwrap();
    let doc = import_markdown(&md);

    assert_eq!(doc.metadata.get("title").unwrap(), "Rust Programming Language");

    let html = render_html(&doc);
    assert!(html.contains("<title>Rust Programming Language</title>"));
    assert!(html.contains("class=\"language-rust\""));

    let md_out = render_markdown(&doc);
    assert!(md_out.contains("# Rust Programming Language"));
    assert!(md_out.contains("```rust"));
}

#[test]
fn roundtrip_all_blocks_fixture() {
    let input = std::fs::read_to_string(workspace_path!("tests/fixtures/blocks/all_blocks.aif")).unwrap();
    let doc = parse(&input).unwrap();

    let html = render_html(&doc);
    // All semantic types present
    for t in &["claim", "evidence", "definition", "theorem", "assumption",
               "result", "conclusion", "requirement", "recommendation"] {
        assert!(html.contains(&format!("aif-{}", t)),
            "Missing aif-{} in HTML output", t);
    }
    // All callout types present
    for t in &["note", "warning", "info", "tip"] {
        assert!(html.contains(&format!("aif-{}", t)),
            "Missing aif-{} callout in HTML output", t);
    }

    let md = render_markdown(&doc);
    assert!(md.contains("**Claim:**"));
    assert!(md.contains("**Evidence:**"));
    assert!(md.contains("> **Note:**"));
    assert!(md.contains("> **Warning:**"));
}

#[test]
fn json_roundtrip_preserves_structure() {
    let input = std::fs::read_to_string(workspace_path!("examples/documents/wiki_article.aif")).unwrap();
    let doc = parse(&input).unwrap();

    // Serialize to JSON
    let json = serde_json::to_string(&doc).unwrap();

    // Deserialize back
    let doc2: aif_core::ast::Document = serde_json::from_str(&json).unwrap();

    assert_eq!(doc.metadata, doc2.metadata);
    assert_eq!(doc.blocks.len(), doc2.blocks.len());

    // Re-render should produce identical output
    let html1 = render_html(&doc);
    let html2 = render_html(&doc2);
    assert_eq!(html1, html2);
}
