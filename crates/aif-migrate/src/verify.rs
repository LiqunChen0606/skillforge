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
                let re = Regex::new(pattern).unwrap_or_else(|_| Regex::new(&regex::escape(pattern)).unwrap());
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
            StaticCheckSpec::PatternPresence { name, pattern } => {
                let found = content.contains(pattern);
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
        }
    }).collect()
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
            if lower.contains("no ") || lower.contains("not ") || lower.contains("no remaining") {
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
