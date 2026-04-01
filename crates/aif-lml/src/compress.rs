use aif_core::ast::*;
use std::collections::HashMap;

const MIN_TEXT_LEN: usize = 30;
const MIN_OCCURRENCES: usize = 2;

pub fn render_compressed(doc: &Document) -> String {
    // Phase 1: Collect text occurrences
    let mut counts: HashMap<String, usize> = HashMap::new();
    for block in &doc.blocks {
        collect_text_occurrences(block, &mut counts);
    }

    // Phase 2: Build dictionary
    let mut dict: Vec<(String, String)> = Vec::new();
    let mut text_to_id: HashMap<String, String> = HashMap::new();
    let mut entries: Vec<&String> = counts
        .iter()
        .filter(|(text, &count)| text.len() >= MIN_TEXT_LEN && count >= MIN_OCCURRENCES)
        .map(|(text, _)| text)
        .collect();
    entries.sort(); // deterministic ordering
    for (i, text) in entries.into_iter().enumerate() {
        let id = format!("t{}", i);
        text_to_id.insert(text.clone(), id.clone());
        dict.push((id, text.clone()));
    }

    // Phase 3: Render output
    let mut out = String::new();

    if !dict.is_empty() {
        out.push_str("~dict:\n");
        for (id, text) in &dict {
            out.push_str("  ");
            out.push_str(id);
            out.push('=');
            out.push_str(text);
            out.push('\n');
        }
        out.push_str("~end\n\n");
    }

    // Emit metadata
    for (key, value) in &doc.metadata {
        out.push('#');
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }

    for block in &doc.blocks {
        emit_block_compressed(&mut out, block, 0, &text_to_id);
    }

    out
}

fn collect_text_occurrences(block: &Block, counts: &mut HashMap<String, usize>) {
    match &block.kind {
        BlockKind::Paragraph { content } => collect_inlines(content, counts),
        BlockKind::Section { title, children, .. } => {
            collect_inlines(title, counts);
            for child in children {
                collect_text_occurrences(child, counts);
            }
        }
        BlockKind::SemanticBlock { title, content, .. } => {
            if let Some(t) = title {
                collect_inlines(t, counts);
            }
            collect_inlines(content, counts);
        }
        BlockKind::Callout { content, .. } => collect_inlines(content, counts),
        BlockKind::Table { caption, .. } => {
            if let Some(cap) = caption {
                collect_inlines(cap, counts);
            }
        }
        BlockKind::Figure { caption, .. } => {
            if let Some(cap) = caption {
                collect_inlines(cap, counts);
            }
        }
        BlockKind::Audio { caption, .. } => {
            if let Some(cap) = caption {
                collect_inlines(cap, counts);
            }
        }
        BlockKind::Video { caption, .. } => {
            if let Some(cap) = caption {
                collect_inlines(cap, counts);
            }
        }
        BlockKind::BlockQuote { content } => {
            for child in content {
                collect_text_occurrences(child, counts);
            }
        }
        BlockKind::List { items, .. } => {
            for item in items {
                collect_inlines(&item.content, counts);
                for child in &item.children {
                    collect_text_occurrences(child, counts);
                }
            }
        }
        BlockKind::SkillBlock { title, content, children, .. } => {
            if let Some(t) = title {
                collect_inlines(t, counts);
            }
            collect_inlines(content, counts);
            for child in children {
                collect_text_occurrences(child, counts);
            }
        }
        BlockKind::CodeBlock { .. } | BlockKind::ThematicBreak => {}
    }
}

fn collect_inlines(inlines: &[Inline], counts: &mut HashMap<String, usize>) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => {
                *counts.entry(text.clone()).or_insert(0) += 1;
            }
            Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
                collect_inlines(content, counts);
            }
            Inline::Link { text, .. } => collect_inlines(text, counts),
            _ => {}
        }
    }
}

