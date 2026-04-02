use aif_eval::compliance::{
    ComplianceChecker, ComplianceConfig, DefaultChecks, parse_compliance_response,
};

#[test]
fn default_checks_list() {
    let checks = DefaultChecks::all();
    assert_eq!(checks.len(), 3);
    assert!(checks.iter().any(|c| c.name == "skill-acknowledgment"));
    assert!(checks.iter().any(|c| c.name == "step-order"));
    assert!(checks.iter().any(|c| c.name == "no-skip-mandatory"));
}

#[test]
fn parse_passing_response() {
    let response = r#"{"checks": [
        {"name": "skill-acknowledgment", "passed": true, "evidence": "Agent said: Using skill X"},
        {"name": "step-order", "passed": true, "evidence": "Steps executed 1, 2, 3 in order"},
        {"name": "no-skip-mandatory", "passed": true, "evidence": "All mandatory steps present"}
    ]}"#;
    let results = parse_compliance_response(response).unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.passed));
}

#[test]
fn parse_failing_response() {
    let response = r#"{"checks": [
        {"name": "skill-acknowledgment", "passed": false, "evidence": "Agent did not mention the skill"},
        {"name": "step-order", "passed": true, "evidence": "Steps in order"},
        {"name": "no-skip-mandatory", "passed": true, "evidence": "All steps present"}
    ]}"#;
    let results = parse_compliance_response(response).unwrap();
    assert_eq!(results.iter().filter(|r| !r.passed).count(), 1);
}

#[test]
fn build_compliance_prompt_includes_skill() {
    let skill_text = "# My Skill\n\nStep 1: Do the thing\nStep 2: Verify";
    let task = "Implement a hello-world function";
    let checker = ComplianceChecker::new(ComplianceConfig::default());
    let (system, user_msg) = checker.build_prompt(skill_text, task, &DefaultChecks::all());
    assert!(system.contains("compliance"));
    assert!(user_msg.contains(skill_text));
    assert!(user_msg.contains(task));
}
