use aif_lml::render_lml;
use aif_parser::parse;
use insta::assert_snapshot;

#[test]
fn lml_simple_doc() {
    let input = "#title: Test Doc\n#summary: A test\n\n@section[id=intro]: Introduction\nSome text here.\n\n@claim[id=c1]\nThis is a claim.\n";
    let doc = parse(input).unwrap();
    let lml = render_lml(&doc);
    assert_snapshot!(lml);
}
