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
    if matches!(mode, LmlMode::Conservative | LmlMode::Moderate) {
        out.push_str("# Tags: SK=Skill ST=Step VER=Verify PRE=Precondition OC=OutputContract DEC=Decision TL=Tool FB=Fallback RF=RedFlag EX=Example CL=Claim EV=Evidence DEF=Definition THM=Theorem ASM=Assumption RES=Result CON=Conclusion REQ=Requirement REC=Recommendation N=Note W=Warning I=Info T=Tip\n");
    }

    let use_doc_wrapper = !matches!(mode, LmlMode::Moderate | LmlMode::Aggressive);

    if use_doc_wrapper {
        out.push_str("[DOC");
        for (key, value) in &doc.metadata {
            out.push(' ');
            emit_attr_pair(out, key, value);
        }
        out.push_str("]\n");
    } else {
        // Emit metadata as #key: value lines
        for (key, value) in &doc.metadata {
            out.push('#');
            out.push_str(key);
            out.push_str(": ");
            out.push_str(value);
            out.push('\n');
        }
    }

    for block in &doc.blocks {
        emit_block_mode(out, block, 0, mode);
    }

    if use_doc_wrapper {
        out.push_str("[/DOC]\n");
    }
}

// ── Block emitter ────────────────────────────────────────────────────

