use crate::types::{ChunkStatus, MigrationReport};
use aif_core::ast::*;
use aif_core::span::Span;
use std::collections::BTreeMap;

/// Generate an AIF Document from a MigrationReport.
pub fn generate_report_document(report: &MigrationReport) -> Document {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "title".to_string(),
        format!("Migration Report — {}", report.skill_name),
    );
    metadata.insert("author".to_string(), "aif-migrate".to_string());

    let mut blocks = vec![
        build_executive_summary(report),
        build_risk_assessment(report),
        build_verification_analysis(report),
        build_chunk_results(report),
    ];

    // 5. Failure analysis (only if there are failures)
    if let Some(failure_section) = build_failure_analysis(report) {
        blocks.push(failure_section);
    }

    // 6. Manual review section
    if !report.manual_review.is_empty() {
        let items: Vec<Block> = report
            .manual_review
            .iter()
            .map(|item| make_paragraph(&format!("- {}", item)))
            .collect();
        blocks.push(make_section("Manual Review Required", items));
    }

    // 7. Unresolved issues
    if !report.unresolved.is_empty() {
        let items: Vec<Block> = report
            .unresolved
            .iter()
            .map(|item| make_paragraph(&format!("- {}", item)))
            .collect();
        blocks.push(make_section("Unresolved Issues", items));
    }

    // 8. Recommendations
    blocks.push(build_recommendations(report));

    Document { metadata, blocks }
}

/// Executive summary with key metrics and confidence interpretation.
fn build_executive_summary(report: &MigrationReport) -> Block {
    let (success, partial, failed, skipped) = report.status_counts();
    let total = report.chunks.len();
    let avg_conf = report.average_confidence();
    let conf_label = MigrationReport::confidence_label(avg_conf);

    let summary = format!(
        "Migration skill \"{}\" processed {} chunks from {}.\n\
         Duration: {}s.\n\n\
         Results: {} succeeded, {} partial, {} failed, {} skipped.\n\
         Success rate: {:.0}%.\n\
         Overall confidence: {:.2} ({}).\n\
         Total repair iterations: {}.",
        report.skill_name,
        total,
        report.source_dir.display(),
        report.duration.as_secs(),
        success,
        partial,
        failed,
        skipped,
        report.success_rate() * 100.0,
        avg_conf,
        conf_label,
        report.total_repair_iterations(),
    );
    make_section("Executive Summary", vec![make_paragraph(&summary)])
}

/// Risk assessment section with confidence interpretation.
fn build_risk_assessment(report: &MigrationReport) -> Block {
    let risk = report.risk_level();
    let avg_conf = report.average_confidence();
    let conf_label = MigrationReport::confidence_label(avg_conf);

    let mut paragraphs = vec![];

    // Overall risk callout
    let risk_callout_type = match risk {
        "Low Risk" => CalloutType::Note,
        "Medium Risk" => CalloutType::Info,
        "High Risk" => CalloutType::Warning,
        _ => CalloutType::Tip, // Critical Risk — most severe
    };
    paragraphs.push(Block {
        kind: BlockKind::Callout {
            callout_type: risk_callout_type,
            attrs: Attrs::new(),
            content: vec![Inline::Text {
                text: format!(
                    "Overall Risk Level: {}\n\n\
                     This assessment is based on the combination of success rate ({:.0}%) \
                     and average confidence ({:.2}, {}).",
                    risk,
                    report.success_rate() * 100.0,
                    avg_conf,
                    conf_label,
                ),
            }],
        },
        span: Span::new(0, 0),
    });

    // Confidence interpretation
    let interpretation = match conf_label {
        "High" => "The migration engine has high confidence in the results. \
                   Static and semantic checks strongly support the correctness of the migrated code. \
                   Manual review can focus on edge cases and integration testing.",
        "Medium" => "The migration engine has moderate confidence. \
                    Some chunks may need closer review. Pay particular attention to chunks \
                    with confidence below 0.7 and any semantic check failures.",
        "Low" => "The migration engine has low confidence in the results. \
                 Many chunks may require manual intervention. Review all migrated code carefully \
                 before integrating, especially chunks that went through repair iterations.",
        _ => "The migration engine has very low confidence. \
              The migration may not be suitable for automated application. \
              Consider reviewing the migration skill for completeness, \
              or breaking the migration into smaller, more targeted phases.",
    };
    paragraphs.push(make_paragraph(interpretation));

    // Repair effort analysis
    let total_repairs = report.total_repair_iterations();
    if total_repairs > 0 {
        let repair_analysis = format!(
            "The migration required {} total repair iterations across all chunks. \
             {}",
            total_repairs,
            if total_repairs as usize > report.chunks.len() {
                "Multiple repair attempts per chunk suggest the migration skill's \
                 @verify and @step blocks may need refinement for this codebase."
            } else {
                "Repair iterations were within expected bounds, indicating the \
                 migration skill handled most edge cases effectively."
            }
        );
        paragraphs.push(make_paragraph(&repair_analysis));
    }

    make_section("Risk Assessment", paragraphs)
}

