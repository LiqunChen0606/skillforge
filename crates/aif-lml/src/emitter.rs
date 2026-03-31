use aif_core::ast::*;

/// Controls the verbosity/compression level of LML output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LmlMode {
    /// Full tags, no abbreviation. The original default.
    Standard,
    /// Skill-compact: strips examples and condenses steps (existing behavior).
    SkillCompact,
    /// Abbreviated tags with a legend at top.
    Conservative,
    /// (Placeholder) Will further compress prose in a future task.
    Moderate,
    /// (Placeholder) Will maximally compress prose in a future task.
    Aggressive,
}

impl LmlMode {
    fn uses_short_tags(&self) -> bool {
        matches!(self, LmlMode::Conservative | LmlMode::Moderate | LmlMode::Aggressive)
    }

    fn strips_examples(&self) -> bool {
        matches!(self, LmlMode::SkillCompact)
    }
}

// ── Public entry points ──────────────────────────────────────────────

pub fn emit_lml(doc: &Document) -> String {
    emit_lml_mode(doc, LmlMode::Standard)
}

pub fn emit_lml_skill_compact(doc: &Document) -> String {
    emit_lml_mode(doc, LmlMode::SkillCompact)
}

pub fn emit_lml_conservative(doc: &Document) -> String {
    emit_lml_mode(doc, LmlMode::Conservative)
}

pub fn emit_lml_moderate(doc: &Document) -> String {
    emit_lml_mode(doc, LmlMode::Moderate)
}

pub fn emit_lml_aggressive(doc: &Document) -> String {
    emit_lml_mode(doc, LmlMode::Aggressive)
}

pub fn emit_lml_mode(doc: &Document, mode: LmlMode) -> String {
    let mut out = String::new();
    emit_doc(&mut out, doc, mode);
    out
}

// ── Document wrapper ─────────────────────────────────────────────────

fn emit_doc(out: &mut String, doc: &Document, mode: LmlMode) {
    if mode.uses_short_tags() {
        out.push_str("# Tags: SK=Skill ST=Step VER=Verify PRE=Precondition OC=OutputContract DEC=Decision TL=Tool FB=Fallback RF=RedFlag EX=Example CL=Claim EV=Evidence DEF=Definition THM=Theorem ASM=Assumption RES=Result CON=Conclusion REQ=Requirement REC=Recommendation N=Note W=Warning I=Info T=Tip\n");
    }

    out.push_str("[DOC");
    for (key, value) in &doc.metadata {
        out.push(' ');
        emit_attr_pair(out, key, value);
    }
    out.push_str("]\n");

    for block in &doc.blocks {
        emit_block_mode(out, block, 0, mode);
    }

    out.push_str("[/DOC]\n");
}

// ── Block emitter ────────────────────────────────────────────────────

fn emit_block_mode(out: &mut String, block: &Block, depth: usize, mode: LmlMode) {
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
                emit_block_mode(out, child, depth + 1, mode);
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
            let tag = if mode.uses_short_tags() {
                semantic_block_tag_short(block_type)
            } else {
                semantic_block_tag(block_type)
            };
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
            let tag = if mode.uses_short_tags() {
                callout_tag_short(callout_type)
            } else {
                callout_tag(callout_type)
            };
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
                emit_block_mode(out, child, depth + 1, mode);
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
                    emit_block_mode(out, child, depth + 1, mode);
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
            if mode.strips_examples() && matches!(skill_type, SkillBlockType::Example) {
                return;
            }
            let tag = if mode.uses_short_tags() {
                skill_block_tag_short(skill_type)
            } else {
                skill_block_tag(skill_type)
            };
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
                emit_block_mode(out, child, depth + 1, mode);
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

// ── Attribute helpers ────────────────────────────────────────────────

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

// ── Inline helpers ───────────────────────────────────────────────────

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

// ── Tag mappings: full ───────────────────────────────────────────────

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

// ── Tag mappings: abbreviated (conservative+) ────────────────────────

fn semantic_block_tag_short(bt: &SemanticBlockType) -> &'static str {
    match bt {
        SemanticBlockType::Claim => "CL",
        SemanticBlockType::Evidence => "EV",
        SemanticBlockType::Definition => "DEF",
        SemanticBlockType::Theorem => "THM",
        SemanticBlockType::Assumption => "ASM",
        SemanticBlockType::Result => "RES",
        SemanticBlockType::Conclusion => "CON",
        SemanticBlockType::Requirement => "REQ",
        SemanticBlockType::Recommendation => "REC",
    }
}

fn skill_block_tag_short(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "SK",
        SkillBlockType::Step => "ST",
        SkillBlockType::Verify => "VER",
        SkillBlockType::Precondition => "PRE",
        SkillBlockType::OutputContract => "OC",
        SkillBlockType::Decision => "DEC",
        SkillBlockType::Tool => "TL",
        SkillBlockType::Fallback => "FB",
        SkillBlockType::RedFlag => "RF",
        SkillBlockType::Example => "EX",
    }
}

fn callout_tag_short(ct: &CalloutType) -> &'static str {
    match ct {
        CalloutType::Note => "N",
        CalloutType::Warning => "W",
        CalloutType::Info => "I",
        CalloutType::Tip => "T",
    }
}
