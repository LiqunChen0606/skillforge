//! Integration test: full skill CI pipeline.
//!
//! Tests: parse skill -> run CI (with mock LLM) -> save baseline -> detect regressions -> generate JUnit XML.

use aif_core::ast::*;
use aif_core::span::Span;
use aif_eval::baseline::{detect_regressions, load_baseline, save_baseline, Baseline};
use aif_eval::ci_runner::{run_ci, CiResult};
use aif_eval::junit::generate_junit_xml;
use aif_skill::eval::{ScenarioResult, ScenarioType};

const S: Span = Span { start: 0, end: 0 };

fn sb(st: SkillBlockType, attrs: Attrs, content: Vec<Inline>, children: Vec<Block>) -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: st,
            attrs,
            title: None,
            content,
            children,
        },
        span: S,
    }
}

fn text(s: &str) -> Inline {
    Inline::Text {
        text: s.to_string(),
    }
}

fn name_attr(name: &str) -> Attrs {
    let mut a = Attrs::new();
    a.pairs.insert("name".into(), name.into());
    a
}

/// Build a skill block with scenarios that can be tested.
fn build_test_skill() -> Block {
    let mut skill_attrs = Attrs::new();
    skill_attrs
        .pairs
        .insert("name".into(), "integration-test-skill".into());
    skill_attrs.pairs.insert(
        "description".into(),
        "Use when running integration tests".into(),
    );

    let precondition = sb(
        SkillBlockType::Precondition,
        Attrs::default(),
        vec![text("When running tests")],
        vec![],
    );

    let step = sb(
        SkillBlockType::Step,
        {
            let mut a = Attrs::new();
            a.pairs.insert("order".into(), "1".into());
            a
        },
        vec![text("Run the test suite")],
        vec![],
    );

    // Two named scenarios inside a @verify block
    let scenario1 = sb(
        SkillBlockType::Verify,
        name_attr("basic-passing"),
        vec![],
        vec![
            sb(
                SkillBlockType::Precondition,
                Attrs::default(),
                vec![text("Given a clean codebase")],
                vec![],
            ),
            sb(
                SkillBlockType::Step,
                Attrs::default(),
                vec![text("Run tests")],
                vec![],
            ),
            sb(
                SkillBlockType::OutputContract,
                Attrs::default(),
                vec![text("All tests pass")],
                vec![],
            ),
        ],
    );

    let scenario2 = sb(
        SkillBlockType::Verify,
        name_attr("edge-case"),
        vec![],
        vec![
            sb(
                SkillBlockType::Precondition,
                Attrs::default(),
                vec![text("Given a broken dependency")],
                vec![],
            ),
            sb(
                SkillBlockType::Step,
                Attrs::default(),
                vec![text("Handle gracefully")],
                vec![],
            ),
            sb(
                SkillBlockType::OutputContract,
                Attrs::default(),
                vec![text("Error message is clear")],
                vec![],
            ),
        ],
    );

    let verify = sb(
        SkillBlockType::Verify,
        Attrs::default(),
        vec![text("Verify the skill works")],
        vec![scenario1, scenario2],
    );

    sb(
        SkillBlockType::Skill,
        skill_attrs,
        vec![],
        vec![precondition, step, verify],
    )
}

#[test]
fn full_ci_pipeline_with_mock_llm() {
    let skill = build_test_skill();

    // Step 1: Run CI with mock LLM (all pass)
    let result = run_ci(&skill, |spec| ScenarioResult {
        name: spec.name.clone(),
        passed: true,
        evidence: format!("Mock: {} passed", spec.name),
        scenario_type: ScenarioType::Scenario,
    });

    assert!(result.passed());
    let scenario_results = match &result {
        CiResult::Completed(r) => r.clone(),
        CiResult::LintFailed(_) => panic!("Expected Completed"),
    };
    assert_eq!(scenario_results.len(), 2);

    // Step 2: Save baseline
    let dir = tempfile::tempdir().unwrap();
    let baseline_path = dir.path().join("baseline.json");

    let baseline = Baseline {
        skill_name: "integration-test-skill".into(),
        model: "mock".into(),
        timestamp: "2026-04-04T00:00:00Z".into(),
        results: scenario_results.clone(),
    };
    save_baseline(&baseline, &baseline_path).unwrap();

    // Step 3: Verify baseline loads back correctly
    let loaded = load_baseline(&baseline_path).unwrap();
    assert_eq!(loaded.skill_name, "integration-test-skill");
    assert_eq!(loaded.results.len(), 2);

    // Step 4: Run CI again with one failure (regression)
    let result2 = run_ci(&skill, |spec| ScenarioResult {
        name: spec.name.clone(),
        passed: spec.name == "basic-passing", // edge-case now fails
        evidence: if spec.name == "basic-passing" {
            "Still passing".into()
        } else {
            "Now failing due to new dependency".into()
        },
        scenario_type: ScenarioType::Scenario,
    });

    let current_results = match &result2 {
        CiResult::Completed(r) => r.clone(),
        CiResult::LintFailed(_) => panic!("Expected Completed"),
    };

    // Step 5: Detect regressions
    let regressions = detect_regressions(&loaded, &current_results);
    assert_eq!(regressions.len(), 1);
    assert_eq!(regressions[0].scenario_name, "edge-case");
    assert!(regressions[0].baseline_passed);
    assert!(!regressions[0].current_passed);

    // Step 6: Generate JUnit XML from results
    let junit = generate_junit_xml("integration-test-skill", &current_results);
    assert!(junit.contains("tests=\"2\""));
    assert!(junit.contains("failures=\"1\""));
    assert!(junit.contains("name=\"basic-passing\""));
    assert!(junit.contains("name=\"edge-case\""));
    assert!(junit.contains("<failure"));
    assert!(junit.contains("Now failing due to new dependency"));
}

#[test]
fn ci_pipeline_lint_failure_produces_no_scenarios() {
    // A skill without required attributes will fail lint
    let bad_skill = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: Attrs::default(),
            title: None,
            content: vec![],
            children: vec![],
        },
        span: S,
    };

    let result = run_ci(&bad_skill, |_| {
        panic!("Scenarios should not run when lint fails");
    });

    assert!(!result.passed());
    match result {
        CiResult::LintFailed(lint_results) => {
            assert!(lint_results.iter().any(|r| !r.passed));
        }
        _ => panic!("Expected LintFailed"),
    }
}

#[test]
fn ci_pipeline_all_pass_no_regressions() {
    let skill = build_test_skill();

    let result = run_ci(&skill, |spec| ScenarioResult {
        name: spec.name.clone(),
        passed: true,
        evidence: "ok".into(),
        scenario_type: ScenarioType::Scenario,
    });

    let results = match result {
        CiResult::Completed(r) => r,
        _ => panic!("Expected Completed"),
    };

    // Save and immediately compare — no regressions
    let baseline = Baseline {
        skill_name: "test".into(),
        model: "mock".into(),
        timestamp: "t".into(),
        results: results.clone(),
    };

    let regressions = detect_regressions(&baseline, &results);
    assert!(regressions.is_empty());

    // JUnit should show 0 failures
    let junit = generate_junit_xml("test", &results);
    assert!(junit.contains("failures=\"0\""));
}
