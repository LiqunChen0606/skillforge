//! CI runner for skill testing — non-stopping execution.
//!
//! Runs lint first, then executes all scenarios independently (no early exit).
//! Returns structured results for CI consumption.

use aif_core::ast::*;
use aif_skill::eval::ScenarioResult;
use aif_skill::lint::{self, LintResult, LintSeverity};

use crate::scenario::{extract_scenarios, ScenarioSpec};

/// Result of a CI run.
#[derive(Debug)]
pub enum CiResult {
    /// Lint had errors — scenarios were not run.
    LintFailed(Vec<LintResult>),
    /// All scenarios were executed (some may have failed).
    Completed(Vec<ScenarioResult>),
}

impl CiResult {
    /// Whether the CI run is considered passing.
    pub fn passed(&self) -> bool {
        match self {
            CiResult::LintFailed(_) => false,
            CiResult::Completed(results) => results.iter().all(|r| r.passed),
        }
    }

    /// Whether any regressions or failures exist.
    pub fn has_failures(&self) -> bool {
        !self.passed()
    }
}

/// Extract all scenario specs from a skill block's @verify children.
pub fn extract_all_scenarios(skill_block: &Block) -> Vec<ScenarioSpec> {
    let children = match &skill_block.kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => return vec![],
    };

    let mut all = Vec::new();
    for child in children {
        if let BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify | SkillBlockType::Scenario,
            ..
        } = &child.kind
        {
            all.extend(extract_scenarios(child));
        }
    }
    all
}

/// Run skill CI: lint first, then execute all scenarios via the provided callback.
///
/// The `run_scenario` callback is invoked for each scenario independently.
/// All scenarios run even if some fail (no early exit).
///
/// If lint has errors, returns `CiResult::LintFailed` without running scenarios.
pub fn run_ci<F>(skill_block: &Block, mut run_scenario: F) -> CiResult
where
    F: FnMut(&ScenarioSpec) -> ScenarioResult,
{
    // Step 1: Run lint
    let lint_results = lint::lint_skill(skill_block);
    let has_errors = lint_results
        .iter()
        .any(|r| !r.passed && r.severity == LintSeverity::Error);

    if has_errors {
        return CiResult::LintFailed(lint_results);
    }

    // Step 2: Extract scenarios
    let scenarios = extract_all_scenarios(skill_block);

    // Step 3: Run each scenario independently (no early exit)
    let results: Vec<ScenarioResult> = scenarios.iter().map(&mut run_scenario).collect();

    CiResult::Completed(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;
    use aif_skill::eval::ScenarioType;

    const S: Span = Span { start: 0, end: 0 };

    fn skill_block(_name: &str, attrs: Attrs, content: Vec<Inline>, children: Vec<Block>) -> Block {
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content,
                children,
            },
            span: S,
        }
    }

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

    fn skill_attrs(name: &str) -> Attrs {
        let mut a = Attrs::new();
        a.pairs.insert("name".into(), name.into());
        a.pairs
            .insert("description".into(), "Use when testing CI runner".into());
        a
    }

    fn name_attr(name: &str) -> Attrs {
        let mut a = Attrs::new();
        a.pairs.insert("name".into(), name.into());
        a
    }

    /// Build a minimal valid skill block with named scenarios.
    fn make_skill_block(name: &str, scenarios: Vec<(&str, &str)>) -> Block {
        let mut children = vec![
            sb(
                SkillBlockType::Precondition,
                Attrs::default(),
                vec![text("When debugging")],
                vec![],
            ),
            sb(
                SkillBlockType::Step,
                {
                    let mut a = Attrs::new();
                    a.pairs.insert("order".into(), "1".into());
                    a
                },
                vec![text("Do the thing")],
                vec![],
            ),
        ];

        if !scenarios.is_empty() {
            let scenario_blocks: Vec<Block> = scenarios
                .into_iter()
                .map(|(sname, task)| {
                    let scenario_children = vec![
                        sb(
                            SkillBlockType::Precondition,
                            Attrs::default(),
                            vec![text("Given a codebase")],
                            vec![],
                        ),
                        sb(
                            SkillBlockType::Step,
                            Attrs::default(),
                            vec![text(task)],
                            vec![],
                        ),
                        sb(
                            SkillBlockType::OutputContract,
                            Attrs::default(),
                            vec![text("Must pass")],
                            vec![],
                        ),
                    ];

                    sb(
                        SkillBlockType::Verify,
                        name_attr(sname),
                        vec![],
                        scenario_children,
                    )
                })
                .collect();

            children.push(sb(
                SkillBlockType::Verify,
                Attrs::default(),
                vec![text("Check it works")],
                scenario_blocks,
            ));
        } else {
            children.push(sb(
                SkillBlockType::Verify,
                Attrs::default(),
                vec![text("Check it works")],
                vec![],
            ));
        }

        skill_block(name, skill_attrs(name), vec![], children)
    }

    #[test]
    fn all_scenarios_run_even_when_some_fail() {
        let skill = make_skill_block(
            "test-skill",
            vec![
                ("scenario-pass", "Do something good"),
                ("scenario-fail", "Do something bad"),
                ("scenario-pass-2", "Do something else good"),
            ],
        );

        let mut call_count = 0;
        let result = run_ci(&skill, |spec| {
            call_count += 1;
            ScenarioResult {
                name: spec.name.clone(),
                passed: spec.name.contains("pass"),
                evidence: format!("Ran {}", spec.name),
                scenario_type: ScenarioType::Scenario,
            }
        });

        assert_eq!(call_count, 3);

        match result {
            CiResult::Completed(results) => {
                assert_eq!(results.len(), 3);
                assert!(results[0].passed);
                assert!(!results[1].passed);
                assert!(results[2].passed);
            }
            CiResult::LintFailed(_) => panic!("Expected Completed, got LintFailed"),
        }
    }

    #[test]
    fn lint_failure_stops_scenarios() {
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

        let mut scenario_called = false;
        let result = run_ci(&bad_skill, |_spec| {
            scenario_called = true;
            ScenarioResult {
                name: "should-not-run".into(),
                passed: true,
                evidence: "".into(),
                scenario_type: ScenarioType::Scenario,
            }
        });

        assert!(!scenario_called);
        assert!(matches!(result, CiResult::LintFailed(_)));
        assert!(!result.passed());
    }

    #[test]
    fn passed_when_all_scenarios_pass() {
        let skill = make_skill_block("good-skill", vec![("s1", "task1"), ("s2", "task2")]);

        let result = run_ci(&skill, |spec| ScenarioResult {
            name: spec.name.clone(),
            passed: true,
            evidence: "ok".into(),
            scenario_type: ScenarioType::Scenario,
        });

        assert!(result.passed());
        assert!(!result.has_failures());
    }

    #[test]
    fn not_passed_when_any_scenario_fails() {
        let skill = make_skill_block("mixed-skill", vec![("s1", "task1"), ("s2", "task2")]);

        let result = run_ci(&skill, |spec| ScenarioResult {
            name: spec.name.clone(),
            passed: spec.name == "s1",
            evidence: "evidence".into(),
            scenario_type: ScenarioType::Scenario,
        });

        assert!(!result.passed());
        assert!(result.has_failures());
    }

    #[test]
    fn no_scenarios_is_passing() {
        let skill = make_skill_block("empty-skill", vec![]);

        let result = run_ci(&skill, |_| {
            panic!("Should not be called");
        });

        assert!(result.passed());
        match result {
            CiResult::Completed(results) => assert!(results.is_empty()),
            _ => panic!("Expected Completed"),
        }
    }
}
