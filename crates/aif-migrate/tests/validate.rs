use aif_parser::parse;
use aif_migrate::validate::{validate_migration_skill, MigrationLintCheck};

fn parse_skill(source: &str) -> aif_core::ast::Document {
    parse(source).expect("parse failed")
}

#[test]
fn valid_migration_skill_passes() {
    let source = r#"
#title: Test Migration

@skill[name="test-migrate", version="1.0", profile=migration]
  @precondition
    Source uses framework X.
  @end

  @step[order=1]
    Replace X with Y.
  @end

  @verify
    No remaining X references.
  @end

  @output_contract
    All files use Y.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    assert!(results.iter().all(|r| r.passed), "All checks should pass: {:?}", results);
}

#[test]
fn missing_precondition_fails() {
    let source = r#"
#title: Bad Migration

@skill[name="bad-migrate", version="1.0", profile=migration]
  @step[order=1]
    Do something.
  @end

  @verify
    Check something.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let precondition_check = results.iter().find(|r| r.check == MigrationLintCheck::HasPrecondition).unwrap();
    assert!(!precondition_check.passed);
}

#[test]
fn missing_steps_fails() {
    let source = r#"
#title: No Steps

@skill[name="no-steps", version="1.0", profile=migration]
  @precondition
    Has framework.
  @end

  @verify
    Check it.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let step_check = results.iter().find(|r| r.check == MigrationLintCheck::HasSteps).unwrap();
    assert!(!step_check.passed);
}

#[test]
fn missing_verify_fails() {
    let source = r#"
#title: No Verify

@skill[name="no-verify", version="1.0", profile=migration]
  @precondition
    Has framework.
  @end

  @step[order=1]
    Migrate it.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let verify_check = results.iter().find(|r| r.check == MigrationLintCheck::HasVerify).unwrap();
    assert!(!verify_check.passed);
}

#[test]
fn missing_output_contract_fails() {
    let source = r#"
#title: No Output

@skill[name="no-output", version="1.0", profile=migration]
  @precondition
    Has framework.
  @end

  @step[order=1]
    Migrate it.
  @end

  @verify
    Check it.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let contract_check = results.iter().find(|r| r.check == MigrationLintCheck::HasOutputContract).unwrap();
    assert!(!contract_check.passed);
}

#[test]
fn not_a_migration_profile_fails() {
    let source = r#"
#title: Regular Skill

@skill[name="regular", version="1.0"]
  @precondition
    Something.
  @end

  @step[order=1]
    Do something.
  @end

  @verify
    Check it.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let profile_check = results.iter().find(|r| r.check == MigrationLintCheck::HasMigrationProfile).unwrap();
    assert!(!profile_check.passed);
}
