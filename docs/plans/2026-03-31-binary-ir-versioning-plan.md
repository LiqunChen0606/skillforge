# Binary IR + Versioning Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add binary serialization (wire + token-optimized) and skill versioning with semantic diff to AIF.

**Architecture:** New `aif-binary` crate with postcard-based wire format and custom token-optimized encoder. Extend `aif-skill` with version management, block-level diff, and change classification. Wire into CLI and benchmarks.

**Tech Stack:** Rust, postcard (binary serde), sha2 (existing), semver parsing (manual — 3 fields, no crate needed)

---

### Task 1: Scaffold `aif-binary` crate

**Files:**
- Create: `crates/aif-binary/Cargo.toml`
- Create: `crates/aif-binary/src/lib.rs`
- Create: `crates/aif-binary/src/wire.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "aif-binary"
version.workspace = true
edition.workspace = true

[dependencies]
aif-core = { workspace = true }
postcard = { version = "1", features = ["alloc"] }
serde = { workspace = true }

[dev-dependencies]
aif-core = { workspace = true }
serde_json = { workspace = true }
```

**Step 2: Create lib.rs**

```rust
pub mod wire;
```

**Step 3: Create wire.rs (empty placeholder)**

```rust
// Wire format: postcard-based binary serialization
```

**Step 4: Add to workspace Cargo.toml**

Add `"crates/aif-binary"` to `members` list and add to `[workspace.dependencies]`:
```toml
aif-binary = { path = "crates/aif-binary" }
```

**Step 5: Verify it compiles**

Run: `cargo build -p aif-binary`
Expected: Success

**Step 6: Commit**

```bash
git add crates/aif-binary/ Cargo.toml Cargo.lock
git commit -m "feat(binary): scaffold aif-binary crate with postcard dependency"
```

---

### Task 2: Wire format — encode and decode

**Files:**
- Modify: `crates/aif-binary/src/wire.rs`
- Modify: `crates/aif-binary/src/lib.rs`
- Create: `crates/aif-binary/tests/wire_roundtrip.rs`

**Step 1: Write the failing test**

Create `crates/aif-binary/tests/wire_roundtrip.rs`:

```rust
use aif_core::ast::*;
use std::collections::BTreeMap;

fn sample_doc() -> Document {
    Document {
        metadata: {
            let mut m = BTreeMap::new();
            m.insert("title".to_string(), "Test".to_string());
            m
        },
        blocks: vec![Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text {
                    text: "Hello world".to_string(),
                }],
            },
            span: Span::empty(),
        }],
    }
}

#[test]
fn wire_roundtrip_paragraph() {
    let doc = sample_doc();
    let bytes = aif_binary::wire::encode(&doc);
    let decoded = aif_binary::wire::decode(&bytes).unwrap();
    assert_eq!(doc, decoded);
}

#[test]
fn wire_is_smaller_than_json() {
    let doc = sample_doc();
    let wire_bytes = aif_binary::wire::encode(&doc);
    let json_bytes = serde_json::to_string(&doc).unwrap();
    assert!(
        wire_bytes.len() < json_bytes.len(),
        "wire ({}) should be smaller than JSON ({})",
        wire_bytes.len(),
        json_bytes.len()
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-binary --test wire_roundtrip`
Expected: FAIL — `encode` and `decode` don't exist

**Step 3: Implement wire.rs**

```rust
use aif_core::ast::Document;

/// Encode a Document to compact binary (postcard wire format).
pub fn encode(doc: &Document) -> Vec<u8> {
    postcard::to_allocvec(doc).expect("postcard serialization failed")
}

/// Decode a Document from binary wire format.
pub fn decode(bytes: &[u8]) -> Result<Document, postcard::Error> {
    postcard::from_bytes(bytes)
}
```

Update `lib.rs`:
```rust
pub mod wire;

pub use wire::{encode as render_wire, decode as decode_wire};
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p aif-binary --test wire_roundtrip`
Expected: 2 tests pass

**Step 5: Commit**

```bash
git add crates/aif-binary/
git commit -m "feat(binary): add postcard wire format encode/decode with roundtrip tests"
```

---

### Task 3: Wire format — skill block roundtrip

