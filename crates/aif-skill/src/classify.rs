use crate::diff::{Change, ChangeKind};
use crate::version::BumpLevel;
use aif_core::ast::SkillBlockType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeClass {
    Breaking,
    Additive,
    Cosmetic,
}

impl ChangeClass {
    pub fn bump_level(self) -> BumpLevel {
        match self {
            ChangeClass::Breaking => BumpLevel::Major,
            ChangeClass::Additive => BumpLevel::Minor,
            ChangeClass::Cosmetic => BumpLevel::Patch,
        }
    }
}

/// Classify a single change based on its kind and block type.
pub fn classify_change(change: &Change) -> ChangeClass {
    match change.kind {
        ChangeKind::Removed => {
            // Removing any structural block is breaking
            ChangeClass::Breaking
        }
        ChangeKind::Added => {
            // Adding is always additive
            ChangeClass::Additive
        }
        ChangeKind::Modified => {
            // Modifying critical blocks (precondition, verify, output_contract) is breaking
            // Modifying other blocks is cosmetic (text rewording)
            match change.block_type {
                SkillBlockType::Precondition
                | SkillBlockType::Verify
                | SkillBlockType::OutputContract => ChangeClass::Breaking,
                _ => ChangeClass::Cosmetic,
            }
        }
    }
}

/// Given a list of changes, return the highest-severity bump level needed.
pub fn highest_bump(changes: &[Change]) -> BumpLevel {
    changes
        .iter()
        .map(|c| classify_change(c).bump_level())
        .max_by_key(|b| match b {
            BumpLevel::Major => 2,
            BumpLevel::Minor => 1,
            BumpLevel::Patch => 0,
        })
        .unwrap_or(BumpLevel::Patch)
}
