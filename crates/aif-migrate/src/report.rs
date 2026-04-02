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

    let mut blocks = Vec::new();

    // Summary section
    let summary_text = format!(
        "Migrated {} chunks from {}.\nOverall confidence: {:.2}.\nDuration: {}s.\nSuccess rate: {:.0}%.",
        report.chunks.len(),
        report.source_dir.display(),
        report.overall_confidence,
        report.duration.as_secs(),
        report.success_rate() * 100.0,
    );
    blocks.push(make_section(
        "Summary",
        vec![make_paragraph(&summary_text)],
    ));

    // Results by chunk
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
        let files_str = chunk
            .files
            .iter()
            .map(|f| f.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut text = format!(
            "Chunk {} ({}): {} — confidence {:.2}, {} repair iterations",
            chunk.chunk_id, files_str, status_label, chunk.confidence, chunk.repair_iterations,
        );
        for note in &chunk.notes {
            text.push_str(&format!("\n{}", note));
        }
        chunk_blocks.push(Block {
            kind: BlockKind::Callout {
                callout_type,
                attrs: Attrs::new(),
                content: vec![Inline::Text {
                    text: text.to_string(),
                }],
            },
            span: Span::new(0, 0),
        });
    }
    blocks.push(make_section("Results by Chunk", chunk_blocks));

    // Manual review section
    if !report.manual_review.is_empty() {
        let items: Vec<Block> = report
            .manual_review
            .iter()
            .map(|item| make_paragraph(&format!("- {}", item)))
            .collect();
        blocks.push(make_section("Manual Review Required", items));
    }

    // Unresolved issues
    if !report.unresolved.is_empty() {
        let items: Vec<Block> = report
            .unresolved
            .iter()
            .map(|item| make_paragraph(&format!("- {}", item)))
            .collect();
        blocks.push(make_section("Unresolved Issues", items));
    }

    Document { metadata, blocks }
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
