# Phase 2: All 9 Tasks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all 9 Phase 2 features across 3 tiers: core format evolution, ecosystem, and intelligence/measurement.

**Architecture:** Each task is independent — different crates/modules, no shared state. All follow TDD. Tasks can run in parallel.

**Tech Stack:** Rust, serde, postcard, SHA-256, Python (benchmarks)

---

## Task 1: Token-Optimized Binary Decode

**Files:**
- Modify: `crates/aif-binary/src/token_opt.rs`
- Modify: `crates/aif-binary/src/lib.rs`
- Modify: `crates/aif-binary/src/dictionary.rs`
- Test: `crates/aif-binary/tests/token_opt_roundtrip.rs`

Currently `token_opt.rs` only has `encode()`. Add `decode()` for full roundtrip.

- [ ] **Step 1: Write failing roundtrip test**

Create `crates/aif-binary/tests/token_opt_roundtrip.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_binary::{render_token_optimized, decode_token_optimized};

#[test]
fn roundtrip_simple_paragraph() {
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Test".into());
    doc.blocks.push(Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: "Hello world".into() }],
        },
        span: Span::new(0, 11),
    });
    let bytes = render_token_optimized(&doc);
    let decoded = decode_token_optimized(&bytes).unwrap();
    assert_eq!(decoded.metadata.get("title").unwrap(), "Test");
    assert_eq!(decoded.blocks.len(), 1);
    match &decoded.blocks[0].kind {
        BlockKind::Paragraph { content } => {
            assert_eq!(content.len(), 1);
            match &content[0] {
                Inline::Text { text } => assert_eq!(text, "Hello world"),
                _ => panic!("expected Text"),
            }
        }
        _ => panic!("expected Paragraph"),
    }
}

#[test]
fn roundtrip_skill_block() {
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Do the thing".into() }],
            children: vec![],
        },
        span: Span::new(0, 20),
    };
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "test-skill".into());
                    a
                },
                title: Some(vec![Inline::Text { text: "Test Skill".into() }]),
                content: vec![],
                children: vec![step],
            },
            span: Span::new(0, 50),
        }],
    };
    let bytes = render_token_optimized(&doc);
    let decoded = decode_token_optimized(&bytes).unwrap();
    assert_eq!(decoded.blocks.len(), 1);
    match &decoded.blocks[0].kind {
        BlockKind::SkillBlock { skill_type, attrs, children, .. } => {
            assert_eq!(*skill_type, SkillBlockType::Skill);
            assert_eq!(attrs.get("name"), Some("test-skill"));
            assert_eq!(children.len(), 1);
        }
        _ => panic!("expected SkillBlock"),
    }
}

#[test]
fn roundtrip_all_block_types() {
    let doc = Document {
        metadata: {
            let mut m = std::collections::BTreeMap::new();
            m.insert("title".into(), "All blocks".into());
            m
        },
        blocks: vec![
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![
                        Inline::Text { text: "plain ".into() },
                        Inline::Strong { content: vec![Inline::Text { text: "bold".into() }] },
                        Inline::Emphasis { content: vec![Inline::Text { text: "italic".into() }] },
                        Inline::InlineCode { code: "code".into() },
                        Inline::Link { text: vec![Inline::Text { text: "link".into() }], url: "http://example.com".into() },
                        Inline::Reference { target: "ref1".into() },
                        Inline::SoftBreak,
                        Inline::HardBreak,
                    ],
                },
                span: Span::new(0, 10),
            },
            Block {
                kind: BlockKind::CodeBlock {
                    lang: Some("rust".into()),
                    attrs: Attrs::new(),
                    code: "fn main() {}".into(),
                },
                span: Span::new(10, 30),
            },
            Block {
                kind: BlockKind::List {
                    ordered: true,
                    items: vec![ListItem {
                        content: vec![Inline::Text { text: "item 1".into() }],
                        children: vec![],
                    }],
                },
                span: Span::new(30, 40),
            },
            Block {
                kind: BlockKind::BlockQuote {
                    content: vec![Block {
                        kind: BlockKind::Paragraph {
                            content: vec![Inline::Text { text: "quoted".into() }],
                        },
                        span: Span::new(40, 50),
                    }],
                },
                span: Span::new(40, 55),
            },
            Block {
                kind: BlockKind::ThematicBreak,
                span: Span::new(55, 60),
            },
            Block {
                kind: BlockKind::Table {
                    attrs: Attrs::new(),
                    caption: Some(vec![Inline::Text { text: "My table".into() }]),
                    headers: vec![vec![Inline::Text { text: "Col1".into() }]],
                    rows: vec![vec![vec![Inline::Text { text: "Val1".into() }]]],
                },
                span: Span::new(60, 80),
            },
            Block {
                kind: BlockKind::Figure {
                    attrs: Attrs::new(),
                    caption: Some(vec![Inline::Text { text: "A figure".into() }]),
                    src: "img.png".into(),
                },
                span: Span::new(80, 90),
            },
            Block {
                kind: BlockKind::SemanticBlock {
                    block_type: SemanticBlockType::Claim,
                    attrs: Attrs::new(),
                    title: Some(vec![Inline::Text { text: "My claim".into() }]),
                    content: vec![Inline::Text { text: "Claim content".into() }],
                },
                span: Span::new(90, 100),
            },
            Block {
                kind: BlockKind::Callout {
                    callout_type: CalloutType::Warning,
                    attrs: Attrs::new(),
                    content: vec![Inline::Text { text: "Be careful".into() }],
                },
                span: Span::new(100, 110),
            },
        ],
    };
    let bytes = render_token_optimized(&doc);
    let decoded = decode_token_optimized(&bytes).unwrap();
    // Span info is lost in token-opt format, so compare block count and kinds
    assert_eq!(decoded.blocks.len(), doc.blocks.len());
    assert_eq!(decoded.metadata, doc.metadata);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-binary --test token_opt_roundtrip`
Expected: FAIL — `decode_token_optimized` not found

- [ ] **Step 3: Add decode_varint and decode_str to dictionary.rs**

Add to `crates/aif-binary/src/dictionary.rs`:

```rust
/// Decode a varint (LEB128) from a byte slice, returning (value, bytes_consumed).
pub fn decode_varint(data: &[u8]) -> Result<(usize, usize), &'static str> {
    let mut result: usize = 0;
    let mut shift = 0;
    for (i, &byte) in data.iter().enumerate() {
        result |= ((byte & 0x7F) as usize) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
        if shift >= 64 {
            return Err("varint overflow");
        }
    }
    Err("unexpected end of varint")
}

/// Decode a length-prefixed UTF-8 string from a byte slice.
pub fn decode_str(data: &[u8]) -> Result<(String, usize), &'static str> {
    let (len, consumed) = decode_varint(data)?;
    let end = consumed + len;
    if data.len() < end {
        return Err("unexpected end of string");
    }
    let s = std::str::from_utf8(&data[consumed..end]).map_err(|_| "invalid utf-8")?;
    Ok((s.to_string(), end))
}
```

- [ ] **Step 4: Implement decode() in token_opt.rs**

Add to `crates/aif-binary/src/token_opt.rs`:

