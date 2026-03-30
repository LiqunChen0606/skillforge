use aif_markdown::render_markdown;
use aif_parser::parse;
use insta::assert_snapshot;

#[test]
fn render_md_simple() {
    let input = "#title: Test Doc\n\nHello **world**.\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert_snapshot!(md);
}

#[test]
fn render_md_section_and_list() {
    let input = "@section[id=intro]: Introduction\nSome text.\n\n- item 1\n- item 2\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert_snapshot!(md);
}
