use aif_lml::render_lml_conservative;
use aif_parser::parse;
use insta::assert_snapshot;

#[test]
fn conservative_simple_skill() {
    let input = r#"
@skill[name="test-skill", version="1.0"]
  @precondition
    When debugging fails.
  @end

  @step[order=1]
    Read the error message carefully.
  @end

  @step[order=2]
    Reproduce the issue.
  @end

  @verify
    Confirm root cause found.
  @end
@end
"#;
    let doc = parse(input).unwrap();
    let out = render_lml_conservative(&doc);
    assert!(out.contains("[SK"), "should use abbreviated skill tag");
    assert!(out.contains("[ST"), "should use abbreviated step tag");
    assert!(out.contains("[PRE]"), "should use abbreviated precondition tag");
    assert!(out.contains("[VER]"), "should use abbreviated verify tag");
    assert_snapshot!(out);
}
