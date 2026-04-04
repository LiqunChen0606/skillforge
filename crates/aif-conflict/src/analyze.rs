use aif_core::ast::Document;

use crate::detect::detect_conflict;
use crate::extract::extract_directives;
use crate::types::ConflictReport;

/// Analyze multiple skill documents for cross-skill conflicts.
///
/// Extracts directives from all documents, then performs pairwise comparison
/// of directives from *different* skills.
pub fn analyze_skills(docs: &[&Document]) -> ConflictReport {
    let mut all_directives = Vec::new();
    for doc in docs {
        let directives = extract_directives(doc);
        all_directives.extend(directives);
    }

    let total_directives = all_directives.len();
    let skills: std::collections::HashSet<&str> = all_directives
        .iter()
        .map(|d| d.source_skill.as_str())
        .collect();
    let skills_count = skills.len();

    let mut conflicts = Vec::new();

    // Pairwise comparison — only between directives from different skills
    for i in 0..all_directives.len() {
        for j in (i + 1)..all_directives.len() {
            if all_directives[i].source_skill == all_directives[j].source_skill {
                continue;
            }
            if let Some(conflict) = detect_conflict(&all_directives[i], &all_directives[j]) {
                conflicts.push(conflict);
            }
        }
    }

    ConflictReport {
        conflicts,
        skills_analyzed: skills_count,
        directives_extracted: total_directives,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_conflicting_skills() {
        let tdd_source = r#"
@skill[name="tdd-strict", version="1.0"]
  @step[order=1]
    Always write tests before writing implementation code.

  @red_flag
    Never skip writing tests for any code changes.
@/skill
"#;
        let rapid_source = r#"
@skill[name="rapid-prototype", version="1.0"]
  @step[order=1]
    Don't write tests during initial code prototyping phase.

  @step[order=2]
    Write implementation code first, optimize for speed.
@/skill
"#;
        let tdd_doc = aif_parser::parse(tdd_source).unwrap();
        let rapid_doc = aif_parser::parse(rapid_source).unwrap();

        let report = analyze_skills(&[&tdd_doc, &rapid_doc]);

        assert_eq!(report.skills_analyzed, 2);
        assert!(report.directives_extracted >= 3);
        assert!(
            !report.conflicts.is_empty(),
            "Expected conflicts between TDD and rapid prototype skills"
        );
    }

    #[test]
    fn test_analyze_non_conflicting_skills() {
        let code_review_source = r#"
@skill[name="code-review", version="1.0"]
  @step[order=1]
    Review all pull requests for correctness and clarity.

  @verify
    Every pull request should have at least one approval.
@/skill
"#;
        let security_source = r#"
@skill[name="security-review", version="1.0"]
  @step[order=1]
    Scan dependencies for known vulnerabilities.

  @verify
    No critical vulnerabilities should remain unaddressed.
@/skill
"#;
        let cr_doc = aif_parser::parse(code_review_source).unwrap();
        let sec_doc = aif_parser::parse(security_source).unwrap();

        let report = analyze_skills(&[&cr_doc, &sec_doc]);

        assert_eq!(report.skills_analyzed, 2);
        assert!(
            report.conflicts.is_empty(),
            "Expected no conflicts between code review and security skills, but found: {:?}",
            report.conflicts.iter().map(|c| &c.explanation).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_analyze_three_skills_selective_conflicts() {
        let tdd_source = r#"
@skill[name="tdd-strict", version="1.0"]
  @step[order=1]
    Always write tests before writing implementation code.
@/skill
"#;
        let rapid_source = r#"
@skill[name="rapid-prototype", version="1.0"]
  @step[order=1]
    Don't write tests during initial code prototyping phase.
@/skill
"#;
        let security_source = r#"
@skill[name="security-audit", version="1.0"]
  @step[order=1]
    Scan all dependencies for known vulnerabilities regularly.
@/skill
"#;
        let tdd_doc = aif_parser::parse(tdd_source).unwrap();
        let rapid_doc = aif_parser::parse(rapid_source).unwrap();
        let sec_doc = aif_parser::parse(security_source).unwrap();

        let report = analyze_skills(&[&tdd_doc, &rapid_doc, &sec_doc]);

        assert_eq!(report.skills_analyzed, 3);

        // Should have conflicts between tdd and rapid, but not with security
        let conflict_skills: Vec<(&str, &str)> = report
            .conflicts
            .iter()
            .map(|c| (c.directive_a.source_skill.as_str(), c.directive_b.source_skill.as_str()))
            .collect();

        // All conflicts should involve tdd-strict and rapid-prototype
        for (a, b) in &conflict_skills {
            let involves_security = *a == "security-audit" || *b == "security-audit";
            assert!(
                !involves_security,
                "Security skill should not conflict with others"
            );
        }
    }
}
