# Skill Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add structured skill representation to AIF with parsing, validation, integrity hashing, SKILL.md import/export, manifest generation, and LML-compact rendering.

**Architecture:** New `aif-skill` crate owns skill-specific logic (validation, hashing, import/export, manifest). Core AST types extended with skill block variants. Parser extended to handle `@skill` container with `@end`-terminated inner blocks. CLI gains `skill` subcommand group.

**Tech Stack:** Rust, sha2 (SHA-256 hashing), pulldown-cmark (MD import reuse), serde/serde_json (manifest), clap (CLI)

---

## File Structure

### New Files
- `crates/aif-skill/Cargo.toml` — New crate manifest
- `crates/aif-skill/src/lib.rs` — Public API re-exports
- `crates/aif-skill/src/validate.rs` — Skill-specific AST validation
- `crates/aif-skill/src/hash.rs` — SHA-256 content hashing
- `crates/aif-skill/src/import.rs` — SKILL.md → AIF import
- `crates/aif-skill/src/export.rs` — AIF → SKILL.md export
- `crates/aif-skill/src/manifest.rs` — Skill manifest generation
- `crates/aif-skill/tests/validate_skill.rs` — Validation tests
- `crates/aif-skill/tests/hash_skill.rs` — Hashing tests
- `crates/aif-skill/tests/import_skill.rs` — Import pipeline tests
- `crates/aif-skill/tests/export_skill.rs` — Export tests
- `crates/aif-skill/tests/manifest_skill.rs` — Manifest tests
- `tests/fixtures/skills/debugging.aif` — Test fixture: full skill
- `tests/fixtures/skills/minimal.aif` — Test fixture: minimal skill
- `tests/fixtures/skills/debugging.md` — Test fixture: SKILL.md input
- `tests/fixtures/skills/multi_skill.md` — Test fixture: multiple skills
- `examples/debugging.aif` — Example skill document

### Modified Files
- `Cargo.toml` — Add `aif-skill` to workspace members
- `crates/aif-core/src/ast.rs` — Add skill block types to `BlockKind` and `SemanticBlockType`
- `crates/aif-parser/src/block.rs` — Parse `@skill` container and `@end` terminators
- `crates/aif-parser/src/lexer.rs` — Add `End` token for `@end`
- `crates/aif-lml/src/emitter.rs` — Add skill block LML emission + `--skill-compact` mode
- `crates/aif-html/src/emitter.rs` — Add skill block HTML rendering
- `crates/aif-markdown/src/emitter.rs` — Add skill block Markdown rendering
- `crates/aif-cli/Cargo.toml` — Add `aif-skill` dependency
- `crates/aif-cli/src/main.rs` — Add `skill` subcommand group

---

### Task 1: Extend Core AST with Skill Block Types

**Files:**
- Modify: `crates/aif-core/src/ast.rs`

- [ ] **Step 1: Add SkillBlockType enum**

Add after the `CalloutType` enum in `crates/aif-core/src/ast.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SkillBlockType {
    Step,
    Verify,
    Precondition,
    OutputContract,
    Decision,
    Tool,
    Fallback,
    RedFlag,
    Example,
}
```

- [ ] **Step 2: Add Skill and SkillInner variants to BlockKind**

Add two new variants to the `BlockKind` enum:

```rust
    Skill {
        attrs: Attrs,
        children: Vec<Block>,
    },
    SkillInner {
        block_type: SkillBlockType,
        attrs: Attrs,
        content: Vec<Inline>,
        children: Vec<Block>,
    },
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p aif-core`
Expected: Compiles with warnings about unused variants (OK for now).

- [ ] **Step 4: Commit**

```bash
git add crates/aif-core/src/ast.rs
git commit -m "feat(core): add Skill and SkillInner block types to AST"
```

---

### Task 2: Add @end Token to Lexer

**Files:**
- Modify: `crates/aif-parser/src/lexer.rs`

- [ ] **Step 1: Write test for @end tokenization**

Add to `crates/aif-parser/tests/parse_blocks.rs`:

```rust
#[test]
fn test_parse_skill_minimal() {
    let input = "@skill[name=\"test\"]\n  Hello world.\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        aif_core::ast::BlockKind::Skill { attrs, children } => {
            assert_eq!(attrs.pairs.get("name").unwrap(), "test");
            assert_eq!(children.len(), 1); // paragraph
        }
        other => panic!("Expected Skill, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-parser test_parse_skill_minimal -- --nocapture`
Expected: FAIL — `Skill` variant not handled in parser yet.

- [ ] **Step 3: Add @end handling to lexer**

The lexer doesn't need a dedicated `@end` token — `@end` is already lexed as `BlockDirective` with slice `"end"`. The parser will check for this. Verify by reading the lexer — `BlockDirective` matches `@[a-zA-Z_]+`.