```rust
use crate::dictionary::{decode_varint, decode_str};

/// Decode a Document from token-optimized binary format.
pub fn decode(bytes: &[u8]) -> Result<Document, Box<dyn std::error::Error>> {
    if bytes.len() < 3 || &bytes[0..2] != b"AT" {
        return Err("invalid magic bytes".into());
    }
    let _version = bytes[2];
    let mut pos = 3;

    // Metadata
    let (meta_count, n) = decode_varint(&bytes[pos..])?;
    pos += n;
    let mut metadata = std::collections::BTreeMap::new();
    for _ in 0..meta_count {
        let (key, n) = decode_str(&bytes[pos..])?;
        pos += n;
        let (val, n) = decode_str(&bytes[pos..])?;
        pos += n;
        metadata.insert(key, val);
    }

    // Blocks
    let (block_count, n) = decode_varint(&bytes[pos..])?;
    pos += n;
    let mut blocks = Vec::new();
    for _ in 0..block_count {
        let (block, n) = decode_block(&bytes[pos..])?;
        pos += n;
        blocks.push(block);
    }

    Ok(Document { metadata, blocks })
}

fn decode_block(data: &[u8]) -> Result<(Block, usize), Box<dyn std::error::Error>> {
    if data.is_empty() {
        return Err("unexpected end of block".into());
    }
    let tag = data[0];
    let mut pos = 1;
    let kind = match tag {
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
            let mut children = Vec::new();
            for _ in 0..child_count {
                let (child, n) = decode_block(&data[pos..])?;
                pos += n;
                children.push(child);
            }
            BlockKind::Section { attrs, title, children }
        }
        SKILL_BLOCK => {
            let skill_type = decode_skill_type(data[pos]);
            pos += 1;
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let has_title = data[pos];
            pos += 1;
            let title = if has_title == 1 {
                let (t, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(t)
            } else {
                None
            };
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            let (child_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut children = Vec::new();
            for _ in 0..child_count {
                let (child, n) = decode_block(&data[pos..])?;
                pos += n;
                children.push(child);
            }
            BlockKind::SkillBlock { skill_type, attrs, title, content, children }
        }
        SEMANTIC_BLOCK => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let has_title = data[pos];
            pos += 1;
            let title = if has_title == 1 {
                let (t, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(t)
            } else {
                None
            };
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            BlockKind::SemanticBlock {
                block_type: SemanticBlockType::Claim, // default; type info not stored
                attrs, title, content,
            }
        }
        CALLOUT => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let (content, n) = decode_inlines(&data[pos..])?;
            pos += n;
            BlockKind::Callout {
                callout_type: CalloutType::Note, // default; type info not stored
                attrs, content,
            }
        }
        CODE_BLOCK => {
            let (lang_str, n) = decode_str(&data[pos..])?;
            pos += n;
            let lang = if lang_str.is_empty() { None } else { Some(lang_str) };
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let (code, n) = decode_str(&data[pos..])?;
            pos += n;
            BlockKind::CodeBlock { lang, attrs, code }
        }
        BLOCK_QUOTE => {
            let (child_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut content = Vec::new();
            for _ in 0..child_count {
                let (child, n) = decode_block(&data[pos..])?;
                pos += n;
                content.push(child);
            }
            BlockKind::BlockQuote { content }
        }
        LIST => {
            let ordered = data[pos] == 1;
            pos += 1;
            let (item_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut items = Vec::new();
            for _ in 0..item_count {
                let (item_content, n) = decode_inlines(&data[pos..])?;
                pos += n;
                let (child_count, n) = decode_varint(&data[pos..])?;
                pos += n;
                let mut children = Vec::new();
                for _ in 0..child_count {
                    let (child, n) = decode_block(&data[pos..])?;
                    pos += n;
                    children.push(child);
                }
                items.push(ListItem { content: item_content, children });
            }
            BlockKind::List { ordered, items }
        }
        TABLE => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let has_caption = data[pos];
            pos += 1;
            let caption = if has_caption == 1 {
                let (c, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(c)
            } else {
                None
            };
            let (header_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut headers = Vec::new();
            for _ in 0..header_count {
                let (h, n) = decode_inlines(&data[pos..])?;
                pos += n;
                headers.push(h);
            }
            let (row_count, n) = decode_varint(&data[pos..])?;
            pos += n;
            let mut rows = Vec::new();
            for _ in 0..row_count {
                let (cell_count, n) = decode_varint(&data[pos..])?;
                pos += n;
                let mut row = Vec::new();
                for _ in 0..cell_count {
                    let (cell, n) = decode_inlines(&data[pos..])?;
                    pos += n;
                    row.push(cell);
                }
                rows.push(row);
            }
            BlockKind::Table { attrs, caption, headers, rows }
        }
        FIGURE => {
            let (attrs, n) = decode_attrs(&data[pos..])?;
            pos += n;
            let has_caption = data[pos];
            pos += 1;
            let caption = if has_caption == 1 {
                let (c, n) = decode_inlines(&data[pos..])?;
                pos += n;
                Some(c)
            } else {
                None
            };
            let (src, n) = decode_str(&data[pos..])?;
            pos += n;
            BlockKind::Figure { attrs, caption, src }
        }
        THEMATIC_BREAK => BlockKind::ThematicBreak,
        _ => return Err(format!("unknown block tag: 0x{:02x}", tag).into()),
    };
    Ok((Block { kind, span: Span::new(0, 0) }, pos))
}

fn decode_inlines(data: &[u8]) -> Result<(Vec<Inline>, usize), Box<dyn std::error::Error>> {
    let (count, mut pos) = decode_varint(data)?;
    let mut inlines = Vec::new();
    for _ in 0..count {
        let (inline, n) = decode_inline(&data[pos..])?;
        pos += n;
        inlines.push(inline);
    }
    Ok((inlines, pos))
}

fn decode_inline(data: &[u8]) -> Result<(Inline, usize), Box<dyn std::error::Error>> {
    if data.is_empty() {
        return Err("unexpected end of inline".into());
    }
    let tag = data[0];
    let mut pos = 1;
    let inline = match tag {
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
        _ => return Err(format!("unknown inline tag: 0x{:02x}", tag).into()),
    };
    Ok((inline, pos))
}

fn decode_attrs(data: &[u8]) -> Result<(Attrs, usize), Box<dyn std::error::Error>> {
    let mut pos = 0;
    let has_id = data[pos];
    pos += 1;
    let id = if has_id == 1 {
        let (id_str, n) = decode_str(&data[pos..])?;
        pos += n;
        Some(id_str)
    } else {
        None
    };
    let (pair_count, n) = decode_varint(&data[pos..])?;
    pos += n;
    let mut pairs = std::collections::BTreeMap::new();
    for _ in 0..pair_count {
        let (k, n) = decode_str(&data[pos..])?;
        pos += n;
        let (v, n) = decode_str(&data[pos..])?;
        pos += n;
        pairs.insert(k, v);
    }
    Ok((Attrs { id, pairs }, pos))
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
        _ => SkillBlockType::Step, // fallback
    }
}
```

- [ ] **Step 5: Export decode from lib.rs**

Add to `crates/aif-binary/src/lib.rs`:
```rust
pub use token_opt::decode as decode_token_optimized;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p aif-binary --test token_opt_roundtrip`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/aif-binary/
git commit -m "feat(binary): add token-optimized decode for full roundtrip"
```

---

## Task 2: LML+Binary Hybrid Format

**Files:**
- Create: `crates/aif-lml/src/hybrid.rs`
- Modify: `crates/aif-lml/src/lib.rs`
- Modify: `crates/aif-lml/Cargo.toml`
- Modify: `crates/aif-cli/src/main.rs`
- Test: `crates/aif-lml/tests/hybrid.rs`

A hybrid format: LML structural tags (readable) with base64-encoded binary payloads for large text content. Tags stay human-readable, content is compressed.

- [ ] **Step 1: Write failing test**

Create `crates/aif-lml/tests/hybrid.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_lml::render_lml_hybrid;

#[test]
fn hybrid_preserves_tags_with_binary_content() {
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Test".into());
    doc.blocks.push(Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Do something important".into() }],
            children: vec![],
        },
        span: Span::new(0, 30),
    });
    let output = render_lml_hybrid(&doc);
    // Tags should be readable LML
    assert!(output.contains("@step"));
    // Content should be base64-encoded
    assert!(output.contains("~b64:"));
}