fn emit_block_mode(out: &mut String, block: &Block, depth: usize, mode: LmlMode) {
    if mode == LmlMode::Aggressive {
        emit_block_aggressive(out, block, depth);
        return;
    }
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
            headers,
            rows,
        } => {
            out.push_str("[TABLE");
            emit_attrs(out, attrs);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_plain(out, cap);
            }
            out.push('\n');
            emit_table_rows(out, headers, rows);
            out.push_str("[/TABLE]\n\n");
        }
        BlockKind::Figure {
            attrs,
            caption,
            src,
            meta,
        } => {
            out.push_str("[FIGURE");
            emit_attrs(out, attrs);
            out.push(' ');
            emit_attr_pair(out, "src", src);
            emit_media_meta_attrs(out, meta);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_plain(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::Audio {
            attrs,
            caption,
            src,
            meta,
        } => {
            out.push_str("[AUDIO");
            emit_attrs(out, attrs);
            out.push(' ');
            emit_attr_pair(out, "src", src);
            emit_media_meta_attrs(out, meta);
            out.push(']');
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_plain(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::Video {
            attrs,
            caption,
            src,
            meta,
        } => {
            out.push_str("[VIDEO");
            emit_attrs(out, attrs);
            out.push(' ');
            emit_attr_pair(out, "src", src);
            emit_media_meta_attrs(out, meta);
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
            let needs_closing = if matches!(mode, LmlMode::Moderate | LmlMode::Aggressive) {
                // Only close blocks that have children
                !children.is_empty()
            } else {
                // Standard/Conservative: close Skill blocks and blocks with children
                matches!(skill_type, SkillBlockType::Skill) || !children.is_empty()
            };
            if needs_closing {
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

// ── Aggressive mode emitter ──────────────────────────────────────────

fn emit_block_aggressive(out: &mut String, block: &Block, depth: usize) {
    match &block.kind {
        BlockKind::Section { attrs: _, title, children } => {
            // Use # depth-based headings like Markdown
            let hashes = "#".repeat(depth + 1);
            out.push_str(&hashes);
            out.push(' ');
            emit_inlines_plain(out, title);
            out.push('\n');
            for child in children {
                emit_block_aggressive(out, child, depth + 1);
            }
        }
        BlockKind::Paragraph { content } => {
            emit_inlines_plain(out, content);
            out.push_str("\n\n");
        }
        BlockKind::SemanticBlock { block_type, attrs: _, title, content } => {
            out.push('@');
            out.push_str(semantic_block_name_aggressive(block_type));
            out.push_str(": ");
            if let Some(t) = title {
                emit_inlines_plain(out, t);
                out.push_str(" — ");
            }
            emit_inlines_plain(out, content);
            out.push_str("\n\n");
        }
        BlockKind::Callout { callout_type, attrs: _, content } => {
            out.push_str("> [");
            out.push_str(callout_tag(callout_type));
            out.push_str("] ");
            emit_inlines_plain(out, content);
            out.push_str("\n\n");
        }
        BlockKind::CodeBlock { lang, attrs: _, code } => {
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
                // Emit inline content for paragraphs directly after "> "
                if let BlockKind::Paragraph { content: para_content } = &child.kind {
                    emit_inlines_plain(out, para_content);
                    out.push_str("\n\n");
                } else {
                    emit_block_aggressive(out, child, depth + 1);
                }
            }
        }
        BlockKind::Table { attrs: _, caption, headers, rows } => {
            out.push_str("@table:");
            if let Some(cap) = caption {
                out.push(' ');
                emit_inlines_plain(out, cap);
            }
            out.push('\n');
            emit_table_rows(out, headers, rows);
            out.push('\n');
        }
        BlockKind::Figure { attrs, caption, src, meta } => {
            out.push_str("@fig(src=");
            out.push_str(src);
            emit_media_meta_aggressive(out, meta);
            emit_attrs_aggressive(out, attrs);
            out.push(')');
            if let Some(cap) = caption {
                out.push_str(": ");
                emit_inlines_plain(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::Audio { attrs, caption, src, meta } => {
            out.push_str("@audio(src=");
            out.push_str(src);
            emit_media_meta_aggressive(out, meta);
            emit_attrs_aggressive(out, attrs);
            out.push(')');
            if let Some(cap) = caption {
                out.push_str(": ");
                emit_inlines_plain(out, cap);
            }
            out.push_str("\n\n");
        }
        BlockKind::Video { attrs, caption, src, meta } => {
            out.push_str("@vid(src=");
            out.push_str(src);
            emit_media_meta_aggressive(out, meta);
            emit_attrs_aggressive(out, attrs);
            out.push(')');
            if let Some(cap) = caption {
                out.push_str(": ");
                emit_inlines_plain(out, cap);
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
                emit_inlines_plain(out, &item.content);
                out.push('\n');
                for child in &item.children {
                    emit_block_aggressive(out, child, depth + 1);
                }
            }
            out.push('\n');
        }
        BlockKind::SkillBlock { skill_type, attrs, title, content, children } => {
            let prefix = skill_block_prefix_aggressive(skill_type);
            out.push_str(prefix);
            emit_attrs_aggressive(out, attrs);
            if let Some(t) = title {
                out.push_str(": ");
                emit_inlines_plain(out, t);
            } else if !content.is_empty() || !children.is_empty() {
                out.push(':');
            }
            if !content.is_empty() {
                if title.is_some() || children.is_empty() {
                    // Leaf with content: put on same line
                    out.push(' ');
                    emit_inlines_plain(out, content);
                    out.push('\n');
                } else {
                    // Container with content (unusual but handle it)
                    out.push('\n');
                    emit_inlines_plain(out, content);
                    out.push('\n');
                }
            } else {
                out.push('\n');
            }
            for child in children {
                emit_block_aggressive(out, child, depth + 1);
            }
            if !children.is_empty() {
                // Blank line after container's children
            }
        }
        BlockKind::ThematicBreak => {
            out.push_str("---\n\n");
        }
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

pub fn skill_block_prefix_aggressive(st: &SkillBlockType) -> &'static str {
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

fn semantic_block_name_aggressive(bt: &SemanticBlockType) -> &'static str {
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

/// Emit MediaMeta fields as standard LML attributes.
fn emit_media_meta_attrs(out: &mut String, meta: &MediaMeta) {
    if let Some(alt) = &meta.alt {
        out.push(' ');
        emit_attr_pair(out, "alt", alt);
    }
    if let Some(w) = meta.width {
        out.push(' ');
        emit_attr_pair(out, "width", &w.to_string());
    }
    if let Some(h) = meta.height {
        out.push(' ');
        emit_attr_pair(out, "height", &h.to_string());
    }
    if let Some(d) = meta.duration {
        out.push(' ');
        emit_attr_pair(out, "duration", &format!("{}", d));
    }
    if let Some(m) = &meta.mime {
        out.push(' ');
        emit_attr_pair(out, "mime", m);
    }
    if let Some(p) = &meta.poster {
        out.push(' ');
        emit_attr_pair(out, "poster", p);
    }
}

/// Emit MediaMeta fields in aggressive compact format: w=, h=, dur=
/// Mime is not emitted in aggressive mode (derivable from src extension).
fn emit_media_meta_aggressive(out: &mut String, meta: &MediaMeta) {
    if let Some(alt) = &meta.alt {
        out.push_str(", alt=");
        if needs_quotes(alt) {
            out.push('"');
            out.push_str(alt);
            out.push('"');
        } else {
            out.push_str(alt);
        }
    }
    if let Some(w) = meta.width {
        out.push_str(", w=");
        out.push_str(&w.to_string());
    }
    if let Some(h) = meta.height {
        out.push_str(", h=");
        out.push_str(&h.to_string());
    }
    if let Some(d) = meta.duration {
        out.push_str(", dur=");
        out.push_str(&format!("{}", d));
    }
    if let Some(p) = &meta.poster {
        out.push_str(", poster=");
        if needs_quotes(p) {
            out.push('"');
            out.push_str(p);
            out.push('"');
        } else {
            out.push_str(p);
        }
    }
}

// ── Table helpers ────────────────────────────────────────────────────

fn emit_table_rows(out: &mut String, headers: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>]) {
    // Header row
    out.push('|');
    for header in headers {
        out.push(' ');
        emit_inlines_plain(out, header);
        out.push_str(" |");
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
            out.push(' ');
            emit_inlines_plain(out, cell);
            out.push_str(" |");
        }
        out.push('\n');
    }
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
        SkillBlockType::Scenario => "SCENARIO",
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
        SkillBlockType::Scenario => "SCN",
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
