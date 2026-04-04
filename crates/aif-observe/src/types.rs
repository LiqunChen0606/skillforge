use aif_core::ast::SkillBlockType;
use serde::{Deserialize, Serialize};

/// Whether an observable block was followed, skipped, violated, or partially followed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationStatus {
    /// The block's intent was clearly followed in the output.
    Followed,
    /// The block was not addressed at all in the output.
    Skipped,
    /// The block's constraint was actively violated (e.g., a red_flag was triggered).
    Violated,
    /// Some keywords matched but coverage was incomplete.
    Partial,
    /// The block is not applicable to this output (e.g., a tool block).
    NotApplicable,
}

/// A block extracted from a skill that can be observed in LLM output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableBlock {
    /// The skill block type (Step, Verify, RedFlag, Precondition, OutputContract).
    pub block_type: SkillBlockType,
    /// Block ID from attrs, if present.
    pub block_id: Option<String>,
    /// Order attribute, if present (for steps).
    pub order: Option<u32>,
    /// First 100 characters of the block's plain-text content.
    pub content_snippet: String,
    /// Full plain-text content of the block (used for matching).
    pub full_content: String,
}

/// Observation result for a single block after matching against LLM output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockObservation {
    /// The observable block that was matched.
    pub block: ObservableBlock,
    /// The observation status.
    pub status: ObservationStatus,
    /// Keyword match ratio (0.0 to 1.0).
    pub match_score: f64,
    /// Keywords that matched in the output.
    pub matched_keywords: Vec<String>,
    /// Keywords that were not found in the output.
    pub missing_keywords: Vec<String>,
}

/// Top-level observability report for a skill applied to LLM output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityReport {
    /// Per-block observations.
    pub observations: Vec<BlockObservation>,
    /// Fraction of steps that were followed or partially followed (0.0 to 1.0).
    pub step_coverage: f64,
    /// Number of constraint violations (red_flag violations + output_contract violations).
    pub constraint_violations: u32,
    /// Overall compliance score (0.0 to 1.0).
    pub overall_compliance: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_status_serializes() {
        let status = ObservationStatus::Followed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""Followed""#);

        let deserialized: ObservationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ObservationStatus::Followed);
    }

    #[test]
    fn observable_block_roundtrip() {
        let block = ObservableBlock {
            block_type: SkillBlockType::Step,
            block_id: Some("step1".into()),
            order: Some(1),
            content_snippet: "Check correctness of the code".into(),
            full_content: "Check correctness of the code under review".into(),
        };
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: ObservableBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.block_type, SkillBlockType::Step);
        assert_eq!(deserialized.block_id, Some("step1".into()));
        assert_eq!(deserialized.order, Some(1));
    }

    #[test]
    fn block_observation_roundtrip() {
        let obs = BlockObservation {
            block: ObservableBlock {
                block_type: SkillBlockType::Verify,
                block_id: None,
                order: None,
                content_snippet: "Every blocking issue includes a fix".into(),
                full_content: "Every blocking issue includes a suggested fix or alternative approach".into(),
            },
            status: ObservationStatus::Followed,
            match_score: 0.85,
            matched_keywords: vec!["blocking".into(), "issue".into(), "fix".into()],
            missing_keywords: vec!["alternative".into()],
        };
        let json = serde_json::to_string(&obs).unwrap();
        let deserialized: BlockObservation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, ObservationStatus::Followed);
        assert!((deserialized.match_score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn observability_report_roundtrip() {
        let report = ObservabilityReport {
            observations: vec![],
            step_coverage: 0.75,
            constraint_violations: 1,
            overall_compliance: 0.80,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        let deserialized: ObservabilityReport = serde_json::from_str(&json).unwrap();
        assert!((deserialized.step_coverage - 0.75).abs() < f64::EPSILON);
        assert_eq!(deserialized.constraint_violations, 1);
        assert!((deserialized.overall_compliance - 0.80).abs() < f64::EPSILON);
    }
}
