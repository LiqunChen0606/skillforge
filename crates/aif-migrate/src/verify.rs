use crate::types::StaticCheck;
use regex::Regex;

#[derive(Debug, Clone)]
pub enum StaticCheckSpec {
    PatternAbsence { name: String, pattern: String },
    PatternPresence { name: String, pattern: String },
}

pub fn run_static_checks(content: &str, specs: &[StaticCheckSpec]) -> Vec<StaticCheck> {
    specs.iter().map(|spec| {
        match spec {
            StaticCheckSpec::PatternAbsence { name, pattern } => {
                match Regex::new(pattern) {
                    Ok(re) => {
                        let found: Vec<&str> = re.find_iter(content).map(|m| m.as_str()).collect();
                        StaticCheck {
                            name: name.clone(),
                            passed: found.is_empty(),
                            detail: if found.is_empty() {
                                format!("Pattern '{}' not found (good)", pattern)
                            } else {
                                format!("Found forbidden pattern '{}': {}", pattern, found.join(", "))
                            },
                        }
                    }
                    Err(e) => StaticCheck {
                        name: name.clone(),
                        passed: false,
                        detail: format!("Invalid regex pattern '{}': {}", pattern, e),
                    },
                }
            }
            StaticCheckSpec::PatternPresence { name, pattern } => {
                match Regex::new(pattern) {
                    Ok(re) => {
                        let found = re.is_match(content);
                        StaticCheck {
                            name: name.clone(),
                            passed: found,
                            detail: if found {
                                format!("Required pattern '{}' found", pattern)
                            } else {
                                format!("Required pattern '{}' not found", pattern)
                            },
                        }
                    }
                    Err(e) => StaticCheck {
                        name: name.clone(),
                        passed: false,
                        detail: format!("Invalid regex pattern '{}': {}", pattern, e),
                    },
                }
            }
        }
    }).collect()
}

/// Detect whether a verify line describes absence (negation) of a pattern.
///
/// Uses word-boundary-aware regex to avoid false positives from substrings
/// like "note" or "notify" matching "no".
fn is_negation_line(lower: &str) -> bool {
    // Lazy-static equivalent: compile once per call is fine for small verify blocks.
    // Matches: "no ", "not ", "must not", "should not", "cannot", "never", "removed",
    // "forbidden", "absent", "without", "eliminated", "no remaining".
    let negation_re = Regex::new(
        r"\b(no\s|not\s|must\s+not|should\s+not|cannot|never\b|removed\b|forbidden\b|absent\b|without\b|eliminated\b)"
    ).unwrap();
    negation_re.is_match(lower)
}

pub fn extract_static_specs(verify_text: &str) -> Vec<StaticCheckSpec> {
    let mut specs = Vec::new();
    let backtick_re = Regex::new(r"`([^`]+)`").unwrap();

    for line in verify_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();
        let patterns: Vec<String> = backtick_re.captures_iter(trimmed)
            .map(|c| c[1].to_string())
            .collect();

        for pattern in patterns {
            if is_negation_line(&lower) {
                specs.push(StaticCheckSpec::PatternAbsence {
                    name: trimmed.to_string(),
                    pattern,
                });
            } else {
                specs.push(StaticCheckSpec::PatternPresence {
                    name: trimmed.to_string(),
                    pattern,
                });
            }
        }
    }

    specs
}
