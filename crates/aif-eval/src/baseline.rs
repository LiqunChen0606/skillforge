//! Baseline save/load and regression detection for skill CI.
//!
//! A baseline captures a snapshot of scenario results for a skill at a point in time.
//! `detect_regressions()` compares current results against a saved baseline to find
//! scenarios that regressed (were passing, now failing) or whose score dropped significantly.

use serde::{Deserialize, Serialize};
use std::path::Path;

use aif_skill::eval::ScenarioResult;

/// A saved baseline of scenario results for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Name of the skill.
    pub skill_name: String,
    /// Model used during baseline run.
    pub model: String,
    /// ISO 8601 timestamp of the baseline run.
    pub timestamp: String,
    /// Scenario results captured at baseline time.
    pub results: Vec<ScenarioResult>,
}

/// A detected regression between baseline and current results.
#[derive(Debug, Clone)]
pub struct Regression {
    /// Name of the scenario that regressed.
    pub scenario_name: String,
    /// Whether the scenario passed in the baseline.
    pub baseline_passed: bool,
    /// Whether the scenario passed in the current run.
    pub current_passed: bool,
    /// Score from the baseline (1.0 if passed, 0.0 if failed).
    pub baseline_score: f64,
    /// Score from the current run (1.0 if passed, 0.0 if failed).
    pub current_score: f64,
    /// Score delta (current - baseline). Negative means regression.
    pub score_delta: f64,
}

/// Save a baseline to a JSON file.
pub fn save_baseline(baseline: &Baseline, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(baseline)
        .map_err(|e| format!("Failed to serialize baseline: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write baseline: {}", e))
}

/// Load a baseline from a JSON file.
pub fn load_baseline(path: &Path) -> Result<Baseline, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read baseline: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse baseline: {}", e))
}

/// Score threshold for regression detection.
/// A score drop greater than this triggers a regression.
const REGRESSION_THRESHOLD: f64 = 0.15;

/// Convert a pass/fail to a score (1.0 or 0.0).
fn pass_to_score(passed: bool) -> f64 {
    if passed {
        1.0
    } else {
        0.0
    }
}

/// Detect regressions between a baseline and current results.
///
/// A regression is detected when:
/// - A scenario was passing in the baseline but is now failing, OR
/// - The score dropped by more than 0.15
///
/// Scenarios present in current but not in baseline are ignored (new tests).
/// Scenarios present in baseline but not in current are ignored (removed tests).
pub fn detect_regressions(baseline: &Baseline, current: &[ScenarioResult]) -> Vec<Regression> {
    let mut regressions = Vec::new();

    for baseline_result in &baseline.results {
        if let Some(current_result) = current.iter().find(|r| r.name == baseline_result.name) {
            let baseline_score = pass_to_score(baseline_result.passed);
            let current_score = pass_to_score(current_result.passed);
            let delta = current_score - baseline_score;

            let is_regression = (baseline_result.passed && !current_result.passed)
                || delta < -REGRESSION_THRESHOLD;

            if is_regression {
                regressions.push(Regression {
                    scenario_name: baseline_result.name.clone(),
                    baseline_passed: baseline_result.passed,
                    current_passed: current_result.passed,
                    baseline_score,
                    current_score,
                    score_delta: delta,
                });
            }
        }
    }

    regressions
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_skill::eval::ScenarioType;
    use std::io::Write;

    fn make_result(name: &str, passed: bool) -> ScenarioResult {
        ScenarioResult {
            name: name.into(),
            passed,
            evidence: if passed {
                "ok".into()
            } else {
                "failed".into()
            },
            scenario_type: ScenarioType::Scenario,
        }
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baseline.json");

        let baseline = Baseline {
            skill_name: "test-skill".into(),
            model: "claude-sonnet-4-20250514".into(),
            timestamp: "2026-04-04T12:00:00Z".into(),
            results: vec![make_result("scenario-1", true), make_result("scenario-2", false)],
        };

        save_baseline(&baseline, &path).unwrap();
        let loaded = load_baseline(&path).unwrap();

        assert_eq!(loaded.skill_name, "test-skill");
        assert_eq!(loaded.model, "claude-sonnet-4-20250514");
        assert_eq!(loaded.timestamp, "2026-04-04T12:00:00Z");
        assert_eq!(loaded.results.len(), 2);
        assert!(loaded.results[0].passed);
        assert!(!loaded.results[1].passed);
    }

    #[test]
    fn detect_pass_to_fail_regression() {
        let baseline = Baseline {
            skill_name: "s".into(),
            model: "m".into(),
            timestamp: "t".into(),
            results: vec![make_result("a", true), make_result("b", true)],
        };

        let current = vec![make_result("a", true), make_result("b", false)];

        let regressions = detect_regressions(&baseline, &current);
        assert_eq!(regressions.len(), 1);
        assert_eq!(regressions[0].scenario_name, "b");
        assert!(regressions[0].baseline_passed);
        assert!(!regressions[0].current_passed);
        assert!((regressions[0].score_delta - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn no_regression_when_still_passing() {
        let baseline = Baseline {
            skill_name: "s".into(),
            model: "m".into(),
            timestamp: "t".into(),
            results: vec![make_result("a", true)],
        };

        let current = vec![make_result("a", true)];
        let regressions = detect_regressions(&baseline, &current);
        assert!(regressions.is_empty());
    }

    #[test]
    fn no_regression_when_fail_to_pass() {
        let baseline = Baseline {
            skill_name: "s".into(),
            model: "m".into(),
            timestamp: "t".into(),
            results: vec![make_result("a", false)],
        };

        let current = vec![make_result("a", true)];
        let regressions = detect_regressions(&baseline, &current);
        assert!(regressions.is_empty());
    }

    #[test]
    fn new_scenario_not_flagged() {
        let baseline = Baseline {
            skill_name: "s".into(),
            model: "m".into(),
            timestamp: "t".into(),
            results: vec![make_result("a", true)],
        };

        let current = vec![make_result("a", true), make_result("new-test", false)];
        let regressions = detect_regressions(&baseline, &current);
        assert!(regressions.is_empty());
    }

    #[test]
    fn removed_scenario_not_flagged() {
        let baseline = Baseline {
            skill_name: "s".into(),
            model: "m".into(),
            timestamp: "t".into(),
            results: vec![make_result("a", true), make_result("removed", true)],
        };

        let current = vec![make_result("a", true)];
        let regressions = detect_regressions(&baseline, &current);
        assert!(regressions.is_empty());
    }

    #[test]
    fn load_nonexistent_file_errors() {
        let result = load_baseline(Path::new("/tmp/nonexistent_baseline_12345.json"));
        assert!(result.is_err());
    }

    #[test]
    fn load_invalid_json_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"not json").unwrap();

        let result = load_baseline(&path);
        assert!(result.is_err());
    }
}
