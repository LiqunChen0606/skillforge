use aif_core::ast::{Block, BlockKind, Document, Inline};
use krilla::geom::Point;
use krilla::page::PageSettings;
use krilla::text::{Font, TextDirection};
use std::fmt;

use super::styles::{PdfOptions, PageSize};

#[derive(Debug)]
pub enum PdfExportError {
    FontLoad(String),
    Render(String),
}

impl fmt::Display for PdfExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfExportError::FontLoad(msg) => write!(f, "Font loading error: {}", msg),
            PdfExportError::Render(msg) => write!(f, "PDF render error: {}", msg),
        }
    }
}

impl std::error::Error for PdfExportError {}

/// Export an AIF Document to PDF bytes with default options.
pub fn export_pdf(doc: &Document) -> Result<Vec<u8>, PdfExportError> {
    export_pdf_with_options(doc, &PdfOptions::default())
}

/// Export an AIF Document to PDF bytes with custom options.
pub fn export_pdf_with_options(
    doc: &Document,
    opts: &PdfOptions,
) -> Result<Vec<u8>, PdfExportError> {
    let font = load_embedded_font()?;
    let (page_width, page_height) = opts.page_size.dimensions_pt();

    let mut pdf_doc = krilla::Document::new();
    let content_width = page_width - opts.margins.left - opts.margins.right;

    // Collect all text lines with their styles
    let mut lines: Vec<TextLine> = Vec::new();

    // Add title from metadata
    if let Some(title) = doc.metadata.get("title") {
        lines.push(TextLine {
            text: title.clone(),
            font_size: opts.base_font_size * 2.0,
            y_spacing: opts.base_font_size * 2.5,
        });
    }

    // Render blocks
    for block in &doc.blocks {
        collect_block_lines(block, opts.base_font_size, 0, &mut lines);
    }

    // Paginate and render
    let usable_height = page_height - opts.margins.top - opts.margins.bottom;
    let mut current_y = 0.0_f32;
    let mut page_lines: Vec<Vec<&TextLine>> = vec![vec![]];

    for line in &lines {
        if current_y + line.y_spacing > usable_height && !page_lines.last().unwrap().is_empty() {
            page_lines.push(vec![]);
            current_y = 0.0;
        }
        current_y += line.y_spacing;
        page_lines.last_mut().unwrap().push(line);
    }

    for page_content in &page_lines {
        let page_settings = match opts.page_size {
            PageSize::A4 => PageSettings::from_wh(page_width, page_height),
            PageSize::Letter => PageSettings::from_wh(page_width, page_height),
            PageSize::Custom { width_pt, height_pt } => {
                PageSettings::from_wh(width_pt, height_pt)
            }
        };
        let page_settings =
            page_settings.ok_or_else(|| PdfExportError::Render("invalid page size".into()))?;
        let mut page = pdf_doc.start_page_with(page_settings);
        let mut surface = page.surface();

        let mut y = opts.margins.top;
        for line in page_content {
            y += line.y_spacing;
            // Word-wrap long lines
            let wrapped = wrap_text(&line.text, line.font_size, content_width);
            for (i, text_line) in wrapped.iter().enumerate() {
                if i > 0 {
                    y += line.font_size * 1.4;
                }
                surface.draw_text(
                    Point::from_xy(opts.margins.left, y),
                    font.clone(),
                    line.font_size,
                    text_line,
                    false,
                    TextDirection::Auto,
                );
            }
        }

        surface.finish();
        page.finish();
    }

    pdf_doc
        .finish()
        .map_err(|e| PdfExportError::Render(format!("{:?}", e)))
}

struct TextLine {
    text: String,
    font_size: f32,
    y_spacing: f32,
}

