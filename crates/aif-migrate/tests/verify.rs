use aif_migrate::verify::{run_static_checks, StaticCheckSpec};
use aif_migrate::types::StaticCheck;

#[test]
fn pattern_absence_check_passes_when_pattern_missing() {
    let content = "import { describe } from 'vitest';\nvi.fn();";
    let spec = StaticCheckSpec::PatternAbsence {
        name: "no jest calls".to_string(),
        pattern: "jest\\.".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert_eq!(result.len(), 1);
    assert!(result[0].passed);
}

#[test]
fn pattern_absence_check_fails_when_pattern_present() {
    let content = "import jest from 'jest';\njest.fn();";
    let spec = StaticCheckSpec::PatternAbsence {
        name: "no jest calls".to_string(),
        pattern: "jest\\.".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(!result[0].passed);
    assert!(result[0].detail.contains("jest."));
}

#[test]
fn pattern_presence_check_passes_when_found() {
    let content = "import { describe } from 'vitest';";
    let spec = StaticCheckSpec::PatternPresence {
        name: "vitest imports".to_string(),
        pattern: "from 'vitest'".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(result[0].passed);
}

#[test]
fn pattern_presence_check_fails_when_missing() {
    let content = "import { describe } from 'jest';";
    let spec = StaticCheckSpec::PatternPresence {
        name: "vitest imports".to_string(),
        pattern: "from 'vitest'".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(!result[0].passed);
}

#[test]
fn multiple_checks_run_independently() {
    let content = "import { vi } from 'vitest';\nvi.fn();";
    let specs = vec![
        StaticCheckSpec::PatternPresence {
            name: "has vitest".to_string(),
            pattern: "vitest".to_string(),
        },
        StaticCheckSpec::PatternAbsence {
            name: "no jest".to_string(),
            pattern: "jest".to_string(),
        },
    ];
    let results = run_static_checks(content, &specs);
    assert_eq!(results.len(), 2);
    assert!(results[0].passed);
    assert!(results[1].passed);
}

#[test]
fn extract_check_specs_from_verify_text() {
    use aif_migrate::verify::extract_static_specs;

    let verify_text = r#"
No remaining `jest.` calls in test files.
All test files import from 'vitest'.
"#;
    let specs = extract_static_specs(verify_text);
    assert!(!specs.is_empty(), "Should extract at least one check spec");
}