fn emit_block_compressed(
    out: &mut String,
    block: &Block,
    depth: usize,
    dict: &HashMap<String, String>,
) {
    match &block.kind {
        BlockKind::Section { title, children, .. } => {
            let hashes = "#".repeat(depth + 1);
            out.push_str(&hashes);
            out.push(' ');
            emit_inlines_compressed(out, title, dict);
            out.push('\n');
            for child in children {
                emit_block_compressed(out, child, depth + 1, dict);
            }
        }
        BlockKind::Paragraph { content } => {
            emit_inlines_compressed(out, content, dict);
            out.push_str("\n\n");
        }
        BlockKind::SemanticBlock { block_type, title, content, .. } => {
            out.push('@');
            out.push_str(semantic_block_name(block_type));
            out.push_str(": ");
            if let Some(t) = title {
                emit_inlines_compressed(out, t, dict);
                out.push_str(" — ");
            }
            emit_inlines_compressed(out, content, dict);
            out.push_str("\n\n");
        }
        BlockKind::Callout { callout_type, content, .. } => {
            out.push_str("> [");
            out.push_str(callout_tag(callout_type));
            out.push_str("] ");
            emit_inlines_compressed(out, content, dict);
            out.push_str("\n\n");
        }
        BlockKind::CodeBlock { lang, code, .. } => {
            out.push_str("```");
            if let Some(lang) = lang {
                out.push_str(lang);
            }
            out.push('\n');
            out.push_str(code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n\n");
        }
        BlockKind::BlockQuote { content } => {
            for child in content {
                out.push_str("> ");
                if let BlockKind::Paragraph { content: para_content } = &child.kind {
                    emit_inlines_compressed(out, para_content, dict);
                    out.push_str("\n\n");
                } else {
                    emit_block_compressed(out, child, depth + 1, dict);
                }
            }
        }
        BlockKind::Table { attrs, caption, .. } => {
            out.push_str("[TABLE");
            emit_attrs(out, attrs);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_compressed(out, cap, dict);
            }
            out.push_str("\n\n");
        }
        BlockKind::Figure { attrs, caption, src, meta } => {
            out.push_str("[FIGURE");
            emit_attrs(out, attrs);
            out.push_str(" src=");
            out.push_str(src);
            emit_media_meta_compressed(out, meta);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_compressed(out, cap, dict);
            }
            out.push_str("\n\n");
        }
        BlockKind::Audio { attrs, caption, src, meta } => {
            out.push_str("[AUDIO");
            emit_attrs(out, attrs);
            out.push_str(" src=");
            out.push_str(src);
            emit_media_meta_compressed(out, meta);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_compressed(out, cap, dict);
            }
            out.push_str("\n\n");
        }
        BlockKind::Video { attrs, caption, src, meta } => {
            out.push_str("[VIDEO");
            emit_attrs(out, attrs);
            out.push_str(" src=");
            out.push_str(src);
            emit_media_meta_compressed(out, meta);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_compressed(out, cap, dict);
            }
            out.push_str("\n\n");
        }
        BlockKind::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                if *ordered {
                    out.push_str(&format!("{}. ", i + 1));
                } else {
                    out.push_str("- ");
                }
                emit_inlines_compressed(out, &item.content, dict);
                out.push('\n');
                for child in &item.children {
                    emit_block_compressed(out, child, depth + 1, dict);
                }
            }
            out.push('\n');
        }
        BlockKind::SkillBlock { skill_type, attrs, title, content, children } => {
            let prefix = crate::emitter::skill_block_prefix_aggressive(skill_type);
            out.push_str(prefix);
            emit_attrs_aggressive(out, attrs);
            if let Some(t) = title {
                out.push_str(": ");
                emit_inlines_compressed(out, t, dict);
            } else if !content.is_empty() || !children.is_empty() {
                out.push(':');
            }
            if !content.is_empty() {
                if title.is_some() || children.is_empty() {
                    out.push(' ');
                    emit_inlines_compressed(out, content, dict);
                    out.push('\n');
                } else {
                    out.push('\n');
                    emit_inlines_compressed(out, content, dict);
                    out.push('\n');
                }
            } else {
                out.push('\n');
            }
            for child in children {
                emit_block_compressed(out, child, depth + 1, dict);
            }
        }
        BlockKind::ThematicBreak => {
            out.push_str("---\n\n");
        }
    }
}

