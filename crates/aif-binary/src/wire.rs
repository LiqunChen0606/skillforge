// Wire format: postcard-based binary serialization
//
// Postcard does not support `#[serde(tag = "type")]` (internally tagged enums).
// We define mirror types without that attribute and convert to/from the real AST.

use aif_core::ast;
use aif_core::span::Span;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Mirror types (no #[serde(tag = ...)])
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct WireDocument {
    metadata: BTreeMap<String, String>,
    blocks: Vec<WireBlock>,
}

#[derive(Serialize, Deserialize)]
struct WireBlock {
    kind: WireBlockKind,
    span: WireSpan,
}

#[derive(Serialize, Deserialize)]
struct WireSpan {
    start: usize,
    end: usize,
}

#[derive(Serialize, Deserialize)]
enum WireBlockKind {
    Section {
        attrs: WireAttrs,
        title: Vec<WireInline>,
        children: Vec<WireBlock>,
    },
    Paragraph {
        content: Vec<WireInline>,
    },
    SemanticBlock {
        block_type: ast::SemanticBlockType,
        attrs: WireAttrs,
        title: Option<Vec<WireInline>>,
        content: Vec<WireInline>,
    },
    Callout {
        callout_type: ast::CalloutType,
        attrs: WireAttrs,
        content: Vec<WireInline>,
    },
    Table {
        attrs: WireAttrs,
        caption: Option<Vec<WireInline>>,
        headers: Vec<Vec<WireInline>>,
        rows: Vec<Vec<Vec<WireInline>>>,
    },
    Figure {
        attrs: WireAttrs,
        caption: Option<Vec<WireInline>>,
        src: String,
        meta: WireMediaMeta,
    },
    Audio {
        attrs: WireAttrs,
        caption: Option<Vec<WireInline>>,
        src: String,
        meta: WireMediaMeta,
    },
    Video {
        attrs: WireAttrs,
        caption: Option<Vec<WireInline>>,
        src: String,
        meta: WireMediaMeta,
    },
    CodeBlock {
        lang: Option<String>,
        attrs: WireAttrs,
        code: String,
    },
    BlockQuote {
        content: Vec<WireBlock>,
    },
    List {
        ordered: bool,
        items: Vec<WireListItem>,
    },
    SkillBlock {
        skill_type: ast::SkillBlockType,
        attrs: WireAttrs,
        title: Option<Vec<WireInline>>,
        content: Vec<WireInline>,
        children: Vec<WireBlock>,
    },
    ThematicBreak,
}

#[derive(Serialize, Deserialize)]
struct WireListItem {
    content: Vec<WireInline>,
    children: Vec<WireBlock>,
}

#[derive(Serialize, Deserialize)]
struct WireAttrs {
    id: Option<String>,
    pairs: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct WireMediaMeta {
    alt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<f64>,
    mime: Option<String>,
    poster: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum WireInline {
    Text { text: String },
    Emphasis { content: Vec<WireInline> },
    Strong { content: Vec<WireInline> },
    InlineCode { code: String },
    Link { text: Vec<WireInline>, url: String },
    Image { alt: String, src: String },
    Reference { target: String },
    Footnote { content: Vec<WireInline> },
    SoftBreak,
    HardBreak,
}

// ---------------------------------------------------------------------------
// Conversion: AST → Wire
// ---------------------------------------------------------------------------

fn doc_to_wire(doc: &ast::Document) -> WireDocument {
    WireDocument {
        metadata: doc.metadata.clone(),
        blocks: doc.blocks.iter().map(block_to_wire).collect(),
    }
}

fn block_to_wire(b: &ast::Block) -> WireBlock {
    WireBlock {
        kind: blockkind_to_wire(&b.kind),
        span: WireSpan {
            start: b.span.start,
            end: b.span.end,
        },
    }
}

fn blockkind_to_wire(k: &ast::BlockKind) -> WireBlockKind {
    match k {
        ast::BlockKind::Section {
            attrs,
            title,
            children,
        } => WireBlockKind::Section {
            attrs: attrs_to_wire(attrs),
            title: title.iter().map(inline_to_wire).collect(),
            children: children.iter().map(block_to_wire).collect(),
        },
        ast::BlockKind::Paragraph { content } => WireBlockKind::Paragraph {
            content: content.iter().map(inline_to_wire).collect(),
        },
        ast::BlockKind::SemanticBlock {
            block_type,
            attrs,
            title,
            content,
        } => WireBlockKind::SemanticBlock {
            block_type: block_type.clone(),
            attrs: attrs_to_wire(attrs),
            title: title.as_ref().map(|t| t.iter().map(inline_to_wire).collect()),
            content: content.iter().map(inline_to_wire).collect(),
        },
        ast::BlockKind::Callout {
            callout_type,
            attrs,
            content,
        } => WireBlockKind::Callout {
            callout_type: callout_type.clone(),
            attrs: attrs_to_wire(attrs),
            content: content.iter().map(inline_to_wire).collect(),
        },
        ast::BlockKind::Table {
            attrs,
            caption,
            headers,
            rows,
        } => WireBlockKind::Table {
            attrs: attrs_to_wire(attrs),
            caption: caption.as_ref().map(|c| c.iter().map(inline_to_wire).collect()),
            headers: headers
                .iter()
                .map(|h| h.iter().map(inline_to_wire).collect())
                .collect(),
            rows: rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| cell.iter().map(inline_to_wire).collect())
                        .collect()
                })
                .collect(),
        },
        ast::BlockKind::Figure {
            attrs,
            caption,
            src,
            meta,
        } => WireBlockKind::Figure {
            attrs: attrs_to_wire(attrs),
            caption: caption.as_ref().map(|c| c.iter().map(inline_to_wire).collect()),
            src: src.clone(),
            meta: media_meta_to_wire(meta),
        },
        ast::BlockKind::Audio {
            attrs,
            caption,
            src,
            meta,
        } => WireBlockKind::Audio {
            attrs: attrs_to_wire(attrs),
            caption: caption.as_ref().map(|c| c.iter().map(inline_to_wire).collect()),
            src: src.clone(),
            meta: media_meta_to_wire(meta),
        },
        ast::BlockKind::Video {
            attrs,
            caption,
            src,
            meta,
        } => WireBlockKind::Video {
            attrs: attrs_to_wire(attrs),
            caption: caption.as_ref().map(|c| c.iter().map(inline_to_wire).collect()),
            src: src.clone(),
            meta: media_meta_to_wire(meta),
        },
        ast::BlockKind::CodeBlock { lang, attrs, code } => WireBlockKind::CodeBlock {
            lang: lang.clone(),
            attrs: attrs_to_wire(attrs),
            code: code.clone(),
        },
        ast::BlockKind::BlockQuote { content } => WireBlockKind::BlockQuote {
            content: content.iter().map(block_to_wire).collect(),
        },
        ast::BlockKind::List { ordered, items } => WireBlockKind::List {
            ordered: *ordered,
            items: items.iter().map(listitem_to_wire).collect(),
        },
        ast::BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        } => WireBlockKind::SkillBlock {
            skill_type: skill_type.clone(),
            attrs: attrs_to_wire(attrs),
            title: title.as_ref().map(|t| t.iter().map(inline_to_wire).collect()),
            content: content.iter().map(inline_to_wire).collect(),
            children: children.iter().map(block_to_wire).collect(),
        },
        ast::BlockKind::ThematicBreak => WireBlockKind::ThematicBreak,
    }
}