#[test]
fn hybrid_small_content_stays_plain() {
    let mut doc = Document::new();
    doc.blocks.push(Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: "Hi".into() }],
        },
        span: Span::new(0, 2),
    });
    let output = render_lml_hybrid(&doc);
    // Short content should NOT be base64-encoded (threshold: 50 chars)
    assert!(output.contains("Hi"));
    assert!(!output.contains("~b64:"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-lml --test hybrid`
Expected: FAIL — `render_lml_hybrid` not found

- [ ] **Step 3: Implement hybrid.rs**

Create `crates/aif-lml/src/hybrid.rs`:

```rust
use aif_core::ast::*;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

const BINARY_THRESHOLD: usize = 50;

pub fn emit_lml_hybrid(doc: &Document) -> String {
    let mut out = String::new();
    for (key, value) in &doc.metadata {
        out.push('#');
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }
    for block in &doc.blocks {
        emit_block(&mut out, block);
    }
    out
}

fn emit_block(out: &mut String, block: &Block) {
    match &block.kind {
        BlockKind::Paragraph { content } => {
            emit_content_maybe_binary(out, content);
            out.push_str("\n\n");
        }
        BlockKind::Section { attrs: _, title, children } => {
            out.push_str("# ");
            emit_inlines_plain(out, title);
            out.push('\n');
            for child in children {
                emit_block(out, child);
            }
        }
        BlockKind::SkillBlock { skill_type, attrs, title, content, children } => {
            out.push_str(skill_prefix(skill_type));
            emit_attrs(out, attrs);
            if let Some(t) = title {
                out.push_str(": ");
                emit_inlines_plain(out, t);
            }
            if !content.is_empty() {
                out.push(' ');
                emit_content_maybe_binary(out, content);
            }
            out.push('\n');
            for child in children {
                emit_block(out, child);
            }
        }
        BlockKind::CodeBlock { lang, attrs: _, code } => {
            out.push_str("```");
            if let Some(l) = lang { out.push_str(l); }
            out.push('\n');
            out.push_str(code);
            if !code.ends_with('\n') { out.push('\n'); }
            out.push_str("```\n\n");
        }
        BlockKind::SemanticBlock { block_type: _, attrs: _, title, content } => {
            if let Some(t) = title {
                emit_inlines_plain(out, t);
                out.push_str(": ");
            }
            emit_content_maybe_binary(out, content);
            out.push_str("\n\n");
        }
        BlockKind::Callout { callout_type: _, attrs: _, content } => {
            out.push_str("> ");
            emit_content_maybe_binary(out, content);
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
            }
            out.push('\n');
        }
        BlockKind::BlockQuote { content } => {
            for child in content {
                out.push_str("> ");
                emit_block(out, child);
            }
        }
        BlockKind::Table { .. } => out.push_str("[TABLE]\n\n"),
        BlockKind::Figure { src, .. } => {
            out.push_str(&format!("[FIGURE src={}]\n\n", src));
        }
        BlockKind::ThematicBreak => out.push_str("---\n\n"),
    }
}

fn emit_content_maybe_binary(out: &mut String, inlines: &[Inline]) {
    let plain = inlines_to_string(inlines);
    if plain.len() > BINARY_THRESHOLD {
        out.push_str("~b64:");
        out.push_str(&B64.encode(plain.as_bytes()));
    } else {
        out.push_str(&plain);
    }
}

fn inlines_to_string(inlines: &[Inline]) -> String {
    let mut s = String::new();
    emit_inlines_plain(&mut s, inlines);
    s
}

fn emit_inlines_plain(out: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
                emit_inlines_plain(out, content);
            }
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
            Inline::SoftBreak => out.push(' '),
            Inline::HardBreak => out.push('\n'),
        }
    }
}

fn emit_attrs(out: &mut String, attrs: &Attrs) {
    let has_content = attrs.id.is_some() || !attrs.pairs.is_empty();
    if !has_content { return; }
    out.push('(');
    let mut first = true;
    if let Some(id) = &attrs.id {
        out.push_str("id=");
        out.push_str(id);
        first = false;
    }
    for (k, v) in &attrs.pairs {
        if !first { out.push_str(", "); }
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
    }
}
```

- [ ] **Step 4: Add base64 dep to aif-lml Cargo.toml**

Add to `crates/aif-lml/Cargo.toml` under `[dependencies]`:
```toml
base64 = "0.22"
```

- [ ] **Step 5: Export from lib.rs**

Add to `crates/aif-lml/src/lib.rs`:
```rust
mod hybrid;

pub fn render_lml_hybrid(doc: &Document) -> String {
    hybrid::emit_lml_hybrid(doc)
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p aif-lml --test hybrid`
Expected: PASS

- [ ] **Step 7: Add CLI format `lml-hybrid`**

In `crates/aif-cli/src/main.rs`, add `"lml-hybrid"` to the compile format match:
```rust
"lml-hybrid" => aif_lml::render_lml_hybrid(&doc),
```
And similarly for skill import format.

- [ ] **Step 8: Commit**

```bash
git add crates/aif-lml/ crates/aif-cli/
git commit -m "feat(lml): add hybrid LML+binary format with base64 content encoding"
```

---

## Task 3: Bidirectional LML Parsing

**Files:**
- Create: `crates/aif-lml/src/parser.rs`
- Modify: `crates/aif-lml/src/lib.rs`
- Test: `crates/aif-lml/tests/roundtrip.rs`

Parse LML Aggressive mode back into AST. Focus on the aggressive format since it's the most compact and Markdown-like.

- [ ] **Step 1: Write failing roundtrip test**

Create `crates/aif-lml/tests/roundtrip.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_lml::{render_lml_aggressive, parse_lml};

#[test]
fn roundtrip_paragraph() {
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Test".into());
    doc.blocks.push(Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: "Hello world".into() }],
        },
        span: Span::new(0, 11),
    });
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.metadata.get("title").unwrap(), "Test");
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::Paragraph { content } => {
            assert!(!content.is_empty());
        }
        _ => panic!("expected Paragraph"),
    }
}

#[test]
fn roundtrip_skill_blocks() {
    let lml = r#"#title: Test Skill
@skill(name=debug):
@pre: When something breaks
@step(order=1): Reproduce the bug
@step(order=2): Find root cause
@verify: Tests pass
"#;
    let parsed = parse_lml(lml).unwrap();
    assert_eq!(parsed.metadata.get("title").unwrap(), "Test Skill");
    // Should find skill, pre, steps, verify
    let skill_blocks: Vec<_> = parsed.blocks.iter().filter(|b| matches!(&b.kind, BlockKind::SkillBlock { .. })).collect();
    assert!(!skill_blocks.is_empty());
}

#[test]
fn roundtrip_code_block() {
    let lml = "```rust\nfn main() {}\n```\n\n";
    let parsed = parse_lml(lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert!(code.contains("fn main()"));
        }
        _ => panic!("expected CodeBlock"),
    }
}

