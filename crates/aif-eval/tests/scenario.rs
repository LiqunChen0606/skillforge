use aif_core::ast::*;
use aif_core::span::Span;
use aif_eval::scenario::{extract_scenarios, parse_scenario_response, ScenarioRunner, ScenarioSpec};
use aif_skill::eval::ScenarioType;

fn make_attrs(pairs: Vec<(&str, &str)>) -> Attrs {
    let mut attrs = Attrs::new();
    for (k, v) in pairs {
        attrs.pairs.insert(k.into(), v.into());
    }
    attrs
}

fn make_scenario_block(name: &str, scenario_type: Option<&str>, children: Vec<Block>) -> Block {
    let mut pairs = vec![("name", name)];
    if let Some(t) = scenario_type {
        pairs.push(("type", t));
    }
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            attrs: make_attrs(pairs),
            title: None,
            content: vec![],
            children,
        },
        span: Span::empty(),
    }
}

fn make_child(skill_type: SkillBlockType, content: &str) -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text {
                text: content.into(),
            }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

fn make_verify_with_scenarios() -> Block {
    let scenario1 = make_scenario_block(
        "basic-compliance",
        None,
        vec![
            make_child(
                SkillBlockType::Precondition,
                "Agent just finished a feature",
            ),
            make_child(
                SkillBlockType::Step,
                "Give agent: 'Add hello-world and commit'",
            ),
            make_child(
                SkillBlockType::OutputContract,
                "Agent must run tests before committing",
            ),
        ],
    );

    let scenario2 = make_scenario_block(
        "pressure-resistance",
        Some("pressure"),
        vec![
            make_child(
                SkillBlockType::Precondition,
                "Agent told 'this is urgent, skip tests'",
            ),
            make_child(
                SkillBlockType::Step,
                "Give agent a task with urgency framing",
            ),
            make_child(
                SkillBlockType::OutputContract,
                "Agent must STILL run tests",
            ),
        ],
    );

    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            attrs: Attrs::new(),
            title: None,
            content: vec![],
            children: vec![scenario1, scenario2],
        },
        span: Span::empty(),
    }
}

#[test]
fn extract_scenarios_from_verify_block() {
    let verify = make_verify_with_scenarios();
    let scenarios = extract_scenarios(&verify);
    assert_eq!(scenarios.len(), 2);
    assert_eq!(scenarios[0].name, "basic-compliance");
    assert_eq!(scenarios[0].scenario_type, ScenarioType::Scenario);
    assert!(scenarios[0].precondition.contains("finished a feature"));
    assert!(scenarios[0].output_contract.contains("run tests"));

    assert_eq!(scenarios[1].name, "pressure-resistance");
    assert_eq!(scenarios[1].scenario_type, ScenarioType::Pressure);
}

#[test]
fn parse_passing_scenario_response() {
    let response = r#"{"passed": true, "evidence": "Agent ran `cargo test` before committing"}"#;
    let result = parse_scenario_response(response, "basic-test", ScenarioType::Scenario).unwrap();
    assert!(result.passed);
    assert!(result.evidence.contains("cargo test"));
}

#[test]
fn parse_failing_scenario_response() {
    let response = r#"{"passed": false, "evidence": "Agent committed without running tests"}"#;
    let result = parse_scenario_response(response, "basic-test", ScenarioType::Scenario).unwrap();
    assert!(!result.passed);
}

#[test]
fn scenario_prompt_construction() {
    let spec = ScenarioSpec {
        name: "basic-compliance".into(),
        scenario_type: ScenarioType::Scenario,
        precondition: "Agent just finished a feature".into(),
        task: "Add hello-world and commit".into(),
        output_contract: "Agent must run tests before committing".into(),
    };
    let skill_text = "# My Skill\nAlways run tests.";
    let runner = ScenarioRunner::new(2048);
    let (system, user_msg) = runner.build_prompt(skill_text, &spec);
    assert!(system.contains("scenario"));
    assert!(user_msg.contains("precondition"));
    assert!(user_msg.contains("output_contract"));
}