fn emit_inlines_compressed(out: &mut String, inlines: &[Inline], dict: &HashMap<String, String>) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => {
                if let Some(id) = dict.get(text.as_str()) {
                    out.push_str("~ref:");
                    out.push_str(id);
                } else {
                    out.push_str(text);
                }
            }
            Inline::Emphasis { content } => emit_inlines_compressed(out, content, dict),
            Inline::Strong { content } => emit_inlines_compressed(out, content, dict),
            Inline::InlineCode { code } => out.push_str(code),
            Inline::Link { text, url } => {
                emit_inlines_compressed(out, text, dict);
                out.push_str(" (");
                out.push_str(url);
                out.push(')');
            }
            Inline::Image { alt, src } => {
                out.push_str("![");
                out.push_str(alt);
                out.push_str("](");
                out.push_str(src);
                out.push(')');
            }
            Inline::Reference { target } => {
                out.push('@');
                out.push_str(target);
            }
            Inline::Footnote { content } => emit_inlines_compressed(out, content, dict),
            Inline::SoftBreak => out.push(' '),
            Inline::HardBreak => out.push('\n'),
        }
    }
}

fn needs_quotes(value: &str) -> bool {
    value.is_empty() || value.contains(' ') || value.contains('"') || value.contains(']')
}

fn emit_quoted_value(out: &mut String, value: &str) {
    if needs_quotes(value) {
        out.push('"');
        out.push_str(value);
        out.push('"');
    } else {
        out.push_str(value);
    }
}

fn emit_media_meta_compressed(out: &mut String, meta: &MediaMeta) {
    if let Some(alt) = &meta.alt {
        out.push_str(" alt=");
        emit_quoted_value(out, alt);
    }
    if let Some(w) = meta.width {
        out.push_str(" width=");
        out.push_str(&w.to_string());
    }
    if let Some(h) = meta.height {
        out.push_str(" height=");
        out.push_str(&h.to_string());
    }
    if let Some(d) = meta.duration {
        out.push_str(" duration=");
        out.push_str(&format!("{}", d));
    }
    if let Some(m) = &meta.mime {
        out.push_str(" mime=");
        emit_quoted_value(out, m);
    }
    if let Some(p) = &meta.poster {
        out.push_str(" poster=");
        emit_quoted_value(out, p);
    }
}

fn emit_attrs(out: &mut String, attrs: &Attrs) {
    if let Some(id) = &attrs.id {
        out.push_str(" id=");
        emit_quoted_value(out, id);
    }
    for (key, value) in &attrs.pairs {
        out.push(' ');
        out.push_str(key);
        out.push('=');
        emit_quoted_value(out, value);
    }
}

fn emit_attrs_aggressive(out: &mut String, attrs: &Attrs) {
    let has_content = attrs.id.is_some() || !attrs.pairs.is_empty();
    if !has_content {
        return;
    }
    out.push('(');
    let mut first = true;
    if let Some(id) = &attrs.id {
        out.push_str("id=");
        out.push_str(id);
        first = false;
    }
    for (k, v) in &attrs.pairs {
        if !first {
            out.push_str(", ");
        }
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        first = false;
    }
    out.push(')');
}

fn semantic_block_name(bt: &SemanticBlockType) -> &'static str {
    match bt {
        SemanticBlockType::Claim => "claim",
        SemanticBlockType::Evidence => "evidence",
        SemanticBlockType::Definition => "definition",
        SemanticBlockType::Theorem => "theorem",
        SemanticBlockType::Assumption => "assumption",
        SemanticBlockType::Result => "result",
        SemanticBlockType::Conclusion => "conclusion",
        SemanticBlockType::Requirement => "requirement",
        SemanticBlockType::Recommendation => "recommendation",
    }
}

fn callout_tag(ct: &CalloutType) -> &'static str {
    match ct {
        CalloutType::Note => "NOTE",
        CalloutType::Warning => "WARNING",
        CalloutType::Info => "INFO",
        CalloutType::Tip => "TIP",
    }
}