/// Verification analysis with static and semantic check breakdowns.
fn build_verification_analysis(report: &MigrationReport) -> Block {
    let mut children = vec![];

    // Static checks summary
    let total_static: usize = report.chunks.iter()
        .map(|c| c.verification.static_checks.len())
        .sum();
    let passed_static: usize = report.chunks.iter()
        .map(|c| c.verification.static_checks.iter().filter(|s| s.passed).count())
        .sum();
    let failed_static = report.failed_static_checks();

    if total_static > 0 {
        let static_summary = format!(
            "Static Checks: {}/{} passed ({:.0}%).",
            passed_static,
            total_static,
            if total_static > 0 { passed_static as f64 / total_static as f64 * 100.0 } else { 0.0 },
        );
        children.push(make_paragraph(&static_summary));

        if !failed_static.is_empty() {
            let mut detail = String::from("Failed static checks:\n");
            for (chunk_id, check_name) in &failed_static {
                detail.push_str(&format!("- Chunk \"{}\": check \"{}\" failed\n", chunk_id, check_name));
            }
            children.push(Block {
                kind: BlockKind::Callout {
                    callout_type: CalloutType::Warning,
                    attrs: Attrs::new(),
                    content: vec![Inline::Text { text: detail }],
                },
                span: Span::new(0, 0),
            });
        }
    }

    // Semantic checks summary
    let total_semantic: usize = report.chunks.iter()
        .map(|c| c.verification.semantic_checks.len())
        .sum();
    let passed_semantic: usize = report.chunks.iter()
        .map(|c| c.verification.semantic_checks.iter().filter(|s| s.passed).count())
        .sum();
    let failed_semantic = report.failed_semantic_checks();

    if total_semantic > 0 {
        let semantic_summary = format!(
            "Semantic Checks: {}/{} passed ({:.0}%).",
            passed_semantic,
            total_semantic,
            if total_semantic > 0 { passed_semantic as f64 / total_semantic as f64 * 100.0 } else { 0.0 },
        );
        children.push(make_paragraph(&semantic_summary));

        if !failed_semantic.is_empty() {
            let mut detail = String::from("Failed semantic checks:\n");
            for (chunk_id, criterion, reasoning) in &failed_semantic {
                detail.push_str(&format!(
                    "- Chunk \"{}\": criterion \"{}\" — {}\n",
                    chunk_id, criterion, reasoning
                ));
            }
            children.push(Block {
                kind: BlockKind::Callout {
                    callout_type: CalloutType::Warning,
                    attrs: Attrs::new(),
                    content: vec![Inline::Text { text: detail }],
                },
                span: Span::new(0, 0),
            });
        }
    }

    if total_static == 0 && total_semantic == 0 {
        children.push(make_paragraph("No verification checks were executed. \
            Ensure the migration skill's @verify block defines static patterns \
            and semantic criteria for automated validation."));
    }

    make_section("Verification Analysis", children)
}

