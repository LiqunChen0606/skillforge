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
                    static_checks: vec![StaticCheck {
                        name: "no jest".to_string(),
                        passed: false,
                        detail: "Found jest.useFakeTimers()".to_string(),
                    }],
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

fn make_success_report() -> MigrationReport {
    MigrationReport {
        skill_name: "eslint-flat".to_string(),
        source_dir: PathBuf::from("./src"),
        chunks: vec![
            ChunkResult {
                chunk_id: "chunk-0".to_string(),
                files: vec![PathBuf::from(".eslintrc.js")],
                status: ChunkStatus::Success,
                confidence: 0.95,
                verification: VerificationResult {
                    static_checks: vec![
                        StaticCheck { name: "no eslintrc".to_string(), passed: true, detail: "Clean".to_string() },
                        StaticCheck { name: "has flat config".to_string(), passed: true, detail: "Found".to_string() },
                    ],
                    semantic_checks: vec![
                        SemanticCheck { criterion: "rule parity".to_string(), passed: true, reasoning: "All rules migrated".to_string(), confidence: 0.92 },
                    ],
                    passed: true,
                },
                repair_iterations: 0,
                notes: vec![],
            },
        ],
        overall_confidence: 0.95,
        unresolved: vec![],
        manual_review: vec![],
        duration: Duration::from_secs(5),
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
fn report_includes_executive_summary() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Executive Summary"), "Report should contain Executive Summary");
    assert!(json.contains("jest-to-vitest"));
    assert!(json.contains("50%"), "Should show success rate");
}

#[test]
fn report_includes_risk_assessment() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Risk Assessment"), "Report should contain Risk Assessment");
    assert!(json.contains("Risk Level"), "Should include risk level");
}

#[test]
fn report_includes_verification_analysis() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Verification Analysis"), "Report should contain Verification Analysis");
    assert!(json.contains("Static Checks"), "Should include static check summary");
    assert!(json.contains("Semantic Checks"), "Should include semantic check summary");
}

#[test]
fn report_includes_failure_analysis() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Failure Analysis"), "Report should contain Failure Analysis");
    assert!(json.contains("Recurring failure patterns"), "Should show failure patterns");
}

#[test]
fn report_includes_recommendations() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Recommendations"), "Report should contain Recommendations");
    assert!(json.contains("manual review"), "Should mention manual review items");
    assert!(json.contains("unresolved"), "Should mention unresolved issues");
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
    assert!(json.contains("Manual Review"), "Report should have manual review section");
    assert!(json.contains("src/b.test.ts"));
}

#[test]
fn report_chunk_details_include_verification_breakdown() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    // Chunk details should include pass/fail for individual checks
    assert!(json.contains("[PASS]") || json.contains("[FAIL]"),
        "Chunk results should show individual check pass/fail status");
    assert!(json.contains("Found jest.useFakeTimers()"),
        "Should show static check detail for failed checks");
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

#[test]
fn report_status_counts() {
    let report = make_report();
    let (s, p, f, sk) = report.status_counts();
    assert_eq!(s, 1);
    assert_eq!(p, 0);
    assert_eq!(f, 1);
    assert_eq!(sk, 0);
}

#[test]
fn report_failed_checks_collected() {
    let report = make_report();
    let failed_static = report.failed_static_checks();
    assert_eq!(failed_static.len(), 1);
    assert_eq!(failed_static[0], ("chunk-1", "no jest"));

    let failed_semantic = report.failed_semantic_checks();
    assert_eq!(failed_semantic.len(), 1);
    assert_eq!(failed_semantic[0].1, "timer mocking");
}

#[test]
fn report_total_repair_iterations() {
    let report = make_report();
    assert_eq!(report.total_repair_iterations(), 3);
}

#[test]
fn report_average_confidence() {
    let report = make_report();
    // (0.98 + 0.40) / 2 = 0.69
    assert!((report.average_confidence() - 0.69).abs() < 0.01);
}

#[test]
fn report_confidence_labels() {
    assert_eq!(MigrationReport::confidence_label(0.95), "High");
    assert_eq!(MigrationReport::confidence_label(0.75), "Medium");
    assert_eq!(MigrationReport::confidence_label(0.55), "Low");
    assert_eq!(MigrationReport::confidence_label(0.30), "Very Low");
}

#[test]
fn report_risk_level() {
    let report = make_report();
    assert_eq!(report.risk_level(), "High Risk");

    let good = make_success_report();
    assert_eq!(good.risk_level(), "Low Risk"); // 0.95 conf >= 0.9, 100% rate >= 0.95
}

#[test]
fn success_report_has_no_failure_analysis() {
    let report = make_success_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(!json.contains("Failure Analysis"),
        "Success report should not contain Failure Analysis");
    assert!(json.contains("successfully with high confidence"),
        "Should recommend integration for high-success migrations");
}

#[test]
fn report_compiles_to_html() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let html = aif_html::render_html(&doc);
    assert!(html.contains("Migration Report"));
    assert!(html.contains("Executive Summary"));
    assert!(html.contains("Risk Assessment"));
    assert!(html.contains("Verification Analysis"));
    assert!(html.contains("Recommendations"));
}