fn attrs_to_wire(a: &ast::Attrs) -> WireAttrs {
    WireAttrs {
        id: a.id.clone(),
        pairs: a.pairs.clone(),
    }
}

fn media_meta_to_wire(m: &ast::MediaMeta) -> WireMediaMeta {
    WireMediaMeta {
        alt: m.alt.clone(),
        width: m.width,
        height: m.height,
        duration: m.duration,
        mime: m.mime.clone(),
        poster: m.poster.clone(),
    }
}

fn inline_to_wire(i: &ast::Inline) -> WireInline {
    match i {
        ast::Inline::Text { text } => WireInline::Text { text: text.clone() },
        ast::Inline::Emphasis { content } => WireInline::Emphasis {
            content: content.iter().map(inline_to_wire).collect(),
        },
        ast::Inline::Strong { content } => WireInline::Strong {
            content: content.iter().map(inline_to_wire).collect(),
        },
        ast::Inline::InlineCode { code } => WireInline::InlineCode { code: code.clone() },
        ast::Inline::Link { text, url } => WireInline::Link {
            text: text.iter().map(inline_to_wire).collect(),
            url: url.clone(),
        },
        ast::Inline::Image { alt, src } => WireInline::Image {
            alt: alt.clone(),
            src: src.clone(),
        },
        ast::Inline::Reference { target } => WireInline::Reference {
            target: target.clone(),
        },
        ast::Inline::Footnote { content } => WireInline::Footnote {
            content: content.iter().map(inline_to_wire).collect(),
        },
        ast::Inline::SoftBreak => WireInline::SoftBreak,
        ast::Inline::HardBreak => WireInline::HardBreak,
    }
}

