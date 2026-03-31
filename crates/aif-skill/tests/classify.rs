use aif_core::ast::*;
use aif_skill::diff::{Change, ChangeKind};
use aif_skill::classify::{classify_change, ChangeClass};

#[test]
fn removed_step_is_breaking() {
    let change = Change {
        kind: ChangeKind::Removed,
        block_type: SkillBlockType::Step,
        description: "Removed Step/1".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Breaking);
}

#[test]
fn removed_precondition_is_breaking() {
    let change = Change {
        kind: ChangeKind::Removed,
        block_type: SkillBlockType::Precondition,
        description: "Removed".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Breaking);
}

#[test]
fn added_step_is_additive() {
    let change = Change {
        kind: ChangeKind::Added,
        block_type: SkillBlockType::Step,
        description: "Added Step/2".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Additive);
}

#[test]
fn added_example_is_additive() {
    let change = Change {
        kind: ChangeKind::Added,
        block_type: SkillBlockType::Example,
        description: "Added".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Additive);
}

#[test]
fn modified_step_is_cosmetic() {
    let change = Change {
        kind: ChangeKind::Modified,
        block_type: SkillBlockType::Step,
        description: "Modified Step/1".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Cosmetic);
}

#[test]
fn modified_precondition_is_breaking() {
    let change = Change {
        kind: ChangeKind::Modified,
        block_type: SkillBlockType::Precondition,
        description: "Modified".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Breaking);
}