**Files:**
- Create: `crates/aif-binary/tests/wire_skill.rs`

**Step 1: Write the failing test**

Create `crates/aif-binary/tests/wire_skill.rs`:

```rust
use aif_core::ast::*;
use std::collections::BTreeMap;

#[test]
fn wire_roundtrip_skill_block() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".to_string(), "test-skill".to_string());
                    a.pairs.insert("version".to_string(), "1.0.0".to_string());
                    a
                },
                title: Some(vec![Inline::Text {
                    text: "Test Skill".to_string(),
                }]),
                content: vec![Inline::Text {
                    text: "Description".to_string(),
                }],
                children: vec![
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            attrs: {
                                let mut a = Attrs::new();
                                a.pairs.insert("order".to_string(), "1".to_string());
                                a
                            },
                            title: None,
                            content: vec![Inline::Text {
                                text: "Do something".to_string(),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Verify,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text {
                                text: "Check it".to_string(),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                ],
            },
            span: Span::empty(),
        }],
    };

    let bytes = aif_binary::wire::encode(&doc);
    let decoded = aif_binary::wire::decode(&bytes).unwrap();
    assert_eq!(doc, decoded);

    // Verify compactness
    let json = serde_json::to_string(&doc).unwrap();
    let ratio = bytes.len() as f64 / json.len() as f64;
    assert!(ratio < 0.6, "wire should be <60% of JSON size, got {:.1}%", ratio * 100.0);
}
```

**Step 2: Run test to verify it passes** (implementation already exists from Task 2)

Run: `cargo test -p aif-binary --test wire_skill`
Expected: PASS (postcard handles all serde types)

If it fails, investigate — it means a type isn't serde-compatible with postcard.

**Step 3: Commit**

```bash
git add crates/aif-binary/tests/wire_skill.rs
git commit -m "test(binary): add skill block wire roundtrip test"
```

---

### Task 4: Token-optimized format — dictionary and encoder

**Files:**
- Create: `crates/aif-binary/src/dictionary.rs`
- Create: `crates/aif-binary/src/token_opt.rs`
- Modify: `crates/aif-binary/src/lib.rs`
- Create: `crates/aif-binary/tests/token_opt.rs`

**Step 1: Write the failing test**

Create `crates/aif-binary/tests/token_opt.rs`:

```rust
use aif_core::ast::*;
use std::collections::BTreeMap;

#[test]
fn token_opt_produces_bytes() {
    let doc = Document {
        metadata: {
            let mut m = BTreeMap::new();
            m.insert("title".to_string(), "Test".to_string());
            m
        },
        blocks: vec![Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text {
                    text: "Hello world".to_string(),
                }],
            },
            span: Span::empty(),
        }],
    };

    let bytes = aif_binary::token_opt::encode(&doc);
    assert!(!bytes.is_empty());
    // Should be smaller than JSON
    let json = serde_json::to_string(&doc).unwrap();
    assert!(bytes.len() < json.len());
}

#[test]
fn token_opt_smaller_than_wire() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".to_string(), "test".to_string());
                    a
                },
                title: None,
                content: vec![Inline::Text {
                    text: "A skill".to_string(),
                }],
                children: vec![
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            attrs: {
                                let mut a = Attrs::new();
                                a.pairs.insert("order".to_string(), "1".to_string());
                                a
                            },
                            title: None,
                            content: vec![Inline::Text {
                                text: "Step one".to_string(),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            attrs: {
                                let mut a = Attrs::new();
                                a.pairs.insert("order".to_string(), "2".to_string());
                                a
                            },
                            title: None,
                            content: vec![Inline::Text {
                                text: "Step two".to_string(),
                            }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                ],
            },
            span: Span::empty(),
        }],
    };

    let wire = aif_binary::wire::encode(&doc);
    let token = aif_binary::token_opt::encode(&doc);
    assert!(
        token.len() <= wire.len(),
        "token-opt ({}) should be <= wire ({})",
        token.len(),
        wire.len()
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-binary --test token_opt`
Expected: FAIL — `token_opt` module doesn't exist

**Step 3: Implement dictionary.rs**

