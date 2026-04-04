use crate::types::{
    Conflict, ConflictSeverity, ConflictType, Directive, DirectiveType, Polarity, Strength,
};
use std::collections::HashSet;

/// Compute Jaccard similarity between two keyword sets.
fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let set_a: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Get the shared keywords between two directives.
fn shared_keywords(a: &[String], b: &[String]) -> Vec<String> {
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    a.iter()
        .filter(|kw| set_b.contains(kw.as_str()))
        .cloned()
        .collect()
}

/// Order-related words for detecting ordering conflicts.
const ORDER_BEFORE: &[&str] = &["before", "first", "prior", "prerequisite", "initially", "start with"];
const ORDER_AFTER: &[&str] = &["after", "then", "subsequently", "following", "next", "finally"];

/// Check if text contains ordering language and return the implied position.
fn has_order_language(text: &str) -> (bool, bool) {
    let lower = text.to_lowercase();
    let has_before = ORDER_BEFORE.iter().any(|w| lower.contains(w));
    let has_after = ORDER_AFTER.iter().any(|w| lower.contains(w));
    (has_before, has_after)
}

/// Attempt to detect a conflict between two directives.
///
/// Returns `None` if no conflict is found (different domains or compatible).
pub fn detect_conflict(d1: &Directive, d2: &Directive) -> Option<Conflict> {
    // Skip if from the same skill
    if d1.source_skill == d2.source_skill {
        return None;
    }

    let similarity = jaccard_similarity(&d1.keywords, &d2.keywords);

    // Below 0.2 = different domains, skip
    if similarity < 0.2 {
        return None;
    }

    let shared = shared_keywords(&d1.keywords, &d2.keywords);

    // Check 1: RedFlag vs Step — a red_flag in one skill contradicts a step in another
    // (Checked first because red_flag+step is more specific than generic polarity conflict)
    if similarity >= 0.3 {
        let rf_vs_step = (d1.block_type == DirectiveType::RedFlag
            && d2.block_type == DirectiveType::Step)
            || (d1.block_type == DirectiveType::Step
                && d2.block_type == DirectiveType::RedFlag);

        if rf_vs_step {
            // A red_flag is inherently negative — if a step prescribes what a red_flag warns against
            let severity = compute_severity(d1, d2, ConflictType::ConstraintIncompatible);
            return Some(Conflict {
                conflict_type: ConflictType::ConstraintIncompatible,
                severity,
                directive_a: d1.clone(),
                directive_b: d2.clone(),
                explanation: format!(
                    "Constraint incompatibility: a @red_flag in '{}' warns against what a @step in '{}' prescribes [{}]",
                    if d1.block_type == DirectiveType::RedFlag { &d1.source_skill } else { &d2.source_skill },
                    if d1.block_type == DirectiveType::Step { &d1.source_skill } else { &d2.source_skill },
                    shared.join(", "),
                ),
                shared_keywords: shared,
            });
        }
    }

    // Check 2: Direct contradiction — same topic, opposite polarity
    if d1.polarity != d2.polarity && similarity >= 0.3 {
        let severity = compute_severity(d1, d2, ConflictType::DirectContradiction);
        return Some(Conflict {
            conflict_type: ConflictType::DirectContradiction,
            severity,
            directive_a: d1.clone(),
            directive_b: d2.clone(),
            explanation: format!(
                "Direct contradiction: '{}' ({}) says {} while '{}' ({}) says {} on the same topic [{}]",
                d1.source_skill,
                format_block_type(d1.block_type),
                if d1.polarity == Polarity::Positive { "DO" } else { "DON'T" },
                d2.source_skill,
                format_block_type(d2.block_type),
                if d2.polarity == Polarity::Positive { "DO" } else { "DON'T" },
                shared.join(", "),
            ),
            shared_keywords: shared,
        });
    }

    // Check 3: Order contradiction — both discuss ordering but imply different sequences
    if similarity >= 0.25 {
        let (d1_before, d1_after) = has_order_language(&d1.text);
        let (d2_before, d2_after) = has_order_language(&d2.text);

        if (d1_before && d2_after) || (d1_after && d2_before) {
            let severity = compute_severity(d1, d2, ConflictType::OrderContradiction);
            return Some(Conflict {
                conflict_type: ConflictType::OrderContradiction,
                severity,
                directive_a: d1.clone(),
                directive_b: d2.clone(),
                explanation: format!(
                    "Order contradiction: '{}' and '{}' prescribe incompatible ordering for [{}]",
                    d1.source_skill, d2.source_skill, shared.join(", "),
                ),
                shared_keywords: shared,
            });
        }
    }

    // Check 4: Precedence ambiguity — both are steps with the same order on overlapping topics
    if let (DirectiveType::Step, DirectiveType::Step, Some(order_a), Some(order_b)) =
        (d1.block_type, d2.block_type, d1.order, d2.order)
    {
        if order_a == order_b && similarity >= 0.3 {
            let severity = compute_severity(d1, d2, ConflictType::PrecedenceAmbiguity);
            return Some(Conflict {
                conflict_type: ConflictType::PrecedenceAmbiguity,
                severity,
                directive_a: d1.clone(),
                directive_b: d2.clone(),
                explanation: format!(
                    "Precedence ambiguity: '{}' and '{}' both define step order={} on overlapping topic [{}]",
                    d1.source_skill,
                    d2.source_skill,
                    order_a,
                    shared.join(", "),
                ),
                shared_keywords: shared,
            });
        }
    }

    None
}

