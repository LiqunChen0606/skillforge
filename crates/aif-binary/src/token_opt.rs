use aif_core::ast::*;

use crate::dictionary::*;

/// Encode a Document in token-optimized binary format.
/// Layout: magic(2) + version(1) + metadata + blocks
pub fn encode(doc: &Document) -> Vec<u8> {
    let mut out = Vec::new();
    // Magic bytes: "AT" (AIF Token-optimized)
    out.extend_from_slice(b"AT");
    // Version byte
    out.push(0x01);

    // Metadata: count + (key, value) pairs
    encode_varint(doc.metadata.len(), &mut out);
    for (k, v) in &doc.metadata {
        encode_str(k, &mut out);
        encode_str(v, &mut out);
    }

    // Blocks
    encode_varint(doc.blocks.len(), &mut out);
    for block in &doc.blocks {
        encode_block(block, &mut out);
    }

    out
}

fn encode_block(block: &Block, out: &mut Vec<u8>) {
    match &block.kind {
        BlockKind::Paragraph { content } => {
            out.push(PARAGRAPH);
            encode_inlines(content, out);
        }
        BlockKind::Section {
            attrs,
            title,
            children,
        } => {
            out.push(SECTION);
            encode_attrs(attrs, out);
            encode_inlines(title, out);
            encode_varint(children.len(), out);
            for child in children {
                encode_block(child, out);
            }
        }
        BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        } => {
            out.push(SKILL_BLOCK);
            out.push(skill_type_id(skill_type));
            encode_attrs(attrs, out);
            // Optional title
            if let Some(t) = title {
                out.push(1);
                encode_inlines(t, out);
            } else {
                out.push(0);
            }
            encode_inlines(content, out);
            encode_varint(children.len(), out);
            for child in children {
                encode_block(child, out);
            }
        }
        BlockKind::SemanticBlock {
            block_type: _,
            attrs,
            title,
            content,
        } => {
            out.push(SEMANTIC_BLOCK);
            encode_attrs(attrs, out);
            if let Some(t) = title {
                out.push(1);
                encode_inlines(t, out);
            } else {
                out.push(0);
            }
            encode_inlines(content, out);
        }
        BlockKind::Callout {
            callout_type: _,
            attrs,
            content,
        } => {
            out.push(CALLOUT);
            encode_attrs(attrs, out);
            encode_inlines(content, out);
        }
        BlockKind::CodeBlock { lang, attrs, code } => {
            out.push(CODE_BLOCK);
            encode_str(lang.as_deref().unwrap_or(""), out);
            encode_attrs(attrs, out);
            encode_str(code, out);
        }
        BlockKind::BlockQuote { content } => {
            out.push(BLOCK_QUOTE);
            encode_varint(content.len(), out);
            for child in content {
                encode_block(child, out);
            }
        }
        BlockKind::List { ordered, items } => {
            out.push(LIST);
            out.push(if *ordered { 1 } else { 0 });
            encode_varint(items.len(), out);
            for item in items {
                encode_inlines(&item.content, out);
                encode_varint(item.children.len(), out);
                for child in &item.children {
                    encode_block(child, out);
                }
            }
        }
        BlockKind::Table {
            attrs,
            caption,
            headers,
            rows,
        } => {
            out.push(TABLE);
            encode_attrs(attrs, out);
            // caption is Option<Vec<Inline>>
            if let Some(cap) = caption {
                out.push(1);
                encode_inlines(cap, out);
            } else {
                out.push(0);
            }
            encode_varint(headers.len(), out);
            for h in headers {
                encode_inlines(h, out);
            }
            encode_varint(rows.len(), out);
            for row in rows {
                encode_varint(row.len(), out);
                for cell in row {
                    encode_inlines(cell, out);
                }
            }
        }
        BlockKind::Figure { attrs, caption, src } => {
            out.push(FIGURE);
            encode_attrs(attrs, out);
            // caption is Option<Vec<Inline>>
            if let Some(cap) = caption {
                out.push(1);
                encode_inlines(cap, out);
            } else {
                out.push(0);
            }
            encode_str(src, out);
        }
        BlockKind::ThematicBreak => {
            out.push(THEMATIC_BREAK);
        }
    }
}

fn encode_inlines(inlines: &[Inline], out: &mut Vec<u8>) {
    encode_varint(inlines.len(), out);
    for inline in inlines {
        encode_inline(inline, out);
    }
}

fn encode_inline(inline: &Inline, out: &mut Vec<u8>) {
    match inline {
        Inline::Text { text } => {
            out.push(TEXT);
            encode_str(text, out);
        }
        Inline::Emphasis { content } => {
            out.push(EMPHASIS);
            encode_inlines(content, out);
        }
        Inline::Strong { content } => {
            out.push(STRONG);
            encode_inlines(content, out);
        }
        Inline::InlineCode { code } => {
            out.push(INLINE_CODE);
            encode_str(code, out);
        }
        Inline::Link { text, url } => {
            out.push(LINK);
            // text is Vec<Inline>, not String
            encode_inlines(text, out);
            encode_str(url, out);
        }
        Inline::Reference { target } => {
            out.push(REFERENCE);
            encode_str(target, out);
        }
        Inline::Footnote { content } => {
            out.push(FOOTNOTE);
            encode_inlines(content, out);
        }
        Inline::SoftBreak => out.push(SOFT_BREAK),
        Inline::HardBreak => out.push(HARD_BREAK),
    }
}

fn encode_attrs(attrs: &Attrs, out: &mut Vec<u8>) {
    // id: optional string
    if let Some(id) = &attrs.id {
        out.push(1);
        encode_str(id, out);
    } else {
        out.push(0);
    }
    // pairs: count + (key, value)
    encode_varint(attrs.pairs.len(), out);
    for (k, v) in &attrs.pairs {
        encode_str(k, out);
        encode_str(v, out);
    }
}

fn skill_type_id(st: &SkillBlockType) -> u8 {
    match st {
        SkillBlockType::Skill => SK_SKILL,
        SkillBlockType::Step => SK_STEP,
        SkillBlockType::Verify => SK_VERIFY,
        SkillBlockType::Precondition => SK_PRECONDITION,
        SkillBlockType::OutputContract => SK_OUTPUT_CONTRACT,
        SkillBlockType::Decision => SK_DECISION,
        SkillBlockType::Tool => SK_TOOL,
        SkillBlockType::Fallback => SK_FALLBACK,
        SkillBlockType::RedFlag => SK_RED_FLAG,
        SkillBlockType::Example => SK_EXAMPLE,
    }
}
