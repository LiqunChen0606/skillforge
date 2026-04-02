use aif_core::ast::*;
use aif_core::span::Span;
use aif_eval::pipeline::{EvalPipeline, PipelineConfig, StageFilter};
use aif_skill::eval::EvalStage;

fn make_attrs(pairs: Vec<(&str, &str)>) -> Attrs {
    let mut attrs = Attrs::new();
    for (k, v) in pairs {
        attrs.pairs.insert(k.into(), v.into());
    }
    attrs
}

fn make_valid_skill() -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(vec![
                ("name", "test-skill"),
                ("description", "Use when testing things"),
            ]),
            title: None,
            content: vec![],
            children: vec![
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Step,
                        attrs: {
                            let mut a = Attrs::new();
                            a.pairs.insert("order".into(), "1".into());
                            a
                        },
                        title: None,
                        content: vec![Inline::Text {
                            text: "Do the thing".into(),
                        }],
                        children: vec![],
                    },
                    span: Span::empty(),
                },
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Verify,
                        attrs: Attrs::new(),
                        title: None,
                        content: vec![Inline::Text {
                            text: "Check it worked".into(),
                        }],
                        children: vec![],
                    },
                    span: Span::empty(),
                },
            ],
        },
        span: Span::empty(),
    }
}

fn make_invalid_skill() -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(vec![("name", "bad skill")]),
            title: None,
            content: vec![],
            children: vec![],
        },
        span: Span::empty(),
    }
}

#[test]
fn lint_only_valid_skill() {
    let pipeline = EvalPipeline::new(PipelineConfig {
        stages: StageFilter::LintOnly,
        ..Default::default()
    });
    let report = pipeline.run_lint(&make_valid_skill());
    assert!(report.all_passed());
    assert_eq!(report.stages.len(), 1);
    assert_eq!(report.stages[0].stage, EvalStage::StructuralLint);
}

#[test]
fn lint_only_invalid_skill() {
    let pipeline = EvalPipeline::new(PipelineConfig {
        stages: StageFilter::LintOnly,
        ..Default::default()
    });
    let report = pipeline.run_lint(&make_invalid_skill());
    assert!(!report.all_passed());
}

#[test]
fn stage_filter_parsing() {
    assert_eq!(
        StageFilter::from_stage_number(1),
        Some(StageFilter::LintOnly)
    );
    assert_eq!(
        StageFilter::from_stage_number(2),
        Some(StageFilter::UpToCompliance)
    );
    assert_eq!(StageFilter::from_stage_number(3), Some(StageFilter::All));
    assert_eq!(StageFilter::from_stage_number(4), None);
}

#[test]
fn pipeline_config_defaults() {
    let config = PipelineConfig::default();
    assert!(matches!(config.stages, StageFilter::All));
}
