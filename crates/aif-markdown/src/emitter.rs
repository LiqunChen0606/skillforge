use aif_core::ast::*;

/// Convert an AIF Document to Markdown.
pub fn emit_markdown(doc: &Document) -> String {
    let mut out = String::new();

    // Document title from metadata
    if let Some(title) = doc.metadata.get("title") {
        out.push_str(&format!("# {}\n", title.trim()));
    }

    for (i, block) in doc.blocks.iter().enumerate() {
        if i > 0 || !out.is_empty() {
            // Blank line separator between blocks (and after title)
            if !out.ends_with("\n\n") {
                if out.ends_with('\n') {
                    out.push('\n');
                } else {
                    out.push_str("\n\n");
                }
            }
        }
        emit_block(&mut out, block, 2);
    }

    // Ensure output ends with exactly one newline
    if out.is_empty() {
        return out;
    }
    let trimmed = out.trim_end();
    let mut result = trimmed.to_string();
    result.push('\n');
    result
}

fn emit_block(out: &mut String, block: &Block, heading_level: usize) {
    match &block.kind {
        BlockKind::Section {
            attrs: _,
            title,
            children,
        } => {
            let hashes = "#".repeat(heading_level);
            out.push_str(&format!("{} {}\n", hashes, inlines_to_text(title)));
            for child in children {
                if !out.ends_with("\n\n") {
                    if out.ends_with('\n') {
                        out.push('\n');
                    } else {
                        out.push_str("\n\n");
                    }
                }
                emit_block(out, child, heading_level + 1);
            }
        }
        BlockKind::Paragraph { content } => {
            out.push_str(&inlines_to_text(content));
            out.push('\n');
        }
        BlockKind::SemanticBlock {
            block_type,
            attrs: _,
            title,
            content,
        } => {
            let type_name = semantic_block_type_name(block_type);
            match title {
                Some(t) => {
                    out.push_str(&format!(
                        "**{}:** {}\n\n{}\n",
                        type_name,
                        inlines_to_text(t),
                        inlines_to_text(content)
                    ));
                }
                None => {
                    out.push_str(&format!(
                        "**{}:**\n\n{}\n",
                        type_name,
                        inlines_to_text(content)
                    ));
                }
            }
        }
        BlockKind::Callout {
            callout_type,
            attrs: _,
            content,
        } => {
            let type_name = callout_type_name(callout_type);
            out.push_str(&format!(
                "> **{}:** {}\n",
                type_name,
                inlines_to_text(content)
            ));
        }
        BlockKind::Table {
            attrs: _,
            caption: _,
            headers,
            rows,
        } => {
            // Header row
            out.push('|');
            for header in headers {
                out.push_str(&format!(" {} |", inlines_to_text(header)));
            }
            out.push('\n');
            // Separator row
            out.push('|');
            for _ in headers {
                out.push_str(" --- |");
            }
            out.push('\n');
            // Data rows
            for row in rows {
                out.push('|');
                for cell in row {
                    out.push_str(&format!(" {} |", inlines_to_text(cell)));
                }
                out.push('\n');
            }
        }
        BlockKind::Figure {
            attrs: _,
            caption,
            src,
        } => {
            let alt = caption
                .as_ref()
                .map(|c| inlines_to_text(c))
                .unwrap_or_default();
            out.push_str(&format!("![{}]({})\n", alt, src));
        }
        BlockKind::CodeBlock {
            lang,
            attrs: _,
            code,
        } => {
            let lang_str = lang.as_deref().unwrap_or("");
            out.push_str(&format!("```{}\n{}\n```\n", lang_str, code.trim_end()));
        }
        BlockKind::BlockQuote { content } => {
            // Render child blocks, then prefix each line with "> "
            let mut inner = String::new();
            for (i, child) in content.iter().enumerate() {
                if i > 0 {
                    inner.push('\n');
                }
                emit_block(&mut inner, child, 2);
            }
            for line in inner.trim_end().lines() {
                if line.is_empty() {
                    out.push_str(">\n");
                } else {
                    out.push_str(&format!("> {}\n", line));
                }
            }
        }
        BlockKind::List { ordered, items } => {
            emit_list(out, items, *ordered, 0);
        }
        BlockKind::ThematicBreak => {
            out.push_str("---\n");
        }
    }
}

fn emit_list(out: &mut String, items: &[ListItem], ordered: bool, indent: usize) {
    let prefix_space = " ".repeat(indent);
    for (i, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!("{}. ", i + 1)
        } else {
            "- ".to_string()
        };
        out.push_str(&format!(
            "{}{}{}\n",
            prefix_space,
            marker,
            inlines_to_text(&item.content)
        ));
        // Render child blocks indented
        for child in &item.children {
            match &child.kind {
                BlockKind::List {
                    ordered: child_ordered,
                    items: child_items,
                } => {
                    emit_list(out, child_items, *child_ordered, indent + 2);
                }
                _ => {
                    let mut inner = String::new();
                    emit_block(&mut inner, child, 2);
                    for line in inner.lines() {
                        out.push_str(&format!("{}  {}\n", prefix_space, line));
                    }
                }
            }
        }
    }
}

/// Convert inline elements to their Markdown text representation.
pub fn inlines_to_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Strong { content } => {
                out.push_str("**");
                out.push_str(&inlines_to_text(content));
                out.push_str("**");
            }
            Inline::Emphasis { content } => {
                out.push('*');
                out.push_str(&inlines_to_text(content));
                out.push('*');
            }
            Inline::InlineCode { code } => {
                out.push('`');
                out.push_str(code);
                out.push('`');
            }
            Inline::Link { text, url } => {
                out.push('[');
                out.push_str(&inlines_to_text(text));
                out.push_str("](");
                out.push_str(url);
                out.push(')');
            }
            Inline::Reference { target } => {
                out.push_str(&format!("[{}](#{})", target, target));
            }
            Inline::Footnote { content } => {
                out.push_str("[^");
                out.push_str(&inlines_to_text(content));
                out.push(']');
            }
            Inline::SoftBreak => out.push('\n'),
            Inline::HardBreak => out.push_str("  \n"),
        }
    }
    out
}

fn semantic_block_type_name(bt: &SemanticBlockType) -> &'static str {
    match bt {
        SemanticBlockType::Claim => "Claim",
        SemanticBlockType::Evidence => "Evidence",
        SemanticBlockType::Definition => "Definition",
        SemanticBlockType::Theorem => "Theorem",
        SemanticBlockType::Assumption => "Assumption",
        SemanticBlockType::Result => "Result",
        SemanticBlockType::Conclusion => "Conclusion",
        SemanticBlockType::Requirement => "Requirement",
        SemanticBlockType::Recommendation => "Recommendation",
    }
}

fn callout_type_name(ct: &CalloutType) -> &'static str {
    match ct {
        CalloutType::Note => "Note",
        CalloutType::Warning => "Warning",
        CalloutType::Info => "Info",
        CalloutType::Tip => "Tip",
    }
}