fn listitem_to_wire(li: &ast::ListItem) -> WireListItem {
    WireListItem {
        content: li.content.iter().map(inline_to_wire).collect(),
        children: li.children.iter().map(block_to_wire).collect(),
    }
}

// ---------------------------------------------------------------------------
// Conversion: Wire → AST
// ---------------------------------------------------------------------------

fn wire_to_doc(w: WireDocument) -> ast::Document {
    ast::Document {
        metadata: w.metadata,
        blocks: w.blocks.into_iter().map(wire_to_block).collect(),
    }
}

fn wire_to_block(b: WireBlock) -> ast::Block {
    ast::Block {
        kind: wire_to_blockkind(b.kind),
        span: Span::new(b.span.start, b.span.end),
    }
}

fn wire_to_blockkind(k: WireBlockKind) -> ast::BlockKind {
    match k {
        WireBlockKind::Section {
            attrs,
            title,
            children,
        } => ast::BlockKind::Section {
            attrs: wire_to_attrs(attrs),
            title: title.into_iter().map(wire_to_inline).collect(),
            children: children.into_iter().map(wire_to_block).collect(),
        },
        WireBlockKind::Paragraph { content } => ast::BlockKind::Paragraph {
            content: content.into_iter().map(wire_to_inline).collect(),
        },
        WireBlockKind::SemanticBlock {
            block_type,
            attrs,
            title,
            content,
        } => ast::BlockKind::SemanticBlock {
            block_type,
            attrs: wire_to_attrs(attrs),
            title: title.map(|t| t.into_iter().map(wire_to_inline).collect()),
            content: content.into_iter().map(wire_to_inline).collect(),
        },
        WireBlockKind::Callout {
            callout_type,
            attrs,
            content,
        } => ast::BlockKind::Callout {
            callout_type,
            attrs: wire_to_attrs(attrs),
            content: content.into_iter().map(wire_to_inline).collect(),
        },
        WireBlockKind::Table {
            attrs,
            caption,
            headers,
            rows,
        } => ast::BlockKind::Table {
            attrs: wire_to_attrs(attrs),
            caption: caption.map(|c| c.into_iter().map(wire_to_inline).collect()),
            headers: headers
                .into_iter()
                .map(|h| h.into_iter().map(wire_to_inline).collect())
                .collect(),
            rows: rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|cell| cell.into_iter().map(wire_to_inline).collect())
                        .collect()
                })
                .collect(),
        },
        WireBlockKind::Figure {
            attrs,
            caption,
            src,
            meta,
        } => ast::BlockKind::Figure {
            attrs: wire_to_attrs(attrs),
            caption: caption.map(|c| c.into_iter().map(wire_to_inline).collect()),
            src,
            meta: wire_to_media_meta(meta),
        },
        WireBlockKind::Audio {
            attrs,
            caption,
            src,
            meta,
        } => ast::BlockKind::Audio {
            attrs: wire_to_attrs(attrs),
            caption: caption.map(|c| c.into_iter().map(wire_to_inline).collect()),
            src,
            meta: wire_to_media_meta(meta),
        },
        WireBlockKind::Video {
            attrs,
            caption,
            src,
            meta,
        } => ast::BlockKind::Video {
            attrs: wire_to_attrs(attrs),
            caption: caption.map(|c| c.into_iter().map(wire_to_inline).collect()),
            src,
            meta: wire_to_media_meta(meta),
        },
        WireBlockKind::CodeBlock { lang, attrs, code } => ast::BlockKind::CodeBlock {
            lang,
            attrs: wire_to_attrs(attrs),
            code,
        },
        WireBlockKind::BlockQuote { content } => ast::BlockKind::BlockQuote {
            content: content.into_iter().map(wire_to_block).collect(),
        },
        WireBlockKind::List { ordered, items } => ast::BlockKind::List {
            ordered,
            items: items.into_iter().map(wire_to_listitem).collect(),
        },
        WireBlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        } => ast::BlockKind::SkillBlock {
            skill_type,
            attrs: wire_to_attrs(attrs),
            title: title.map(|t| t.into_iter().map(wire_to_inline).collect()),
            content: content.into_iter().map(wire_to_inline).collect(),
            children: children.into_iter().map(wire_to_block).collect(),
        },
        WireBlockKind::ThematicBreak => ast::BlockKind::ThematicBreak,
    }
}

