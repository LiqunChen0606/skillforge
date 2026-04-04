use serde::{Deserialize, Serialize};

/// Whether a directive tells the user to do something or avoid something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Polarity {
    /// Do this (positive instruction).
    Positive,
    /// Don't do this / avoid this (negative instruction).
    Negative,
}

/// How strongly the directive is stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Strength {
    /// "may", "can", "could" — optional guidance.
    May,
    /// "should", "recommend" — strong guidance.
    Should,
    /// "must", "always", "never", "required" — mandatory.
    Must,
}

/// Which skill block type the directive came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectiveType {
    Step,
    Precondition,
    RedFlag,
    Verify,
    OutputContract,
}

/// A single extracted directive from a skill block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directive {
    /// Name of the source skill (from @skill[name=...]).
    pub source_skill: String,
    /// The block type this directive was extracted from.
    pub block_type: DirectiveType,
    /// Order attribute if present (for steps).
    pub order: Option<u32>,
    /// The plain-text content of the block.
    pub text: String,
    /// Extracted keywords (content words > 3 chars, no stopwords).
    pub keywords: Vec<String>,
    /// Whether this is a positive or negative instruction.
    pub polarity: Polarity,
    /// How strongly stated.
    pub strength: Strength,
}

/// The type of conflict detected between two directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// One directive says "do X", another says "don't do X" on the same topic.
    DirectContradiction,
    /// Two directives prescribe incompatible ordering ("do A before B" vs "do B before A").
    OrderContradiction,
    /// Multiple skills claim authority over the same step without clear precedence.
    PrecedenceAmbiguity,
    /// Two constraints on the same topic are incompatible (e.g., "always use X" vs "never use X").
    ConstraintIncompatible,
}

/// How severe the conflict is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// A detected conflict between two directives from different skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// The type of conflict.
    pub conflict_type: ConflictType,
    /// Severity of the conflict.
    pub severity: ConflictSeverity,
    /// First directive involved.
    pub directive_a: Directive,
    /// Second directive involved.
    pub directive_b: Directive,
    /// Human-readable explanation of the conflict.
    pub explanation: String,
    /// Overlapping keywords that triggered the match.
    pub shared_keywords: Vec<String>,
}

/// Result of analyzing multiple skills for conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    /// List of all detected conflicts.
    pub conflicts: Vec<Conflict>,
    /// Total number of skills analyzed.
    pub skills_analyzed: usize,
    /// Total number of directives extracted.
    pub directives_extracted: usize,
}

impl ConflictReport {
    /// Returns true if any critical conflicts were found.
    pub fn has_critical(&self) -> bool {
        self.conflicts
            .iter()
            .any(|c| c.severity == ConflictSeverity::Critical)
    }

    /// Returns conflicts filtered by minimum severity.
    pub fn by_severity(&self, min: ConflictSeverity) -> Vec<&Conflict> {
        self.conflicts.iter().filter(|c| c.severity >= min).collect()
    }

    /// Returns the number of conflicts at each severity level.
    pub fn severity_counts(&self) -> (usize, usize, usize, usize) {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;
        for c in &self.conflicts {
            match c.severity {
                ConflictSeverity::Critical => critical += 1,
                ConflictSeverity::High => high += 1,
                ConflictSeverity::Medium => medium += 1,
                ConflictSeverity::Low => low += 1,
            }
        }
        (critical, high, medium, low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polarity_equality() {
        assert_eq!(Polarity::Positive, Polarity::Positive);
        assert_ne!(Polarity::Positive, Polarity::Negative);
    }

    #[test]
    fn strength_ordering() {
        assert!(Strength::Must > Strength::Should);
        assert!(Strength::Should > Strength::May);
    }

    #[test]
    fn conflict_severity_ordering() {
        assert!(ConflictSeverity::Critical > ConflictSeverity::High);
        assert!(ConflictSeverity::High > ConflictSeverity::Medium);
        assert!(ConflictSeverity::Medium > ConflictSeverity::Low);
    }

    #[test]
    fn conflict_report_has_critical() {
        let report = ConflictReport {
            conflicts: vec![Conflict {
                conflict_type: ConflictType::DirectContradiction,
                severity: ConflictSeverity::Critical,
                directive_a: make_test_directive("skill-a", Polarity::Positive),
                directive_b: make_test_directive("skill-b", Polarity::Negative),
                explanation: "test".into(),
                shared_keywords: vec!["test".into()],
            }],
            skills_analyzed: 2,
            directives_extracted: 2,
        };
        assert!(report.has_critical());
    }

    #[test]
    fn conflict_report_no_critical() {
        let report = ConflictReport {
            conflicts: vec![Conflict {
                conflict_type: ConflictType::PrecedenceAmbiguity,
                severity: ConflictSeverity::Low,
                directive_a: make_test_directive("skill-a", Polarity::Positive),
                directive_b: make_test_directive("skill-b", Polarity::Positive),
                explanation: "test".into(),
                shared_keywords: vec!["test".into()],
            }],
            skills_analyzed: 2,
            directives_extracted: 2,
        };
        assert!(!report.has_critical());
    }

    #[test]
    fn conflict_report_severity_counts() {
        let report = ConflictReport {
            conflicts: vec![
                Conflict {
                    conflict_type: ConflictType::DirectContradiction,
                    severity: ConflictSeverity::Critical,
                    directive_a: make_test_directive("a", Polarity::Positive),
                    directive_b: make_test_directive("b", Polarity::Negative),
                    explanation: "crit".into(),
                    shared_keywords: vec![],
                },
                Conflict {
                    conflict_type: ConflictType::OrderContradiction,
                    severity: ConflictSeverity::High,
                    directive_a: make_test_directive("a", Polarity::Positive),
                    directive_b: make_test_directive("b", Polarity::Positive),
                    explanation: "high".into(),
                    shared_keywords: vec![],
                },
            ],
            skills_analyzed: 2,
            directives_extracted: 4,
        };
        assert_eq!(report.severity_counts(), (1, 1, 0, 0));
    }

    #[test]
    fn conflict_report_by_severity() {
        let report = ConflictReport {
            conflicts: vec![
                Conflict {
                    conflict_type: ConflictType::DirectContradiction,
                    severity: ConflictSeverity::Critical,
                    directive_a: make_test_directive("a", Polarity::Positive),
                    directive_b: make_test_directive("b", Polarity::Negative),
                    explanation: "crit".into(),
                    shared_keywords: vec![],
                },
                Conflict {
                    conflict_type: ConflictType::PrecedenceAmbiguity,
                    severity: ConflictSeverity::Low,
                    directive_a: make_test_directive("a", Polarity::Positive),
                    directive_b: make_test_directive("b", Polarity::Positive),
                    explanation: "low".into(),
                    shared_keywords: vec![],
                },
            ],
            skills_analyzed: 2,
            directives_extracted: 4,
        };
        let high_plus = report.by_severity(ConflictSeverity::High);
        assert_eq!(high_plus.len(), 1);
        assert_eq!(high_plus[0].severity, ConflictSeverity::Critical);
    }

    #[test]
    fn directive_serializes_to_json() {
        let d = make_test_directive("test-skill", Polarity::Positive);
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("test-skill"));
        assert!(json.contains("Positive"));
    }

    fn make_test_directive(skill: &str, polarity: Polarity) -> Directive {
        Directive {
            source_skill: skill.into(),
            block_type: DirectiveType::Step,
            order: Some(1),
            text: "Write tests first".into(),
            keywords: vec!["write".into(), "tests".into()],
            polarity,
            strength: Strength::Must,
        }
    }
}