/// Compute severity based on strength of the two directives and conflict type.
fn compute_severity(d1: &Directive, d2: &Directive, conflict_type: ConflictType) -> ConflictSeverity {
    match conflict_type {
        ConflictType::DirectContradiction => {
            if d1.strength == Strength::Must && d2.strength == Strength::Must {
                ConflictSeverity::Critical
            } else if d1.strength == Strength::Must || d2.strength == Strength::Must {
                ConflictSeverity::High
            } else if d1.strength == Strength::Should || d2.strength == Strength::Should {
                ConflictSeverity::Medium
            } else {
                ConflictSeverity::Low
            }
        }
        ConflictType::OrderContradiction => {
            if d1.strength == Strength::Must && d2.strength == Strength::Must {
                ConflictSeverity::High
            } else if d1.strength == Strength::Must || d2.strength == Strength::Must {
                ConflictSeverity::Medium
            } else {
                ConflictSeverity::Low
            }
        }
        ConflictType::ConstraintIncompatible => {
            if d1.strength == Strength::Must || d2.strength == Strength::Must {
                ConflictSeverity::High
            } else {
                ConflictSeverity::Medium
            }
        }
        ConflictType::PrecedenceAmbiguity => {
            if d1.strength == Strength::Must && d2.strength == Strength::Must {
                ConflictSeverity::Medium
            } else {
                ConflictSeverity::Low
            }
        }
    }
}