fn wire_to_attrs(a: WireAttrs) -> ast::Attrs {
    ast::Attrs {
        id: a.id,
        pairs: a.pairs,
    }
}

fn wire_to_media_meta(m: WireMediaMeta) -> ast::MediaMeta {
    ast::MediaMeta {
        alt: m.alt,
        width: m.width,
        height: m.height,
        duration: m.duration,
        mime: m.mime,
        poster: m.poster,
    }
}

fn wire_to_inline(i: WireInline) -> ast::Inline {
    match i {
        WireInline::Text { text } => ast::Inline::Text { text },
        WireInline::Emphasis { content } => ast::Inline::Emphasis {
            content: content.into_iter().map(wire_to_inline).collect(),
        },
        WireInline::Strong { content } => ast::Inline::Strong {
            content: content.into_iter().map(wire_to_inline).collect(),
        },
        WireInline::InlineCode { code } => ast::Inline::InlineCode { code },
        WireInline::Link { text, url } => ast::Inline::Link {
            text: text.into_iter().map(wire_to_inline).collect(),
            url,
        },
        WireInline::Image { alt, src } => ast::Inline::Image { alt, src },
        WireInline::Reference { target } => ast::Inline::Reference { target },
        WireInline::Footnote { content } => ast::Inline::Footnote {
            content: content.into_iter().map(wire_to_inline).collect(),
        },
        WireInline::SoftBreak => ast::Inline::SoftBreak,
        WireInline::HardBreak => ast::Inline::HardBreak,
    }
}

fn wire_to_listitem(li: WireListItem) -> ast::ListItem {
    ast::ListItem {
        content: li.content.into_iter().map(wire_to_inline).collect(),
        children: li.children.into_iter().map(wire_to_block).collect(),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Encode a Document to compact binary (postcard wire format).
pub fn encode(doc: &ast::Document) -> Vec<u8> {
    let wire = doc_to_wire(doc);
    postcard::to_allocvec(&wire).expect("postcard serialization failed")
}

/// Decode a Document from binary wire format.
pub fn decode(bytes: &[u8]) -> Result<ast::Document, postcard::Error> {
    let wire: WireDocument = postcard::from_bytes(bytes)?;
    Ok(wire_to_doc(wire))
}