fn collect_block_lines(block: &Block, base_size: f32, depth: usize, lines: &mut Vec<TextLine>) {
    match &block.kind {
        BlockKind::Section {
            title, children, ..
        } => {
            let heading_size = heading_font_size(base_size, depth);
            let title_text = inlines_to_text(title);
            if !title_text.is_empty() {
                lines.push(TextLine {
                    text: title_text,
                    font_size: heading_size,
                    y_spacing: heading_size * 1.8,
                });
            }
            for child in children {
                collect_block_lines(child, base_size, depth + 1, lines);
            }
        }
        BlockKind::Paragraph { content } => {
            let text = inlines_to_text(content);
            if !text.is_empty() {
                lines.push(TextLine {
                    text,
                    font_size: base_size,
                    y_spacing: base_size * 1.6,
                });
            }
        }
        BlockKind::CodeBlock { code, .. } => {
            let code_size = base_size * 0.85;
            for line in code.lines() {
                lines.push(TextLine {
                    text: line.to_string(),
                    font_size: code_size,
                    y_spacing: code_size * 1.3,
                });
            }
        }
        BlockKind::BlockQuote { content } => {
            for child in content {
                // Indent blockquotes by using a prefix
                let mut sub_lines = Vec::new();
                collect_block_lines(child, base_size, depth, &mut sub_lines);
                for mut line in sub_lines {
                    line.text = format!("  {}", line.text);
                    lines.push(line);
                }
            }
        }
        BlockKind::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                let prefix = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "• ".to_string()
                };
                let text = inlines_to_text(&item.content);
                lines.push(TextLine {
                    text: format!("{}{}", prefix, text),
                    font_size: base_size,
                    y_spacing: base_size * 1.4,
                });
                for child in &item.children {
                    collect_block_lines(child, base_size, depth + 1, lines);
                }
            }
        }
        BlockKind::Table {
            headers, rows, ..
        } => {
            // Render table as text rows
            if !headers.is_empty() {
                let header_text = headers
                    .iter()
                    .map(|h| inlines_to_text(h))
                    .collect::<Vec<_>>()
                    .join(" | ");
                lines.push(TextLine {
                    text: header_text,
                    font_size: base_size,
                    y_spacing: base_size * 1.5,
                });
            }
            for row in rows {
                let row_text = row
                    .iter()
                    .map(|cell| inlines_to_text(cell))
                    .collect::<Vec<_>>()
                    .join(" | ");
                lines.push(TextLine {
                    text: row_text,
                    font_size: base_size,
                    y_spacing: base_size * 1.4,
                });
            }
        }
        BlockKind::SemanticBlock { content, title, .. } => {
            if let Some(title_inlines) = title {
                let text = inlines_to_text(title_inlines);
                if !text.is_empty() {
                    lines.push(TextLine {
                        text,
                        font_size: base_size * 1.1,
                        y_spacing: base_size * 1.6,
                    });
                }
            }
            let text = inlines_to_text(content);
            if !text.is_empty() {
                lines.push(TextLine {
                    text,
                    font_size: base_size,
                    y_spacing: base_size * 1.4,
                });
            }
        }
        BlockKind::Callout { content, .. } => {
            let text = inlines_to_text(content);
            if !text.is_empty() {
                lines.push(TextLine {
                    text: format!("  {}", text),
                    font_size: base_size,
                    y_spacing: base_size * 1.4,
                });
            }
        }
        BlockKind::SkillBlock {
            content, children, title, ..
        } => {
            if let Some(title_inlines) = title {
                let text = inlines_to_text(title_inlines);
                if !text.is_empty() {
                    lines.push(TextLine {
                        text,
                        font_size: base_size * 1.1,
                        y_spacing: base_size * 1.6,
                    });
                }
            }
            let text = inlines_to_text(content);
            if !text.is_empty() {
                lines.push(TextLine {
                    text,
                    font_size: base_size,
                    y_spacing: base_size * 1.4,
                });
            }
            for child in children {
                collect_block_lines(child, base_size, depth + 1, lines);
            }
        }
        BlockKind::Figure { caption, .. } => {
            if let Some(cap) = caption {
                let text = inlines_to_text(cap);
                lines.push(TextLine {
                    text: format!("[Figure: {}]", text),
                    font_size: base_size * 0.9,
                    y_spacing: base_size * 1.4,
                });
            }
        }
        BlockKind::ThematicBreak => {
            lines.push(TextLine {
                text: "---".to_string(),
                font_size: base_size,
                y_spacing: base_size * 1.5,
            });
        }
    }
}

fn heading_font_size(base: f32, depth: usize) -> f32 {
    match depth {
        0 => base * 1.8,
        1 => base * 1.5,
        2 => base * 1.3,
        3 => base * 1.1,
        _ => base,
    }
}

fn inlines_to_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Emphasis { content } => out.push_str(&inlines_to_text(content)),
            Inline::Strong { content } => out.push_str(&inlines_to_text(content)),
            Inline::InlineCode { code } => {
                out.push('`');
                out.push_str(code);
                out.push('`');
            }
            Inline::Link { text, .. } => out.push_str(&inlines_to_text(text)),
            Inline::Reference { target } => {
                out.push_str("[ref:");
                out.push_str(target);
                out.push(']');
            }
            Inline::Footnote { content } => {
                out.push_str("[^");
                out.push_str(&inlines_to_text(content));
                out.push(']');
            }
            Inline::SoftBreak => out.push(' '),
            Inline::HardBreak => out.push('\n'),
        }
    }
    out
}

/// Simple word-wrap: estimate character width and break at word boundaries.
fn wrap_text(text: &str, font_size: f32, max_width: f32) -> Vec<String> {
    // Approximate chars per point for a typical font
    let avg_char_width = font_size * 0.5;
    let max_chars = (max_width / avg_char_width) as usize;
    if max_chars == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            result.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// Load a font for PDF rendering.
/// Searches common system font paths for a suitable TrueType font.
fn load_embedded_font() -> Result<Font, PdfExportError> {
    let candidates = [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Geneva.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            if let Some(font) = Font::new(data.into(), 0) {
                return Ok(font);
            }
        }
    }
    Err(PdfExportError::FontLoad(
        "no suitable system font found; install DejaVuSans or Arial".into(),
    ))
}

/// Load a font from specific file path.
pub fn load_font_from_path(path: &std::path::Path) -> Result<Font, PdfExportError> {
    let data = std::fs::read(path)
        .map_err(|e| PdfExportError::FontLoad(format!("cannot read {}: {}", path.display(), e)))?;
    Font::new(data.into(), 0)
        .ok_or_else(|| PdfExportError::FontLoad(format!("invalid font: {}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inlines_to_text() {
        let inlines = vec![
            Inline::Text {
                text: "Hello ".into(),
            },
            Inline::Strong {
                content: vec![Inline::Text {
                    text: "world".into(),
                }],
            },
        ];
        assert_eq!(inlines_to_text(&inlines), "Hello world");
    }

    #[test]
    fn test_wrap_text_short() {
        let lines = wrap_text("short", 12.0, 500.0);
        assert_eq!(lines, vec!["short"]);
    }

    #[test]
    fn test_wrap_text_long() {
        let text = "This is a very long line that should be wrapped into multiple lines when rendered";
        let lines = wrap_text(text, 12.0, 100.0);
        assert!(lines.len() > 1);
    }
}