#[test]
fn roundtrip_list() {
    let lml = "- item one\n- item two\n\n";
    let parsed = parse_lml(lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
        }
        _ => panic!("expected List"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-lml --test roundtrip`
Expected: FAIL — `parse_lml` not found

- [ ] **Step 3: Implement parser.rs**

Create `crates/aif-lml/src/parser.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;

/// Parse LML aggressive-mode text back into a Document AST.
pub fn parse_lml(input: &str) -> Result<Document, String> {
    let mut metadata = std::collections::BTreeMap::new();
    let mut blocks = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Metadata: #key: value
        if line.starts_with('#') && !line.starts_with("##") && line.contains(": ") {
            if let Some(colon) = line.find(": ") {
                let key = line[1..colon].to_string();
                let value = line[colon + 2..].to_string();
                metadata.insert(key, value);
                i += 1;
                continue;
            }
        }

        // Heading: ## Title (sections)
        if line.starts_with('#') {
            let level = line.chars().take_while(|c| *c == '#').count();
            let title_text = line[level..].trim().to_string();
            blocks.push(Block {
                kind: BlockKind::Section {
                    attrs: Attrs::new(),
                    title: vec![Inline::Text { text: title_text }],
                    children: vec![],
                },
                span: Span::new(0, 0),
            });
            i += 1;
            continue;
        }

        // Code fence
        if line.starts_with("```") {
            let lang = line[3..].trim();
            let lang = if lang.is_empty() { None } else { Some(lang.to_string()) };
            let mut code = String::new();
            i += 1;
            while i < lines.len() && !lines[i].starts_with("```") {
                code.push_str(lines[i]);
                code.push('\n');
                i += 1;
            }
            if i < lines.len() { i += 1; } // skip closing ```
            blocks.push(Block {
                kind: BlockKind::CodeBlock {
                    lang,
                    attrs: Attrs::new(),
                    code,
                },
                span: Span::new(0, 0),
            });
            // skip blank lines
            while i < lines.len() && lines[i].trim().is_empty() { i += 1; }
            continue;
        }

        // Thematic break
        if line.trim() == "---" {
            blocks.push(Block {
                kind: BlockKind::ThematicBreak,
                span: Span::new(0, 0),
            });
            i += 1;
            while i < lines.len() && lines[i].trim().is_empty() { i += 1; }
            continue;
        }

        // Skill directives: @type(attrs): content
        if line.starts_with('@') {
            let (skill_type, rest) = parse_skill_directive(line);
            if let Some(st) = skill_type {
                let (attrs, content_str) = parse_directive_attrs_content(rest);
                let content = if content_str.is_empty() {
                    vec![]
                } else {
                    vec![Inline::Text { text: content_str }]
                };
                blocks.push(Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: st,
                        attrs,
                        title: None,
                        content,
                        children: vec![],
                    },
                    span: Span::new(0, 0),
                });
                i += 1;
                continue;
            }
        }

        // Blockquote: > content
        if line.starts_with("> ") {
            let content_text = line[2..].to_string();
            blocks.push(Block {
                kind: BlockKind::BlockQuote {
                    content: vec![Block {
                        kind: BlockKind::Paragraph {
                            content: vec![Inline::Text { text: content_text }],
                        },
                        span: Span::new(0, 0),
                    }],
                },
                span: Span::new(0, 0),
            });
            i += 1;
            continue;
        }

        // Lists: - item or N. item
        if line.starts_with("- ") || (line.len() > 2 && line.chars().next().unwrap().is_ascii_digit() && line.contains(". ")) {
            let ordered = !line.starts_with("- ");
            let mut items = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                if l.starts_with("- ") {
                    items.push(ListItem {
                        content: vec![Inline::Text { text: l[2..].to_string() }],
                        children: vec![],
                    });
                    i += 1;
                } else if l.len() > 2 && l.chars().next().unwrap().is_ascii_digit() && l.contains(". ") {
                    let dot = l.find(". ").unwrap();
                    items.push(ListItem {
                        content: vec![Inline::Text { text: l[dot + 2..].to_string() }],
                        children: vec![],
                    });
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(Block {
                kind: BlockKind::List { ordered, items },
                span: Span::new(0, 0),
            });
            while i < lines.len() && lines[i].trim().is_empty() { i += 1; }
            continue;
        }

        // Blank line
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Default: paragraph — collect until blank line
        let mut para_text = String::new();
        while i < lines.len() && !lines[i].trim().is_empty()
            && !lines[i].starts_with('#')
            && !lines[i].starts_with('@')
            && !lines[i].starts_with("```")
            && !lines[i].starts_with("- ")
            && !lines[i].starts_with("> ")
        {
            if !para_text.is_empty() { para_text.push(' '); }
            para_text.push_str(lines[i]);
            i += 1;
        }
        if !para_text.is_empty() {
            blocks.push(Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: para_text }],
                },
                span: Span::new(0, 0),
            });
        }
        // skip trailing blank lines
        while i < lines.len() && lines[i].trim().is_empty() { i += 1; }
    }

    Ok(Document { metadata, blocks })
}

fn parse_skill_directive(line: &str) -> (Option<SkillBlockType>, &str) {
    let directives = [
        ("@skill", SkillBlockType::Skill),
        ("@step", SkillBlockType::Step),
        ("@verify", SkillBlockType::Verify),
        ("@pre", SkillBlockType::Precondition),
        ("@output", SkillBlockType::OutputContract),
        ("@decision", SkillBlockType::Decision),
        ("@tool", SkillBlockType::Tool),
        ("@fallback", SkillBlockType::Fallback),
        ("@redflag", SkillBlockType::RedFlag),
        ("@example", SkillBlockType::Example),
    ];
    for (prefix, st) in directives {
        if line.starts_with(prefix) {
            let rest = &line[prefix.len()..];
            // Must be followed by (, :, space, or end
            if rest.is_empty() || rest.starts_with('(') || rest.starts_with(':') || rest.starts_with(' ') {
                return (Some(st), rest);
            }
        }
    }
    (None, line)
}

fn parse_directive_attrs_content(rest: &str) -> (Attrs, String) {
    let mut attrs = Attrs::new();
    let mut remaining = rest;

    if remaining.starts_with('(') {
        if let Some(close) = remaining.find(')') {
            let attr_str = &remaining[1..close];
            for pair in attr_str.split(',') {
                let pair = pair.trim();
                if let Some(eq) = pair.find('=') {
                    let key = pair[..eq].trim().to_string();
                    let value = pair[eq + 1..].trim().to_string();
                    if key == "id" {
                        attrs.id = Some(value);
                    } else {
                        attrs.pairs.insert(key, value);
                    }
                }
            }
            remaining = &remaining[close + 1..];
        }
    }

    // Skip ": " or ":"
    if remaining.starts_with(": ") {
        remaining = &remaining[2..];
    } else if remaining.starts_with(':') {
        remaining = &remaining[1..];
    }

    (attrs, remaining.trim().to_string())
}
```

- [ ] **Step 4: Export from lib.rs**

Add to `crates/aif-lml/src/lib.rs`:
```rust
mod parser;

