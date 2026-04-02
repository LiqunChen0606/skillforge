use aif_migrate::engine::MigrationEngine;
use aif_migrate::types::MigrationConfig;
use aif_migrate::chunk::ChunkStrategy;
use std::path::PathBuf;

fn load_and_validate(source: &str, expected_name: &str, expected_steps: usize) {
    let doc = aif_parser::parse(source).expect("parse failed");
    let skill = doc.blocks
        .into_iter()
        .find(|b| matches!(b.kind, aif_core::ast::BlockKind::SkillBlock { .. }))
        .expect("no skill block found");

    let engine = MigrationEngine::new(MigrationConfig {
        skill_path: PathBuf::from("test.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./out"),
        max_repair_iterations: 3,
        file_patterns: vec![],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });

    // Validate migration profile
    let validation = engine.validate_skill(&skill);
    assert!(
        validation.is_valid(),
        "Skill '{}' should be valid: {:?}",
        expected_name, validation
    );

    // Check steps extracted
    let steps = engine.extract_steps(&skill);
    assert_eq!(
        steps.len(), expected_steps,
        "Skill '{}' should have {} steps, got {}",
        expected_name, expected_steps, steps.len()
    );

    // Check verify criteria extracted
    let criteria = engine.extract_verify_criteria(&skill);
    assert!(
        !criteria.is_empty(),
        "Skill '{}' should have verify criteria",
        expected_name
    );

    // Check fallback extracted
    let fallback = engine.extract_fallback(&skill);
    assert!(
        fallback.is_some(),
        "Skill '{}' should have a fallback block",
        expected_name
    );
}

#[test]
fn jest_to_vitest_fixture_validates() {
    let source = include_str!("fixtures/jest-to-vitest.aif");
    load_and_validate(source, "jest-to-vitest", 2);
}

#[test]
fn nextjs_13_to_15_example_validates() {
    let source = include_str!("../../../examples/migrations/migration_nextjs_13_to_15.aif");
    load_and_validate(source, "nextjs-13-to-15", 7);
}

#[test]
fn eslint_flat_config_example_validates() {
    let source = include_str!("../../../examples/migrations/migration_eslint_flat_config.aif");
    load_and_validate(source, "eslint-legacy-to-flat", 7);
}

#[test]
fn typescript_strict_example_validates() {
    let source = include_str!("../../../examples/migrations/migration_typescript_strict.aif");
    load_and_validate(source, "typescript-strict-mode", 8);
}

#[test]
fn nextjs_skill_has_rich_steps() {
    let source = include_str!("../../../examples/migrations/migration_nextjs_13_to_15.aif");
    let doc = aif_parser::parse(source).unwrap();
    let skill = doc.blocks.into_iter()
        .find(|b| matches!(b.kind, aif_core::ast::BlockKind::SkillBlock { .. }))
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

    let steps = engine.extract_steps(&skill);

    // Verify steps contain meaningful content
    assert!(steps[0].contains("package.json"), "Step 1 should mention package.json");
    assert!(steps[1].contains("async"), "Step 2 should mention async request APIs");
    assert!(steps[3].contains("caching") || steps[3].contains("cache"),
        "Step 4 should address caching semantics");

    let criteria = engine.extract_verify_criteria(&skill);
    assert!(!criteria.is_empty(), "Should have verify criteria");
    // The single @verify block should contain multiple verification lines
    let criteria_text = &criteria[0];
    assert!(criteria_text.contains("@next/font"), "Should verify @next/font removal");
    assert!(criteria_text.contains("await"), "Should verify async params usage");
    assert!(criteria_text.contains("next") && criteria_text.contains("15"),
        "Should verify next version >= 15");
}

#[test]
fn typescript_strict_has_phased_steps() {
    let source = include_str!("../../../examples/migrations/migration_typescript_strict.aif");
    let doc = aif_parser::parse(source).unwrap();
    let skill = doc.blocks.into_iter()
        .find(|b| matches!(b.kind, aif_core::ast::BlockKind::SkillBlock { .. }))
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

    let steps = engine.extract_steps(&skill);

    // Should have 8 phases covering all strict sub-flags
    assert_eq!(steps.len(), 8);
    assert!(steps[0].contains("alwaysStrict"), "Phase 1 should start with alwaysStrict");
    assert!(steps[2].contains("noImplicitAny"), "Phase 3 should handle noImplicitAny");
    assert!(steps[3].contains("strictNullChecks"), "Phase 4 should handle strictNullChecks");
    assert!(steps[7].contains("strict: true") || steps[7].contains("strict"),
        "Final phase should consolidate to strict: true");
}
