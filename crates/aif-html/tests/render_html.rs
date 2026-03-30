use aif_html::render_html;
use aif_parser::parse;
use insta::assert_snapshot;

#[test]
fn render_simple_document() {
    let input = "#title: Test Doc\n#summary: A test\n\nHello **world**.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert_snapshot!(html);
}

#[test]
fn render_section_with_claim() {
    let input = "@section[id=intro]: Introduction\nSome text here.\n\n@claim[id=c1]\nThis is a claim.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert_snapshot!(html);
}

#[test]
fn render_code_block() {
    let input = "```rust\nfn main() {}\n```\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert_snapshot!(html);
}

#[test]
fn render_list() {
    let input = "- item one\n- item two\n- item three\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert_snapshot!(html);
}
