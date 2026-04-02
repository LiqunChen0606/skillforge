use aif_migrate::apply::{build_migration_prompt, build_semantic_verify_prompt, parse_migration_response, parse_semantic_response};

#[test]
fn build_migration_prompt_includes_skill_steps_and_source() {
    let steps = vec![
        "Replace jest.fn() with vi.fn()".to_string(),
        "Update imports to vitest".to_string(),
    ];
    let source = "import { jest } from '@jest/globals';\njest.fn();";
    let prompt = build_migration_prompt(&steps, source, None);
    assert!(prompt.contains("Replace jest.fn() with vi.fn()"));
    assert!(prompt.contains("Update imports to vitest"));
    assert!(prompt.contains("jest.fn()"));
    assert!(prompt.contains("import"));
}

#[test]
fn build_migration_prompt_includes_repair_context() {
    let steps = vec!["Migrate imports".to_string()];
    let source = "old code";
    let repair = Some("Previous attempt failed: missing vitest import".to_string());
    let prompt = build_migration_prompt(&steps, source, repair.as_deref());
    assert!(prompt.contains("Previous attempt failed"));
}

#[test]
fn parse_migration_response_extracts_code_block() {
    let response = r#"Here's the migrated code:

```
import { vi } from 'vitest';
vi.fn();
```

I replaced the jest imports with vitest."#;
    let code = parse_migration_response(response);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("import { vi } from 'vitest'"));
    assert!(code.contains("vi.fn()"));
}

#[test]
fn parse_migration_response_handles_no_code_block() {
    let response = "I can't migrate this code because it's too complex.";
    let code = parse_migration_response(response);
    assert!(code.is_none());
}

#[test]
fn build_semantic_verify_prompt_includes_criteria() {
    let original = "jest.fn()";
    let migrated = "vi.fn()";
    let criteria = vec![
        "No remaining jest calls".to_string(),
        "Vitest imports present".to_string(),
    ];
    let prompt = build_semantic_verify_prompt(original, migrated, &criteria);
    assert!(prompt.contains("No remaining jest calls"));
    assert!(prompt.contains("Vitest imports present"));
    assert!(prompt.contains("jest.fn()"));
    assert!(prompt.contains("vi.fn()"));
}

#[test]
fn parse_semantic_response_extracts_checks() {
    let response = r#"## Criterion 1: No remaining jest calls
**PASS** — confidence: 0.95
The migrated code contains no references to jest.

## Criterion 2: Vitest imports present
**PASS** — confidence: 0.90
The code correctly imports from vitest.
"#;
    let criteria = vec![
        "No remaining jest calls".to_string(),
        "Vitest imports present".to_string(),
    ];
    let checks = parse_semantic_response(response, &criteria);
    assert_eq!(checks.len(), 2);
    assert!(checks[0].passed);
    assert!(checks[1].passed);
}

#[test]
fn parse_semantic_response_handles_failures() {
    let response = r#"## Criterion 1: No remaining jest calls
**FAIL** — confidence: 0.85
Found `jest.mock` on line 5.
"#;
    let criteria = vec!["No remaining jest calls".to_string()];
    let checks = parse_semantic_response(response, &criteria);
    assert_eq!(checks.len(), 1);
    assert!(!checks[0].passed);
    assert!(checks[0].reasoning.contains("jest.mock"));
}