```rust
/// Maps repeated tag/type names to single-byte IDs.
/// Written as a header in the token-optimized format.

// Block type IDs (0x01-0x1F)
pub const PARAGRAPH: u8 = 0x01;
pub const SECTION: u8 = 0x02;
pub const SEMANTIC_BLOCK: u8 = 0x03;
pub const CALLOUT: u8 = 0x04;
pub const TABLE: u8 = 0x05;
pub const FIGURE: u8 = 0x06;
pub const CODE_BLOCK: u8 = 0x07;
pub const BLOCK_QUOTE: u8 = 0x08;
pub const LIST: u8 = 0x09;
pub const SKILL_BLOCK: u8 = 0x0A;
pub const THEMATIC_BREAK: u8 = 0x0B;

// Inline type IDs (0x20-0x3F)
pub const TEXT: u8 = 0x20;
pub const EMPHASIS: u8 = 0x21;
pub const STRONG: u8 = 0x22;
pub const INLINE_CODE: u8 = 0x23;
pub const LINK: u8 = 0x24;
pub const REFERENCE: u8 = 0x25;
pub const FOOTNOTE: u8 = 0x26;
pub const SOFT_BREAK: u8 = 0x27;
pub const HARD_BREAK: u8 = 0x28;

// Skill block type IDs (0x40-0x4F)
pub const SK_SKILL: u8 = 0x40;
pub const SK_STEP: u8 = 0x41;
pub const SK_VERIFY: u8 = 0x42;
pub const SK_PRECONDITION: u8 = 0x43;
pub const SK_OUTPUT_CONTRACT: u8 = 0x44;
pub const SK_DECISION: u8 = 0x45;
pub const SK_TOOL: u8 = 0x46;
pub const SK_FALLBACK: u8 = 0x47;
pub const SK_RED_FLAG: u8 = 0x48;
pub const SK_EXAMPLE: u8 = 0x49;

/// Encode a usize as a varint (LEB128).
pub fn encode_varint(mut n: usize, out: &mut Vec<u8>) {
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

/// Encode a string: varint length + raw UTF-8 bytes.
pub fn encode_str(s: &str, out: &mut Vec<u8>) {
    encode_varint(s.len(), out);
    out.extend_from_slice(s.as_bytes());
}
```

**Step 4: Implement token_opt.rs**

```rust
use aif_core::ast::*;
use crate::dictionary::*;

/// Encode a Document in token-optimized binary format.
/// Layout: magic(2) + metadata + blocks
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
        BlockKind::Section { attrs, title, children } => {
            out.push(SECTION);
            encode_attrs(attrs, out);
            encode_inlines(title, out);
            encode_varint(children.len(), out);
            for child in children {
                encode_block(child, out);
            }
        }
        BlockKind::SkillBlock { skill_type, attrs, title, content, children } => {
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
        BlockKind::SemanticBlock { block_type: _, attrs, title, content } => {
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
        BlockKind::Callout { callout_type: _, attrs, content } => {
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
        BlockKind::Table { attrs, caption, headers, rows } => {
            out.push(TABLE);
            encode_attrs(attrs, out);
            encode_str(caption.as_deref().unwrap_or(""), out);
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
            encode_str(caption.as_deref().unwrap_or(""), out);
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
            encode_str(text, out);
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
```

**Step 5: Update lib.rs**

```rust
pub mod wire;
pub mod dictionary;
pub mod token_opt;

pub use wire::{encode as render_wire, decode as decode_wire};
pub use token_opt::encode as render_token_optimized;
```

**Step 6: Run tests**

Run: `cargo test -p aif-binary`
Expected: All tests pass

**Step 7: Commit**

```bash
git add crates/aif-binary/
git commit -m "feat(binary): add token-optimized format with dictionary encoding"
```

---

### Task 5: Wire binary formats into CLI

**Files:**
- Modify: `crates/aif-cli/Cargo.toml`
- Modify: `crates/aif-cli/src/main.rs`

**Step 1: Write the failing test** (manual CLI test)

Run: `cargo run -p aif-cli -- compile tests/fixtures/basic/paragraph.aif -f binary-wire 2>&1`
Expected: "Unknown format: binary-wire"

**Step 2: Add aif-binary dependency to aif-cli**

