use aif_migrate::repair::{build_repair_context, RepairOutcome, RepairState};
use aif_migrate::types::{SemanticCheck, StaticCheck, VerificationResult};

#[test]
fn build_repair_context_includes_failures() {
    let verification = VerificationResult {
        static_checks: vec![
            StaticCheck {
                name: "no jest".to_string(),
                passed: false,
                detail: "Found jest.mock on line 5".to_string(),
            },
            StaticCheck {
                name: "has vitest".to_string(),
                passed: true,
                detail: "vitest import found".to_string(),
            },
        ],
        semantic_checks: vec![SemanticCheck {
            criterion: "Behavior preserved".to_string(),
            passed: false,
            reasoning: "Timer mocking semantics differ".to_string(),
            confidence: 0.6,
        }],
        passed: false,
    };
    let fallback =
        Some("If timer mocking fails, preserve original and flag for review.".to_string());
    let context = build_repair_context(&verification, fallback.as_deref());
    assert!(context.contains("no jest"));
    assert!(context.contains("jest.mock on line 5"));
    assert!(context.contains("Timer mocking semantics differ"));
    assert!(context.contains("preserve original"));
    assert!(!context.contains("has vitest"));
}

#[test]
fn repair_state_tracks_iterations() {
    let mut state = RepairState::new(3);
    assert_eq!(state.iteration(), 0);
    assert!(state.can_retry());

    state.record_attempt(false);
    assert_eq!(state.iteration(), 1);
    assert!(state.can_retry());

    state.record_attempt(false);
    state.record_attempt(false);
    assert_eq!(state.iteration(), 3);
    assert!(!state.can_retry());
}

#[test]
fn repair_state_stops_on_success() {
    let mut state = RepairState::new(3);
    state.record_attempt(true);
    assert_eq!(state.outcome(), RepairOutcome::Fixed);
}

#[test]
fn repair_state_exhausted_after_max() {
    let mut state = RepairState::new(2);
    state.record_attempt(false);
    state.record_attempt(false);
    assert_eq!(state.outcome(), RepairOutcome::Exhausted);
}