/// Detailed per-chunk results with verification breakdowns.
fn build_chunk_results(report: &MigrationReport) -> Block {
    let mut chunk_blocks = Vec::new();
    for chunk in &report.chunks {
        let callout_type = match chunk.status {
            ChunkStatus::Success => CalloutType::Note,
            ChunkStatus::PartialSuccess => CalloutType::Warning,
            ChunkStatus::Failed => CalloutType::Warning,
            ChunkStatus::Skipped => CalloutType::Note,
        };
        let status_label = match chunk.status {
            ChunkStatus::Success => "Success",
            ChunkStatus::PartialSuccess => "Partial Success",
            ChunkStatus::Failed => "Failed",
            ChunkStatus::Skipped => "Skipped",
        };
        let conf_label = MigrationReport::confidence_label(chunk.confidence);
        let files_str = chunk
            .files
            .iter()
            .map(|f| f.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let mut text = format!(
            "Chunk: {}\nFiles: {}\nStatus: {}\nConfidence: {:.2} ({})\nRepair iterations: {}",
            chunk.chunk_id, files_str, status_label, chunk.confidence, conf_label,
            chunk.repair_iterations,
        );

        // Static check details
        if !chunk.verification.static_checks.is_empty() {
            text.push_str("\n\nStatic checks:");
            for sc in &chunk.verification.static_checks {
                let icon = if sc.passed { "PASS" } else { "FAIL" };
                text.push_str(&format!("\n  [{}] {} — {}", icon, sc.name, sc.detail));
            }
        }

        // Semantic check details
        if !chunk.verification.semantic_checks.is_empty() {
            text.push_str("\n\nSemantic checks:");
            for sc in &chunk.verification.semantic_checks {
                let icon = if sc.passed { "PASS" } else { "FAIL" };
                text.push_str(&format!(
                    "\n  [{}] {} (confidence: {:.2}) — {}",
                    icon, sc.criterion, sc.confidence, sc.reasoning
                ));
            }
        }

        for note in &chunk.notes {
            text.push_str(&format!("\n\nNote: {}", note));
        }

        chunk_blocks.push(Block {
            kind: BlockKind::Callout {
                callout_type,
                attrs: Attrs::new(),
                content: vec![Inline::Text { text }],
            },
            span: Span::new(0, 0),
        });
    }
    make_section("Results by Chunk", chunk_blocks)
}

/// Failure analysis section — only generated when there are failures.
fn build_failure_analysis(report: &MigrationReport) -> Option<Block> {
    let failed_chunks: Vec<_> = report.chunks.iter()
        .filter(|c| matches!(c.status, ChunkStatus::Failed | ChunkStatus::PartialSuccess))
        .collect();

    if failed_chunks.is_empty() {
        return None;
    }

    let mut children = vec![];

    // Pattern analysis: group failures by common check names
    let mut failure_patterns: BTreeMap<String, usize> = BTreeMap::new();
    for chunk in &failed_chunks {
        for sc in &chunk.verification.static_checks {
            if !sc.passed {
                *failure_patterns.entry(format!("Static: {}", sc.name)).or_insert(0) += 1;
            }
        }
        for sc in &chunk.verification.semantic_checks {
            if !sc.passed {
                *failure_patterns.entry(format!("Semantic: {}", sc.criterion)).or_insert(0) += 1;
            }
        }
    }

    if !failure_patterns.is_empty() {
        let mut pattern_text = String::from("Recurring failure patterns across chunks:\n");
        for (pattern, count) in &failure_patterns {
            pattern_text.push_str(&format!("- {} (occurred in {} chunks)\n", pattern, count));
        }
        pattern_text.push_str(
            "\nRecurring patterns suggest systematic issues in the migration skill's \
             handling of these specific code patterns. Consider adding dedicated @step \
             blocks or refining existing steps to address these patterns."
        );
        children.push(make_paragraph(&pattern_text));
    }

    // Repair effort analysis for failed chunks
    let exhausted: Vec<_> = failed_chunks.iter()
        .filter(|c| c.status == ChunkStatus::Failed && c.repair_iterations > 0)
        .collect();
    if !exhausted.is_empty() {
        let text = format!(
            "{} chunk(s) exhausted the repair loop without reaching a passing state. \
             These chunks may contain code patterns that the migration skill cannot \
             handle automatically. Consider:\n\
             - Reviewing the @fallback block for more specific guidance\n\
             - Adding pattern-specific @step blocks\n\
             - Increasing max_repair_iterations if failures are close to resolution\n\
             - Flagging these patterns for manual migration",
            exhausted.len()
        );
        children.push(Block {
            kind: BlockKind::Callout {
                callout_type: CalloutType::Warning,
                attrs: Attrs::new(),
                content: vec![Inline::Text { text }],
            },
            span: Span::new(0, 0),
        });
    }

    Some(make_section("Failure Analysis", children))
}

/// Actionable recommendations based on migration results.
fn build_recommendations(report: &MigrationReport) -> Block {
    let mut recommendations = vec![];
    let rate = report.success_rate();
    let avg_conf = report.average_confidence();

    // High-level recommendation
    if rate >= 0.95 && avg_conf >= 0.9 {
        recommendations.push(
            "The migration completed successfully with high confidence. \
             Recommended next steps:\n\
             1. Run your project's full test suite against the migrated code\n\
             2. Perform a brief manual review of edge cases\n\
             3. Integrate the migrated code into your main branch".to_string()
        );
    } else if rate >= 0.8 {
        recommendations.push(
            "The migration mostly succeeded but some chunks need attention:\n\
             1. Review all chunks marked as 'Partial Success' or 'Failed'\n\
             2. Run your full test suite to catch integration issues\n\
             3. Consider re-running failed chunks with adjusted migration steps\n\
             4. Manually migrate any remaining items flagged for review".to_string()
        );
    } else if rate >= 0.5 {
        recommendations.push(
            "The migration had significant issues. Consider:\n\
             1. Reviewing the migration skill — it may not cover all patterns in this codebase\n\
             2. Breaking the migration into smaller, more targeted phases\n\
             3. Adding codebase-specific @step blocks to handle recurring failures\n\
             4. Manually handling the most complex transformations first, then re-running".to_string()
        );
    } else {
        recommendations.push(
            "The migration had a low success rate. Before re-running:\n\
             1. Verify the migration skill's @precondition matches this codebase\n\
             2. Check that the source code patterns align with the skill's expectations\n\
             3. Consider authoring a custom migration skill tailored to this codebase\n\
             4. Start with a smaller subset of files to validate the approach".to_string()
        );
    }

    // Specific recommendations based on data
    if !report.manual_review.is_empty() {
        recommendations.push(format!(
            "{} item(s) require manual review. Prioritize these before considering \
             the migration complete.",
            report.manual_review.len()
        ));
    }

    if !report.unresolved.is_empty() {
        recommendations.push(format!(
            "{} unresolved issue(s) remain. These should be addressed before \
             merging the migrated code.",
            report.unresolved.len()
        ));
    }

    let total_repairs = report.total_repair_iterations();
    if total_repairs > report.chunks.len() as u32 * 2 {
        recommendations.push(
            "High repair iteration count detected. The migration skill may benefit from \
             more precise @step blocks or additional @verify patterns to reduce the need \
             for iterative repair.".to_string()
        );
    }

    let paragraphs: Vec<Block> = recommendations
        .iter()
        .map(|r| make_paragraph(r))
        .collect();
    make_section("Recommendations", paragraphs)
}

fn make_section(title: &str, children: Vec<Block>) -> Block {
    Block {
        kind: BlockKind::Section {
            attrs: Attrs::new(),
            title: vec![Inline::Text {
                text: title.to_string(),
            }],
            children,
        },
        span: Span::new(0, 0),
    }
}

fn make_paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text {
                text: text.to_string(),
            }],
        },
        span: Span::new(0, 0),
    }
}