In `crates/aif-cli/Cargo.toml`, add to `[dependencies]`:
```toml
aif-binary = { path = "../aif-binary" }
```

**Step 3: Add binary formats to compile command**

In `main.rs`, in the `Commands::Compile` match arm, add before the `_ =>` branch:

```rust
"binary-wire" => {
    let bytes = aif_binary::render_wire(&doc);
    if let Some(output_path) = output.as_ref() {
        std::fs::write(output_path, &bytes).unwrap_or_else(|e| {
            eprintln!("Error writing {}: {}", output_path.display(), e);
            std::process::exit(1);
        });
        eprintln!("Wrote {} ({} bytes)", output_path.display(), bytes.len());
    } else {
        use std::io::Write;
        std::io::stdout().write_all(&bytes).unwrap();
    }
    return;
}
"binary-token" => {
    let bytes = aif_binary::render_token_optimized(&doc);
    if let Some(output_path) = output.as_ref() {
        std::fs::write(output_path, &bytes).unwrap_or_else(|e| {
            eprintln!("Error writing {}: {}", output_path.display(), e);
            std::process::exit(1);
        });
        eprintln!("Wrote {} ({} bytes)", output_path.display(), bytes.len());
    } else {
        use std::io::Write;
        std::io::stdout().write_all(&bytes).unwrap();
    }
    return;
}
```

Also add binary formats to `SkillAction::Import` match and update help strings.

**Step 4: Verify**

Run: `cargo build -p aif-cli && cargo run -p aif-cli -- skill import --format binary-wire tests/fixtures/skills/tdd.md 2>/dev/null | wc -c`
Expected: Outputs byte count (should be much smaller than JSON)

**Step 5: Commit**

```bash
git add crates/aif-cli/
git commit -m "feat(cli): add binary-wire and binary-token output formats"
```

---

### Task 6: Skill versioning — semver parsing and bump

**Files:**
- Create: `crates/aif-skill/src/version.rs`
- Modify: `crates/aif-skill/src/lib.rs`
- Create: `crates/aif-skill/tests/version.rs`

**Step 1: Write the failing test**

Create `crates/aif-skill/tests/version.rs`:

```rust
use aif_skill::version::{Semver, BumpLevel};

#[test]
fn parse_semver() {
    let v = Semver::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
}

#[test]
fn parse_invalid() {
    assert!(Semver::parse("not-a-version").is_none());
    assert!(Semver::parse("1.2").is_none());
}

#[test]
fn bump_major() {
    let v = Semver::parse("1.2.3").unwrap();
    let bumped = v.bump(BumpLevel::Major);
    assert_eq!(bumped.to_string(), "2.0.0");
}

#[test]
fn bump_minor() {
    let v = Semver::parse("1.2.3").unwrap();
    let bumped = v.bump(BumpLevel::Minor);
    assert_eq!(bumped.to_string(), "1.3.0");
}

#[test]
fn bump_patch() {
    let v = Semver::parse("1.2.3").unwrap();
    let bumped = v.bump(BumpLevel::Patch);
    assert_eq!(bumped.to_string(), "1.2.4");
}

#[test]
fn default_version() {
    assert_eq!(Semver::default().to_string(), "0.1.0");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-skill --test version`
Expected: FAIL — module doesn't exist

**Step 3: Implement version.rs**

```rust
/// Simple semver: major.minor.patch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Semver {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
}

impl Semver {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Semver {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    pub fn bump(self, level: BumpLevel) -> Self {
        match level {
            BumpLevel::Major => Semver { major: self.major + 1, minor: 0, patch: 0 },
            BumpLevel::Minor => Semver { major: self.major, minor: self.minor + 1, patch: 0 },
            BumpLevel::Patch => Semver { major: self.major, minor: self.minor, patch: self.patch + 1 },
        }
    }
}

impl Default for Semver {
    fn default() -> Self {
        Semver { major: 0, minor: 1, patch: 0 }
    }
}

impl std::fmt::Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
```

**Step 4: Add to lib.rs**

Add `pub mod version;` to `crates/aif-skill/src/lib.rs`.

**Step 5: Run tests**

Run: `cargo test -p aif-skill --test version`
Expected: All 6 tests pass