pub fn parse_lml(input: &str) -> Result<Document, String> {
    parser::parse_lml(input)
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p aif-lml --test roundtrip`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/aif-lml/
git commit -m "feat(lml): add bidirectional LML parser for aggressive-mode roundtrip"
```

---

## Task 4: JSON Schema Generation (Cross-Language Foundation)

**Files:**
- Create: `crates/aif-core/src/schema.rs`
- Modify: `crates/aif-core/src/lib.rs`
- Modify: `crates/aif-core/Cargo.toml`
- Modify: `crates/aif-cli/src/main.rs`
- Modify: `crates/aif-cli/Cargo.toml`
- Test: `crates/aif-core/tests/schema.rs`

Generate JSON Schema from AST types using `schemars`. This enables auto-generation of Python/TS types via existing tools (e.g., `datamodel-code-generator`, `json-schema-to-typescript`).

- [ ] **Step 1: Write failing test**

Create `crates/aif-core/tests/schema.rs`:

```rust
use aif_core::schema::generate_schema;

#[test]
fn schema_contains_document_definition() {
    let schema = generate_schema();
    let json: serde_json::Value = serde_json::from_str(&schema).unwrap();
    assert!(json.get("title").is_some() || json.get("$defs").is_some() || json.get("definitions").is_some());
    // Should contain Document type
    let schema_str = schema.to_lowercase();
    assert!(schema_str.contains("document"));
}

#[test]
fn schema_is_valid_json_schema() {
    let schema = generate_schema();
    let json: serde_json::Value = serde_json::from_str(&schema).unwrap();
    // Must have $schema or be a valid JSON Schema object
    assert!(json.is_object());
    assert!(json.get("type").is_some() || json.get("$ref").is_some() || json.get("oneOf").is_some());
}

#[test]
fn schema_contains_block_types() {
    let schema = generate_schema();
    assert!(schema.contains("Paragraph"));
    assert!(schema.contains("Section"));
    assert!(schema.contains("SkillBlock"));
    assert!(schema.contains("SemanticBlock"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-core --test schema`
Expected: FAIL — `schema` module not found

- [ ] **Step 3: Add schemars dependency**

Add to `crates/aif-core/Cargo.toml` under `[dependencies]`:
```toml
schemars = "0.8"
```

And add `JsonSchema` derive to AST types in `crates/aif-core/src/ast.rs`:
Add `use schemars::JsonSchema;` at the top, then add `#[derive(JsonSchema)]` to: `Document`, `Block`, `BlockKind`, `ListItem`, `SemanticBlockType`, `CalloutType`, `SkillBlockType`, `Attrs`, `Inline`.

Also add `#[derive(JsonSchema)]` to `Span` in `crates/aif-core/src/span.rs`.

- [ ] **Step 4: Implement schema.rs**

Create `crates/aif-core/src/schema.rs`:

```rust
use crate::ast::Document;
use schemars::schema_for;

/// Generate a JSON Schema string for the AIF Document type.
pub fn generate_schema() -> String {
    let schema = schema_for!(Document);
    serde_json::to_string_pretty(&schema).expect("schema serialization failed")
}
```

- [ ] **Step 5: Export from lib.rs**

Add to `crates/aif-core/src/lib.rs`:
```rust
pub mod schema;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p aif-core --test schema`
Expected: PASS

- [ ] **Step 7: Add CLI `schema` subcommand**

Add to CLI in `crates/aif-cli/src/main.rs`:
```rust
/// Emit JSON Schema for AIF document types
Schema {},
```
Handler:
```rust
Commands::Schema {} => {
    println!("{}", aif_core::schema::generate_schema());
}
```

Add `aif-core` as dependency to `crates/aif-cli/Cargo.toml` if not already present.

- [ ] **Step 8: Commit**

```bash
git add crates/aif-core/ crates/aif-cli/
git commit -m "feat(core): add JSON Schema generation for cross-language SDK support"
```

---

## Task 5: Incremental Diff Transport

**Files:**
- Create: `crates/aif-skill/src/delta.rs`
- Modify: `crates/aif-skill/src/lib.rs`
- Modify: `crates/aif-cli/src/main.rs`
- Test: `crates/aif-skill/tests/delta.rs`

Encode skill diffs as compact binary deltas for transport. Uses existing `diff_skills()` + binary encoding.

- [ ] **Step 1: Write failing test**

Create `crates/aif-skill/tests/delta.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_skill::delta::{encode_delta, apply_delta};

fn make_skill(steps: Vec<(&str, &str)>) -> Block {
    let children: Vec<Block> = steps.iter().map(|(order, text)| {
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("order".into(), order.to_string());
                    a
                },
                title: None,
                content: vec![Inline::Text { text: text.to_string() }],
                children: vec![],
            },
            span: Span::new(0, 0),
        }
    }).collect();

    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("name".into(), "test".into());
                a
            },
            title: None,
            content: vec![],
            children,
        },
        span: Span::new(0, 0),
    }
}

#[test]
fn delta_encode_decode_roundtrip() {
    let old = make_skill(vec![("1", "Step one"), ("2", "Step two")]);
    let new = make_skill(vec![("1", "Step one modified"), ("2", "Step two"), ("3", "Step three")]);

    let delta = encode_delta(&old, &new);
    assert!(!delta.is_empty());

    let result = apply_delta(&old, &delta).unwrap();
    // After applying delta, should have 3 children
    if let BlockKind::SkillBlock { children, .. } = &result.kind {
        assert_eq!(children.len(), 3);
    } else {
        panic!("expected SkillBlock");
    }
}

#[test]
fn delta_no_changes_is_minimal() {
    let skill = make_skill(vec![("1", "Step one")]);
    let delta = encode_delta(&skill, &skill);
    // Delta for no changes should be very small (just a header)
    assert!(delta.len() < 20);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-skill --test delta`
Expected: FAIL — `delta` module not found

- [ ] **Step 3: Implement delta.rs**

Create `crates/aif-skill/src/delta.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use crate::diff::{diff_skills, ChangeKind};

/// A delta operation on a skill's children.
#[derive(Debug, Clone)]
enum DeltaOp {
    Keep { index: usize },
    Remove { index: usize },
    Add { block_json: String },
    Replace { index: usize, block_json: String },
}

/// Encode the diff between two skill blocks as a compact binary delta.
/// Format: varint(op_count) + ops
/// Op encoding: tag(1) + data
pub fn encode_delta(old: &Block, new: &Block) -> Vec<u8> {
    let changes = diff_skills(old, new);
    let mut out = Vec::new();

    // Magic: "AD" (AIF Delta)
    out.extend_from_slice(b"AD");
    out.push(0x01); // version

    let old_children = skill_children(old);
    let new_children = skill_children(new);

    // Simple encoding: serialize each change as JSON
    let ops: Vec<DeltaOp> = build_ops(&changes, old_children, new_children);

    // Encode op count
    encode_varint(ops.len(), &mut out);

    for op in &ops {
        match op {
            DeltaOp::Keep { index } => {
                out.push(0x01);
                encode_varint(*index, &mut out);
            }
            DeltaOp::Remove { index } => {
                out.push(0x02);
                encode_varint(*index, &mut out);
            }
            DeltaOp::Add { block_json } => {
                out.push(0x03);
                let bytes = block_json.as_bytes();
                encode_varint(bytes.len(), &mut out);
                out.extend_from_slice(bytes);
            }
            DeltaOp::Replace { index, block_json } => {
                out.push(0x04);
                encode_varint(*index, &mut out);
                let bytes = block_json.as_bytes();
                encode_varint(bytes.len(), &mut out);
                out.extend_from_slice(bytes);
            }
        }
    }

    out
}

/// Apply a binary delta to a skill block, producing the new version.
pub fn apply_delta(old: &Block, delta: &[u8]) -> Result<Block, String> {
    if delta.len() < 3 || &delta[0..2] != b"AD" {
        return Err("invalid delta magic".into());
    }
    let _version = delta[2];
    let mut pos = 3;

    let (op_count, n) = decode_varint(&delta[pos..]).map_err(|e| e.to_string())?;
    pos += n;

    let old_children = skill_children(old);
    let mut new_children: Vec<Block> = Vec::new();
    let mut removed: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // First pass: collect removes
    let mut ops = Vec::new();
    let saved_pos = pos;
    for _ in 0..op_count {
        if pos >= delta.len() { return Err("truncated delta".into()); }
        let tag = delta[pos];
        pos += 1;
        match tag {
            0x01 => { // Keep
                let (idx, n) = decode_varint(&delta[pos..]).map_err(|e| e.to_string())?;
                pos += n;
                ops.push(DeltaOp::Keep { index: idx });
            }
            0x02 => { // Remove
                let (idx, n) = decode_varint(&delta[pos..]).map_err(|e| e.to_string())?;
                pos += n;
                ops.push(DeltaOp::Remove { index: idx });
                removed.insert(idx);
            }
            0x03 => { // Add
                let (len, n) = decode_varint(&delta[pos..]).map_err(|e| e.to_string())?;
                pos += n;
                let json = std::str::from_utf8(&delta[pos..pos + len]).map_err(|_| "invalid utf-8")?;
                pos += len;
                ops.push(DeltaOp::Add { block_json: json.to_string() });
            }
            0x04 => { // Replace
                let (idx, n) = decode_varint(&delta[pos..]).map_err(|e| e.to_string())?;
                pos += n;
                let (len, n) = decode_varint(&delta[pos..]).map_err(|e| e.to_string())?;
                pos += n;
                let json = std::str::from_utf8(&delta[pos..pos + len]).map_err(|_| "invalid utf-8")?;
                pos += len;
                ops.push(DeltaOp::Replace { index: idx, block_json: json.to_string() });
            }
            _ => return Err(format!("unknown delta op: 0x{:02x}", tag)),
        }
    }

    // Apply ops to build new children
    // Keep all non-removed old children, apply replacements, add new ones
    for op in &ops {
        match op {
            DeltaOp::Keep { index } => {
                if let Some(block) = old_children.get(*index) {
                    new_children.push((*block).clone());
                }
            }
            DeltaOp::Remove { .. } => { /* skip */ }
            DeltaOp::Add { block_json } => {
                let block: Block = serde_json::from_str(block_json).map_err(|e| e.to_string())?;
                new_children.push(block);
            }
            DeltaOp::Replace { index, block_json } => {
                let block: Block = serde_json::from_str(block_json).map_err(|e| e.to_string())?;
                new_children.push(block);
            }
        }
    }

    // Rebuild skill block with new children
    match &old.kind {
        BlockKind::SkillBlock { skill_type, attrs, title, content, .. } => {
            Ok(Block {
                kind: BlockKind::SkillBlock {
                    skill_type: skill_type.clone(),
                    attrs: attrs.clone(),
                    title: title.clone(),
                    content: content.clone(),
                    children: new_children,
                },
                span: old.span.clone(),
            })
        }
        _ => Err("not a skill block".into()),
    }
}

fn skill_children(block: &Block) -> Vec<&Block> {
    match &block.kind {
        BlockKind::SkillBlock { children, .. } => children.iter().collect(),
        _ => vec![],
    }
}

fn build_ops(changes: &[crate::diff::Change], old_children: Vec<&Block>, new_children: Vec<&Block>) -> Vec<DeltaOp> {
    let mut ops = Vec::new();

    // Index old children for matching
    let mut old_matched: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for change in changes {
        match change.kind {
            ChangeKind::Removed => {
                // Find the old child index
                for (i, child) in old_children.iter().enumerate() {
                    if !old_matched.contains(&i) && format!("{:?}", child_type(child)) == format!("{:?}", change.block_type) {
                        ops.push(DeltaOp::Remove { index: i });
                        old_matched.insert(i);
                        break;
                    }
                }
            }
            ChangeKind::Added => {
                // Find the new child to add
                for new_child in &new_children {
                    let json = serde_json::to_string(new_child).unwrap_or_default();
                    if format!("{:?}", child_type(new_child)) == format!("{:?}", change.block_type) {
                        ops.push(DeltaOp::Add { block_json: json });
                        break;
                    }
                }
            }
            ChangeKind::Modified => {
                // Find old index and new replacement
                for (i, child) in old_children.iter().enumerate() {
                    if !old_matched.contains(&i) && format!("{:?}", child_type(child)) == format!("{:?}", change.block_type) {
                        // Find corresponding new child
                        for new_child in &new_children {
                            if format!("{:?}", child_type(new_child)) == format!("{:?}", change.block_type) {
                                let json = serde_json::to_string(new_child).unwrap_or_default();
                                ops.push(DeltaOp::Replace { index: i, block_json: json });
                                old_matched.insert(i);
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    // Keep all unmatched old children
    for i in 0..old_children.len() {
        if !old_matched.contains(&i) {
            ops.push(DeltaOp::Keep { index: i });
        }
    }

    ops
}

fn child_type(block: &Block) -> SkillBlockType {
    match &block.kind {
        BlockKind::SkillBlock { skill_type, .. } => skill_type.clone(),
        _ => SkillBlockType::Step,
    }
}

fn encode_varint(mut n: usize, out: &mut Vec<u8>) {
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn decode_varint(data: &[u8]) -> Result<(usize, usize), &'static str> {
    let mut result: usize = 0;
    let mut shift = 0;
    for (i, &byte) in data.iter().enumerate() {
        result |= ((byte & 0x7F) as usize) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
        if shift >= 64 {
            return Err("varint overflow");
        }
    }
    Err("unexpected end of varint")
}
```

- [ ] **Step 4: Export from lib.rs**

Add to `crates/aif-skill/src/lib.rs`:
```rust
pub mod delta;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p aif-skill --test delta`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/aif-skill/
git commit -m "feat(skill): add incremental diff transport with binary delta encoding"
```

---

## Task 6: Skill Registry

**Files:**
- Create: `crates/aif-skill/src/registry.rs`
- Modify: `crates/aif-skill/src/lib.rs`
- Modify: `crates/aif-cli/src/main.rs`
- Test: `crates/aif-skill/tests/registry.rs`

A local file-based registry for tracking skills by name, version, hash, and file path.

- [ ] **Step 1: Write failing test**

Create `crates/aif-skill/tests/registry.rs`:

```rust
use aif_skill::registry::Registry;
use std::path::PathBuf;

#[test]
fn register_and_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let mut reg = Registry::new(dir.path().join("registry.json"));

    reg.register("debugging", "1.0.0", "sha256:abc123", "skills/debugging.aif");
    let entry = reg.lookup("debugging").unwrap();
    assert_eq!(entry.name, "debugging");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.hash, "sha256:abc123");
}

