use aif_migrate::report::generate_report_document;
use aif_migrate::types::*;
use std::path::PathBuf;
use std::time::Duration;

fn make_report() -> MigrationReport {
    MigrationReport {
        skill_name: "jest-to-vitest".to_string(),
        source_dir: PathBuf::from("./src"),
        chunks: vec![
            ChunkResult {
                chunk_id: "chunk-0".to_string(),
                files: vec![PathBuf::from("src/a.test.ts")],
                status: ChunkStatus::Success,
                confidence: 0.98,
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
            },
            ChunkResult {
                chunk_id: "chunk-1".to_string(),
                files: vec![PathBuf::from("src/b.test.ts")],
                status: ChunkStatus::Failed,
                confidence: 0.40,
                verification: VerificationResult {
                    static_checks: vec![],
                    semantic_checks: vec![SemanticCheck {
                        criterion: "timer mocking".to_string(),
                        passed: false,
                        reasoning: "Not convertible".to_string(),
                        confidence: 0.3,
                    }],
                    passed: false,
                },
                repair_iterations: 3,
                notes: vec!["Needs manual review".to_string()],
            },
        ],
        overall_confidence: 0.69,
        unresolved: vec!["Snapshot files need regeneration".to_string()],
        manual_review: vec!["src/b.test.ts".to_string()],
        duration: Duration::from_secs(120),
    }
}

#[test]
fn report_generates_valid_aif_document() {
    let report = make_report();
    let doc = generate_report_document(&report);
    assert!(doc.metadata.contains_key("title"));
    assert!(doc.metadata.contains_key("author"));
    assert!(!doc.blocks.is_empty());
}

#[test]
fn report_includes_summary_section() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let has_summary = doc.blocks.iter().any(|b| {
        if let aif_core::ast::BlockKind::Section { title, .. } = &b.kind {
            aif_core::text::inlines_to_text(&title, aif_core::text::TextMode::Plain)
                .contains("Summary")
        } else {
            false
        }
    });
    assert!(has_summary, "Report should contain a Summary section");
}

#[test]
fn report_includes_failed_chunks() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("chunk-1"), "Report should mention failed chunk");
    assert!(json.contains("Failed"), "Report should mention failure status");
}

#[test]
fn report_includes_manual_review_section() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(
        json.contains("Manual Review"),
        "Report should have manual review section"
    );
    assert!(json.contains("src/b.test.ts"));
}

#[test]
fn report_all_passed_false_for_mixed() {
    let report = make_report();
    assert!(!report.all_passed());
}

#[test]
fn report_success_rate_correct() {
    let report = make_report();
    assert!((report.success_rate() - 0.5).abs() < 0.01);
}
