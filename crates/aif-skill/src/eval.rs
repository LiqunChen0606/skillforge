use serde::{Deserialize, Serialize};

use crate::lint::LintResult;

/// Which stage of the eval pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalStage {
    StructuralLint,
    BehavioralCompliance,
    EffectivenessEval,
}

/// Type of scenario test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioType {
    Scenario,
    Compliance,
    Pressure,
}

/// Result of a single scenario test (Stage 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub name: String,
    pub passed: bool,
    pub evidence: String,
    pub scenario_type: ScenarioType,
}

/// Result of a behavioral compliance check (Stage 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceResult {
    pub check_name: String,
    pub passed: bool,
    pub evidence: String,
}

/// Details for each stage type.
#[derive(Debug, Clone)]
pub enum StageDetails {
    Lint(Vec<LintResult>),
    Compliance(Vec<ComplianceResult>),
    Effectiveness(Vec<ScenarioResult>),
    Skipped(String),
}

/// Result of one pipeline stage.
#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage: EvalStage,
    pub passed: bool,
    pub duration_ms: u64,
    pub details: StageDetails,
}

/// Full eval pipeline report.
#[derive(Debug, Clone)]
pub struct EvalReport {
    pub skill_name: String,
    pub stages: Vec<StageResult>,
}

impl EvalReport {
    pub fn all_passed(&self) -> bool {
        self.stages.iter().all(|s| s.passed)
    }

    pub fn first_failure(&self) -> Option<&StageResult> {
        self.stages.iter().find(|s| !s.passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::{LintCheck, LintResult, LintSeverity};

    #[test]
    fn eval_report_all_passed() {
        let report = EvalReport {
            skill_name: "my-skill".into(),
            stages: vec![StageResult {
                stage: EvalStage::StructuralLint,
                passed: true,
                duration_ms: 50,
                details: StageDetails::Lint(vec![]),
            }],
        };
        assert!(report.all_passed());
    }

    #[test]
    fn eval_report_with_failure() {
        let report = EvalReport {
            skill_name: "my-skill".into(),
            stages: vec![StageResult {
                stage: EvalStage::StructuralLint,
                passed: false,
                duration_ms: 50,
                details: StageDetails::Lint(vec![LintResult {
                    check: LintCheck::Frontmatter,
                    passed: false,
                    severity: LintSeverity::Error,
                    message: "Missing description".into(),
                }]),
            }],
        };
        assert!(!report.all_passed());
        assert_eq!(
            report.first_failure().unwrap().stage,
            EvalStage::StructuralLint
        );
    }

    #[test]
    fn scenario_result_serializes() {
        let sr = ScenarioResult {
            name: "basic-test".into(),
            passed: true,
            evidence: "Agent ran tests before committing".into(),
            scenario_type: ScenarioType::Scenario,
        };
        let json = serde_json::to_string(&sr).unwrap();
        assert!(json.contains("basic-test"));
    }
}
