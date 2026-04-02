use aif_migrate::verify::{run_static_checks, extract_static_specs, StaticCheckSpec};

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
    let verify_text = r#"
No remaining `jest.` calls in test files.
All test files import from 'vitest'.
"#;
    let specs = extract_static_specs(verify_text);
    assert_eq!(specs.len(), 1, "Should extract exactly one check spec (only backtick patterns)");
    assert!(matches!(&specs[0], StaticCheckSpec::PatternAbsence { pattern, .. } if pattern == "jest."));
}

// --- Critical fix: PatternPresence now uses regex, not literal contains ---

#[test]
fn pattern_presence_uses_regex_matching() {
    let content = "const x = fooBar();";
    let spec = StaticCheckSpec::PatternPresence {
        name: "has foo".to_string(),
        pattern: r"foo\w+".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(result[0].passed, "PatternPresence should support regex patterns");
}

// --- Critical fix: invalid regex reports error instead of silent fallback ---

#[test]
fn invalid_regex_reports_error_absence() {
    let content = "anything";
    let spec = StaticCheckSpec::PatternAbsence {
        name: "bad pattern".to_string(),
        pattern: "[invalid".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(!result[0].passed, "Invalid regex should fail the check");
    assert!(result[0].detail.contains("Invalid regex"), "Should report regex error: {}", result[0].detail);
}

#[test]
fn invalid_regex_reports_error_presence() {
    let content = "anything";
    let spec = StaticCheckSpec::PatternPresence {
        name: "bad pattern".to_string(),
        pattern: "[invalid".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(!result[0].passed, "Invalid regex should fail the check");
    assert!(result[0].detail.contains("Invalid regex"), "Should report regex error: {}", result[0].detail);
}

// --- Important fix: negation heuristic uses word boundaries ---

#[test]
fn negation_heuristic_no_false_positives() {
    // "note" and "notify" should NOT trigger absence classification
    let specs = extract_static_specs("Note: all files should use `vitest`.");
    assert!(
        matches!(&specs[0], StaticCheckSpec::PatternPresence { .. }),
        "'Note' should not be classified as negation"
    );

    let specs = extract_static_specs("Notify users about `migration` changes.");
    assert!(
        matches!(&specs[0], StaticCheckSpec::PatternPresence { .. }),
        "'Notify' should not be classified as negation"
    );
}

#[test]
fn negation_heuristic_ambiguous_lines() {
    // Lines containing both affirmative and negation signals — negation should win
    let specs = extract_static_specs("Note that no `jest` imports remain.");
    assert!(
        matches!(&specs[0], StaticCheckSpec::PatternAbsence { .. }),
        "Line with 'Note' AND 'no' should classify as absence because negation is present"
    );

    let specs = extract_static_specs("Ensure `vitest` is used and not `jest`.");
    assert_eq!(specs.len(), 2, "Should extract two patterns");
    // Both patterns on the same line — the line contains "not", so both are absence
    assert!(
        matches!(&specs[0], StaticCheckSpec::PatternAbsence { .. }),
        "Line with 'not' should classify both patterns as absence"
    );
}

#[test]
fn negation_heuristic_extended_keywords() {
    // Various negation forms should all classify as absence
    let cases = vec![
        "Must not contain `jest.` calls",
        "Should not use `require()` syntax",
        "Never use `var` declarations",
        "All `commonjs` imports must be removed",
        "The `old_api` calls are forbidden",
    ];
    for text in cases {
        let specs = extract_static_specs(text);
        assert!(
            matches!(&specs[0], StaticCheckSpec::PatternAbsence { .. }),
            "Expected absence for: {}",
            text
        );
    }
}
