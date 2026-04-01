use aif_core::ast::{Attrs, Block, BlockKind, Document, Inline};
use aif_core::span::Span;
use std::fmt;

#[derive(Debug)]
pub enum PdfImportError {
    Parse(String),
    Extraction(String),
}

impl fmt::Display for PdfImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfImportError::Parse(msg) => write!(f, "PDF parse error: {}", msg),
            PdfImportError::Extraction(msg) => write!(f, "PDF extraction error: {}", msg),
        }
    }
}

impl std::error::Error for PdfImportError {}

#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    LowConfidence,
    UnrecognizedElement,
    SkippedContent,
}

#[derive(Debug, Clone)]
pub struct ImportDiagnostic {
    pub page: usize,
    pub kind: DiagnosticKind,
    pub message: String,
    pub confidence: f32,
}

pub struct ImportResult {
    pub document: Document,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub page_count: usize,
    pub avg_confidence: f32,
}

/// Import a PDF file into an AIF Document.
///
/// Extracts text per page, splits into paragraphs based on blank lines,
/// and detects headings based on font-size heuristics (via line length/structure).
pub fn import_pdf(pdf_bytes: &[u8]) -> Result<ImportResult, PdfImportError> {
    let mut pdf_doc = pdf_oxide::document::PdfDocument::from_bytes(pdf_bytes.to_vec())
        .map_err(|e| PdfImportError::Parse(format!("{}", e)))?;

    let page_count = pdf_doc
        .page_count()
        .map_err(|e| PdfImportError::Extraction(format!("cannot get page count: {}", e)))?;

    let mut blocks: Vec<Block> = Vec::new();
    let mut diagnostics: Vec<ImportDiagnostic> = Vec::new();
    let mut total_confidence = 0.0_f32;
    let mut block_count = 0_usize;

    for page_idx in 0..page_count {
        let page_text = match pdf_doc.extract_text(page_idx) {
            Ok(text) => text,
            Err(e) => {
                diagnostics.push(ImportDiagnostic {
                    page: page_idx,
                    kind: DiagnosticKind::SkippedContent,
                    message: format!("failed to extract page {}: {}", page_idx, e),
                    confidence: 0.0,
                });
                continue;
            }
        };

        if page_text.trim().is_empty() {
            continue;
        }

        // Split text into paragraphs on blank lines
        let paragraphs = split_paragraphs(&page_text);

        for para_text in paragraphs {
            let trimmed = para_text.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (block_kind, confidence) = classify_text_block(trimmed);
            let mut attrs = Attrs::new();
            attrs
                .pairs
                .insert("import_confidence".to_string(), format!("{:.2}", confidence));
            attrs
                .pairs
                .insert("source_page".to_string(), page_idx.to_string());

            let block = match block_kind {
                ClassifiedBlock::Heading(text) => {
                    Block {
                        kind: BlockKind::Section {
                            attrs,
                            title: vec![Inline::Text { text }],
                            children: vec![],
                        },
                        span: Span::new(0, 0),
                    }
                }
                ClassifiedBlock::Paragraph(text) => Block {
                    kind: BlockKind::Paragraph {
                        content: vec![Inline::Text { text }],
                    },
                    span: Span::new(0, 0),
                },
                ClassifiedBlock::CodeBlock(text) => Block {
                    kind: BlockKind::CodeBlock {
                        lang: None,
                        attrs,
                        code: text,
                    },
                    span: Span::new(0, 0),
                },
            };

            total_confidence += confidence;
            block_count += 1;
            blocks.push(block);
        }
    }

    let avg_confidence = if block_count > 0 {
        total_confidence / block_count as f32
    } else {
        0.0
    };

    // Add title from first heading if available
    let mut metadata = std::collections::BTreeMap::new();
    if let Some(first_section) = blocks.iter().find(|b| matches!(&b.kind, BlockKind::Section { .. }))
    {
        if let BlockKind::Section { title, .. } = &first_section.kind {
            let title_text: String = title
                .iter()
                .filter_map(|i| {
                    if let Inline::Text { text } = i {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            if !title_text.is_empty() {
                metadata.insert("title".to_string(), title_text);
            }
        }
    }

    Ok(ImportResult {
        document: Document { metadata, blocks },
        diagnostics,
        page_count,
        avg_confidence,
    })
}

/// Split raw text into paragraph chunks by blank lines.
fn split_paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.trim().is_empty() {
                paragraphs.push(current.clone());
                current.clear();
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(line.trim());
        }
    }
    if !current.trim().is_empty() {
        paragraphs.push(current);
    }
    paragraphs
}

enum ClassifiedBlock {
    Heading(String),
    Paragraph(String),
    CodeBlock(String),
}

/// Classify a text block using simple heuristics.
fn classify_text_block(text: &str) -> (ClassifiedBlock, f32) {
    // Short, single-line text that looks like a heading
    let line_count = text.lines().count();
    let word_count = text.split_whitespace().count();

    // Heading heuristic: single line, short, no period at end
    if line_count == 1 && word_count <= 12 && !text.ends_with('.') && !text.ends_with(',') {
        return (ClassifiedBlock::Heading(text.to_string()), 0.65);
    }

    // Code heuristic: many lines with consistent indentation or special chars
    let indented_lines = text
        .lines()
        .filter(|l| l.starts_with("    ") || l.starts_with('\t'))
        .count();
    if line_count > 2 && indented_lines as f32 / line_count as f32 > 0.5 {
        return (ClassifiedBlock::CodeBlock(text.to_string()), 0.55);
    }

    // Default: paragraph
    (ClassifiedBlock::Paragraph(text.to_string()), 0.80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_paragraphs_basic() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let paras = split_paragraphs(text);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0], "First paragraph.");
        assert_eq!(paras[1], "Second paragraph.");
    }

    #[test]
    fn split_paragraphs_multiline() {
        let text = "Line one\nline two\n\nLine three";
        let paras = split_paragraphs(text);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0], "Line one line two");
    }

    #[test]
    fn classify_heading() {
        let (block, conf) = classify_text_block("Introduction");
        assert!(matches!(block, ClassifiedBlock::Heading(_)));
        assert!(conf > 0.0);
    }

    #[test]
    fn classify_paragraph() {
        let (block, conf) = classify_text_block(
            "This is a longer paragraph that contains multiple words and ends with a period.",
        );
        assert!(matches!(block, ClassifiedBlock::Paragraph(_)));
        assert!(conf > 0.0);
    }
}
