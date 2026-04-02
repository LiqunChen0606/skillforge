use aif_core::ast::*;
use aif_core::span::Span;
use aif_skill::lint::{lint_skill, LintCheck, LintSeverity};

fn make_attrs(pairs: Vec<(&str, &str)>) -> Attrs {
    let mut attrs = Attrs::new();
    for (k, v) in pairs {
        attrs.pairs.insert(k.into(), v.into());
    }
    attrs
}

fn make_skill_block(
    name: Option<&str>,
    description: Option<&str>,
    children: Vec<Block>,
) -> Block {
    let mut pairs = vec![];
    if let Some(n) = name {
        pairs.push(("name", n));
    }
    if let Some(d) = description {
        pairs.push(("description", d));
    }
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(pairs),
            title: None,
            content: vec![],
            children,
        },
        span: Span::empty(),
    }
}

fn make_child(skill_type: SkillBlockType, content: &str) -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text {
                text: content.into(),
            }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

fn make_step(order: u32, content: &str) -> Block {
    let mut attrs = Attrs::new();
    attrs.pairs.insert("order".into(), order.to_string());
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs,
            title: None,
            content: vec![Inline::Text {
                text: content.into(),
            }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

#[test]
fn valid_skill_passes_all_checks() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when debugging failures"),
        vec![
            make_step(1, "Do the thing"),
            make_child(SkillBlockType::Verify, "Check it worked"),
        ],
    );
    let results = lint_skill(&skill);
    assert_eq!(results.len(), 7, "Expected exactly 7 lint results");
    let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(
        failures.is_empty(),
        "Expected no failures, got: {:?}",
        failures
    );
}

#[test]
fn missing_description_fails_frontmatter() {
    let skill = make_skill_block(Some("my-skill"), None, vec![]);
    let results = lint_skill(&skill);
    let frontmatter = results
        .iter()
        .find(|r| r.check == LintCheck::Frontmatter)
        .unwrap();
    assert!(!frontmatter.passed);
    assert!(frontmatter.message.contains("description"));
}

#[test]
fn description_not_starting_with_use_when_fails() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("This skill helps with debugging"),
        vec![
            make_step(1, "Do thing"),
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let frontmatter = results
        .iter()
        .find(|r| r.check == LintCheck::Frontmatter)
        .unwrap();
    assert!(!frontmatter.passed);
    assert!(frontmatter.message.contains("Use when"));
}

#[test]
fn missing_step_fails_required_sections() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when testing"),
        vec![make_child(SkillBlockType::Verify, "Check it")],
    );
    let results = lint_skill(&skill);
    let sections = results
        .iter()
        .find(|r| r.check == LintCheck::RequiredSections)
        .unwrap();
    assert!(!sections.passed);
    assert!(sections.message.contains("@step"));
}

#[test]
fn missing_verify_fails_required_sections() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when testing"),
        vec![make_step(1, "Do thing")],
    );
    let results = lint_skill(&skill);
    let sections = results
        .iter()
        .find(|r| r.check == LintCheck::RequiredSections)
        .unwrap();
    assert!(!sections.passed);
    assert!(sections.message.contains("@verify"));
}

#[test]
fn description_over_1024_chars_fails() {
    let long_desc = format!("Use when {}", "x".repeat(1020));
    let skill = make_skill_block(
        Some("my-skill"),
        Some(&long_desc),
        vec![
            make_step(1, "Do thing"),
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let len_check = results
        .iter()
        .find(|r| r.check == LintCheck::DescriptionLength)
        .unwrap();
    assert!(!len_check.passed);
}

#[test]
fn name_with_spaces_fails_name_format() {
    let skill = make_skill_block(
        Some("my skill"),
        Some("Use when testing"),
        vec![
            make_step(1, "Do thing"),
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let name_check = results
        .iter()
        .find(|r| r.check == LintCheck::NameFormat)
        .unwrap();
    assert!(!name_check.passed);
}

#[test]
fn empty_step_block_fails() {
    let empty_step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: Attrs::new(),
            title: None,
            content: vec![],
            children: vec![],
        },
        span: Span::empty(),
    };
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when testing"),
        vec![
            empty_step,
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let empty_check = results
        .iter()
        .find(|r| r.check == LintCheck::NoEmptyBlocks)
        .unwrap();
    assert!(!empty_check.passed);
}

#[test]
fn version_hash_consistency() {
    let pairs = vec![
        ("name", "my-skill"),
        ("description", "Use when testing"),
        ("version", "1.0.0"),
    ];
    let skill = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(pairs),
            title: None,
            content: vec![],
            children: vec![
                make_step(1, "Do thing"),
                make_child(SkillBlockType::Verify, "Check"),
            ],
        },
        span: Span::empty(),
    };
    let results = lint_skill(&skill);
    let vh = results
        .iter()
        .find(|r| r.check == LintCheck::VersionHash)
        .unwrap();
    assert!(vh.passed);
    assert_eq!(vh.severity, LintSeverity::Warning);
}
