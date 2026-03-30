use aif_core::ast::*;

pub fn emit_lml(doc: &Document) -> String {
    let mut out = String::new();
    emit_doc(&mut out, doc);
    out
}

fn emit_doc(out: &mut String, doc: &Document) {
    // Opening [DOC ...] tag with metadata
    out.push_str("[DOC");
    for (key, value) in &doc.metadata {
        out.push(' ');
        emit_attr_pair(out, key, value);
    }
    out.push_str("]\n");

    for block in &doc.blocks {
        emit_block(out, block, 0);
    }

    out.push_str("[/DOC]\n");
}

fn emit_block(out: &mut String, block: &Block, _depth: usize) {
    match &block.kind {
        BlockKind::Section {
            attrs,
            title,
            children,
        } => {
            out.push_str("[SECTION");
            emit_attrs(out, attrs);
            out.push_str("] ");
            emit_inlines_plain(out, title);
            out.push('\n');
            for child in children {
                emit_block(out, child, _depth + 1);
            }
            out.push_str("[/SECTION]\n");
        }
        BlockKind::Paragraph { content } => {
            emit_inlines_plain(out, content);
            out.push_str("\n\n");
        }
        BlockKind::SemanticBlock {
            block_type,
            attrs,
            title,
            content,
        } => {
            let tag = semantic_block_tag(block_type);
            out.push('[');
            out.push_str(tag);
            emit_attrs(out, attrs);
            out.push(']');
            if let Some(title) = title {
                out.push(' ');
                emit_inlines_plain(out, title);
            }
            out.push('\n');
            emit_inlines_plain(out, content);
            out.push_str("\n\n");
        }
        BlockKind::Callout {
            callout_type,
            attrs: _,
            content,
        } => {
            let tag = callout_tag(callout_type);
            out.push('[');
            out.push_str(tag);
            out.push_str("] ");
            emit_inlines_plain(out, content);
            out.push_str("\n\n");
        }
        BlockKind::Table {
            attrs,
            caption,
            headers: _,
            rows: _,
        } => {
            out.push_str("[TABLE");
            emit_attrs(out, attrs);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_plain(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::Figure {
            attrs,
            caption,
            src,
        } => {
            out.push_str("[FIGURE");
            emit_attrs(out, attrs);
            out.push(' ');
            emit_attr_pair(out, "src", src);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_plain(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::CodeBlock {
            lang,
            attrs: _,
            code,
        } => {
            out.push_str("[CODE");
            if let Some(lang) = lang {
                out.push(' ');
                emit_attr_pair(out, "lang", lang);
            }
            out.push_str("]\n");
            out.push_str(code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("[/CODE]\n\n");
        }
        BlockKind::BlockQuote { content } => {
            out.push_str("[QUOTE]\n");
            for child in content {
                emit_block(out, child, _depth + 1);
            }
            out.push_str("[/QUOTE]\n\n");
        }
        BlockKind::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                if *ordered {
                    out.push_str(&format!("{}. ", i + 1));
                } else {
                    out.push_str("- ");
                }
                emit_inlines_plain(out, &item.content);
                out.push('\n');
                for child in &item.children {
                    emit_block(out, child, _depth + 1);
                }
            }
            out.push('\n');
        }
        BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        } => {
            let tag = skill_block_tag(skill_type);
            out.push('[');
            out.push_str(tag);
            emit_attrs(out, attrs);
            out.push(']');
            if let Some(t) = title {
                out.push(' ');
                emit_inlines_plain(out, t);
            }
            out.push('\n');
            if !content.is_empty() {
                emit_inlines_plain(out, content);
                out.push('\n');
            }
            for child in children {
                emit_block(out, child, _depth + 1);
            }
            if matches!(skill_type, SkillBlockType::Skill) || !children.is_empty() {
                out.push_str("[/");
                out.push_str(tag);
                out.push_str("]\n");
            }
            out.push('\n');
        }
        BlockKind::ThematicBreak => {
            out.push_str("---\n\n");
        }
    }
}

fn emit_attrs(out: &mut String, attrs: &Attrs) {
    if let Some(id) = &attrs.id {
        out.push(' ');
        emit_attr_pair(out, "id", id);
    }
    for (key, value) in &attrs.pairs {
        out.push(' ');
        emit_attr_pair(out, key, value);
    }
}

fn emit_attr_pair(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    if needs_quotes(value) {
        out.push('"');
        out.push_str(value);
        out.push('"');
    } else {
        out.push_str(value);
    }
}

fn needs_quotes(value: &str) -> bool {
    value.is_empty() || value.contains(' ') || value.contains('"') || value.contains(']')
}

fn emit_inlines_plain(out: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        emit_inline_plain(out, inline);
    }
}

fn emit_inline_plain(out: &mut String, inline: &Inline) {
    match inline {
        Inline::Text { text } => out.push_str(text),
        Inline::Emphasis { content } => emit_inlines_plain(out, content),
        Inline::Strong { content } => emit_inlines_plain(out, content),
        Inline::InlineCode { code } => out.push_str(code),
        Inline::Link { text, url } => {
            emit_inlines_plain(out, text);
            out.push_str(" (");
            out.push_str(url);
            out.push(')');
        }
        Inline::Reference { target } => {
            out.push('@');
            out.push_str(target);
        }
        Inline::Footnote { content } => emit_inlines_plain(out, content),
        Inline::SoftBreak => out.push(' '),
        Inline::HardBreak => out.push('\n'),
    }
}

fn semantic_block_tag(bt: &SemanticBlockType) -> &'static str {
    match bt {
        SemanticBlockType::Claim => "CLAIM",
        SemanticBlockType::Evidence => "EVIDENCE",
        SemanticBlockType::Definition => "DEFINITION",
        SemanticBlockType::Theorem => "THEOREM",
        SemanticBlockType::Assumption => "ASSUMPTION",
        SemanticBlockType::Result => "RESULT",
        SemanticBlockType::Conclusion => "CONCLUSION",
        SemanticBlockType::Requirement => "REQUIREMENT",
        SemanticBlockType::Recommendation => "RECOMMENDATION",
    }
}

fn skill_block_tag(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "SKILL",
        SkillBlockType::Step => "STEP",
        SkillBlockType::Verify => "VERIFY",
        SkillBlockType::Precondition => "PRECONDITION",
        SkillBlockType::OutputContract => "OUTPUT_CONTRACT",
        SkillBlockType::Decision => "DECISION",
        SkillBlockType::Tool => "TOOL",
        SkillBlockType::Fallback => "FALLBACK",
        SkillBlockType::RedFlag => "RED_FLAG",
        SkillBlockType::Example => "EXAMPLE",
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
