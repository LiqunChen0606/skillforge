use aif_migrate::types::{MigrationConfig, ChunkStatus, ChunkResult, VerificationResult};
use aif_migrate::chunk::ChunkStrategy;
use std::path::PathBuf;

#[test]
fn chunk_status_default_variants() {
    let statuses = vec![
        ChunkStatus::Success,
        ChunkStatus::PartialSuccess,
        ChunkStatus::Failed,
        ChunkStatus::Skipped,
    ];
    assert_eq!(statuses.len(), 4);
}

#[test]
fn chunk_result_has_expected_fields() {
    let result = ChunkResult {
        chunk_id: "test-001".to_string(),
        files: vec![PathBuf::from("src/main.rs")],
        status: ChunkStatus::Success,
        confidence: 0.95,
        verification: VerificationResult {
            static_checks: vec![],
            semantic_checks: vec![],
            passed: true,
        },
        repair_iterations: 0,
        notes: vec!["Clean migration".to_string()],
    };
    assert_eq!(result.chunk_id, "test-001");
    assert!(result.confidence > 0.9);
    assert!(result.verification.passed);
}

#[test]
fn migration_config_construction() {
    let config = MigrationConfig {
        skill_path: PathBuf::from("skill.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./migrated"),
        max_repair_iterations: 3,
        file_patterns: vec!["*.rs".to_string()],
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    };
    assert_eq!(config.max_repair_iterations, 3);
    assert_eq!(config.file_patterns.len(), 1);
}