**Step 6: Commit**

```bash
git add crates/aif-skill/src/version.rs crates/aif-skill/src/lib.rs crates/aif-skill/tests/version.rs
git commit -m "feat(skill): add semver parsing and bump logic"
```

---

### Task 7: Skill diff — block-level comparison

**Files:**
- Create: `crates/aif-skill/src/diff.rs`
- Create: `crates/aif-skill/tests/diff.rs`

**Step 1: Write the failing test**

Create `crates/aif-skill/tests/diff.rs`:

```rust
use aif_core::ast::*;
use aif_skill::diff::{diff_skills, ChangeKind};

fn make_skill(steps: Vec<(&str, &str)>, verify: Option<&str>) -> Block {
    let mut children = Vec::new();
    for (i, (order, text)) in steps.iter().enumerate() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("order".to_string(), order.to_string());
        children.push(Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text { text: text.to_string() }],
                children: vec![],
            },
            span: Span::empty(),
        });
    }
    if let Some(v) = verify {
        children.push(Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Verify,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text { text: v.to_string() }],
                children: vec![],
            },
            span: Span::empty(),
        });
    }

    let mut attrs = Attrs::new();
    attrs.pairs.insert("name".to_string(), "test".to_string());
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            title: None,
            content: vec![],
            children,
        },
        span: Span::empty(),
    }
}

#[test]
fn no_changes() {
    let old = make_skill(vec![("1", "step one")], Some("check it"));
    let new = old.clone();
    let changes = diff_skills(&old, &new);
    assert!(changes.is_empty());
}

#[test]
fn added_step() {
    let old = make_skill(vec![("1", "step one")], None);
    let new = make_skill(vec![("1", "step one"), ("2", "step two")], None);
    let changes = diff_skills(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Added);
}

#[test]
fn removed_step() {
    let old = make_skill(vec![("1", "step one"), ("2", "step two")], None);
    let new = make_skill(vec![("1", "step one")], None);
    let changes = diff_skills(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Removed);
}

#[test]
fn modified_text() {
    let old = make_skill(vec![("1", "step one")], None);
    let new = make_skill(vec![("1", "step one updated")], None);
    let changes = diff_skills(&old, &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Modified);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-skill --test diff`
Expected: FAIL — module doesn't exist

**Step 3: Implement diff.rs**

```rust
use aif_core::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub kind: ChangeKind,
    pub block_type: SkillBlockType,
    pub description: String,
}

/// Compare two skill blocks and return a list of changes.
pub fn diff_skills(old: &Block, new: &Block) -> Vec<Change> {
    let old_children = skill_children(old);
    let new_children = skill_children(new);

    let mut changes = Vec::new();

    // Index children by (type, order/position) for matching
    let old_indexed = index_children(&old_children);
    let new_indexed = index_children(&new_children);

    // Find removed and modified
    for (key, old_block) in &old_indexed {
        match new_indexed.get(key) {
            None => {
                changes.push(Change {
                    kind: ChangeKind::Removed,
                    block_type: child_skill_type(old_block),
                    description: format!("Removed {:?} {}", child_skill_type(old_block), key),
                });
            }
            Some(new_block) => {
                if !blocks_equal(old_block, new_block) {
                    changes.push(Change {
                        kind: ChangeKind::Modified,
                        block_type: child_skill_type(old_block),
                        description: format!("Modified {:?} {}", child_skill_type(old_block), key),
                    });
                }
            }
        }
    }

    // Find added
    for (key, new_block) in &new_indexed {
        if !old_indexed.contains_key(key) {
            changes.push(Change {
                kind: ChangeKind::Added,
                block_type: child_skill_type(new_block),
                description: format!("Added {:?} {}", child_skill_type(new_block), key),
            });
        }
    }

    changes
}

fn skill_children(block: &Block) -> Vec<&Block> {
    match &block.kind {
        BlockKind::SkillBlock { children, .. } => children.iter().collect(),
        _ => vec![],
    }
}

fn child_skill_type(block: &Block) -> SkillBlockType {
    match &block.kind {
        BlockKind::SkillBlock { skill_type, .. } => skill_type.clone(),
        _ => SkillBlockType::Step, // fallback
    }
}

fn index_children<'a>(children: &[&'a Block]) -> std::collections::BTreeMap<String, &'a Block> {
    let mut map = std::collections::BTreeMap::new();
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for block in children {
        let (type_name, order) = match &block.kind {
            BlockKind::SkillBlock { skill_type, attrs, .. } => {
                let name = format!("{:?}", skill_type);
                let order = attrs.get("order").map(|s| s.to_string());
                (name, order)
            }
            _ => continue,
        };

        let key = if let Some(ord) = order {
            format!("{}/{}", type_name, ord)
        } else {
            let count = type_counts.entry(type_name.clone()).or_insert(0);
            *count += 1;
            format!("{}/{}", type_name, count)
        };

        map.insert(key, *block);
    }
    map
}

fn blocks_equal(a: &Block, b: &Block) -> bool {
    // Compare ignoring span
    match (&a.kind, &b.kind) {
        (
            BlockKind::SkillBlock { skill_type: st1, attrs: a1, content: c1, children: ch1, title: t1 },
            BlockKind::SkillBlock { skill_type: st2, attrs: a2, content: c2, children: ch2, title: t2 },
        ) => {
            st1 == st2 && a1 == a2 && c1 == c2 && t1 == t2
                && ch1.len() == ch2.len()
                && ch1.iter().zip(ch2.iter()).all(|(x, y)| blocks_equal(x, y))
        }
        _ => a.kind == b.kind,
    }
}
```

