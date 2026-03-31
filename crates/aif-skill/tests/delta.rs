use aif_core::ast::*;
use aif_core::span::Span;
use aif_skill::delta::{apply_delta, encode_delta};

fn make_skill(steps: Vec<(&str, &str)>) -> Block {
    let children: Vec<Block> = steps
        .iter()
        .map(|(order, text)| Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("order".into(), order.to_string());
                    a
                },
                title: None,
                content: vec![Inline::Text {
                    text: text.to_string(),
                }],
                children: vec![],
            },
            span: Span::new(0, 0),
        })
        .collect();

    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("name".into(), "test".into());
                a
            },
            title: None,
            content: vec![],
            children,
        },
        span: Span::new(0, 0),
    }
}

#[test]
fn delta_no_changes_is_minimal() {
    let skill = make_skill(vec![("1", "Step one")]);
    let delta = encode_delta(&skill, &skill);
    // Should have magic + version + small payload
    assert!(delta.len() < 30);
}

#[test]
fn delta_added_step() {
    let old = make_skill(vec![("1", "Step one")]);
    let new = make_skill(vec![("1", "Step one"), ("2", "Step two")]);
    let delta = encode_delta(&old, &new);
    let result = apply_delta(&old, &delta).unwrap();
    if let BlockKind::SkillBlock { children, .. } = &result.kind {
        assert_eq!(children.len(), 2);
    } else {
        panic!("expected SkillBlock");
    }
}

#[test]
fn delta_removed_step() {
    let old = make_skill(vec![("1", "Step one"), ("2", "Step two")]);
    let new = make_skill(vec![("1", "Step one")]);
    let delta = encode_delta(&old, &new);
    let result = apply_delta(&old, &delta).unwrap();
    if let BlockKind::SkillBlock { children, .. } = &result.kind {
        assert_eq!(children.len(), 1);
    } else {
        panic!("expected SkillBlock");
    }
}

#[test]
fn delta_invalid_magic() {
    let old = make_skill(vec![("1", "Step one")]);
    let result = apply_delta(&old, b"XX\x01\x00");
    assert!(result.is_err());
}
