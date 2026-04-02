use aif_core::ast::*;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

const BINARY_THRESHOLD: usize = 50;

/// Emit a document in the hybrid LML+binary format.
///
/// Structure uses aggressive-style LML tags (@skill, @step, etc.).
/// Text content longer than BINARY_THRESHOLD characters is base64-encoded
/// with a `~b64:` prefix. Short text stays plain.
pub fn emit_lml_hybrid(doc: &Document) -> String {
    let mut out = String::new();

    // Metadata as #key: value
    for (key, value) in &doc.metadata {
        out.push('#');
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }

    for block in &doc.blocks {
        emit_block(&mut out, block, 0);
    }

    out
}

fn inlines_to_plain(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for inline in inlines {
        inline_to_plain(&mut s, inline);
    }
    s
}

fn inline_to_plain(out: &mut String, inline: &Inline) {
    match inline {
        Inline::Text { text } => out.push_str(text),
        Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
            for i in content {
                inline_to_plain(out, i);
            }
        }
        Inline::InlineCode { code } => out.push_str(code),
        Inline::Link { text, url } => {
            for i in text {
                inline_to_plain(out, i);
            }
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
        Inline::SoftBreak => out.push(' '),
        Inline::HardBreak => out.push('\n'),
    }
}

/// Emit content, base64-encoding it if it exceeds the threshold.
fn emit_content_maybe_binary(out: &mut String, inlines: &[Inline]) {
    let plain = inlines_to_plain(inlines);
    if plain.len() > BINARY_THRESHOLD {
        let encoded = B64.encode(plain.as_bytes());
        out.push_str("~b64:");
        out.push_str(&encoded);
    } else {
        out.push_str(&plain);
    }
}

fn emit_media_meta_inline(out: &mut String, meta: &MediaMeta) {
    if let Some(alt) = &meta.alt {
        out.push_str(" alt=");
        out.push_str(alt);
    }
    if let Some(w) = meta.width {
        out.push_str(&format!(" w={}", w));
    }
    if let Some(h) = meta.height {
        out.push_str(&format!(" h={}", h));
    }
    if let Some(d) = meta.duration {
        out.push_str(&format!(" dur={}", d));
    }
    if let Some(m) = &meta.mime {
        out.push_str(" mime=");
        out.push_str(m);
    }
    if let Some(p) = &meta.poster {
        out.push_str(" poster=");
        out.push_str(p);
    }
}

fn emit_attrs_inline(out: &mut String, attrs: &Attrs) {
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

fn skill_prefix(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "@skill",
        SkillBlockType::Step => "@step",
        SkillBlockType::Verify => "@verify",
        SkillBlockType::Precondition => "@pre",
        SkillBlockType::OutputContract => "@output",
        SkillBlockType::Decision => "@decision",
        SkillBlockType::Tool => "@tool",
        SkillBlockType::Fallback => "@fallback",
        SkillBlockType::RedFlag => "@redflag",
        SkillBlockType::Example => "@example",
        SkillBlockType::Scenario => "@scenario",
    }
}

fn semantic_prefix(bt: &SemanticBlockType) -> &'static str {
    match bt {
        SemanticBlockType::Claim => "@claim",
        SemanticBlockType::Evidence => "@evidence",
        SemanticBlockType::Definition => "@definition",
        SemanticBlockType::Theorem => "@theorem",
        SemanticBlockType::Assumption => "@assumption",
        SemanticBlockType::Result => "@result",
        SemanticBlockType::Conclusion => "@conclusion",
        SemanticBlockType::Requirement => "@requirement",
        SemanticBlockType::Recommendation => "@recommendation",
    }
}

fn emit_block(out: &mut String, block: &Block, depth: usize) {
    match &block.kind {
        BlockKind::Section { attrs: _, title, children } => {
            let hashes = "#".repeat(depth + 1);
            out.push_str(&hashes);
            out.push(' ');
            emit_content_maybe_binary(out, title);
            out.push('\n');
            for child in children {
                emit_block(out, child, depth + 1);
            }
        }
        BlockKind::Paragraph { content } => {
            emit_content_maybe_binary(out, content);
            out.push_str("\n\n");
        }
        BlockKind::SemanticBlock { block_type, attrs: _, title, content } => {
            out.push_str(semantic_prefix(block_type));
            out.push_str(": ");
            if let Some(t) = title {
                emit_content_maybe_binary(out, t);
                out.push_str(" — ");
            }
            emit_content_maybe_binary(out, content);
            out.push_str("\n\n");
        }
        BlockKind::Callout { callout_type, attrs: _, content } => {
            let tag = match callout_type {
                CalloutType::Note => "NOTE",
                CalloutType::Warning => "WARNING",
                CalloutType::Info => "INFO",
                CalloutType::Tip => "TIP",
            };
            out.push_str("> [");
            out.push_str(tag);
            out.push_str("] ");
            emit_content_maybe_binary(out, content);
            out.push_str("\n\n");
        }
        BlockKind::CodeBlock { lang, attrs: _, code } => {
            // Code blocks are never base64-encoded
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
                    emit_content_maybe_binary(out, para_content);
                    out.push_str("\n\n");
                } else {
                    emit_block(out, child, depth + 1);
                }
            }
        }
        BlockKind::Table { attrs, caption, headers: _, rows: _ } => {
            out.push_str("[TABLE");
            emit_attrs_inline(out, attrs);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_content_maybe_binary(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::Figure { attrs, caption, src, meta } => {
            out.push_str("![");
            if let Some(cap) = caption {
                emit_content_maybe_binary(out, cap);
            }
            out.push_str("](");
            out.push_str(src);
            emit_attrs_inline(out, attrs);
            emit_media_meta_inline(out, meta);
            out.push_str(")\n\n");
        }
        BlockKind::Audio { attrs, caption, src, meta } => {
            out.push_str("[AUDIO");
            emit_attrs_inline(out, attrs);
            out.push_str(" src=");
            out.push_str(src);
            emit_media_meta_inline(out, meta);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_content_maybe_binary(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::Video { attrs, caption, src, meta } => {
            out.push_str("[VIDEO");
            emit_attrs_inline(out, attrs);
            out.push_str(" src=");
            out.push_str(src);
            emit_media_meta_inline(out, meta);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_content_maybe_binary(out, cap);
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
                emit_content_maybe_binary(out, &item.content);
                out.push('\n');
                for child in &item.children {
                    emit_block(out, child, depth + 1);
                }
            }
            out.push('\n');
        }
        BlockKind::SkillBlock { skill_type, attrs, title, content, children } => {
            out.push_str(skill_prefix(skill_type));
            emit_attrs_inline(out, attrs);
            if let Some(t) = title {
                out.push_str(": ");
                emit_content_maybe_binary(out, t);
            } else if !content.is_empty() || !children.is_empty() {
                out.push(':');
            }
            if !content.is_empty() {
                out.push(' ');
                emit_content_maybe_binary(out, content);
                out.push('\n');
            } else {
                out.push('\n');
            }
            for child in children {
                emit_block(out, child, depth + 1);
            }
        }
        BlockKind::ThematicBreak => {
            out.push_str("---\n\n");
        }
    }
}
