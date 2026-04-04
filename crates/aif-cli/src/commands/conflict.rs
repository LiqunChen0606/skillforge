use std::path::PathBuf;

use crate::util::{parse_aif, read_source, truncate_text};

pub fn handle_conflict(files: Vec<PathBuf>, format: String) {
    let docs: Vec<_> = files
        .iter()
        .map(|f| {
            let source = read_source(f);
            parse_aif(&source)
        })
        .collect();
    let doc_refs: Vec<&aif_core::ast::Document> = docs.iter().collect();
    let report = aif_conflict::analyze::analyze_skills(&doc_refs);

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("Skill Conflict Analysis");
        println!("{}", "=".repeat(60));
        println!(
            "Skills analyzed: {}  |  Directives extracted: {}",
            report.skills_analyzed, report.directives_extracted
        );
        println!();

        if report.conflicts.is_empty() {
            println!("No conflicts detected.");
        } else {
            let (critical, high, medium, low) = report.severity_counts();
            println!(
                "Conflicts found: {} (Critical: {}, High: {}, Medium: {}, Low: {})",
                report.conflicts.len(),
                critical,
                high,
                medium,
                low
            );
            println!();

            for (i, conflict) in report.conflicts.iter().enumerate() {
                let sev = match conflict.severity {
                    aif_conflict::types::ConflictSeverity::Critical => "CRITICAL",
                    aif_conflict::types::ConflictSeverity::High => "HIGH",
                    aif_conflict::types::ConflictSeverity::Medium => "MEDIUM",
                    aif_conflict::types::ConflictSeverity::Low => "LOW",
                };
                let ctype = match conflict.conflict_type {
                    aif_conflict::types::ConflictType::DirectContradiction => {
                        "Direct Contradiction"
                    }
                    aif_conflict::types::ConflictType::OrderContradiction => {
                        "Order Contradiction"
                    }
                    aif_conflict::types::ConflictType::PrecedenceAmbiguity => {
                        "Precedence Ambiguity"
                    }
                    aif_conflict::types::ConflictType::ConstraintIncompatible => {
                        "Constraint Incompatible"
                    }
                };
                println!("  {}. [{}] {}", i + 1, sev, ctype);
                println!("     {}", conflict.explanation);
                println!(
                    "     Skill A: {} ({:?})",
                    conflict.directive_a.source_skill, conflict.directive_a.block_type
                );
                println!(
                    "       \"{}\"",
                    truncate_text(&conflict.directive_a.text, 80)
                );
                println!(
                    "     Skill B: {} ({:?})",
                    conflict.directive_b.source_skill, conflict.directive_b.block_type
                );
                println!(
                    "       \"{}\"",
                    truncate_text(&conflict.directive_b.text, 80)
                );
                if !conflict.shared_keywords.is_empty() {
                    println!(
                        "     Shared keywords: {}",
                        conflict.shared_keywords.join(", ")
                    );
                }
                println!();
            }
        }

        println!("{}", "-".repeat(60));
        if report.has_critical() {
            println!("CRITICAL conflicts found — these skills should not be used together.");
            std::process::exit(1);
        } else if !report.conflicts.is_empty() {
            println!(
                "WARNING: {} conflict(s) found. Review before combining these skills.",
                report.conflicts.len()
            );
        } else {
            println!("PASS — no conflicts detected between the provided skills.");
        }
    }

    // Exit 1 if critical conflicts found (for both text and json)
    if report.has_critical() {
        std::process::exit(1);
    }
}
