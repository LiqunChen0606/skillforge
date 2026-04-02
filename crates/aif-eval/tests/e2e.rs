use aif_core::ast::*;
use aif_eval::pipeline::{EvalPipeline, PipelineConfig, StageFilter};
use aif_eval::scenario::extract_scenarios;
use aif_skill::eval::EvalStage;

fn load_fixture() -> aif_core::ast::Document {
    let source =
        std::fs::read_to_string("../../tests/fixtures/skills/eval_test_skill.aif").unwrap();
    aif_parser::parse(&source).unwrap()
}

#[test]
fn fixture_parses_correctly() {
    let doc = load_fixture();
    assert_eq!(doc.blocks.len(), 1);
    if let BlockKind::SkillBlock { attrs, children, .. } = &doc.blocks[0].kind {
        assert_eq!(attrs.get("name"), Some("verification-before-completion"));
        assert!(
            children.len() >= 5,
            "Expected >=5 children, got {}",
            children.len()
        );
    } else {
        panic!("Expected SkillBlock");
    }
}

#[test]
fn stage1_lint_passes_on_fixture() {
    let doc = load_fixture();
    let pipeline = EvalPipeline::new(PipelineConfig {
        stages: StageFilter::LintOnly,
        ..Default::default()
    });
    let report = pipeline.run_lint(&doc.blocks[0]);
    assert!(
        report.all_passed(),
        "Fixture should pass lint. Report: {:?}",
        report.stages[0].details
    );
    assert_eq!(report.stages[0].stage, EvalStage::StructuralLint);
}

#[test]
fn fixture_scenarios_extracted() {
    let doc = load_fixture();
    let children = match &doc.blocks[0].kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => panic!("Expected SkillBlock"),
    };

    let verify_block = children
        .iter()
        .find(|b| {
            matches!(
                &b.kind,
                BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Verify,
                    ..
                }
            )
        })
        .expect("Expected @verify block");

    let scenarios = extract_scenarios(verify_block);
    assert_eq!(scenarios.len(), 2);
    assert_eq!(scenarios[0].name, "basic-compliance");
    assert_eq!(scenarios[1].name, "pressure-resistance");
}
