use aif_core::ast::*;
use aif_core::span::Span;
use std::collections::BTreeMap;

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
            block_type,
            attrs,
            title,
            content,
        } => {
            out.push(SEMANTIC_BLOCK);
            out.push(encode_semantic_block_type(block_type));
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
            callout_type,
            attrs,
            content,
        } => {
            out.push(CALLOUT);
            out.push(encode_callout_type(callout_type));
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
        BlockKind::Figure { attrs, caption, src, meta } => {
            out.push(FIGURE);
            encode_attrs(attrs, out);
            if let Some(cap) = caption {
                out.push(1);
                encode_inlines(cap, out);
            } else {
                out.push(0);
            }
            encode_str(src, out);
            encode_media_meta(meta, out);
        }
        BlockKind::Audio { attrs, caption, src, meta } => {
            out.push(AUDIO);
            encode_attrs(attrs, out);
            if let Some(cap) = caption {
                out.push(1);
                encode_inlines(cap, out);
            } else {
                out.push(0);
            }
            encode_str(src, out);
            encode_media_meta(meta, out);
        }
        BlockKind::Video { attrs, caption, src, meta } => {
            out.push(VIDEO);
            encode_attrs(attrs, out);
            if let Some(cap) = caption {
                out.push(1);
                encode_inlines(cap, out);
            } else {
                out.push(0);
            }
            encode_str(src, out);
            encode_media_meta(meta, out);
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
            encode_inlines(text, out);
            encode_str(url, out);
        }
        Inline::Image { alt, src } => {
            out.push(IMAGE);
            encode_str(alt, out);
            encode_str(src, out);
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

fn encode_media_meta(meta: &MediaMeta, out: &mut Vec<u8>) {
    // Presence flags: bit 0=alt, 1=width, 2=height, 3=duration, 4=mime, 5=poster
    let mut flags: u8 = 0;
    if meta.alt.is_some() { flags |= 1; }
    if meta.width.is_some() { flags |= 2; }
    if meta.height.is_some() { flags |= 4; }
    if meta.duration.is_some() { flags |= 8; }
    if meta.mime.is_some() { flags |= 16; }
    if meta.poster.is_some() { flags |= 32; }
    out.push(flags);

    if let Some(alt) = &meta.alt {
        encode_str(alt, out);
    }
    if let Some(w) = meta.width {
        encode_varint(w as usize, out);
    }
    if let Some(h) = meta.height {
        encode_varint(h as usize, out);
    }
    if let Some(d) = meta.duration {
        out.extend_from_slice(&d.to_le_bytes());
    }
    if let Some(m) = &meta.mime {
        encode_str(m, out);
    }
    if let Some(p) = &meta.poster {
        encode_str(p, out);
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
        SkillBlockType::Scenario => SK_SCENARIO,
    }
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

/// Decode a Document from token-optimized binary format.
pub fn decode(data: &[u8]) -> Result<Document, &'static str> {
    if data.len() < 3 {
        return Err("data too short");
    }
    if &data[0..2] != b"AT" {
        return Err("invalid magic bytes");
    }
    if data[2] != 0x01 {
        return Err("unsupported version");
    }
    let mut pos = 3;

    // Metadata
    let (meta_count, n) = decode_varint(&data[pos..])?;
    pos += n;
    let mut metadata = BTreeMap::new();
    for _ in 0..meta_count {
        let (k, n) = decode_str(&data[pos..])?;
        pos += n;
        let (v, n) = decode_str(&data[pos..])?;
        pos += n;
        metadata.insert(k, v);
    }

    // Blocks
    let (block_count, n) = decode_varint(&data[pos..])?;
    pos += n;
    let mut blocks = Vec::with_capacity(block_count);
    for _ in 0..block_count {
        let (block, n) = decode_block(&data[pos..])?;
        pos += n;
        blocks.push(block);
    }

    Ok(Document { metadata, blocks })
}

fn decode_block(data: &[u8]) -> Result<(Block, usize), &'static str> {
    if data.is_empty() {
        return Err("unexpected end of block data");
    }
    let type_id = data[0];
    let mut pos = 1;

    let kind = match type_id {
        PARAGRAPH => {
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            BlockKind::Paragraph { content }
        }
        SECTION => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let (title, n) = decode_inlines(&data[pos..])?;
            pos += n;
            let (child_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut children = Vec::with_capacity(child_count);
            for _ in 0..child_count {
                let (child, n) = decode_block(&data[pos..])?;
                pos += n;
                children.push(child);
            }
            BlockKind::Section {
                attrs,
                title,
                children,
            }
        }
        SKILL_BLOCK => {
            if pos >= data.len() {
                return Err("unexpected end of skill block");
            }
            let skill_type = decode_skill_type(data[pos]);
            pos += 1;
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            // Optional title
            if pos >= data.len() {
                return Err("unexpected end of skill block title flag");
            }
            let title = if data[pos] == 1 {
                pos += 1;
                let (t, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(t)
            } else {
                pos += 1;
                None
            };
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            let (child_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut children = Vec::with_capacity(child_count);
            for _ in 0..child_count {
                let (child, n) = decode_block(&data[pos..])?;
                pos += n;
                children.push(child);
            }
            BlockKind::SkillBlock {
                skill_type,
                attrs,
                title,
                content,
                children,
            }
        }
        SEMANTIC_BLOCK => {
            let block_type_val = decode_semantic_block_type(data[pos]);
            pos += 1;
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let title = if pos < data.len() && data[pos] == 1 {
                pos += 1;
                let (t, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(t)
            } else {
                pos += 1;
                None
            };
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            BlockKind::SemanticBlock {
                block_type: block_type_val,
                attrs,
                title,
                content,
            }
        }
        CALLOUT => {
            let callout_type_val = decode_callout_type(data[pos]);
            pos += 1;
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            BlockKind::Callout {
                callout_type: callout_type_val,
                attrs,
                content,
            }
        }
        CODE_BLOCK => {
            let (lang_str, n) = decode_str(&data[pos..])?;
            pos += n;
            let lang = if lang_str.is_empty() {
                None
            } else {
                Some(lang_str)
            };
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let (code, n) = decode_str(&data[pos..])?;
            pos += n;
            BlockKind::CodeBlock { lang, attrs, code }
        }
        BLOCK_QUOTE => {
            let (child_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut content = Vec::with_capacity(child_count);
            for _ in 0..child_count {
                let (child, n) = decode_block(&data[pos..])?;
                pos += n;
                content.push(child);
            }
            BlockKind::BlockQuote { content }
        }
        LIST => {
            if pos >= data.len() {
                return Err("unexpected end of list");
            }
            let ordered = data[pos] == 1;
            pos += 1;
            let (item_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut items = Vec::with_capacity(item_count);
            for _ in 0..item_count {
                let (content, n) = decode_inlines(&data[pos..])?;
                pos += n;
                let (child_count, n) = decode_varint(&data[pos..])?;
                pos += n;
                let mut children = Vec::with_capacity(child_count);
                for _ in 0..child_count {
                    let (child, n) = decode_block(&data[pos..])?;
                    pos += n;
                    children.push(child);
                }
                items.push(ListItem { content, children });
            }
            BlockKind::List { ordered, items }
        }
        TABLE => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let caption = if pos < data.len() && data[pos] == 1 {
                pos += 1;
                let (cap, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(cap)
            } else {
                pos += 1;
                None
            };
            let (header_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut headers = Vec::with_capacity(header_count);
            for _ in 0..header_count {
                let (h, n) = decode_inlines(&data[pos..])?;
                pos += n;
                headers.push(h);
            }
            let (row_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut rows = Vec::with_capacity(row_count);
            for _ in 0..row_count {
                let (cell_count, n) = decode_varint(&data[pos..])?;
                pos += n;
                let mut row = Vec::with_capacity(cell_count);
                for _ in 0..cell_count {
                    let (cell, n) = decode_inlines(&data[pos..])?;
                    pos += n;
                    row.push(cell);
                }
                rows.push(row);
            }
            BlockKind::Table {
                attrs,
                caption,
                headers,
                rows,
            }
        }
        FIGURE => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let caption = if pos < data.len() && data[pos] == 1 {
                pos += 1;
                let (cap, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(cap)
            } else {
                pos += 1;
                None
            };
            let (src, n) = decode_str(&data[pos..])?;
            pos += n;
            let (meta, n) = decode_media_meta(&data[pos..])?;
            pos += n;
            BlockKind::Figure {
                attrs,
                caption,
                src,
                meta,
            }
        }
        AUDIO => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let caption = if pos < data.len() && data[pos] == 1 {
                pos += 1;
                let (cap, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(cap)
            } else {
                pos += 1;
                None
            };
            let (src, n) = decode_str(&data[pos..])?;
            pos += n;
            let (meta, n) = decode_media_meta(&data[pos..])?;
            pos += n;
            BlockKind::Audio {
                attrs,
                caption,
                src,
                meta,
            }
        }
        VIDEO => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let caption = if pos < data.len() && data[pos] == 1 {
                pos += 1;
                let (cap, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(cap)
            } else {
                pos += 1;
                None
            };
            let (src, n) = decode_str(&data[pos..])?;
            pos += n;
            let (meta, n) = decode_media_meta(&data[pos..])?;
            pos += n;
            BlockKind::Video {
                attrs,
                caption,
                src,
                meta,
            }
        }
        THEMATIC_BREAK => BlockKind::ThematicBreak,
        _ => return Err("unknown block type ID"),
    };

    Ok((
        Block {
            kind,
            span: Span::new(0, 0),
        },
        pos,
    ))
}

fn decode_inlines(data: &[u8]) -> Result<(Vec<Inline>, usize), &'static str> {
    let (count, mut pos) = decode_varint(data)?;
    let mut inlines = Vec::with_capacity(count);
    for _ in 0..count {
        let (inline, n) = decode_inline(&data[pos..])?;
        pos += n;
        inlines.push(inline);
    }
    Ok((inlines, pos))
}

fn decode_inline(data: &[u8]) -> Result<(Inline, usize), &'static str> {
    if data.is_empty() {
        return Err("unexpected end of inline data");
    }
    let type_id = data[0];
    let mut pos = 1;

    let inline = match type_id {
        TEXT => {
            let (text, n) = decode_str(&data[pos..])?;
            pos += n;
            Inline::Text { text }
        }
        EMPHASIS => {
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            Inline::Emphasis { content }
        }
        STRONG => {
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            Inline::Strong { content }
        }
        INLINE_CODE => {
            let (code, n) = decode_str(&data[pos..])?;
            pos += n;
            Inline::InlineCode { code }
        }
        LINK => {
            let (text, n) = decode_inlines(&data[pos..])?;
            pos += n;
            let (url, n) = decode_str(&data[pos..])?;
            pos += n;
            Inline::Link { text, url }
        }
        IMAGE => {
            let (alt, n) = decode_str(&data[pos..])?;
            pos += n;
            let (src, n) = decode_str(&data[pos..])?;
            pos += n;
            Inline::Image { alt, src }
        }
        REFERENCE => {
            let (target, n) = decode_str(&data[pos..])?;
            pos += n;
            Inline::Reference { target }
        }
        FOOTNOTE => {
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            Inline::Footnote { content }
        }
        SOFT_BREAK => Inline::SoftBreak,
        HARD_BREAK => Inline::HardBreak,
        _ => return Err("unknown inline type ID"),
    };

    Ok((inline, pos))
}

fn decode_attrs(data: &[u8]) -> Result<(Attrs, usize), &'static str> {
    if data.is_empty() {
        return Err("unexpected end of attrs");
    }
    let mut pos = 0;
    let id = if data[pos] == 1 {
        pos += 1;
        let (id_str, n) = decode_str(&data[pos..])?;
        pos += n;
        Some(id_str)
    } else {
        pos += 1;
        None
    };
    let (pair_count, n) = decode_varint(&data[pos..])?;
    pos += n;
    let mut pairs = BTreeMap::new();
    for _ in 0..pair_count {
        let (k, n) = decode_str(&data[pos..])?;
        pos += n;
        let (v, n) = decode_str(&data[pos..])?;
        pos += n;
        pairs.insert(k, v);
    }
    Ok((Attrs { id, pairs }, pos))
}

fn decode_media_meta(data: &[u8]) -> Result<(MediaMeta, usize), &'static str> {
    if data.is_empty() {
        return Err("unexpected end of media meta");
    }
    let flags = data[0];
    let mut pos = 1;
    let mut meta = MediaMeta::default();

    if flags & 1 != 0 {
        let (alt, n) = decode_str(&data[pos..])?;
        pos += n;
        meta.alt = Some(alt);
    }
    if flags & 2 != 0 {
        let (w, n) = decode_varint(&data[pos..])?;
        pos += n;
        meta.width = Some(w as u32);
    }
    if flags & 4 != 0 {
        let (h, n) = decode_varint(&data[pos..])?;
        pos += n;
        meta.height = Some(h as u32);
    }
    if flags & 8 != 0 {
        if pos + 8 > data.len() {
            return Err("unexpected end of duration");
        }
        let d = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;
        meta.duration = Some(d);
    }
    if flags & 16 != 0 {
        let (m, n) = decode_str(&data[pos..])?;
        pos += n;
        meta.mime = Some(m);
    }
    if flags & 32 != 0 {
        let (p, n) = decode_str(&data[pos..])?;
        pos += n;
        meta.poster = Some(p);
    }

    Ok((meta, pos))
}

fn encode_semantic_block_type(t: &SemanticBlockType) -> u8 {
    match t {
        SemanticBlockType::Claim => SEM_CLAIM,
        SemanticBlockType::Evidence => SEM_EVIDENCE,
        SemanticBlockType::Definition => SEM_DEFINITION,
        SemanticBlockType::Theorem => SEM_THEOREM,
        SemanticBlockType::Assumption => SEM_ASSUMPTION,
        SemanticBlockType::Result => SEM_RESULT,
        SemanticBlockType::Conclusion => SEM_CONCLUSION,
        SemanticBlockType::Requirement => SEM_REQUIREMENT,
        SemanticBlockType::Recommendation => SEM_RECOMMENDATION,
    }
}

fn decode_semantic_block_type(byte: u8) -> SemanticBlockType {
    match byte {
        SEM_CLAIM => SemanticBlockType::Claim,
        SEM_EVIDENCE => SemanticBlockType::Evidence,
        SEM_DEFINITION => SemanticBlockType::Definition,
        SEM_THEOREM => SemanticBlockType::Theorem,
        SEM_ASSUMPTION => SemanticBlockType::Assumption,
        SEM_RESULT => SemanticBlockType::Result,
        SEM_CONCLUSION => SemanticBlockType::Conclusion,
        SEM_REQUIREMENT => SemanticBlockType::Requirement,
        SEM_RECOMMENDATION => SemanticBlockType::Recommendation,
        _ => SemanticBlockType::Claim, // fallback
    }
}

fn encode_callout_type(t: &CalloutType) -> u8 {
    match t {
        CalloutType::Note => CT_NOTE,
        CalloutType::Warning => CT_WARNING,
        CalloutType::Info => CT_INFO,
        CalloutType::Tip => CT_TIP,
    }
}

fn decode_callout_type(byte: u8) -> CalloutType {
    match byte {
        CT_NOTE => CalloutType::Note,
        CT_WARNING => CalloutType::Warning,
        CT_INFO => CalloutType::Info,
        CT_TIP => CalloutType::Tip,
        _ => CalloutType::Note, // fallback
    }
}

fn decode_skill_type(byte: u8) -> SkillBlockType {
    match byte {
        SK_SKILL => SkillBlockType::Skill,
        SK_STEP => SkillBlockType::Step,
        SK_VERIFY => SkillBlockType::Verify,
        SK_PRECONDITION => SkillBlockType::Precondition,
        SK_OUTPUT_CONTRACT => SkillBlockType::OutputContract,
        SK_DECISION => SkillBlockType::Decision,
        SK_TOOL => SkillBlockType::Tool,
        SK_FALLBACK => SkillBlockType::Fallback,
        SK_RED_FLAG => SkillBlockType::RedFlag,
        SK_EXAMPLE => SkillBlockType::Example,
        SK_SCENARIO => SkillBlockType::Scenario,
        _ => SkillBlockType::Skill, // fallback
    }
}
