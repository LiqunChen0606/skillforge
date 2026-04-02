use aif_migrate::chunk::{chunk_source_files, ChunkStrategy};
use aif_migrate::engine::MigrationEngine;
use aif_migrate::report::generate_report_document;
use aif_migrate::types::*;
use aif_migrate::verify::{extract_static_specs, run_static_checks};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

fn load_fixture_skill() -> aif_core::ast::Block {
    let source = include_str!("fixtures/jest-to-vitest.aif");
    let doc = aif_parser::parse(source).expect("fixture parse failed");
    doc.blocks
        .into_iter()
        .find(|b| matches!(b.kind, aif_core::ast::BlockKind::SkillBlock { .. }))
        .expect("no skill block in fixture")
}

#[test]
fn full_validation_pipeline() {
    let skill = load_fixture_skill();
    let engine = MigrationEngine::new(MigrationConfig {
        skill_path: PathBuf::from("test.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./out"),
        max_repair_iterations: 3,
        file_patterns: vec![],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });

    // 1. Validate skill
    let validation = engine.validate_skill(&skill);
    assert!(
        validation.is_valid(),
        "Fixture skill should be valid: {:?}",
        validation
    );

    // 2. Extract steps
    let steps = engine.extract_steps(&skill);
    assert_eq!(steps.len(), 2, "Should have 2 steps");

    // 3. Extract verify criteria
    let criteria = engine.extract_verify_criteria(&skill);
    assert!(!criteria.is_empty(), "Should have verify criteria");

    // 4. Extract fallback
    let fallback = engine.extract_fallback(&skill);
    assert!(fallback.is_some(), "Should have fallback");
    assert!(fallback.unwrap().contains("timer mocking"));
}

#[test]
fn chunking_and_static_verification_pipeline() {
    let source_files: HashMap<PathBuf, String> = [
        (
            PathBuf::from("src/a.test.ts"),
            "import { vi } from 'vitest';\nvi.fn();".to_string(),
        ),
        (
            PathBuf::from("src/b.test.ts"),
            "import { vi } from 'vitest';\nvi.mock('./db');".to_string(),
        ),
    ]
    .into_iter()
    .collect();

    let chunks = chunk_source_files(&source_files, ChunkStrategy::FilePerChunk);
    assert_eq!(chunks.len(), 2);

    let verify_text =
        "No remaining `jest.` calls in test files.\nAll test files import from `vitest`.";
    let specs = extract_static_specs(verify_text);

    for chunk in &chunks {
        for (_, content) in &chunk.files {
            let results = run_static_checks(content, &specs);
            assert!(
                results.iter().all(|r| r.passed),
                "Migrated content should pass all static checks: {:?}",
                results
            );
        }
    }
}

#[test]
fn report_generation_end_to_end() {
    let report = MigrationReport {
        skill_name: "jest-to-vitest".to_string(),
        source_dir: PathBuf::from("./src"),
        chunks: vec![ChunkResult {
            chunk_id: "file-0000-src/a.test.ts".to_string(),
            files: vec![PathBuf::from("src/a.test.ts")],
            status: ChunkStatus::Success,
            confidence: 0.95,
            verification: VerificationResult {
                static_checks: vec![StaticCheck {
                    name: "no jest".to_string(),
                    passed: true,
                    detail: "Clean".to_string(),
                }],
                semantic_checks: vec![],
                passed: true,
            },
            repair_iterations: 0,
            notes: vec![],
        }],
        overall_confidence: 0.95,
        unresolved: vec![],
        manual_review: vec![],
        duration: Duration::from_secs(5),
    };

    let doc = generate_report_document(&report);

    // Compile to HTML to verify it's valid AIF
    let html = aif_html::render_html(&doc);
    assert!(html.contains("Migration Report"));
    assert!(html.contains("jest-to-vitest"));
    assert!(html.contains("Success"));

    // Compile to JSON to verify structure
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Executive Summary"));
    assert!(json.contains("Risk Assessment"));
    assert!(json.contains("Recommendations"));
}