**Step 4: Add to lib.rs**

Add `pub mod diff;` to `crates/aif-skill/src/lib.rs`.

**Step 5: Run tests**

Run: `cargo test -p aif-skill --test diff`
Expected: All 4 tests pass

**Step 6: Commit**

```bash
git add crates/aif-skill/src/diff.rs crates/aif-skill/src/lib.rs crates/aif-skill/tests/diff.rs
git commit -m "feat(skill): add block-level semantic diff"
```

---

### Task 8: Change classification

**Files:**
- Create: `crates/aif-skill/src/classify.rs`
- Create: `crates/aif-skill/tests/classify.rs`

**Step 1: Write the failing test**

Create `crates/aif-skill/tests/classify.rs`:

```rust
use aif_core::ast::*;
use aif_skill::diff::{Change, ChangeKind};
use aif_skill::classify::{classify_change, ChangeClass};

#[test]
fn removed_step_is_breaking() {
    let change = Change {
        kind: ChangeKind::Removed,
        block_type: SkillBlockType::Step,
        description: "Removed Step/1".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Breaking);
}

#[test]
fn removed_precondition_is_breaking() {
    let change = Change {
        kind: ChangeKind::Removed,
        block_type: SkillBlockType::Precondition,
        description: "Removed".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Breaking);
}

#[test]
fn added_step_is_additive() {
    let change = Change {
        kind: ChangeKind::Added,
        block_type: SkillBlockType::Step,
        description: "Added Step/2".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Additive);
}

#[test]
fn added_example_is_additive() {
    let change = Change {
        kind: ChangeKind::Added,
        block_type: SkillBlockType::Example,
        description: "Added".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Additive);
}

#[test]
fn modified_step_is_cosmetic() {
    let change = Change {
        kind: ChangeKind::Modified,
        block_type: SkillBlockType::Step,
        description: "Modified Step/1".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Cosmetic);
}

#[test]
fn modified_precondition_is_breaking() {
    let change = Change {
        kind: ChangeKind::Modified,
        block_type: SkillBlockType::Precondition,
        description: "Modified".to_string(),
    };
    assert_eq!(classify_change(&change), ChangeClass::Breaking);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-skill --test classify`
Expected: FAIL — module doesn't exist

**Step 3: Implement classify.rs**

