use aif_core::ast::{Document, SkillBlockType};

use crate::extract::{extract_observables, find_skill_block};
use crate::matcher::match_block;
use crate::types::{ObservabilityReport, ObservationStatus};

/// Run observability analysis: extract observable blocks from the skill document,
/// match each against the LLM output, and compute aggregate metrics.
pub fn observe(doc: &Document, llm_output: &str) -> Result<ObservabilityReport, String> {
    let skill_block = find_skill_block(doc)
        .ok_or_else(|| "No @skill block found in document".to_string())?;

    let observables = extract_observables(skill_block);
    if observables.is_empty() {
        return Err("No observable blocks (step, verify, red_flag, precondition, output_contract) found in skill".to_string());
    }

    let observations: Vec<_> = observables
        .iter()
        .map(|block| match_block(block, llm_output))
        .collect();

    // Step coverage: fraction of Step blocks that are Followed or Partial
    let step_obs: Vec<_> = observations
        .iter()
        .filter(|o| o.block.block_type == SkillBlockType::Step)
        .collect();
    let step_coverage = if step_obs.is_empty() {
        1.0 // No steps = vacuously covered
    } else {
        let covered = step_obs
            .iter()
            .filter(|o| matches!(o.status, ObservationStatus::Followed | ObservationStatus::Partial))
            .count();
        covered as f64 / step_obs.len() as f64
    };

    // Constraint violations: red_flag Violated + output_contract Skipped/Violated
    let constraint_violations = observations
        .iter()
        .filter(|o| {
            (o.block.block_type == SkillBlockType::RedFlag
                && o.status == ObservationStatus::Violated)
                || (o.block.block_type == SkillBlockType::OutputContract
                    && matches!(o.status, ObservationStatus::Skipped | ObservationStatus::Violated))
        })
        .count() as u32;

    // Overall compliance: weighted average
    // Steps: 50% weight, Verify: 20%, RedFlag: 20%, OutputContract: 10%
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    for obs in &observations {
        let (weight, score) = match obs.block.block_type {
            SkillBlockType::Step => (0.5, status_score(&obs.status)),
            SkillBlockType::Verify => (0.2, status_score(&obs.status)),
            SkillBlockType::RedFlag => (0.2, status_score(&obs.status)),
            SkillBlockType::OutputContract => (0.1, status_score(&obs.status)),
            SkillBlockType::Precondition => (0.0, 0.0), // informational
            _ => (0.0, 0.0),
        };
        weighted_sum += weight * score;
        weight_total += weight;
    }

    let overall_compliance = if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        1.0
    };

    Ok(ObservabilityReport {
        observations,
        step_coverage,
        constraint_violations,
        overall_compliance,
    })
}

/// Convert an observation status to a numeric score for aggregation.
fn status_score(status: &ObservationStatus) -> f64 {
    match status {
        ObservationStatus::Followed => 1.0,
        ObservationStatus::Partial => 0.5,
        ObservationStatus::NotApplicable => 1.0,
        ObservationStatus::Skipped => 0.0,
        ObservationStatus::Violated => 0.0,
    }
}

/// Format an observability report as human-readable text.
pub fn format_text(report: &ObservabilityReport) -> String {
    let mut out = String::new();

    out.push_str("Skill Observability Report\n");
    out.push_str("=========================\n\n");

    for obs in &report.observations {
        let icon = match obs.status {
            ObservationStatus::Followed => "[OK]",
            ObservationStatus::Partial => "[~~]",
            ObservationStatus::Skipped => "[--]",
            ObservationStatus::Violated => "[!!]",
            ObservationStatus::NotApplicable => "[NA]",
        };
        let type_label = match obs.block.block_type {
            SkillBlockType::Step => {
                if let Some(order) = obs.block.order {
                    format!("Step {}", order)
                } else {
                    "Step".to_string()
                }
            }
            SkillBlockType::Verify => "Verify".to_string(),
            SkillBlockType::RedFlag => "Red Flag".to_string(),
            SkillBlockType::Precondition => "Precondition".to_string(),
            SkillBlockType::OutputContract => "Output Contract".to_string(),
            _ => format!("{:?}", obs.block.block_type),
        };

        out.push_str(&format!(
            "{} {} (score: {:.0}%): {}\n",
            icon,
            type_label,
            obs.match_score * 100.0,
            obs.block.content_snippet
        ));
    }

    out.push_str(&format!(
        "\nStep Coverage:          {:.0}%\n",
        report.step_coverage * 100.0
    ));
    out.push_str(&format!(
        "Constraint Violations:  {}\n",
        report.constraint_violations
    ));
    out.push_str(&format!(
        "Overall Compliance:     {:.0}%\n",
        report.overall_compliance * 100.0
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_code_review_skill() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/skills/code_review.aif"),
        )
        .expect("failed to read code_review.aif");

        let doc = aif_parser::parse(&source).expect("parse failed");

        // Simulate LLM output that follows the skill well
        let llm_output = r#"
## Code Review

### Understanding Context
I read the PR description and linked issues. The change adds pagination to the user list API.
I ran `git log --oneline -10` on the affected files to understand recent history and commits.

### Correctness Check
I verified the code does what it claims. I found an off-by-one error in the pagination logic
where `end` can exceed `items.len()`, causing a panic. I also checked for null handling,
race conditions, and resource leaks. I ran the test suite and all tests pass.

### Design Quality
The naming is clear and the separation of concerns is good. The API surface is minimal.
No functions exceed 50 lines. The new abstraction is justified by the complexity it handles.

### Actionable Feedback
**Bug (blocking):** The `paginate` function panics when page * per_page exceeds the slice length.
Use `end.min(items.len())` to clamp. This is a correctness issue that affects production.

**Suggestion:** Consider extracting the validation logic into a separate function to avoid
duplication between create and update paths.

### Verification
Every blocking issue includes a suggested fix or alternative approach.
No false positives were flagged. The review covers correctness, security, performance,
and maintainability. The author can act on this feedback without follow-up.

I did not approve without running tests. I avoided bikeshedding on style.

### Output
The review produces a structured list of findings:
- **Blocking:** Off-by-one in pagination (with fix)
- **Suggestion:** Extract duplicated validation
- **Praise:** Clean API surface and good test coverage
Overall verdict: Request changes (due to the pagination bug).
"#;

        let report = observe(&doc, llm_output).expect("observe failed");

        // Should have observations for all observable blocks
        assert!(!report.observations.is_empty());

        // Steps should be mostly followed
        assert!(report.step_coverage >= 0.5, "step_coverage={}", report.step_coverage);

        // Should have zero or minimal violations
        assert!(report.constraint_violations <= 1, "violations={}", report.constraint_violations);

        // Overall compliance should be reasonable
        assert!(report.overall_compliance >= 0.5, "compliance={}", report.overall_compliance);

        // Check text formatting works
        let text = format_text(&report);
        assert!(text.contains("Skill Observability Report"));
        assert!(text.contains("Step Coverage:"));
        assert!(text.contains("Overall Compliance:"));
    }

    #[test]
    fn observe_rejects_non_skill_document() {
        let doc = Document::default();
        let result = observe(&doc, "some output");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No @skill block"));
    }

    #[test]
    fn format_text_includes_all_sections() {
        let report = ObservabilityReport {
            observations: vec![],
            step_coverage: 0.75,
            constraint_violations: 2,
            overall_compliance: 0.60,
        };
        let text = format_text(&report);
        assert!(text.contains("75%"));
        assert!(text.contains("2"));
        assert!(text.contains("60%"));
    }
}