fn format_block_type(dt: DirectiveType) -> &'static str {
    match dt {
        DirectiveType::Step => "@step",
        DirectiveType::Precondition => "@precondition",
        DirectiveType::RedFlag => "@red_flag",
        DirectiveType::Verify => "@verify",
        DirectiveType::OutputContract => "@output_contract",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Directive;

    fn make_directive(
        skill: &str,
        block_type: DirectiveType,
        text: &str,
        keywords: Vec<&str>,
        polarity: Polarity,
        strength: Strength,
        order: Option<u32>,
    ) -> Directive {
        Directive {
            source_skill: skill.into(),
            block_type,
            order,
            text: text.into(),
            keywords: keywords.into_iter().map(String::from).collect(),
            polarity,
            strength,
        }
    }

    #[test]
    fn test_direct_contradiction() {
        let d1 = make_directive(
            "tdd-strict",
            DirectiveType::Step,
            "Always write tests before writing implementation code",
            vec!["write", "tests", "implementation", "code"],
            Polarity::Positive,
            Strength::Must,
            Some(1),
        );
        let d2 = make_directive(
            "rapid-prototype",
            DirectiveType::Step,
            "Don't write tests during initial prototyping of code",
            vec!["write", "tests", "prototyping", "code"],
            Polarity::Negative,
            Strength::Must,
            Some(1),
        );
        let conflict = detect_conflict(&d1, &d2);
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.conflict_type, ConflictType::DirectContradiction);
        assert_eq!(c.severity, ConflictSeverity::Critical);
    }

    #[test]
    fn test_no_conflict_different_domains() {
        let d1 = make_directive(
            "tdd",
            DirectiveType::Step,
            "Write unit tests for all functions",
            vec!["write", "unit", "tests", "functions"],
            Polarity::Positive,
            Strength::Must,
            Some(1),
        );
        let d2 = make_directive(
            "security",
            DirectiveType::Step,
            "Scan dependencies for vulnerabilities",
            vec!["scan", "dependencies", "vulnerabilities"],
            Polarity::Positive,
            Strength::Must,
            Some(1),
        );
        let conflict = detect_conflict(&d1, &d2);
        assert!(conflict.is_none());
    }

    #[test]
    fn test_no_conflict_same_skill() {
        let d1 = make_directive(
            "tdd",
            DirectiveType::Step,
            "Write tests first",
            vec!["write", "tests"],
            Polarity::Positive,
            Strength::Must,
            Some(1),
        );
        let d2 = make_directive(
            "tdd",
            DirectiveType::Step,
            "Don't write tests last",
            vec!["write", "tests"],
            Polarity::Negative,
            Strength::Must,
            Some(2),
        );
        assert!(detect_conflict(&d1, &d2).is_none());
    }

    #[test]
    fn test_order_contradiction() {
        let d1 = make_directive(
            "tdd",
            DirectiveType::Step,
            "Write tests before writing code implementation",
            vec!["write", "tests", "code", "implementation"],
            Polarity::Positive,
            Strength::Must,
            Some(1),
        );
        let d2 = make_directive(
            "prototype",
            DirectiveType::Step,
            "Write code implementation, then write tests after",
            vec!["write", "tests", "code", "implementation"],
            Polarity::Positive,
            Strength::Must,
            Some(1),
        );
        let conflict = detect_conflict(&d1, &d2);
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.conflict_type, ConflictType::OrderContradiction);
    }

    #[test]
    fn test_constraint_incompatible_redflag_vs_step() {
        let d1 = make_directive(
            "security",
            DirectiveType::RedFlag,
            "Deploying code without security review is dangerous",
            vec!["deploy", "code", "security", "review"],
            Polarity::Negative,
            Strength::Must,
            None,
        );
        let d2 = make_directive(
            "rapid",
            DirectiveType::Step,
            "Deploy code directly to production for rapid review",
            vec!["deploy", "code", "production", "review"],
            Polarity::Positive,
            Strength::Should,
            Some(3),
        );
        let conflict = detect_conflict(&d1, &d2);
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.conflict_type, ConflictType::ConstraintIncompatible);
    }

    #[test]
    fn test_severity_must_vs_should() {
        let d1 = make_directive(
            "strict",
            DirectiveType::Step,
            "Always write comprehensive tests for code",
            vec!["write", "comprehensive", "tests", "code"],
            Polarity::Positive,
            Strength::Must,
            Some(1),
        );
        let d2 = make_directive(
            "flexible",
            DirectiveType::Step,
            "Avoid writing tests for prototype code",
            vec!["writing", "tests", "prototype", "code"],
            Polarity::Negative,
            Strength::Should,
            Some(1),
        );
        let conflict = detect_conflict(&d1, &d2);
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.severity, ConflictSeverity::High); // Must vs Should = High
    }

    #[test]
    fn test_jaccard_similarity() {
        let a = vec!["write".into(), "tests".into(), "code".into()];
        let b = vec!["write".into(), "tests".into(), "deploy".into()];
        let sim = jaccard_similarity(&a, &b);
        // intersection = {write, tests} = 2, union = {write, tests, code, deploy} = 4
        assert!((sim - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_jaccard_empty() {
        let a: Vec<String> = vec![];
        let b = vec!["test".into()];
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }
}