#[test]
fn list_all_skills() {
    let dir = tempfile::tempdir().unwrap();
    let mut reg = Registry::new(dir.path().join("registry.json"));

    reg.register("debugging", "1.0.0", "sha256:abc", "a.aif");
    reg.register("tdd", "2.0.0", "sha256:def", "b.aif");

    let all = reg.list();
    assert_eq!(all.len(), 2);
}

#[test]
fn save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("registry.json");

    {
        let mut reg = Registry::new(path.clone());
        reg.register("debugging", "1.0.0", "sha256:abc", "a.aif");
        reg.save().unwrap();
    }

    let reg = Registry::load(&path).unwrap();
    assert_eq!(reg.list().len(), 1);
    assert!(reg.lookup("debugging").is_some());
}

#[test]
fn update_existing_skill() {
    let dir = tempfile::tempdir().unwrap();
    let mut reg = Registry::new(dir.path().join("registry.json"));

    reg.register("debugging", "1.0.0", "sha256:abc", "a.aif");
    reg.register("debugging", "1.1.0", "sha256:def", "a.aif");

    let entry = reg.lookup("debugging").unwrap();
    assert_eq!(entry.version, "1.1.0");
    assert_eq!(entry.hash, "sha256:def");
    assert_eq!(reg.list().len(), 1);
}

#[test]
fn lookup_by_hash() {
    let dir = tempfile::tempdir().unwrap();
    let mut reg = Registry::new(dir.path().join("registry.json"));

    reg.register("debugging", "1.0.0", "sha256:abc123", "a.aif");
    let entry = reg.lookup_by_hash("sha256:abc123").unwrap();
    assert_eq!(entry.name, "debugging");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-skill --test registry`
Expected: FAIL — `registry` module not found

- [ ] **Step 3: Add tempfile dev-dependency**

Add to `crates/aif-skill/Cargo.toml` under `[dev-dependencies]`:
```toml
tempfile = "3"
```

- [ ] **Step 4: Implement registry.rs**

Create `crates/aif-skill/src/registry.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    #[serde(skip)]
    file_path: PathBuf,
    skills: BTreeMap<String, RegistryEntry>,
}

impl Registry {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            skills: BTreeMap::new(),
        }
    }

    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        let mut reg: Registry = serde_json::from_str(&data)?;
        reg.file_path = path.clone();
        Ok(reg)
    }

    pub fn register(&mut self, name: &str, version: &str, hash: &str, path: &str) {
        self.skills.insert(
            name.to_string(),
            RegistryEntry {
                name: name.to_string(),
                version: version.to_string(),
                hash: hash.to_string(),
                path: path.to_string(),
            },
        );
    }

    pub fn lookup(&self, name: &str) -> Option<&RegistryEntry> {
        self.skills.get(name)
    }

    pub fn lookup_by_hash(&self, hash: &str) -> Option<&RegistryEntry> {
        self.skills.values().find(|e| e.hash == hash)
    }

    pub fn list(&self) -> Vec<&RegistryEntry> {
        self.skills.values().collect()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(&self.file_path, json)?;
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.skills.remove(name).is_some()
    }
}
```

- [ ] **Step 5: Export from lib.rs**

Add to `crates/aif-skill/src/lib.rs`:
```rust
pub mod registry;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p aif-skill --test registry`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/aif-skill/
git commit -m "feat(skill): add local file-based skill registry"
```