```rust
use crate::diff::{Change, ChangeKind};
use crate::version::BumpLevel;
use aif_core::ast::SkillBlockType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeClass {
    Breaking,
    Additive,
    Cosmetic,
}

impl ChangeClass {
    pub fn bump_level(self) -> BumpLevel {
        match self {
            ChangeClass::Breaking => BumpLevel::Major,
            ChangeClass::Additive => BumpLevel::Minor,
            ChangeClass::Cosmetic => BumpLevel::Patch,
        }
    }
}

/// Classify a single change based on its kind and block type.
pub fn classify_change(change: &Change) -> ChangeClass {
    match change.kind {
        ChangeKind::Removed => {
            // Removing any structural block is breaking
            ChangeClass::Breaking
        }
        ChangeKind::Added => {
            // Adding is always additive
            ChangeClass::Additive
        }
        ChangeKind::Modified => {
            // Modifying critical blocks (precondition, verify, output_contract) is breaking
            // Modifying other blocks is cosmetic (text rewording)
            match change.block_type {
                SkillBlockType::Precondition
                | SkillBlockType::Verify
                | SkillBlockType::OutputContract => ChangeClass::Breaking,
                _ => ChangeClass::Cosmetic,
            }
        }
    }
}

/// Given a list of changes, return the highest-severity bump level needed.
pub fn highest_bump(changes: &[Change]) -> BumpLevel {
    changes
        .iter()
        .map(|c| classify_change(c).bump_level())
        .max_by_key(|b| match b {
            BumpLevel::Major => 2,
            BumpLevel::Minor => 1,
            BumpLevel::Patch => 0,
        })
        .unwrap_or(BumpLevel::Patch)
}
```

**Step 4: Add to lib.rs**

Add `pub mod classify;` to `crates/aif-skill/src/lib.rs`.

**Step 5: Run tests**

Run: `cargo test -p aif-skill --test classify`
Expected: All 6 tests pass

**Step 6: Commit**

```bash
git add crates/aif-skill/src/classify.rs crates/aif-skill/src/lib.rs crates/aif-skill/tests/classify.rs
git commit -m "feat(skill): add change classification (breaking/additive/cosmetic)"
```

---

### Task 9: CLI — `aif skill diff` and `aif skill bump`

**Files:**
- Modify: `crates/aif-cli/src/main.rs`

**Step 1: Add new SkillAction variants**

Add to the `SkillAction` enum:

```rust
/// Compare two skill versions and show changes
Diff {
    /// Old version .aif file
    old: PathBuf,
    /// New version .aif file
    new: PathBuf,
    /// Output format: text (default) or json
    #[arg(long, default_value = "text")]
    format: String,
},
/// Auto-bump version based on semantic changes
Bump {
    input: PathBuf,
    /// Show what would change without modifying
    #[arg(long)]
    dry_run: bool,
},
```

**Step 2: Implement handlers in handle_skill()**

```rust
SkillAction::Diff { old, new, format } => {
    let old_source = read_source(&old);
    let old_doc = parse_aif(&old_source);
    let new_source = read_source(&new);
    let new_doc = parse_aif(&new_source);

    let old_block = find_skill_block(&old_doc.blocks).unwrap_or_else(|| {
        eprintln!("No skill block found in {}", old.display());
        std::process::exit(1);
    });
    let new_block = find_skill_block(&new_doc.blocks).unwrap_or_else(|| {
        eprintln!("No skill block found in {}", new.display());
        std::process::exit(1);
    });

    let changes = aif_skill::diff::diff_skills(old_block, new_block);
    if changes.is_empty() {
        println!("No changes detected.");
        return;
    }

    let bump = aif_skill::classify::highest_bump(&changes);
    for change in &changes {
        let class = aif_skill::classify::classify_change(change);
        println!("  [{:?}] {:?}: {}", class, change.kind, change.description);
    }
    println!("\nRecommended bump: {:?}", bump);
}
SkillAction::Bump { input, dry_run } => {
    let source = read_source(&input);
    let mut doc = parse_aif(&source);

    let skill_block = doc.blocks.iter_mut().find(|b| {
        matches!(
            &b.kind,
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                ..
            }
        )
    });

    if let Some(block) = skill_block {
        if let BlockKind::SkillBlock { ref mut attrs, .. } = block.kind {
            let current = attrs
                .get("version")
                .and_then(|v| aif_skill::version::Semver::parse(v))
                .unwrap_or_default();

            // For bump without a diff target, just do patch bump
            // Full auto-bump requires a previous version to diff against
            let new_version = current.bump(aif_skill::version::BumpLevel::Patch);

            if dry_run {
                println!("Current: {}", current);
                println!("Would bump to: {}", new_version);
            } else {
                attrs.pairs.insert("version".to_string(), new_version.to_string());
                // Recompute hash
                let hash = aif_skill::hash::compute_skill_hash(block);
                attrs.pairs.insert("hash".to_string(), hash.clone());
                let json = serde_json::to_string_pretty(&doc).unwrap();
                fs::write(&input, &json).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", input.display(), e);
                    std::process::exit(1);
                });
                println!("Bumped {} -> {} (hash: {})", current, new_version, hash);
            }
        }
    } else {
        eprintln!("No skill block found in {}", input.display());
        std::process::exit(1);
    }
}
```

