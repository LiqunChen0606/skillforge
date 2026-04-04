use std::time::Instant;

use aif_core::ast::*;
use aif_core::config::LlmConfig;
use aif_skill::eval::*;
use aif_skill::lint;

use crate::anthropic::AnthropicClient;
use crate::compliance::{ComplianceChecker, ComplianceConfig, DefaultChecks};
use crate::scenario::{extract_scenarios, ScenarioRunner};

/// Which stages to run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StageFilter {
    LintOnly,
    UpToCompliance,
    #[default]
    All,
}

impl StageFilter {
    pub fn from_stage_number(n: u32) -> Option<Self> {
        match n {
            1 => Some(Self::LintOnly),
            2 => Some(Self::UpToCompliance),
            3 => Some(Self::All),
            _ => None,
        }
    }
}

/// Pipeline configuration.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub stages: StageFilter,
    pub llm: Option<LlmConfig>,
    pub compliance_task: Option<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            stages: StageFilter::All,
            llm: None,
            compliance_task: None,
        }
    }
}

/// The eval pipeline orchestrator.
pub struct EvalPipeline {
    config: PipelineConfig,
}

impl EvalPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Run Stage 1 (lint) only. Synchronous, no LLM needed.
    pub fn run_lint(&self, skill_block: &Block) -> EvalReport {
        let skill_name = extract_skill_name(skill_block);
        let start = Instant::now();
        let lint_results = lint::lint_skill(skill_block);
        let duration = start.elapsed().as_millis() as u64;

        let has_errors = lint_results
            .iter()
            .any(|r| !r.passed && r.severity == lint::LintSeverity::Error);

        EvalReport {
            skill_name,
            stages: vec![StageResult {
                stage: EvalStage::StructuralLint,
                passed: !has_errors,
                duration_ms: duration,
                details: StageDetails::Lint(lint_results),
            }],
        }
    }

    /// Run all configured stages. Async because stages 2-3 need LLM.
    pub async fn run(&self, skill_block: &Block, skill_text: &str) -> EvalReport {
        let skill_name = extract_skill_name(skill_block);
        let mut stages = Vec::new();

        // Stage 1: Structural lint (always runs)
        let lint_report = self.run_lint(skill_block);
        let lint_passed = lint_report.stages[0].passed;
        stages.push(lint_report.stages.into_iter().next().unwrap());

        if !lint_passed || matches!(self.config.stages, StageFilter::LintOnly) {
            if !lint_passed && !matches!(self.config.stages, StageFilter::LintOnly) {
                stages.push(StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped("previous stage failed".into()),
                });
                stages.push(StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped("previous stage failed".into()),
                });
            }
            return EvalReport {
                skill_name,
                stages,
            };
        }

        // Stage 2: Behavioral compliance (requires LLM)
        let compliance_result = self.run_compliance(skill_text).await;
        let compliance_passed = compliance_result.passed;
        stages.push(compliance_result);

        if !compliance_passed || matches!(self.config.stages, StageFilter::UpToCompliance) {
            if !compliance_passed && matches!(self.config.stages, StageFilter::All) {
                stages.push(StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped("previous stage failed".into()),
                });
            }
            return EvalReport {
                skill_name,
                stages,
            };
        }

        // Stage 3: Effectiveness eval (requires LLM)
        let effectiveness_result = self.run_scenarios(skill_block, skill_text).await;
        stages.push(effectiveness_result);

        EvalReport {
            skill_name,
            stages,
        }
    }

    async fn run_compliance(&self, skill_text: &str) -> StageResult {
        let llm = match &self.config.llm {
            Some(llm) => llm,
            None => {
                return StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped(
                        "no API key configured — run `aif config set llm.api-key <key>`".into(),
                    ),
                };
            }
        };

        let client = match AnthropicClient::new(
            llm.api_key.as_deref().unwrap_or(""),
            llm.resolved_model(),
            llm.base_url.as_deref(),
        ) {
            Ok(c) => c,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Compliance(vec![ComplianceResult {
                        check_name: "config".into(),
                        passed: false,
                        evidence: format!("LLM client error: {}", e),
                    }]),
                };
            }
        };

        let task = self.config.compliance_task.as_deref().unwrap_or(
            "Implement a simple feature: add a function that returns the sum of two numbers, write a test, and commit.",
        );

        let checker = ComplianceChecker::new(ComplianceConfig::default());
        let checks = DefaultChecks::all();

        let start = Instant::now();
        let results = match checker.evaluate(&client, skill_text, task, &checks).await {
            Ok(r) => r,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: StageDetails::Compliance(vec![ComplianceResult {
                        check_name: "api-call".into(),
                        passed: false,
                        evidence: format!("LLM API error: {}", e),
                    }]),
                };
            }
        };
        let duration = start.elapsed().as_millis() as u64;
        let passed = results.iter().all(|r| r.passed);

        StageResult {
            stage: EvalStage::BehavioralCompliance,
            passed,
            duration_ms: duration,
            details: StageDetails::Compliance(results),
        }
    }

    async fn run_scenarios(&self, skill_block: &Block, skill_text: &str) -> StageResult {
        let llm = match &self.config.llm {
            Some(llm) => llm,
            None => {
                return StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped(
                        "no API key configured — run `aif config set llm.api-key <key>`".into(),
                    ),
                };
            }
        };

        let client = match AnthropicClient::new(
            llm.api_key.as_deref().unwrap_or(""),
            llm.resolved_model(),
            llm.base_url.as_deref(),
        ) {
            Ok(c) => c,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Effectiveness(vec![ScenarioResult {
                        name: "config".into(),
                        passed: false,
                        evidence: format!("LLM client error: {}", e),
                        scenario_type: ScenarioType::Scenario,
                    }]),
                };
            }
        };

        let scenarios = find_all_scenarios(skill_block);

        if scenarios.is_empty() {
            return StageResult {
                stage: EvalStage::EffectivenessEval,
                passed: true,
                duration_ms: 0,
                details: StageDetails::Effectiveness(vec![]),
            };
        }

        let runner = ScenarioRunner::new(2048);
        let start = Instant::now();
        let results = match runner.evaluate_all(&client, skill_text, &scenarios).await {
            Ok(r) => r,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: StageDetails::Effectiveness(vec![ScenarioResult {
                        name: "api-call".into(),
                        passed: false,
                        evidence: format!("LLM API error: {}", e),
                        scenario_type: ScenarioType::Scenario,
                    }]),
                };
            }
        };
        let duration = start.elapsed().as_millis() as u64;
        let passed = results.iter().all(|r| r.passed);

        StageResult {
            stage: EvalStage::EffectivenessEval,
            passed,
            duration_ms: duration,
            details: StageDetails::Effectiveness(results),
        }
    }
}

fn extract_skill_name(block: &Block) -> String {
    if let BlockKind::SkillBlock { attrs, .. } = &block.kind {
        attrs.get("name").unwrap_or("(unnamed)").to_string()
    } else {
        "(not a skill)".to_string()
    }
}

fn find_all_scenarios(skill_block: &Block) -> Vec<crate::scenario::ScenarioSpec> {
    let children = match &skill_block.kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => return vec![],
    };

    let mut all_scenarios = Vec::new();
    for child in children {
        if let BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            ..
        } = &child.kind
        {
            all_scenarios.extend(extract_scenarios(child));
        }
    }
    all_scenarios
}
