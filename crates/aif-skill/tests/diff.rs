use aif_core::ast::*;
use aif_core::span::Span;
use aif_skill::diff::{diff_skills, ChangeKind};

fn make_skill(steps: Vec<(&str, &str)>, verify: Option<&str>) -> Block {
    let mut children = Vec::new();
    for (order, text) in steps.iter() {
        let mut attrs = Attrs::new();
        attrs
            .pairs
            .insert("order".to_string(), order.to_string());
        children.push(Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text {
                    text: text.to_string(),
                }],
                children: vec![],
            },
            span: Span::empty(),
        });
    }
    if let Some(v) = verify {
        children.push(Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Verify,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text {
                    text: v.to_string(),
                }],
                children: vec![],
            },
            span: Span::empty(),
        });
    }

    let mut attrs = Attrs::new();
    attrs
        .pairs
        .insert("name".to_string(), "test".to_string());
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            title: None,
            content: vec![],
            children,
        },
        span: Span::empty(),
    }
}

#[test]
fn no_changes() {
    let old = make_skill(vec![("1", "step one")], Some("check it"));
    let new = old.clone();
    let changes = diff_skills(&old, &new);
    assert!(changes.is_empty());
}

#[test]
fn added_step() {
    let old = make_skill(vec![("1", "step one")], None);
    let new = make_skill(vec![("1", "step one"), ("2", "step two")], None);
    let changes = diff_skills(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Added);
}

#[test]
fn removed_step() {
    let old = make_skill(vec![("1", "step one"), ("2", "step two")], None);
    let new = make_skill(vec![("1", "step one")], None);
    let changes = diff_skills(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Removed);
}

#[test]
fn modified_text() {
    let old = make_skill(vec![("1", "step one")], None);
    let new = make_skill(vec![("1", "step one updated")], None);
    let changes = diff_skills(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Modified);
}
