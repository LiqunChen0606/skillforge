use aif_lml::{render_lml_aggressive, render_lml_conservative, render_lml_moderate};
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

#[test]
fn moderate_drops_closing_tags_for_leaves() {
    let input = r#"
@skill[name="test-skill", version="1.0"]
  @precondition
    When debugging fails.
  @end

  @step[order=1]
    Read the error message carefully.
  @end

  @verify
    Confirm root cause found.
  @end
@end
"#;
    let doc = parse(input).unwrap();
    let out = render_lml_moderate(&doc);
    // Moderate: no [DOC], no closing tags on leaf blocks
    assert!(!out.contains("[DOC"), "should not have DOC wrapper");
    assert!(!out.contains("[/ST]"), "leaf step should not have closing tag");
    assert!(out.contains("[SK"), "should have abbreviated skill tag");
    assert_snapshot!(out);
}

#[test]
fn aggressive_markdown_like() {
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
    let out = render_lml_aggressive(&doc);
    // Aggressive: no bracket tags, uses @prefix style
    assert!(!out.contains("[SK"), "should not use bracket tags");
    assert!(!out.contains("[ST"), "should not use bracket tags");
    assert!(out.contains("@step"), "should use @prefix style");
    assert_snapshot!(out);
}
