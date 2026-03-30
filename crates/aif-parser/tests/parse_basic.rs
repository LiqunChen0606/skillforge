use aif_parser::parse;
use insta::assert_yaml_snapshot;

#[test]
fn parse_metadata_and_paragraph() {
    let input = "#title: Hello World\n#summary: A test document\n\nThis is a paragraph.\n";
    let doc = parse(input).unwrap();
    assert_yaml_snapshot!(doc);
}

#[test]
fn parse_section_with_content() {
    let input = "@section[id=intro]: Introduction\nThis is the intro paragraph.\n";
    let doc = parse(input).unwrap();
    assert_yaml_snapshot!(doc);
}
