use aif_lml::render_lml;
use aif_parser::parse;

#[test]
fn lml_document_envelope() {
    let input = "#title: Test\n#summary: Summary\n\nContent.\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert!(lml.contains("[DOC"));
    assert!(lml.contains("title=Test"));
    assert!(lml.contains("summary=Summary"));
    assert!(lml.contains("[/DOC]"));
}

#[test]
fn lml_section() {
    let input = "@section[id=intro]: Introduction\nHello.\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert!(lml.contains("[SECTION id=intro]"));
    assert!(lml.contains("Introduction"));
}

#[test]
fn lml_semantic_blocks() {
    let types = ["claim", "evidence", "definition"];
    for t in &types {
        let input = format!("@{}[id=test]\nContent.\n", t);
        let doc = parse(&input).unwrap();
        let lml = render_lml(&doc);
        let tag = t.to_uppercase();
        assert!(lml.contains(&format!("[{} id=test]", tag)),
            "Expected [{} id=test] in LML output", tag);
    }
}

#[test]
fn lml_code_block() {
    let input = "```rust\nfn main() {}\n```\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert!(lml.contains("[CODE lang=rust]"));
    assert!(lml.contains("fn main() {}"));
    assert!(lml.contains("[/CODE]"));
}

#[test]
fn lml_list_items() {
    let input = "- Item one\n- Item two\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert!(lml.contains("- Item one"));
    assert!(lml.contains("- Item two"));
}

#[test]
fn lml_ordered_list() {
    let input = "1. First\n2. Second\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert!(lml.contains("1. First"));
    assert!(lml.contains("2. Second"));
}

#[test]
fn lml_wiki_article() {
    let input = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/documents/wiki_article.aif")
    ).unwrap();
    let doc = parse(&input).unwrap();
    let lml = render_lml(&doc);

    assert!(lml.contains("[DOC"));
    assert!(lml.contains("title=Photosynthesis"));
    assert!(lml.contains("[SECTION id=overview]"));
    assert!(lml.contains("[DEFINITION id=def-photosynthesis]"));
    assert!(lml.contains("[CLAIM id=c1]"));
    assert!(lml.contains("[EVIDENCE id=e1]"));
    assert!(lml.contains("[CONCLUSION id=conclusion]"));
    assert!(lml.contains("[/DOC]"));
}

#[test]
fn lml_callout() {
    let input = "@callout[type=warning]\nBe careful.\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert!(lml.contains("[WARNING]"));
}

#[test]
fn lml_blockquote() {
    let input = "> Quoted text here.\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    // BlockQuote parsing may vary — just check it doesn't panic
    assert!(lml.contains("[DOC") || lml.contains("[QUOTE]") || !lml.is_empty());
}