---

## Task 7: Compliance Benchmarks for All Formats

**Files:**
- Modify: `benchmarks/skill_token_benchmark.py`
- Test: manual run

Extend compliance scoring to HTML and Markdown formats (currently only LML formats get scored).

- [ ] **Step 1: Add HTML and Markdown tag patterns**

In `benchmarks/skill_token_benchmark.py`, extend `TAG_PATTERNS`:

```python
TAG_PATTERNS = {
    # ... existing LML patterns ...
    "html": {
        "step": r'class="aif-step"',
        "verify": r'class="aif-verify"',
        "precondition": r'class="aif-precondition"',
        "skill": r'class="aif-skill"',
    },
    "markdown": {
        "step": r'\*\*Step\b',
        "verify": r'\*\*Verify\b|\*\*Verification\b',
        "precondition": r'\*\*Precondition\b|\*\*Prerequisites?\b',
        "skill": r'^# ',
    },
    "json": {
        "step": r'"Step"',
        "verify": r'"Verify"',
        "precondition": r'"Precondition"',
        "skill": r'"Skill"',
    },
}
```

- [ ] **Step 2: Verify benchmark runs**

Run: `cargo build --release && python benchmarks/skill_token_benchmark.py` (requires ANTHROPIC_API_KEY)
Expected: All formats now show compliance scores

- [ ] **Step 3: Commit**

```bash
git add benchmarks/
git commit -m "bench: extend compliance scoring to HTML, Markdown, and JSON formats"
```

---

## Task 8: Format Recommender (Learned Token Optimization)

**Files:**
- Create: `crates/aif-skill/src/recommend.rs`
- Modify: `crates/aif-skill/src/lib.rs`
- Test: `crates/aif-skill/tests/recommend.rs`

Analyze document structure to recommend optimal output format. Uses heuristics based on benchmark data.

- [ ] **Step 1: Write failing test**

Create `crates/aif-skill/tests/recommend.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_skill::recommend::{recommend_format, FormatRecommendation};

#[test]
fn skill_heavy_doc_recommends_lml_aggressive() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: Attrs::new(),
                title: None,
                content: vec![],
                children: vec![
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text { text: "step".into() }],
                            children: vec![],
                        },
                        span: Span::new(0, 0),
                    },
                ],
            },
            span: Span::new(0, 0),
        }],
    };
    let rec = recommend_format(&doc);
    assert_eq!(rec.format, "lml-aggressive");
    assert!(rec.reason.contains("skill"));
}

#[test]
fn code_heavy_doc_recommends_markdown() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            Block {
                kind: BlockKind::CodeBlock {
                    lang: Some("rust".into()),
                    attrs: Attrs::new(),
                    code: "fn main() { println!(\"hello\"); }".into(),
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::CodeBlock {
                    lang: Some("python".into()),
                    attrs: Attrs::new(),
                    code: "print('hello')".into(),
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "Some text".into() }],
                },
                span: Span::new(0, 0),
            },
        ],
    };
    let rec = recommend_format(&doc);
    assert_eq!(rec.format, "markdown");
}

#[test]
fn wire_transfer_recommends_binary() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text { text: "data".into() }],
            },
            span: Span::new(0, 0),
        }],
    };
    let rec = recommend_format_for_purpose(&doc, "wire");
    assert!(rec.format.contains("binary"));
}

use aif_skill::recommend::recommend_format_for_purpose;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-skill --test recommend`
Expected: FAIL — `recommend` module not found

- [ ] **Step 3: Implement recommend.rs**

Create `crates/aif-skill/src/recommend.rs`:

```rust
use aif_core::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct FormatRecommendation {
    pub format: String,
    pub reason: String,
}

/// Analyze document structure and recommend optimal output format for LLM context.
pub fn recommend_format(doc: &Document) -> FormatRecommendation {
    let stats = analyze(doc);

    if stats.skill_blocks > 0 {
        // Skill-heavy docs benefit most from LML aggressive (benchmark winner)
        return FormatRecommendation {
            format: "lml-aggressive".into(),
            reason: format!("skill-heavy document ({} skill blocks) — LML aggressive has best TNO", stats.skill_blocks),
        };
    }

    if stats.code_blocks as f64 / stats.total_blocks.max(1) as f64 > 0.4 {
        // Code-heavy docs: Markdown preserves code blocks naturally
        return FormatRecommendation {
            format: "markdown".into(),
            reason: format!("code-heavy document ({}/{} blocks are code) — Markdown preserves code naturally", stats.code_blocks, stats.total_blocks),
        };
    }

    if stats.semantic_blocks > 0 {
        // Semantic-rich docs benefit from LML conservative (preserves semantics, saves tokens)
        return FormatRecommendation {
            format: "lml-conservative".into(),
            reason: format!("semantic-rich document ({} semantic blocks) — LML conservative preserves types", stats.semantic_blocks),
        };
    }

    // Default: Markdown (best general-purpose token efficiency from benchmarks)
    FormatRecommendation {
        format: "markdown".into(),
        reason: "general document — Markdown has best token efficiency for plain content".into(),
    }
}

/// Recommend format based on purpose (llm-context, wire, storage).
pub fn recommend_format_for_purpose(doc: &Document, purpose: &str) -> FormatRecommendation {
    match purpose {
        "wire" | "transport" => FormatRecommendation {
            format: "binary-wire".into(),
            reason: "wire transport — binary wire format is most compact in bytes".into(),
        },
        "storage" | "archive" => FormatRecommendation {
            format: "json".into(),
            reason: "storage — JSON preserves full AST and is widely readable".into(),
        },
        _ => recommend_format(doc),
    }
}

struct DocStats {
    total_blocks: usize,
    skill_blocks: usize,
    code_blocks: usize,
    semantic_blocks: usize,
}

fn analyze(doc: &Document) -> DocStats {
    let mut stats = DocStats {
        total_blocks: 0,
        skill_blocks: 0,
        code_blocks: 0,
        semantic_blocks: 0,
    };
    count_blocks(&doc.blocks, &mut stats);
    stats
}

fn count_blocks(blocks: &[Block], stats: &mut DocStats) {
    for block in blocks {
        stats.total_blocks += 1;
        match &block.kind {
            BlockKind::SkillBlock { children, .. } => {
                stats.skill_blocks += 1;
                count_blocks(children, stats);
            }
            BlockKind::CodeBlock { .. } => {
                stats.code_blocks += 1;
            }
            BlockKind::SemanticBlock { .. } => {
                stats.semantic_blocks += 1;
            }
            BlockKind::Section { children, .. } => {
                count_blocks(children, stats);
            }
            BlockKind::BlockQuote { content } => {
                count_blocks(content, stats);
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 4: Export from lib.rs**

Add to `crates/aif-skill/src/lib.rs`:
```rust
pub mod recommend;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p aif-skill --test recommend`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/aif-skill/
git commit -m "feat(skill): add format recommender based on document structure analysis"
```

---

## Task 9: Semantic Compression

**Files:**
- Create: `crates/aif-lml/src/compress.rs`
- Modify: `crates/aif-lml/src/lib.rs`
- Test: `crates/aif-lml/tests/compress.rs`

Deduplicate repeated inline text patterns across blocks. If the same text appears in multiple blocks, replace with a reference and emit a lookup table.

- [ ] **Step 1: Write failing test**