**Step 3: Verify**

Run: `cargo build -p aif-cli && cargo run -p aif-cli -- skill diff --help`
Expected: Shows diff subcommand help

Run: `cargo run -p aif-cli -- skill bump --dry-run tests/fixtures/skills/tdd.md 2>/dev/null`
(Note: this will only work on .aif files, not .md — adjust test accordingly)

**Step 4: Commit**

```bash
git add crates/aif-cli/src/main.rs
git commit -m "feat(cli): add skill diff and skill bump subcommands"
```

---

### Task 10: Add binary formats to benchmark

**Files:**
- Modify: `benchmarks/skill_token_benchmark.py`

**Step 1: Add binary formats to FORMATS list**

Add after the existing entries:

```python
("binary_wire",     "Binary Wire",     "binary-wire"),
("binary_token",    "Binary Token",    "binary-token"),
```

**Step 2: Handle binary output in skill_import()**

Binary formats output raw bytes, not text. The benchmark needs to handle this:
- For byte measurement: use raw bytes directly
- For token measurement: base64-encode the binary and count tokens on that
- Note: the `skill_import` function uses `text=True` — for binary formats, need `text=False`

Add a new function:

```python
def skill_import_binary(md_path: str, fmt: str) -> bytes:
    """Import a SKILL.md via CLI, returns raw bytes for binary formats."""
    cmd = [str(AIF_CLI), "skill", "import", "--format", fmt, md_path]
    result = subprocess.run(cmd, capture_output=True, timeout=30)
    if result.returncode != 0:
        print(f"  Warning: import --format {fmt} failed: {result.stderr.decode()}", file=sys.stderr)
        return b""
    return result.stdout
```

Update the main loop to handle binary formats specially (base64 for token counting).

**Step 3: Run benchmark**

Run: `cargo build --release && python3 benchmarks/skill_token_benchmark.py`
Expected: Binary formats appear in results

**Step 4: Commit**

```bash
git add benchmarks/skill_token_benchmark.py
git commit -m "bench: add binary-wire and binary-token formats to benchmark"
```

---

### Task 11: Update CLAUDE.md and architecture docs

**Files:**
- Modify: `CLAUDE.md`
- Modify: `docs/plans/2026-03-31-binary-ir-versioning-design.md`

**Step 1: Add aif-binary to workspace crates table in CLAUDE.md**

Add row: `| aif-binary | Binary serialization — wire (postcard) and token-optimized formats |`

**Step 2: Add new CLI commands**

Update CLI Commands section with:
```bash
aif compile input.aif -f binary-wire|binary-token [-o output]
aif skill diff old.aif new.aif [--format text|json]
aif skill bump input.aif [--dry-run]
```

**Step 3: Commit**

```bash
git add CLAUDE.md docs/
git commit -m "docs: update CLAUDE.md and architecture for binary IR and versioning"
```

---

### Task 12: Full workspace verification

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Build release**

Run: `cargo build --release`
Expected: Success

**Step 3: Quick smoke test**

```bash
./target/release/aif-cli skill import --format binary-wire tests/fixtures/skills/tdd.md 2>/dev/null | wc -c
./target/release/aif-cli skill import --format binary-token tests/fixtures/skills/tdd.md 2>/dev/null | wc -c
./target/release/aif-cli skill import --format json tests/fixtures/skills/tdd.md 2>/dev/null | wc -c
```

Expected: binary-wire < binary-token < JSON in byte count

**Step 4: Commit any final fixes**
