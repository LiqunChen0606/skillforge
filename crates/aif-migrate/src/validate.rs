use aif_core::ast::{Block, BlockKind, SkillBlockType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationLintCheck {
    HasMigrationProfile,
    HasPrecondition,
    HasSteps,
    HasVerify,
    HasOutputContract,
}

#[derive(Debug, Clone)]
pub struct MigrationLintResult {
    pub check: MigrationLintCheck,
    pub passed: bool,
    pub message: String,
}

/// Validate that a skill block conforms to the migration profile requirements.
pub fn validate_migration_skill(skill_block: &Block) -> Vec<MigrationLintResult> {
    let mut results = Vec::new();

    // Check migration profile attribute
    let has_profile = match &skill_block.kind {
        BlockKind::SkillBlock { attrs, .. } => {
            attrs.pairs.iter().any(|(k, v)| k == "profile" && v == "migration")
        }
        _ => false,
    };
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasMigrationProfile,
        passed: has_profile,
        message: if has_profile {
            "Skill has profile=migration attribute".to_string()
        } else {
            "Skill missing profile=migration attribute".to_string()
        },
    });

    // Extract children from SkillBlock
    let children = match &skill_block.kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => return results,
    };

    let has_precondition = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { skill_type: SkillBlockType::Precondition, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasPrecondition,
        passed: has_precondition,
        message: if has_precondition {
            "@precondition block present".to_string()
        } else {
            "Missing @precondition block — migration skills must specify when to apply".to_string()
        },
    });

    let has_steps = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { skill_type: SkillBlockType::Step, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasSteps,
        passed: has_steps,
        message: if has_steps {
            "At least one @step block present".to_string()
        } else {
            "Missing @step blocks — migration skills must have at least one step".to_string()
        },
    });

    let has_verify = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { skill_type: SkillBlockType::Verify, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasVerify,
        passed: has_verify,
        message: if has_verify {
            "@verify block present".to_string()
        } else {
            "Missing @verify block — migration skills must define verification criteria".to_string()
        },
    });

    let has_output_contract = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { skill_type: SkillBlockType::OutputContract, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasOutputContract,
        passed: has_output_contract,
        message: if has_output_contract {
            "@output_contract block present".to_string()
        } else {
            "Missing @output_contract block — migration skills must define success criteria".to_string()
        },
    });

    results
}