Create `crates/aif-lml/tests/compress.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_lml::render_lml_compressed;

#[test]
fn repeated_text_gets_deduplicated() {
    let repeated = "This is a long repeated phrase that appears multiple times in the document";
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: repeated.into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: repeated.into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: repeated.into() }],
                },
                span: Span::new(0, 0),
            },
        ],
    };
    let output = render_lml_compressed(&doc);
    // Should have a lookup table
    assert!(output.contains("~dict:"));
    // The repeated text should appear only once (in the dict)
    let count = output.matches(repeated).count();
    assert_eq!(count, 1, "repeated text should appear exactly once in output");
    // References should appear
    assert!(output.contains("~ref:"));
}

#[test]
fn short_text_not_deduplicated() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "short".into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "short".into() }],
                },
                span: Span::new(0, 0),
            },
        ],
    };
    let output = render_lml_compressed(&doc);
    // Short text shouldn't be deduplicated (overhead > savings)
    assert!(!output.contains("~dict:"));
}

#[test]
fn unique_text_not_deduplicated() {
    let doc = Document {
        metadata: Default::default(),
        blocks: vec![
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "unique text one that is long enough".into() }],
                },
                span: Span::new(0, 0),
            },
            Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "unique text two that is also long enough".into() }],
                },
                span: Span::new(0, 0),
            },
        ],
    };
    let output = render_lml_compressed(&doc);
    assert!(!output.contains("~dict:"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-lml --test compress`
Expected: FAIL — `render_lml_compressed` not found

- [ ] **Step 3: Implement compress.rs**

Create `crates/aif-lml/src/compress.rs`:

```rust
use aif_core::ast::*;
use std::collections::HashMap;

const MIN_TEXT_LEN: usize = 30;
const MIN_OCCURRENCES: usize = 2;

/// Render LML with semantic compression: deduplicate repeated text patterns.
pub fn render_compressed(doc: &Document) -> String {
    // Phase 1: Collect all text fragments and count occurrences
    let mut text_counts: HashMap<String, usize> = HashMap::new();
    for block in &doc.blocks {
        collect_texts(block, &mut text_counts);
    }

    // Phase 2: Build dictionary of texts worth deduplicating
    let mut dict: HashMap<String, String> = HashMap::new();
    let mut dict_id = 0u32;
    for (text, count) in &text_counts {
        if text.len() >= MIN_TEXT_LEN && *count >= MIN_OCCURRENCES {
            let ref_id = format!("t{}", dict_id);
            dict.insert(text.clone(), ref_id);
            dict_id += 1;
        }
    }

    // Phase 3: Render with references
    let mut out = String::new();

    // Emit dictionary header if non-empty
    if !dict.is_empty() {
        out.push_str("~dict:\n");
        let mut sorted: Vec<_> = dict.iter().collect();
        sorted.sort_by_key(|(_, id)| id.clone());
        for (text, id) in &sorted {
            out.push_str(&format!("  {}={}\n", id, text));
        }
        out.push_str("~end\n\n");
    }

    // Emit blocks with references
    for block in &doc.blocks {
        emit_block_compressed(&mut out, block, &dict);
    }

    out
}

fn collect_texts(block: &Block, counts: &mut HashMap<String, usize>) {
    match &block.kind {
        BlockKind::Paragraph { content } => collect_inline_texts(content, counts),
        BlockKind::Section { title, children, .. } => {
            collect_inline_texts(title, counts);
            for child in children { collect_texts(child, counts); }
        }
        BlockKind::SkillBlock { content, children, title, .. } => {
            collect_inline_texts(content, counts);
            if let Some(t) = title { collect_inline_texts(t, counts); }
            for child in children { collect_texts(child, counts); }
        }
        BlockKind::SemanticBlock { content, title, .. } => {
            collect_inline_texts(content, counts);
            if let Some(t) = title { collect_inline_texts(t, counts); }
        }
        BlockKind::Callout { content, .. } => collect_inline_texts(content, counts),
        BlockKind::BlockQuote { content } => {
            for child in content { collect_texts(child, counts); }
        }
        BlockKind::List { items, .. } => {
            for item in items {
                collect_inline_texts(&item.content, counts);
                for child in &item.children { collect_texts(child, counts); }
            }
        }
        _ => {}
    }
}

fn collect_inline_texts(inlines: &[Inline], counts: &mut HashMap<String, usize>) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => {
                *counts.entry(text.clone()).or_insert(0) += 1;
            }
            Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
                collect_inline_texts(content, counts);
            }
            Inline::Link { text, .. } => collect_inline_texts(text, counts),
            _ => {}
        }
    }
}

fn emit_block_compressed(out: &mut String, block: &Block, dict: &HashMap<String, String>) {
    match &block.kind {
        BlockKind::Paragraph { content } => {
            emit_inlines_compressed(out, content, dict);
            out.push_str("\n\n");
        }
        BlockKind::Section { title, children, .. } => {
            out.push_str("# ");
            emit_inlines_compressed(out, title, dict);
            out.push('\n');
            for child in children { emit_block_compressed(out, child, dict); }
        }
        BlockKind::SkillBlock { skill_type, attrs, title, content, children } => {
            out.push_str(crate::emitter::skill_block_prefix_aggressive(skill_type));
            if let Some(t) = title {
                out.push_str(": ");
                emit_inlines_compressed(out, t, dict);
            }
            if !content.is_empty() {
                out.push(' ');
                emit_inlines_compressed(out, content, dict);
            }
            out.push('\n');
            for child in children { emit_block_compressed(out, child, dict); }
        }
        BlockKind::CodeBlock { lang, code, .. } => {
            out.push_str("```");
            if let Some(l) = lang { out.push_str(l); }
            out.push('\n');
            out.push_str(code);
            if !code.ends_with('\n') { out.push('\n'); }
            out.push_str("```\n\n");
        }
        BlockKind::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                if *ordered { out.push_str(&format!("{}. ", i + 1)); }
                else { out.push_str("- "); }
                emit_inlines_compressed(out, &item.content, dict);
                out.push('\n');
            }
            out.push('\n');
        }
        BlockKind::ThematicBreak => out.push_str("---\n\n"),
        _ => {
            // Fallback: render as aggressive LML
            out.push_str("[BLOCK]\n\n");
        }
    }
}

fn emit_inlines_compressed(out: &mut String, inlines: &[Inline], dict: &HashMap<String, String>) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => {
                if let Some(ref_id) = dict.get(text) {
                    out.push_str(&format!("~ref:{}", ref_id));
                } else {
                    out.push_str(text);
                }
            }
            Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
                emit_inlines_compressed(out, content, dict);
            }
            Inline::InlineCode { code } => out.push_str(code),
            Inline::Link { text, url } => {
                emit_inlines_compressed(out, text, dict);
                out.push_str(" (");
                out.push_str(url);
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
}
```

- [ ] **Step 4: Make skill_block_prefix_aggressive public**

In `crates/aif-lml/src/emitter.rs`, change:
```rust
fn skill_block_prefix_aggressive(st: &SkillBlockType) -> &'static str {
```
to:
```rust
pub fn skill_block_prefix_aggressive(st: &SkillBlockType) -> &'static str {
```

- [ ] **Step 5: Export from lib.rs**

Add to `crates/aif-lml/src/lib.rs`:
```rust
mod compress;

pub fn render_lml_compressed(doc: &Document) -> String {
    compress::render_compressed(doc)
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p aif-lml --test compress`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/aif-lml/
git commit -m "feat(lml): add semantic compression with text deduplication dictionary"
```

---

## Post-Implementation

- [ ] **Run full test suite**: `cargo test --workspace`
- [ ] **Update CLAUDE.md**: Add new formats, CLI commands, crate modules
- [ ] **Update README.md**: Add Phase 2 features
- [ ] **Commit docs**: `git commit -m "docs: update for Phase 2 features"`
