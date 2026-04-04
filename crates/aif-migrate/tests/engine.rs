use aif_migrate::engine::MigrationEngine;
use aif_migrate::types::MigrationConfig;
use aif_migrate::chunk::ChunkStrategy;
use std::path::PathBuf;

#[test]
fn migration_config_defaults() {
    let config = MigrationConfig {
        skill_path: PathBuf::from("test.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./out"),
        max_repair_iterations: 3,
        file_patterns: vec![],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    };
    assert_eq!(config.max_repair_iterations, 3);
}

#[test]
fn engine_validates_skill_before_running() {
    let source = r#"
#title: Not a Migration

@skill[name="regular", version="1.0"]
  @step[order=1]
    Do something.
@/skill
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc
        .blocks
        .iter()
        .find(|b| {
            matches!(
                b.kind,
                aif_core::ast::BlockKind::SkillBlock {
                    skill_type: aif_core::ast::SkillBlockType::Skill,
                    ..
                }
            )
        })
        .unwrap();

    let engine = MigrationEngine::new(MigrationConfig {
        skill_path: PathBuf::from("test.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./out"),
        max_repair_iterations: 3,
        file_patterns: vec![],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let validation = engine.validate_skill(skill_block);
    assert!(
        !validation.is_valid(),
        "Should reject non-migration skill"
    );
}

#[test]
fn engine_validates_valid_migration_skill() {
    let source = r#"
#title: Test

@skill[name="test", version="1.0", profile=migration]
  @precondition
    Has framework.

  @step[order=1]
    Migrate it.

  @verify
    Check it.

  @output_contract
    Done.
@/skill
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc
        .blocks
        .iter()
        .find(|b| {
            matches!(
                b.kind,
                aif_core::ast::BlockKind::SkillBlock {
                    skill_type: aif_core::ast::SkillBlockType::Skill,
                    ..
                }
            )
        })
        .unwrap();

    let engine = MigrationEngine::new(MigrationConfig {
        skill_path: PathBuf::from("test.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./out"),
        max_repair_iterations: 3,
        file_patterns: vec![],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let validation = engine.validate_skill(skill_block);
    assert!(
        validation.is_valid(),
        "Should accept valid migration skill: {:?}",
        validation
    );
}

#[test]
fn engine_extracts_steps_from_skill() {
    let source = r#"
#title: Test

@skill[name="test", version="1.0", profile=migration]
  @precondition
    When to use.

  @step[order=1]
    First step.

  @step[order=2]
    Second step.

  @verify
    Check it.

  @output_contract
    Done.
@/skill
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc
        .blocks
        .iter()
        .find(|b| {
            matches!(
                b.kind,
                aif_core::ast::BlockKind::SkillBlock {
                    skill_type: aif_core::ast::SkillBlockType::Skill,
                    ..
                }
            )
        })
        .unwrap();

    let engine = MigrationEngine::new(MigrationConfig {
        skill_path: PathBuf::from("test.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./out"),
        max_repair_iterations: 3,
        file_patterns: vec![],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let steps = engine.extract_steps(skill_block);
    assert_eq!(steps.len(), 2);
    assert!(steps[0].contains("First step"));
    assert!(steps[1].contains("Second step"));
}

#[test]
fn engine_extracts_verify_criteria() {
    let source = r#"
#title: Test

@skill[name="test", version="1.0", profile=migration]
  @precondition
    When to use.

  @step[order=1]
    Migrate.

  @verify
    No remaining `old_api` calls.
    All files import `new_api`.

  @output_contract
    Done.
@/skill
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc
        .blocks
        .iter()
        .find(|b| {
            matches!(
                b.kind,
                aif_core::ast::BlockKind::SkillBlock {
                    skill_type: aif_core::ast::SkillBlockType::Skill,
                    ..
                }
            )
        })
        .unwrap();

    let engine = MigrationEngine::new(MigrationConfig {
        skill_path: PathBuf::from("test.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./out"),
        max_repair_iterations: 3,
        file_patterns: vec![],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let criteria = engine.extract_verify_criteria(skill_block);
    assert!(
        !criteria.is_empty(),
        "Should extract verification criteria"
    );
}