Run: `cargo check -p aif-parser`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/aif-parser/tests/parse_blocks.rs
git commit -m "test(parser): add failing test for @skill block parsing"
```

---

### Task 3: Parse @skill Container Blocks

**Files:**
- Modify: `crates/aif-parser/src/block.rs`

- [ ] **Step 1: Add skill block type mapping helper**

Add a function to `block.rs`:

```rust
fn skill_inner_type(name: &str) -> Option<SkillBlockType> {
    match name {
        "step" => Some(SkillBlockType::Step),
        "verify" => Some(SkillBlockType::Verify),
        "precondition" => Some(SkillBlockType::Precondition),
        "output_contract" => Some(SkillBlockType::OutputContract),
        "decision" => Some(SkillBlockType::Decision),
        "tool" => Some(SkillBlockType::Tool),
        "fallback" => Some(SkillBlockType::Fallback),
        "red_flag" => Some(SkillBlockType::RedFlag),
        "example" => Some(SkillBlockType::Example),
        _ => None,
    }
}
```

- [ ] **Step 2: Add @skill parsing to parse_directive**

In the `parse_directive` method, add a branch for the `"skill"` directive type. When `block_type == "skill"`:

1. Parse attributes from `[...]`
2. Enter a loop parsing inner lines until `@end` is found
3. Inside the loop, detect inner directives (`@step`, `@verify`, etc.) and parse them as `SkillInner` blocks — each inner block also terminates at `@end`
4. Non-directive lines inside `@skill` are parsed as paragraphs
5. Build `BlockKind::Skill { attrs, children }`

```rust
"skill" => {
    let attrs = attr_str.map(|s| parse_attrs(s)).unwrap_or_default();
    let mut children = Vec::new();

    while self.pos < self.lines.len() {
        let line = self.lines[self.pos].1.trim();

        // Check for @end closing the skill container
        if line == "@end" {
            self.pos += 1;
            break;
        }

        // Check for inner skill directives
        if line.starts_with('@') {
            if let Some(inner) = self.parse_skill_inner() {
                children.push(inner);
                continue;
            }
        }

        // Skip blank lines
        if line.is_empty() {
            self.pos += 1;
            continue;
        }

        // Otherwise parse as paragraph content
        let mut para_lines = Vec::new();
        while self.pos < self.lines.len() {
            let l = self.lines[self.pos].1.trim();
            if l.is_empty() || l.starts_with('@') {
                break;
            }
            para_lines.push(l);
            self.pos += 1;
        }
        if !para_lines.is_empty() {
            let text = para_lines.join(" ");
            children.push(Block {
                kind: BlockKind::Paragraph {
                    content: parse_inline(&text),
                },
                span: Span::empty(),
            });
        }
    }

    Some(Block {
        kind: BlockKind::Skill { attrs, children },
        span: Span::empty(),
    })
}
```

- [ ] **Step 3: Add parse_skill_inner method**

```rust
fn parse_skill_inner(&mut self) -> Option<Block> {
    let line = self.lines[self.pos].1.trim();

    // Parse @type[attrs] pattern
    let rest = line.strip_prefix('@')?;
    let (type_name, attr_str) = if let Some(bracket_start) = rest.find('[') {
        let bracket_end = rest.find(']')?;
        (&rest[..bracket_start], Some(&rest[bracket_start + 1..bracket_end]))
    } else {
        // Type name might have trailing content after space
        let type_name = rest.split_whitespace().next().unwrap_or(rest);
        (type_name, None)
    };

    let block_type = skill_inner_type(type_name)?;
    let attrs = attr_str.map(|s| parse_attrs(s)).unwrap_or_default();

    self.pos += 1;

    // Collect body lines until @end
    let mut body_lines = Vec::new();
    let mut child_blocks = Vec::new();

    while self.pos < self.lines.len() {
        let l = self.lines[self.pos].1.trim();
        if l == "@end" {
            self.pos += 1;
            break;
        }
        body_lines.push(l);
        self.pos += 1;
    }

    let content = if body_lines.is_empty() {
        Vec::new()
    } else {
        let text = body_lines.join("\n");
        parse_inline(text.trim())
    };

    Some(Block {
        kind: BlockKind::SkillInner {
            block_type,
            attrs,
            content,
            children: child_blocks,
        },
        span: Span::empty(),
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aif-parser test_parse_skill_minimal -- --nocapture`
Expected: PASS

- [ ] **Step 5: Add comprehensive skill parsing tests**

Add to `crates/aif-parser/tests/parse_blocks.rs`:

```rust
#[test]
fn test_parse_skill_with_inner_blocks() {
    let input = r#"@skill[name="debugging", version="1.0"]
  @precondition
    User has reported a bug.
  @end

  @step[order=1]
    Reproduce the issue.
  @end

  @step[order=2]
    Find root cause.
  @end

  @verify
    Fix resolves the issue.
  @end

  @fallback
    Escalate to user.
  @end
@end
"#;
    let doc = aif_parser::parse(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        aif_core::ast::BlockKind::Skill { attrs, children } => {
            assert_eq!(attrs.pairs.get("name").unwrap(), "debugging");
            assert_eq!(attrs.pairs.get("version").unwrap(), "1.0");
            // precondition + 2 steps + verify + fallback = 5
            assert_eq!(children.len(), 5);
        }
        other => panic!("Expected Skill, got {:?}", other),
    }
}

#[test]
fn test_parse_skill_inner_block_types() {
    let input = r#"@skill[name="test"]
  @tool[name="grep"]
    Search for patterns.
  @end

  @red_flag
    Don't ignore errors.
  @end

  @example
    See usage below.
  @end

  @decision[condition="if unclear"]
    Ask the user.
  @end

  @output_contract
    Returns a summary.
  @end
@end
"#;
    let doc = aif_parser::parse(input).unwrap();
    match &doc.blocks[0].kind {
        aif_core::ast::BlockKind::Skill { children, .. } => {
            assert_eq!(children.len(), 5);
            // Verify each inner block type
            for child in children {
                match &child.kind {
                    aif_core::ast::BlockKind::SkillInner { block_type, .. } => {
                        match block_type {
                            aif_core::ast::SkillBlockType::Tool
                            | aif_core::ast::SkillBlockType::RedFlag
                            | aif_core::ast::SkillBlockType::Example
                            | aif_core::ast::SkillBlockType::Decision
                            | aif_core::ast::SkillBlockType::OutputContract => {}
                            other => panic!("Unexpected block type: {:?}", other),
                        }
                    }
                    other => panic!("Expected SkillInner, got {:?}", other),
                }
            }
        }
        _ => panic!("Expected Skill"),
    }
}

#[test]
fn test_parse_skill_free_text_only() {
    let input = "@skill[name=\"simple\"]\n  Just some text.\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    match &doc.blocks[0].kind {
        aif_core::ast::BlockKind::Skill { children, .. } => {
            assert_eq!(children.len(), 1);
            assert!(matches!(&children[0].kind, aif_core::ast::BlockKind::Paragraph { .. }));
        }
        _ => panic!("Expected Skill"),
    }
}
```

- [ ] **Step 6: Run all parser tests**

Run: `cargo test -p aif-parser`
Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/aif-core/src/ast.rs crates/aif-parser/src/block.rs crates/aif-parser/tests/parse_blocks.rs
git commit -m "feat(parser): parse @skill container blocks with inner block types"
```

---

### Task 4: Add Skill Block Rendering to HTML, Markdown, and LML Emitters

**Files:**
- Modify: `crates/aif-html/src/emitter.rs`
- Modify: `crates/aif-markdown/src/emitter.rs`
- Modify: `crates/aif-lml/src/emitter.rs`

- [ ] **Step 1: Write HTML rendering test**

Add to `crates/aif-html/tests/render_html.rs` (or create `render_html_extended.rs`):

```rust
#[test]
fn test_render_skill_html() {
    let input = r#"@skill[name="debugging"]
  @precondition
    User reports a bug.
  @end

  @step[order=1]
    Reproduce it.
  @end
@end
"#;
    let doc = aif_parser::parse(input).unwrap();
    let html = aif_html::render_html(&doc);
    assert!(html.contains(r#"<section class="skill" data-name="debugging">"#));
    assert!(html.contains(r#"<div class="skill-precondition">"#));
    assert!(html.contains(r#"<div class="skill-step" data-order="1">"#));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-html test_render_skill_html`
Expected: FAIL — missing match arm.

- [ ] **Step 3: Add Skill rendering to HTML emitter**

In `crates/aif-html/src/emitter.rs`, add match arms in `emit_block`:

```rust
BlockKind::Skill { attrs, children } => {
    let name = attrs.pairs.get("name").map(|s| s.as_str()).unwrap_or("unnamed");
    out.push_str(&format!("<section class=\"skill\" data-name=\"{}\">", name));
    if let Some(version) = attrs.pairs.get("version") {
        out.push_str(&format!("<span class=\"skill-version\">{}</span>", version));
    }
    for child in children {
        emit_block(out, child);
    }
    out.push_str("</section>");
}
BlockKind::SkillInner { block_type, attrs, content, children } => {
    let tag = skill_block_css_class(block_type);
    out.push_str(&format!("<div class=\"{}\">", tag));
    emit_skill_inner_attrs(out, block_type, attrs);
    if !content.is_empty() {
        out.push_str("<p>");
        for inline in content {
            emit_inline(out, inline);
        }
        out.push_str("</p>");
    }
    for child in children {
        emit_block(out, child);
    }
    out.push_str("</div>");
}
```

Add helpers:

```rust
fn skill_block_css_class(bt: &SkillBlockType) -> &'static str {
    match bt {
        SkillBlockType::Step => "skill-step",
        SkillBlockType::Verify => "skill-verify",
        SkillBlockType::Precondition => "skill-precondition",
        SkillBlockType::OutputContract => "skill-output-contract",
        SkillBlockType::Decision => "skill-decision",
        SkillBlockType::Tool => "skill-tool",
        SkillBlockType::Fallback => "skill-fallback",
        SkillBlockType::RedFlag => "skill-red-flag",
        SkillBlockType::Example => "skill-example",
    }
}

fn emit_skill_inner_attrs(out: &mut String, block_type: &SkillBlockType, attrs: &Attrs) {
    match block_type {
        SkillBlockType::Step => {
            if let Some(order) = attrs.pairs.get("order") {
                out.push_str(&format!(" data-order=\"{}\"", order));
                // Fixup: need to embed in opening tag. Adjust approach.
            }
        }
        SkillBlockType::Tool => {
            if let Some(name) = attrs.pairs.get("name") {
                out.push_str(&format!("<strong>{}</strong>", name));
            }
        }
        _ => {}
    }
}
```

Note: The `data-order` attribute needs to be in the opening div tag. Adjust the `SkillInner` arm to build opening tag with attributes:

```rust
BlockKind::SkillInner { block_type, attrs, content, children } => {
    let class = skill_block_css_class(block_type);
    let mut tag = format!("<div class=\"{}\"", class);
    if let Some(order) = attrs.pairs.get("order") {
        tag.push_str(&format!(" data-order=\"{}\"", order));
    }
    if let Some(name) = attrs.pairs.get("name") {
        tag.push_str(&format!(" data-name=\"{}\"", name));
    }
    if let Some(condition) = attrs.pairs.get("condition") {
        tag.push_str(&format!(" data-condition=\"{}\"", condition));
    }
    tag.push('>');
    out.push_str(&tag);
    if !content.is_empty() {
        out.push_str("<p>");
        for inline in content {
            emit_inline(out, inline);
        }
        out.push_str("</p>");
    }
    for child in children {
        emit_block(out, child);
    }
    out.push_str("</div>");
}
```

- [ ] **Step 4: Run HTML test**

Run: `cargo test -p aif-html test_render_skill_html`
Expected: PASS

- [ ] **Step 5: Add Skill rendering to Markdown emitter**

In `crates/aif-markdown/src/emitter.rs`, add match arms:

```rust
BlockKind::Skill { attrs, children } => {
    let name = attrs.pairs.get("name").map(|s| s.as_str()).unwrap_or("unnamed");
    out.push_str(&format!("# Skill: {}\n\n", name));
    if let Some(version) = attrs.pairs.get("version") {
        out.push_str(&format!("*Version: {}*\n\n", version));
    }
    for child in children {
        emit_block(out, child, depth);
    }
}
BlockKind::SkillInner { block_type, attrs, content, children } => {
    let heading = skill_block_heading(block_type);
    out.push_str(&format!("## {}\n\n", heading));
    if let Some(order) = attrs.pairs.get("order") {
        out.push_str(&format!("**Step {}:** ", order));
    }
    if !content.is_empty() {
        for inline in content {
            emit_inline(out, inline);
        }
        out.push('\n');
    }
    out.push('\n');
    for child in children {
        emit_block(out, child, depth);
    }
}
```

Add helper:

```rust
fn skill_block_heading(bt: &SkillBlockType) -> &'static str {
    match bt {
        SkillBlockType::Step => "Steps",
        SkillBlockType::Verify => "Verification",
        SkillBlockType::Precondition => "Prerequisites",
        SkillBlockType::OutputContract => "Expected Output",
        SkillBlockType::Decision => "Decision",
        SkillBlockType::Tool => "Tools",
        SkillBlockType::Fallback => "Fallback",
        SkillBlockType::RedFlag => "Anti-patterns",
        SkillBlockType::Example => "Examples",
    }
}
```

- [ ] **Step 6: Add Skill rendering to LML emitter**

In `crates/aif-lml/src/emitter.rs`, add match arms:

```rust
BlockKind::Skill { attrs, children } => {
    out.push_str("[SKILL");
    emit_attrs(out, attrs);
    out.push_str("]\n");
    for child in children {
        emit_block(out, child, _depth + 1);
    }
    out.push_str("[/SKILL]\n");
}
BlockKind::SkillInner { block_type, attrs, content, children } => {
    let tag = skill_block_lml_tag(block_type);
    out.push_str(&format!("[{}", tag));
    emit_attrs(out, attrs);
    out.push(']');
    if !content.is_empty() {
        out.push(' ');
        emit_inlines_plain(out, content);
    }
    out.push('\n');
    for child in children {
        emit_block(out, child, _depth + 1);
    }
    out.push_str(&format!("[/{}]\n", tag));
}
```

Add helper:

```rust
fn skill_block_lml_tag(bt: &SkillBlockType) -> &'static str {
    match bt {
        SkillBlockType::Step => "STEP",
        SkillBlockType::Verify => "VERIFY",
        SkillBlockType::Precondition => "PRECONDITION",
        SkillBlockType::OutputContract => "OUTPUT",
        SkillBlockType::Decision => "DECISION",
        SkillBlockType::Tool => "TOOL",
        SkillBlockType::Fallback => "FALLBACK",
        SkillBlockType::RedFlag => "RED_FLAG",
        SkillBlockType::Example => "EXAMPLE",
    }
}
```

- [ ] **Step 7: Run all emitter tests**

Run: `cargo test -p aif-html && cargo test -p aif-markdown && cargo test -p aif-lml`
Expected: All PASS

- [ ] **Step 8: Commit**

```bash
git add crates/aif-html/src/emitter.rs crates/aif-markdown/src/emitter.rs crates/aif-lml/src/emitter.rs crates/aif-html/tests/ crates/aif-lml/tests/ crates/aif-markdown/tests/
git commit -m "feat(emitters): render skill blocks in HTML, Markdown, and LML"
```

---

### Task 5: Create aif-skill Crate Skeleton

**Files:**
- Create: `crates/aif-skill/Cargo.toml`
- Create: `crates/aif-skill/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create crate directory**

```bash
mkdir -p crates/aif-skill/src crates/aif-skill/tests
```

- [ ] **Step 2: Write Cargo.toml**

Create `crates/aif-skill/Cargo.toml`:

```toml
[package]
name = "aif-skill"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Skill profile validation, hashing, import/export, and manifest generation for AIF"

[dependencies]
aif-core = { workspace = true }
aif-parser = { path = "../aif-parser" }
pulldown-cmark = "0.12"
sha2 = "0.10"
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
pretty_assertions = "1"
```

- [ ] **Step 3: Write lib.rs**

Create `crates/aif-skill/src/lib.rs`:

```rust
pub mod validate;
pub mod hash;
pub mod import;
pub mod export;
pub mod manifest;

pub use validate::validate_skill;
pub use hash::{compute_hash, verify_hash};
pub use import::import_skill_md;
pub use export::export_skill_md;
pub use manifest::generate_manifest;
```

- [ ] **Step 4: Create empty module files**

Create each with a placeholder function that compiles:

`crates/aif-skill/src/validate.rs`:
```rust
use aif_core::ast::{Block, BlockKind};

pub fn validate_skill(block: &Block) -> Result<(), Vec<String>> {
    todo!()
}
```

`crates/aif-skill/src/hash.rs`:
```rust
use aif_core::ast::Block;

pub fn compute_hash(block: &Block) -> String {
    todo!()
}

pub fn verify_hash(block: &Block) -> bool {
    todo!()
}
```

`crates/aif-skill/src/import.rs`:
```rust
use aif_core::ast::Document;

pub struct ImportResult {
    pub document: Document,
    pub diagnostics: Vec<ImportDiagnostic>,
}

pub struct ImportDiagnostic {
    pub heading: String,
    pub mapped_to: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

pub fn import_skill_md(input: &str) -> ImportResult {
    todo!()
}
```

`crates/aif-skill/src/export.rs`:
```rust
use aif_core::ast::Block;

pub fn export_skill_md(block: &Block) -> String {
    todo!()
}
```

`crates/aif-skill/src/manifest.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillManifest {
    pub skills: Vec<SkillEntry>,
    pub generated: String,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub version: Option<String>,
    pub hash: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub token_count: usize,
    pub blocks: Vec<String>,
    pub path: String,
}

pub fn generate_manifest(dir: &std::path::Path) -> Result<SkillManifest, String> {
    todo!()
}
```

- [ ] **Step 5: Add to workspace**

In root `Cargo.toml`, add `"crates/aif-skill"` to the `members` list and add `aif-skill = { path = "crates/aif-skill" }` to `[workspace.dependencies]`.

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p aif-skill`
Expected: Compiles (with unused warnings).

- [ ] **Step 7: Commit**

```bash
git add crates/aif-skill/ Cargo.toml
git commit -m "feat(skill): create aif-skill crate skeleton with module stubs"
```

---

### Task 6: Implement Skill Validation

**Files:**
- Modify: `crates/aif-skill/src/validate.rs`
- Create: `crates/aif-skill/tests/validate_skill.rs`

- [ ] **Step 1: Write validation tests**

Create `crates/aif-skill/tests/validate_skill.rs`:

```rust
use aif_core::ast::*;
use aif_skill::validate_skill;

fn make_skill(name: &str, children: Vec<Block>) -> Block {
    let mut attrs = Attrs::default();
    attrs.pairs.insert("name".to_string(), name.to_string());
    Block {
        kind: BlockKind::Skill { attrs, children },
        span: Span::empty(),
    }
}

fn make_step(order: u32, text: &str) -> Block {
    let mut attrs = Attrs::default();
    attrs.pairs.insert("order".to_string(), order.to_string());
    Block {
        kind: BlockKind::SkillInner {
            block_type: SkillBlockType::Step,
            attrs,
            content: vec![Inline::Text { text: text.to_string() }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

fn make_inner(block_type: SkillBlockType, text: &str) -> Block {
    Block {
        kind: BlockKind::SkillInner {
            block_type,
            attrs: Attrs::default(),
            content: vec![Inline::Text { text: text.to_string() }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

#[test]
fn test_valid_skill_minimal() {
    let skill = make_skill("test", vec![]);
    assert!(validate_skill(&skill).is_ok());
}

#[test]
fn test_valid_skill_with_steps() {
    let skill = make_skill("debug", vec![
        make_step(1, "First"),
        make_step(2, "Second"),
    ]);
    assert!(validate_skill(&skill).is_ok());
}

#[test]
fn test_invalid_missing_name() {
    let skill = Block {
        kind: BlockKind::Skill {
            attrs: Attrs::default(),
            children: vec![],
        },
        span: Span::empty(),
    };
    let errs = validate_skill(&skill).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("name")));
}

#[test]
fn test_invalid_duplicate_step_order() {
    let skill = make_skill("test", vec![
        make_step(1, "First"),
        make_step(1, "Duplicate"),
    ]);
    let errs = validate_skill(&skill).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("duplicate") || e.contains("order")));
}

#[test]
fn test_invalid_non_contiguous_steps() {
    let skill = make_skill("test", vec![
        make_step(1, "First"),
        make_step(3, "Third"),
    ]);
    let errs = validate_skill(&skill).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("contiguous")));
}

#[test]
fn test_not_a_skill_block() {
    let block = Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: "hello".to_string() }],
        },
        span: Span::empty(),
    };
    let errs = validate_skill(&block).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("not a skill")));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement validation**

Replace `crates/aif-skill/src/validate.rs`:

```rust
use aif_core::ast::{Block, BlockKind, SkillBlockType};

pub fn validate_skill(block: &Block) -> Result<(), Vec<String>> {
    let (attrs, children) = match &block.kind {
        BlockKind::Skill { attrs, children } => (attrs, children),
        _ => return Err(vec!["Block is not a skill block".to_string()]),
    };

    let mut errors = Vec::new();

    // Rule 1: Must have name attribute
    if !attrs.pairs.contains_key("name") {
        errors.push("Skill must have a 'name' attribute".to_string());
    }

    // Collect step orders for validation
    let mut step_orders: Vec<u32> = Vec::new();

    for child in children {
        if let BlockKind::SkillInner { block_type: SkillBlockType::Step, attrs, .. } = &child.kind {
            if let Some(order_str) = attrs.pairs.get("order") {
                match order_str.parse::<u32>() {
                    Ok(order) => step_orders.push(order),
                    Err(_) => errors.push(format!("Step order '{}' is not a valid integer", order_str)),
                }
            }
        }
    }

    // Rule 2: Step orders must be unique
    let mut sorted = step_orders.clone();
    sorted.sort();
    sorted.dedup();
    if sorted.len() != step_orders.len() {
        errors.push("Step blocks have duplicate order values".to_string());
    }

    // Rule 3: Step orders must be contiguous starting from 1
    if !sorted.is_empty() {
        let expected: Vec<u32> = (1..=sorted.len() as u32).collect();
        if sorted != expected {
            errors.push("Step order values must be contiguous starting from 1".to_string());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p aif-skill`
Expected: All PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/validate.rs crates/aif-skill/tests/validate_skill.rs
git commit -m "feat(skill): implement skill block validation with step order checks"
```

---

### Task 7: Implement SHA-256 Integrity Hashing

**Files:**
- Modify: `crates/aif-skill/src/hash.rs`
- Create: `crates/aif-skill/tests/hash_skill.rs`

- [ ] **Step 1: Write hashing tests**

Create `crates/aif-skill/tests/hash_skill.rs`:

```rust
use aif_skill::{compute_hash, verify_hash};

#[test]
fn test_compute_hash_deterministic() {
    let input = "@skill[name=\"test\"]\n  Hello world.\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    let hash1 = compute_hash(&doc.blocks[0]);
    let hash2 = compute_hash(&doc.blocks[0]);
    assert_eq!(hash1, hash2);
    assert!(hash1.starts_with("sha256:"));
}

#[test]
fn test_compute_hash_different_content() {
    let input1 = "@skill[name=\"a\"]\n  Content one.\n@end\n";
    let input2 = "@skill[name=\"a\"]\n  Content two.\n@end\n";
    let doc1 = aif_parser::parse(input1).unwrap();
    let doc2 = aif_parser::parse(input2).unwrap();
    let hash1 = compute_hash(&doc1.blocks[0]);
    let hash2 = compute_hash(&doc2.blocks[0]);
    assert_ne!(hash1, hash2);
}

#[test]
fn test_verify_hash_correct() {
    let input = "@skill[name=\"test\"]\n  Hello world.\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    let hash = compute_hash(&doc.blocks[0]);

    // Re-parse with hash embedded
    let input_with_hash = format!(
        "@skill[name=\"test\", hash=\"{}\"]\n  Hello world.\n@end\n",
        hash
    );
    let doc2 = aif_parser::parse(&input_with_hash).unwrap();
    assert!(verify_hash(&doc2.blocks[0]));
}

#[test]
fn test_verify_hash_tampered() {
    let input = "@skill[name=\"test\", hash=\"sha256:0000000000000000000000000000000000000000000000000000000000000000\"]\n  Tampered content.\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    assert!(!verify_hash(&doc.blocks[0]));
}

#[test]
fn test_verify_hash_no_hash_attr() {
    let input = "@skill[name=\"test\"]\n  No hash.\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    // No hash attribute means verification returns false (nothing to verify)
    assert!(!verify_hash(&doc.blocks[0]));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill hash`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement hashing**

Replace `crates/aif-skill/src/hash.rs`:

```rust
use aif_core::ast::{Block, BlockKind, Inline, SkillBlockType};
use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of a skill block's content.
/// Returns "sha256:<hex>" string.
pub fn compute_hash(block: &Block) -> String {
    let content = normalize_skill_content(block);
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}

/// Verify the hash attribute of a skill block matches its content.
pub fn verify_hash(block: &Block) -> bool {
    let attrs = match &block.kind {
        BlockKind::Skill { attrs, .. } => attrs,
        _ => return false,
    };

    let stored_hash = match attrs.pairs.get("hash") {
        Some(h) => h,
        None => return false,
    };

    let computed = compute_hash(block);
    computed == *stored_hash
}

/// Normalize skill content for hashing.
/// Serializes all children to a canonical text form, excluding the hash attribute.
fn normalize_skill_content(block: &Block) -> String {
    let children = match &block.kind {
        BlockKind::Skill { children, .. } => children,
        _ => return String::new(),
    };

    let mut out = String::new();
    for child in children {
        normalize_block(&mut out, child);
        out.push('\n');
    }
    out.trim().to_string()
}

fn normalize_block(out: &mut String, block: &Block) {
    match &block.kind {
        BlockKind::SkillInner { block_type, attrs, content, children } => {
            out.push_str(&format!("@{}", skill_type_name(block_type)));
            // Include non-hash attributes
            for (key, value) in &attrs.pairs {
                if key != "hash" {
                    out.push_str(&format!("[{}={}]", key, value));
                }
            }
            out.push('\n');
            normalize_inlines(out, content);
            for child in children {
                normalize_block(out, child);
            }
        }
        BlockKind::Paragraph { content } => {
            normalize_inlines(out, content);
        }
        _ => {
            // For other block types, use debug repr as fallback
            out.push_str(&format!("{:?}", block.kind));
        }
    }
}

fn normalize_inlines(out: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text.trim()),
            Inline::Emphasis { content } => {
                out.push('*');
                normalize_inlines(out, content);
                out.push('*');
            }
            Inline::Strong { content } => {
                out.push_str("**");
                normalize_inlines(out, content);
                out.push_str("**");
            }
            Inline::InlineCode { code } => {
                out.push('`');
                out.push_str(code);
                out.push('`');
            }
            Inline::Link { text, url } => {
                out.push('[');
                normalize_inlines(out, text);
                out.push_str("](");
                out.push_str(url);
                out.push(')');
            }
            Inline::SoftBreak | Inline::HardBreak => out.push('\n'),
            Inline::Reference { target } => out.push_str(target),
            Inline::Footnote { content } => normalize_inlines(out, content),
        }
    }
}

fn skill_type_name(bt: &SkillBlockType) -> &'static str {
    match bt {
        SkillBlockType::Step => "step",
        SkillBlockType::Verify => "verify",
        SkillBlockType::Precondition => "precondition",
        SkillBlockType::OutputContract => "output_contract",
        SkillBlockType::Decision => "decision",
        SkillBlockType::Tool => "tool",
        SkillBlockType::Fallback => "fallback",
        SkillBlockType::RedFlag => "red_flag",
        SkillBlockType::Example => "example",
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p aif-skill hash`
Expected: All PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/hash.rs crates/aif-skill/tests/hash_skill.rs
git commit -m "feat(skill): implement SHA-256 integrity hashing for skill blocks"
```

---

### Task 8: Implement SKILL.md Import

**Files:**
- Modify: `crates/aif-skill/src/import.rs`
- Create: `crates/aif-skill/tests/import_skill.rs`
- Create: `tests/fixtures/skills/debugging.md`

- [ ] **Step 1: Create test fixture**

Create `tests/fixtures/skills/debugging.md`:

```markdown
# Debugging

## Prerequisites

User has reported a bug or test failure.

## Steps

1. Reproduce the issue with a minimal test case
2. Identify the root cause using logs and debugger
3. Write a failing test that captures the bug
4. Implement the fix
5. Verify no regressions

## Verification

The fix resolves the original issue without introducing regressions.
All existing tests continue to pass.

## Tools

- `grep` — search for patterns in source code
- `git bisect` — find the commit that introduced the bug

## Fallback

If root cause is unclear after 3 attempts, escalate to the user with findings so far.

## Anti-patterns

- Don't apply fixes without understanding the root cause
- Don't skip writing a regression test

## Examples

A user reports that login fails on mobile. You reproduce by testing on a mobile viewport, find the CSS media query is wrong, write a test, fix it.
```

- [ ] **Step 2: Write import tests**

Create `crates/aif-skill/tests/import_skill.rs`:

```rust
use aif_core::ast::*;
use aif_skill::import::{import_skill_md, Confidence};

#[test]
fn test_import_skill_name_from_h1() {
    let input = "# My Skill\n\n## Steps\n\n1. Do something\n";
    let result = import_skill_md(input);
    match &result.document.blocks[0].kind {
        BlockKind::Skill { attrs, .. } => {
            assert_eq!(attrs.pairs.get("name").unwrap(), "My Skill");
        }
        _ => panic!("Expected Skill block"),
    }
}

#[test]
fn test_import_steps_mapping() {
    let input = "# Test\n\n## Steps\n\n1. First step\n2. Second step\n";
    let result = import_skill_md(input);
    match &result.document.blocks[0].kind {
        BlockKind::Skill { children, .. } => {
            let steps: Vec<_> = children.iter().filter(|b| matches!(
                &b.kind,
                BlockKind::SkillInner { block_type: SkillBlockType::Step, .. }
            )).collect();
            assert_eq!(steps.len(), 2);
        }
        _ => panic!("Expected Skill block"),
    }
}

#[test]
fn test_import_prerequisites_high_confidence() {
    let input = "# Test\n\n## Prerequisites\n\nMust have access.\n";
    let result = import_skill_md(input);
    let diag = result.diagnostics.iter().find(|d| d.mapped_to == "precondition").unwrap();
    assert_eq!(diag.confidence, Confidence::High);
}

#[test]
fn test_import_tools_medium_confidence() {
    let input = "# Test\n\n## Tools\n\nUse grep.\n";
    let result = import_skill_md(input);
    let diag = result.diagnostics.iter().find(|d| d.mapped_to == "tool").unwrap();
    assert_eq!(diag.confidence, Confidence::Medium);
}

#[test]
fn test_import_unrecognized_heading_as_paragraph() {
    let input = "# Test\n\n## Random Section\n\nSome content.\n";
    let result = import_skill_md(input);
    match &result.document.blocks[0].kind {
        BlockKind::Skill { children, .. } => {
            // Unrecognized section becomes paragraph
            assert!(!children.is_empty());
            assert!(children.iter().any(|b| matches!(&b.kind, BlockKind::Paragraph { .. })));
        }
        _ => panic!("Expected Skill block"),
    }
}

#[test]
fn test_import_full_debugging_skill() {
    let input = std::fs::read_to_string("../../tests/fixtures/skills/debugging.md").unwrap();
    let result = import_skill_md(&input);
    match &result.document.blocks[0].kind {
        BlockKind::Skill { attrs, children } => {
            assert_eq!(attrs.pairs.get("name").unwrap(), "Debugging");
            // Should have: precondition, 5 steps, verify, tool, fallback, red_flag, example
            assert!(children.len() >= 5);
        }
        _ => panic!("Expected Skill block"),
    }
    // Should have diagnostics for each mapped section
    assert!(!result.diagnostics.is_empty());
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p aif-skill import`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 4: Implement import**

Replace `crates/aif-skill/src/import.rs`:

```rust
use aif_core::ast::*;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

pub struct ImportResult {
    pub document: Document,
    pub diagnostics: Vec<ImportDiagnostic>,
}

pub struct ImportDiagnostic {
    pub heading: String,
    pub mapped_to: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

struct HeadingMapping {
    patterns: &'static [&'static str],
    block_type: SkillBlockType,
    mapped_name: &'static str,
    confidence: Confidence,
}

const HEADING_MAPPINGS: &[HeadingMapping] = &[
    HeadingMapping {
        patterns: &["steps", "procedure", "how to", "instructions"],
        block_type: SkillBlockType::Step,
        mapped_name: "step",
        confidence: Confidence::High,
    },
    HeadingMapping {
        patterns: &["prerequisites", "requirements", "when to use", "preconditions"],
        block_type: SkillBlockType::Precondition,
        mapped_name: "precondition",
        confidence: Confidence::High,
    },
    HeadingMapping {
        patterns: &["verification", "testing", "acceptance"],
        block_type: SkillBlockType::Verify,
        mapped_name: "verify",
        confidence: Confidence::High,
    },
    HeadingMapping {
        patterns: &["examples", "usage", "example"],
        block_type: SkillBlockType::Example,
        mapped_name: "example",
        confidence: Confidence::High,
    },
    HeadingMapping {
        patterns: &["tools", "commands"],
        block_type: SkillBlockType::Tool,
        mapped_name: "tool",
        confidence: Confidence::Medium,
    },
    HeadingMapping {
        patterns: &["fallback", "recovery", "if stuck"],
        block_type: SkillBlockType::Fallback,
        mapped_name: "fallback",
        confidence: Confidence::Medium,
    },
    HeadingMapping {
        patterns: &["anti-patterns", "don't", "avoid", "anti patterns"],
        block_type: SkillBlockType::RedFlag,
        mapped_name: "red_flag",
        confidence: Confidence::Medium,
    },
    HeadingMapping {
        patterns: &["output", "expected output", "returns"],
        block_type: SkillBlockType::OutputContract,
        mapped_name: "output_contract",
        confidence: Confidence::Medium,
    },
    HeadingMapping {
        patterns: &["decision", "choose", "options"],
        block_type: SkillBlockType::Decision,
        mapped_name: "decision",
        confidence: Confidence::Low,
    },
];

fn match_heading(heading: &str) -> Option<&'static HeadingMapping> {
    let lower = heading.to_lowercase();
    HEADING_MAPPINGS.iter().find(|m| {
        m.patterns.iter().any(|p| lower == *p || lower.starts_with(p))
    })
}

pub fn import_skill_md(input: &str) -> ImportResult {
    let parser = Parser::new(input);
    let events: Vec<Event> = parser.collect();

    let mut skill_name = String::from("unnamed");
    let mut children: Vec<Block> = Vec::new();
    let mut diagnostics: Vec<ImportDiagnostic> = Vec::new();

    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) => {
                // Extract skill name from H1
                i += 1;
                let mut name_parts = Vec::new();
                while i < events.len() && !matches!(&events[i], Event::End(TagEnd::Heading(HeadingLevel::H1))) {
                    if let Event::Text(text) = &events[i] {
                        name_parts.push(text.to_string());
                    }
                    i += 1;
                }
                skill_name = name_parts.join("");
            }
            Event::Start(Tag::Heading { level: HeadingLevel::H2, .. }) => {
                // Extract heading text
                i += 1;
                let mut heading_text = String::new();
                while i < events.len() && !matches!(&events[i], Event::End(TagEnd::Heading(HeadingLevel::H2))) {
                    if let Event::Text(text) = &events[i] {
                        heading_text.push_str(text);
                    }
                    i += 1;
                }
                i += 1; // skip End(Heading)

                // Collect section content until next heading or end
                let mut section_text = Vec::new();
                let mut list_items: Vec<String> = Vec::new();
                let mut in_list_item = false;

                while i < events.len() {
                    match &events[i] {
                        Event::Start(Tag::Heading { .. }) => break,
                        Event::Start(Tag::Item) => {
                            in_list_item = true;
                            i += 1;
                            continue;
                        }
                        Event::End(TagEnd::Item) => {
                            in_list_item = false;
                            i += 1;
                            continue;
                        }
                        Event::Text(text) => {
                            if in_list_item {
                                list_items.push(text.to_string());
                            } else {
                                section_text.push(text.to_string());
                            }
                        }
                        Event::Code(code) => {
                            if in_list_item {
                                list_items.push(format!("`{}`", code));
                            } else {
                                section_text.push(format!("`{}`", code));
                            }
                        }
                        Event::SoftBreak | Event::HardBreak => {
                            section_text.push(" ".to_string());
                        }
                        _ => {}
                    }
                    i += 1;
                }

                // Map heading to block type
                if let Some(mapping) = match_heading(&heading_text) {
                    diagnostics.push(ImportDiagnostic {
                        heading: heading_text.clone(),
                        mapped_to: mapping.mapped_name.to_string(),
                        confidence: mapping.confidence.clone(),
                    });

                    if matches!(mapping.block_type, SkillBlockType::Step) && !list_items.is_empty() {
                        // Split numbered list into individual @step blocks
                        for (idx, item) in list_items.iter().enumerate() {
                            let mut attrs = Attrs::default();
                            attrs.pairs.insert("order".to_string(), (idx + 1).to_string());
                            children.push(Block {
                                kind: BlockKind::SkillInner {
                                    block_type: SkillBlockType::Step,
                                    attrs,
                                    content: vec![Inline::Text { text: item.clone() }],
                                    children: vec![],
                                },
                                span: Span::empty(),
                            });
                        }
                    } else {
                        let text = if !section_text.is_empty() {
                            section_text.join(" ").trim().to_string()
                        } else {
                            list_items.join("\n")
                        };
                        children.push(Block {
                            kind: BlockKind::SkillInner {
                                block_type: mapping.block_type.clone(),
                                attrs: Attrs::default(),
                                content: vec![Inline::Text { text }],
                                children: vec![],
                            },
                            span: Span::empty(),
                        });
                    }
                } else {
                    // Unrecognized heading → paragraph
                    let text = if !section_text.is_empty() {
                        section_text.join(" ").trim().to_string()
                    } else {
                        list_items.join("\n")
                    };
                    if !text.is_empty() {
                        children.push(Block {
                            kind: BlockKind::Paragraph {
                                content: vec![Inline::Text { text }],
                            },
                            span: Span::empty(),
                        });
                    }
                }
                continue; // skip the i += 1 at the end
            }
            _ => {}
        }
        i += 1;
    }

    let mut attrs = Attrs::default();
    attrs.pairs.insert("name".to_string(), skill_name);

    let skill_block = Block {
        kind: BlockKind::Skill { attrs, children },
        span: Span::empty(),
    };

    ImportResult {
        document: Document {
            metadata: std::collections::BTreeMap::new(),
            blocks: vec![skill_block],
        },
        diagnostics,
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p aif-skill import`
Expected: All PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-skill/src/import.rs crates/aif-skill/tests/import_skill.rs tests/fixtures/skills/debugging.md
git commit -m "feat(skill): implement SKILL.md import with heading auto-detection"
```

---

### Task 9: Implement Skill Export (AIF → SKILL.md)

**Files:**
- Modify: `crates/aif-skill/src/export.rs`
- Create: `crates/aif-skill/tests/export_skill.rs`

- [ ] **Step 1: Write export tests**

Create `crates/aif-skill/tests/export_skill.rs`:

```rust
use aif_skill::export_skill_md;

#[test]
fn test_export_skill_roundtrip_name() {
    let input = "@skill[name=\"debugging\", version=\"1.0\"]\n  @precondition\n    User reports a bug.\n  @end\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    let md = export_skill_md(&doc.blocks[0]);
    assert!(md.contains("# debugging"));
    assert!(md.contains("version: \"1.0\""));
}

#[test]
fn test_export_skill_steps_as_numbered_list() {
    let input = "@skill[name=\"test\"]\n  @step[order=1]\n    First step.\n  @end\n  @step[order=2]\n    Second step.\n  @end\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    let md = export_skill_md(&doc.blocks[0]);
    assert!(md.contains("## Steps"));
    assert!(md.contains("1. First step."));
    assert!(md.contains("2. Second step."));
}

#[test]
fn test_export_skill_precondition() {
    let input = "@skill[name=\"test\"]\n  @precondition\n    Must have access.\n  @end\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    let md = export_skill_md(&doc.blocks[0]);
    assert!(md.contains("## Prerequisites"));
    assert!(md.contains("Must have access."));
}

#[test]
fn test_export_skill_red_flag() {
    let input = "@skill[name=\"test\"]\n  @red_flag\n    Don't skip tests.\n  @end\n@end\n";
    let doc = aif_parser::parse(input).unwrap();
    let md = export_skill_md(&doc.blocks[0]);
    assert!(md.contains("## Anti-patterns"));
    assert!(md.contains("Don't skip tests."));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill export`
Expected: FAIL.

- [ ] **Step 3: Implement export**

Replace `crates/aif-skill/src/export.rs`:

```rust
use aif_core::ast::{Attrs, Block, BlockKind, Inline, SkillBlockType};

pub fn export_skill_md(block: &Block) -> String {
    let (attrs, children) = match &block.kind {
        BlockKind::Skill { attrs, children } => (attrs, children),
        _ => return String::from("# Error: not a skill block\n"),
    };

    let mut out = String::new();

    // YAML frontmatter with metadata
    let name = attrs.pairs.get("name").map(|s| s.as_str()).unwrap_or("unnamed");

    // Frontmatter
    out.push_str("---\n");
    if let Some(hash) = attrs.pairs.get("hash") {
        out.push_str(&format!("hash: \"{}\"\n", hash));
    }
    if let Some(version) = attrs.pairs.get("version") {
        out.push_str(&format!("version: \"{}\"\n", version));
    }
    if let Some(tags) = attrs.pairs.get("tags") {
        out.push_str(&format!("tags: \"{}\"\n", tags));
    }
    if let Some(priority) = attrs.pairs.get("priority") {
        out.push_str(&format!("priority: \"{}\"\n", priority));
    }
    out.push_str("---\n\n");

    // Title
    out.push_str(&format!("# {}\n\n", name));

    // Group steps together for numbered list output
    let mut steps: Vec<(&Attrs, &[Inline])> = Vec::new();
    let mut other_blocks: Vec<&Block> = Vec::new();
    let mut step_position: Option<usize> = None;

    for (i, child) in children.iter().enumerate() {
        match &child.kind {
            BlockKind::SkillInner { block_type: SkillBlockType::Step, attrs, content, .. } => {
                if step_position.is_none() {
                    step_position = Some(i);
                }
                steps.push((attrs, content));
            }
            _ => other_blocks.push(child),
        }
    }

    // Emit non-step blocks first, then steps at their position
    // Actually, let's emit in order: group steps, emit others inline
    let mut emitted_steps = false;
    for child in children {
        match &child.kind {
            BlockKind::SkillInner { block_type: SkillBlockType::Step, .. } => {
                if !emitted_steps {
                    emitted_steps = true;
                    out.push_str("## Steps\n\n");
                    for (attrs, content) in &steps {
                        let order = attrs.pairs.get("order").map(|s| s.as_str()).unwrap_or("?");
                        out.push_str(&format!("{}. ", order));
                        emit_inlines_to_text(&mut out, content);
                        out.push('\n');
                    }
                    out.push('\n');
                }
            }
            BlockKind::SkillInner { block_type, content, .. } => {
                let heading = export_heading(block_type);
                out.push_str(&format!("## {}\n\n", heading));
                emit_inlines_to_text(&mut out, content);
                out.push_str("\n\n");
            }
            BlockKind::Paragraph { content } => {
                emit_inlines_to_text(&mut out, content);
                out.push_str("\n\n");
            }
            _ => {}
        }
    }

    out
}

fn export_heading(bt: &SkillBlockType) -> &'static str {
    match bt {
        SkillBlockType::Step => "Steps",
        SkillBlockType::Verify => "Verification",
        SkillBlockType::Precondition => "Prerequisites",
        SkillBlockType::OutputContract => "Expected Output",
        SkillBlockType::Decision => "Decision",
        SkillBlockType::Tool => "Tools",
        SkillBlockType::Fallback => "Fallback",
        SkillBlockType::RedFlag => "Anti-patterns",
        SkillBlockType::Example => "Examples",
    }
}

fn emit_inlines_to_text(out: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Emphasis { content } => {
                out.push('*');
                emit_inlines_to_text(out, content);
                out.push('*');
            }
            Inline::Strong { content } => {
                out.push_str("**");
                emit_inlines_to_text(out, content);
                out.push_str("**");
            }
            Inline::InlineCode { code } => {
                out.push('`');
                out.push_str(code);
                out.push('`');
            }
            Inline::Link { text, url } => {
                out.push('[');
                emit_inlines_to_text(out, text);
                out.push_str("](");
                out.push_str(url);
                out.push(')');
            }
            Inline::SoftBreak | Inline::HardBreak => out.push('\n'),
            Inline::Reference { target } => out.push_str(target),
            Inline::Footnote { content } => emit_inlines_to_text(out, content),
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p aif-skill export`
Expected: All PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/export.rs crates/aif-skill/tests/export_skill.rs
git commit -m "feat(skill): implement AIF-to-SKILL.md export with frontmatter"
```

---

### Task 10: Implement Skill Manifest Generation

**Files:**
- Modify: `crates/aif-skill/src/manifest.rs`
- Create: `crates/aif-skill/tests/manifest_skill.rs`
- Create: `tests/fixtures/skills/debugging.aif`
- Create: `tests/fixtures/skills/minimal.aif`

- [ ] **Step 1: Create test fixtures**

Create `tests/fixtures/skills/debugging.aif`:

```aif
@skill[name="debugging", version="1.0", tags="process,troubleshooting", priority="high"]
  @precondition
    User has reported a bug or test failure.
  @end

  @step[order=1]
    Reproduce the issue.
  @end

  @step[order=2]
    Find root cause.
  @end

  @verify
    Fix resolves issue without regressions.
  @end

  @fallback
    Escalate to user after 3 attempts.
  @end
@end
```

Create `tests/fixtures/skills/minimal.aif`:

```aif
@skill[name="minimal"]
  Just a simple skill.
@end
```

- [ ] **Step 2: Write manifest tests**

Create `crates/aif-skill/tests/manifest_skill.rs`:

```rust
use aif_skill::manifest::generate_manifest;
use std::path::Path;

#[test]
fn test_generate_manifest_from_fixtures() {
    let dir = Path::new("../../tests/fixtures/skills");
    let manifest = generate_manifest(dir).unwrap();
    assert!(manifest.skills.len() >= 2);

    let debugging = manifest.skills.iter().find(|s| s.name == "debugging").unwrap();
    assert_eq!(debugging.version.as_deref(), Some("1.0"));
    assert!(debugging.tags.contains(&"process".to_string()));
    assert_eq!(debugging.priority.as_deref(), Some("high"));
    assert!(debugging.blocks.contains(&"precondition".to_string()));
    assert!(debugging.blocks.contains(&"step".to_string()));

    let minimal = manifest.skills.iter().find(|s| s.name == "minimal").unwrap();
    assert!(minimal.blocks.is_empty() || minimal.blocks.len() == 0);
}

#[test]
fn test_manifest_serializes_to_json() {
    let dir = Path::new("../../tests/fixtures/skills");
    let manifest = generate_manifest(dir).unwrap();
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    assert!(json.contains("\"name\": \"debugging\""));
    assert!(json.contains("\"skills\""));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p aif-skill manifest`
Expected: FAIL.

- [ ] **Step 4: Implement manifest generation**

Replace `crates/aif-skill/src/manifest.rs`:

```rust
use aif_core::ast::{BlockKind, SkillBlockType};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillManifest {
    pub skills: Vec<SkillEntry>,
    pub generated: String,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    pub token_count: usize,
    pub blocks: Vec<String>,
    pub path: String,
}

pub fn generate_manifest(dir: &Path) -> Result<SkillManifest, String> {
    let mut skills = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Cannot read entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("aif") {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;

        let doc = aif_parser::parse(&content)
            .map_err(|errs| format!("Parse error in {}: {:?}", path.display(), errs))?;

        for block in &doc.blocks {
            if let BlockKind::Skill { attrs, children } = &block.kind {
                let name = attrs.pairs.get("name")
                    .cloned()
                    .unwrap_or_else(|| "unnamed".to_string());

                let version = attrs.pairs.get("version").cloned();
                let hash = attrs.pairs.get("hash").cloned();
                let priority = attrs.pairs.get("priority").cloned();

                let tags = attrs.pairs.get("tags")
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                let mut block_types: Vec<String> = Vec::new();
                for child in children {
                    if let BlockKind::SkillInner { block_type, .. } = &child.kind {
                        let type_name = skill_type_name(block_type).to_string();
                        if !block_types.contains(&type_name) {
                            block_types.push(type_name);
                        }
                    }
                }

                // Rough token estimate: ~4 chars per token
                let token_count = content.len() / 4;

                let rel_path = path.file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("unknown.aif")
                    .to_string();

                skills.push(SkillEntry {
                    name,
                    version,
                    hash,
                    tags,
                    priority,
                    token_count,
                    blocks: block_types,
                    path: rel_path,
                });
            }
        }
    }

    let total_tokens: usize = skills.iter().map(|s| s.token_count).sum();

    Ok(SkillManifest {
        skills,
        generated: chrono_now(),
        total_tokens,
    })
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without chrono dependency
    // In production, use proper time crate
    "2026-03-30T00:00:00Z".to_string()
}

fn skill_type_name(bt: &SkillBlockType) -> &'static str {
    match bt {
        SkillBlockType::Step => "step",
        SkillBlockType::Verify => "verify",
        SkillBlockType::Precondition => "precondition",
        SkillBlockType::OutputContract => "output_contract",
        SkillBlockType::Decision => "decision",
        SkillBlockType::Tool => "tool",
        SkillBlockType::Fallback => "fallback",
        SkillBlockType::RedFlag => "red_flag",
        SkillBlockType::Example => "example",
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p aif-skill manifest`
Expected: All PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-skill/src/manifest.rs crates/aif-skill/tests/manifest_skill.rs tests/fixtures/skills/debugging.aif tests/fixtures/skills/minimal.aif
git commit -m "feat(skill): implement skill manifest generation from directory"
```

---

### Task 11: Add LML Skill-Compact Mode

**Files:**
- Modify: `crates/aif-lml/src/emitter.rs`
- Modify: `crates/aif-lml/src/lib.rs`

- [ ] **Step 1: Write compact mode test**

Add to `crates/aif-lml/tests/render_lml.rs` (or `render_lml_extended.rs`):

```rust
#[test]
fn test_render_skill_compact_strips_examples() {
    let input = r#"@skill[name="test"]
  @precondition
    Must have access.
  @end

  @step[order=1]
    Do the first thing carefully and thoroughly.
  @end

  @example
    This is a long example that should be stripped in compact mode.
  @end

  @verify
    Check results.
  @end
@end
"#;
    let doc = aif_parser::parse(input).unwrap();
    let compact = aif_lml::render_lml_compact(&doc);
    // Should NOT contain example content
    assert!(!compact.contains("long example"));
    // Should contain precondition and verify in full
    assert!(compact.contains("Must have access"));
    assert!(compact.contains("Check results"));
    // Steps should be condensed
    assert!(compact.contains("STEP"));
}

#[test]
fn test_render_skill_compact_preserves_red_flags() {
    let input = r#"@skill[name="test"]
  @red_flag
    Never skip verification.
  @end
@end
"#;
    let doc = aif_parser::parse(input).unwrap();
    let compact = aif_lml::render_lml_compact(&doc);
    assert!(compact.contains("Never skip verification"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-lml compact`
Expected: FAIL — function doesn't exist.

- [ ] **Step 3: Add render_lml_compact to lib.rs**

In `crates/aif-lml/src/lib.rs`, add:

```rust
pub use emitter::{emit_lml, emit_lml_compact};

pub fn render_lml(doc: &aif_core::ast::Document) -> String {
    emit_lml(doc)
}

pub fn render_lml_compact(doc: &aif_core::ast::Document) -> String {
    emit_lml_compact(doc)
}
```

- [ ] **Step 4: Add emit_lml_compact to emitter**

In `crates/aif-lml/src/emitter.rs`, add:

```rust
pub fn emit_lml_compact(doc: &Document) -> String {
    let mut out = String::new();
    emit_doc_compact(&mut out, doc);
    out
}

fn emit_doc_compact(out: &mut String, doc: &Document) {
    out.push_str("[DOC");
    for (key, value) in &doc.metadata {
        out.push(' ');
        emit_attr_pair(out, key, value);
    }
    out.push_str("]\n");
    for block in &doc.blocks {
        emit_block_compact(out, block, 0);
    }
    out.push_str("[/DOC]\n");
}

fn emit_block_compact(out: &mut String, block: &Block, depth: usize) {
    match &block.kind {
        BlockKind::Skill { attrs, children } => {
            out.push_str("[SKILL");
            emit_attrs(out, attrs);
            out.push_str("]\n");
            for child in children {
                emit_block_compact(out, child, depth + 1);
            }
            out.push_str("[/SKILL]\n");
        }
        BlockKind::SkillInner { block_type, attrs, content, children } => {
            match block_type {
                // Strip examples entirely
                SkillBlockType::Example => return,
                // Condense steps to single line
                SkillBlockType::Step => {
                    let tag = skill_block_lml_tag(block_type);
                    out.push_str(&format!("[{}", tag));
                    emit_attrs(out, attrs);
                    out.push(']');
                    if !content.is_empty() {
                        out.push(' ');
                        // Truncate step content to first sentence
                        let mut text = String::new();
                        emit_inlines_plain(&mut text, content);
                        let first_sentence = text.split('.').next().unwrap_or(&text);
                        out.push_str(first_sentence.trim());
                    }
                    out.push('\n');
                    return;
                }
                // Preserve precondition, verify, red_flag in full
                _ => {
                    let tag = skill_block_lml_tag(block_type);
                    out.push_str(&format!("[{}", tag));
                    emit_attrs(out, attrs);
                    out.push(']');
                    if !content.is_empty() {
                        out.push(' ');
                        emit_inlines_plain(out, content);
                    }
                    out.push('\n');
                    for child in children {
                        emit_block_compact(out, child, depth + 1);
                    }
                    out.push_str(&format!("[/{}]\n", tag));
                }
            }
        }
        // For non-skill blocks, delegate to normal emission
        other => emit_block(out, block, depth),
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p aif-lml`
Expected: All PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-lml/src/emitter.rs crates/aif-lml/src/lib.rs crates/aif-lml/tests/
git commit -m "feat(lml): add skill-compact rendering mode for lazy loading"
```

---

### Task 12: Add CLI Skill Subcommands

**Files:**
- Modify: `crates/aif-cli/Cargo.toml`
- Modify: `crates/aif-cli/src/main.rs`

- [ ] **Step 1: Add aif-skill dependency to CLI**

In `crates/aif-cli/Cargo.toml`, add:

```toml
aif-skill = { path = "../aif-skill" }
```

- [ ] **Step 2: Add Skill subcommand group to CLI**

In `crates/aif-cli/src/main.rs`, extend the `Commands` enum:

```rust
/// Skill profile operations
Skill {
    #[command(subcommand)]
    action: SkillAction,
},
```

Add the `SkillAction` enum:

```rust
#[derive(Subcommand)]
enum SkillAction {
    /// Import a SKILL.md file into AIF format
    Import {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Export an AIF skill to SKILL.md format
    Export {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify integrity hash of a skill
    Verify {
        input: PathBuf,
    },
    /// Recompute and update the hash of a skill
    Rehash {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate a skill manifest from a directory
    Manifest {
        dir: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show skill metadata
    Inspect {
        input: PathBuf,
    },
}
```

- [ ] **Step 3: Implement skill subcommand handlers**

Add the match arm for `Commands::Skill`:

```rust
Commands::Skill { action } => match action {
    SkillAction::Import { input, output } => {
        let content = fs::read_to_string(&input)
            .expect("Failed to read input file");
        let result = aif_skill::import_skill_md(&content);

        // Print diagnostics
        for diag in &result.diagnostics {
            eprintln!("  {} ← \"{}\" ({:?} confidence)",
                diag.mapped_to, diag.heading, diag.confidence);
        }

        let json = serde_json::to_string_pretty(&result.document)
            .expect("Failed to serialize");

        match output {
            Some(path) => fs::write(&path, &json).expect("Failed to write output"),
            None => println!("{}", json),
        }
    }
    SkillAction::Export { input, output } => {
        let content = fs::read_to_string(&input)
            .expect("Failed to read input file");
        let doc = aif_parser::parse(&content)
            .expect("Failed to parse AIF");

        let md = if let Some(block) = doc.blocks.first() {
            aif_skill::export_skill_md(block)
        } else {
            "No skill block found".to_string()
        };

        match output {
            Some(path) => fs::write(&path, &md).expect("Failed to write output"),
            None => println!("{}", md),
        }
    }
    SkillAction::Verify { input } => {
        let content = fs::read_to_string(&input)
            .expect("Failed to read input file");
        let doc = aif_parser::parse(&content)
            .expect("Failed to parse AIF");

        for block in &doc.blocks {
            if let BlockKind::Skill { attrs, .. } = &block.kind {
                let name = attrs.pairs.get("name").map(|s| s.as_str()).unwrap_or("unnamed");
                if aif_skill::verify_hash(block) {
                    println!("✓ Skill '{}': hash verified", name);
                } else if attrs.pairs.contains_key("hash") {
                    println!("✗ Skill '{}': hash mismatch", name);
                    std::process::exit(1);
                } else {
                    println!("- Skill '{}': no hash attribute", name);
                }
            }
        }
    }
    SkillAction::Rehash { input, output } => {
        let content = fs::read_to_string(&input)
            .expect("Failed to read input file");
        let doc = aif_parser::parse(&content)
            .expect("Failed to parse AIF");

        for block in &doc.blocks {
            if let BlockKind::Skill { attrs, .. } = &block.kind {
                let name = attrs.pairs.get("name").map(|s| s.as_str()).unwrap_or("unnamed");
                let hash = aif_skill::compute_hash(block);
                println!("Skill '{}': {}", name, hash);
            }
        }

        // For output, would need to modify and re-serialize — print hash for now
        if let Some(path) = output {
            eprintln!("Note: Writing rehashed AIF not yet implemented. Hash printed above.");
        }
    }
    SkillAction::Manifest { dir, output } => {
        let manifest = aif_skill::generate_manifest(&dir)
            .expect("Failed to generate manifest");
        let json = serde_json::to_string_pretty(&manifest)
            .expect("Failed to serialize");

        match output {
            Some(path) => fs::write(&path, &json).expect("Failed to write output"),
            None => println!("{}", json),
        }
    }
    SkillAction::Inspect { input } => {
        let content = fs::read_to_string(&input)
            .expect("Failed to read input file");
        let doc = aif_parser::parse(&content)
            .expect("Failed to parse AIF");

        for block in &doc.blocks {
            if let BlockKind::Skill { attrs, children } = &block.kind {
                println!("Skill: {}", attrs.pairs.get("name").map(|s| s.as_str()).unwrap_or("unnamed"));
                if let Some(v) = attrs.pairs.get("version") { println!("  Version: {}", v); }
                if let Some(h) = attrs.pairs.get("hash") { println!("  Hash: {}", h); }
                if let Some(t) = attrs.pairs.get("tags") { println!("  Tags: {}", t); }
                if let Some(p) = attrs.pairs.get("priority") { println!("  Priority: {}", p); }

                let mut block_types = Vec::new();
                for child in children {
                    if let BlockKind::SkillInner { block_type, .. } = &child.kind {
                        block_types.push(format!("{:?}", block_type));
                    }
                }
                println!("  Blocks: {}", block_types.join(", "));
                println!("  Children: {}", children.len());
            }
        }
    }
},
```

- [ ] **Step 4: Add necessary imports to main.rs**

```rust
use aif_core::ast::BlockKind;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p aif-cli`
Expected: Compiles.

- [ ] **Step 6: Test CLI commands manually**

```bash
cargo run -p aif-cli -- skill import tests/fixtures/skills/debugging.md
cargo run -p aif-cli -- skill inspect tests/fixtures/skills/debugging.aif
cargo run -p aif-cli -- skill manifest tests/fixtures/skills/
cargo run -p aif-cli -- skill verify tests/fixtures/skills/debugging.aif
```

- [ ] **Step 7: Commit**

```bash
git add crates/aif-cli/Cargo.toml crates/aif-cli/src/main.rs
git commit -m "feat(cli): add skill subcommand group with import, export, verify, rehash, manifest, inspect"
```

---

### Task 13: Create Example Skill Document

**Files:**
- Create: `examples/debugging.aif`

- [ ] **Step 1: Write example document**

Create `examples/debugging.aif`:

```aif
#title: Debugging Skill
#author: AIF Project

@skill[name="debugging", version="1.0", tags="process,troubleshooting", priority="high"]
  @precondition
    User has reported a bug, test failure, or unexpected behavior.
  @end

  @step[order=1]
    Reproduce the issue with a minimal test case. Confirm the exact error
    message, stack trace, or unexpected output.
  @end

  @step[order=2]
    Identify the root cause using logs, debugger, or `git bisect`.
  @end

  @step[order=3]
    Write a failing test that captures the exact bug behavior.
  @end

  @step[order=4]
    Implement the minimal fix that makes the test pass.
  @end

  @step[order=5]
    Run the full test suite to verify no regressions.
  @end

  @verify
    The fix resolves the original issue without introducing regressions.
    All existing tests continue to pass. The new regression test covers
    the specific failure mode.
  @end

  @tool[name="git bisect"]
    Find the commit that introduced the bug.
  @end

  @tool[name="grep"]
    Search for patterns in source code.
  @end

  @red_flag
    Don't apply fixes without understanding the root cause.
    Don't skip writing a regression test.
    Don't make unrelated changes in the same commit.
  @end

  @fallback
    If root cause is unclear after 3 attempts, escalate to the user
    with findings so far: what you've tried, what you've ruled out,
    and your best hypothesis.
  @end

  @example
    A user reports that login fails on mobile Safari. Steps taken:
    1. Reproduced on iOS 17 Safari — confirmed CSS media query mismatch.
    2. Found breakpoint at 768px using wrong max-width instead of min-width.
    3. Wrote responsive test for login form layout.
    4. Fixed media query, test passes.
    5. Full suite green, deployed.
  @end
@end
```

- [ ] **Step 2: Test the example through CLI**

```bash
cargo run -p aif-cli -- compile examples/debugging.aif -f html
cargo run -p aif-cli -- compile examples/debugging.aif -f lml
cargo run -p aif-cli -- skill inspect examples/debugging.aif
cargo run -p aif-cli -- skill verify examples/debugging.aif
```

- [ ] **Step 3: Commit**

```bash
git add examples/debugging.aif
git commit -m "docs: add example debugging skill document"
```

---

### Task 14: Integration Tests and Final Verification

**Files:**
- Create: `crates/aif-cli/tests/skill_integration.rs`

- [ ] **Step 1: Write integration tests**

Create `crates/aif-cli/tests/skill_integration.rs`:

```rust
use std::process::Command;

fn cargo_run(args: &[&str]) -> (String, String, bool) {
    let output = Command::new("cargo")
        .args(["run", "-p", "aif-cli", "--"])
        .args(args)
        .output()
        .expect("Failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

#[test]
fn test_skill_import_roundtrip() {
    let (stdout, stderr, ok) = cargo_run(&["skill", "import", "tests/fixtures/skills/debugging.md"]);
    assert!(ok, "Import failed: {}", stderr);
    assert!(stdout.contains("Skill"));
}

#[test]
fn test_skill_inspect() {
    let (stdout, _, ok) = cargo_run(&["skill", "inspect", "tests/fixtures/skills/debugging.aif"]);
    assert!(ok);
    assert!(stdout.contains("debugging"));
}

#[test]
fn test_skill_manifest() {
    let (stdout, _, ok) = cargo_run(&["skill", "manifest", "tests/fixtures/skills/"]);
    assert!(ok);
    assert!(stdout.contains("\"skills\""));
    assert!(stdout.contains("debugging"));
}

#[test]
fn test_skill_compile_html() {
    let (stdout, _, ok) = cargo_run(&["compile", "tests/fixtures/skills/debugging.aif", "-f", "html"]);
    assert!(ok);
    assert!(stdout.contains("skill"));
}

#[test]
fn test_skill_compile_lml() {
    let (stdout, _, ok) = cargo_run(&["compile", "tests/fixtures/skills/debugging.aif", "-f", "lml"]);
    assert!(ok);
    assert!(stdout.contains("[SKILL"));
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p aif-cli skill_integration`
Expected: All PASS.

- [ ] **Step 3: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/aif-cli/tests/skill_integration.rs
git commit -m "test(cli): add skill integration tests"
```

- [ ] **Step 5: Final commit with all remaining changes**

```bash
git status
# Stage any remaining unstaged files
git add -A
git commit -m "feat: complete Phase 3 Skill Profile implementation"
```
