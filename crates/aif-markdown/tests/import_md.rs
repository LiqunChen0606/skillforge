use aif_markdown::import_markdown;
use insta::assert_yaml_snapshot;

#[test]
fn import_simple_markdown() {
    let md = "# Hello World\n\nThis is a paragraph.\n\n## Section Two\n\nMore text here.\n";
    let doc = import_markdown(md);
    assert_yaml_snapshot!(doc);
}

#[test]
fn import_markdown_with_list() {
    let md = "# Lists\n\n- item one\n- item two\n- item three\n";
    let doc = import_markdown(md);
    assert_yaml_snapshot!(doc);
}

#[test]
fn import_markdown_with_code() {
    let md = "# Code\n\n```rust\nfn main() {}\n```\n";
    let doc = import_markdown(md);
    assert_yaml_snapshot!(doc);
}
