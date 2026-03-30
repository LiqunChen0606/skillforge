# Phase 3: Skill Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add skill block types (`@skill`, `@step`, `@verify`, etc.) to AIF with parsing, validation, hash verification, SKILL.md import/export, manifest generation, LML skill-compact mode, and token benchmarks.

**Architecture:** New `aif-skill` crate handles skill-specific logic (validation, hashing, SKILL.md import/export, manifest). Core AST gets new `SkillBlock` variant with inner `SkillBlockType` enum. Parser recognizes `@skill` as a container directive with `@end` termination and nested inner blocks. LML emitter gains a `skill_compact` flag. CLI gets a `skill` subcommand group.

**Tech Stack:** Rust, sha2 crate for SHA-256, pulldown-cmark (reused from aif-markdown), clap (existing CLI)

---

### Task 1: Add Skill Block Types to Core AST

**Files:**
- Modify: `crates/aif-core/src/ast.rs`
- Test: `crates/aif-core/src/ast.rs` (inline tests)

- [ ] **Step 1: Write failing test for SkillBlockType enum**

Add to the `#[cfg(test)] mod tests` block in `crates/aif-core/src/ast.rs`:

```rust
#[test]
fn skill_block_serializes_to_json() {
    let skill = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "Debug process".into() }],
            children: vec![],
        },
        span: Span::new(0, 20),
    };
    let json = serde_json::to_string(&skill).unwrap();
    assert!(json.contains("\"type\":\"SkillBlock\""));
    assert!(json.contains("\"Skill\""));
}

#[test]
fn skill_step_with_order_attr() {
    let mut attrs = Attrs::new();
    attrs.pairs.insert("order".into(), "1".into());
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs,
            title: None,
            content: vec![Inline::Text { text: "Reproduce the bug".into() }],
            children: vec![],
        },
        span: Span::new(0, 30),
    };
    if let BlockKind::SkillBlock { attrs, .. } = &step.kind {
        assert_eq!(attrs.get("order"), Some("1"));
    } else {
        panic!("expected SkillBlock");
    }
}

#[test]
fn skill_block_with_children() {
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "Step 1".into() }],
            children: vec![],
        },
        span: Span::new(10, 30),
    };
    let skill = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("name".into(), "debugging".into());
                a
            },
            title: None,
            content: vec![],
            children: vec![step],
        },
        span: Span::new(0, 50),
    };
    if let BlockKind::SkillBlock { children, attrs, .. } = &skill.kind {
        assert_eq!(children.len(), 1);
        assert_eq!(attrs.get("name"), Some("debugging"));
    } else {
        panic!("expected SkillBlock");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-core`
Expected: FAIL — `SkillBlock` and `SkillBlockType` do not exist yet.

- [ ] **Step 3: Add SkillBlockType enum and SkillBlock variant**

In `crates/aif-core/src/ast.rs`, add the enum after `CalloutType`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillBlockType {
    Skill,
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

Add a new variant to `BlockKind`:

```rust
    SkillBlock {
        skill_type: SkillBlockType,
        attrs: Attrs,
        title: Option<Vec<Inline>>,
        content: Vec<Inline>,
        children: Vec<Block>,
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-core`
Expected: PASS — all existing tests still pass plus 3 new tests.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-core/src/ast.rs
git commit -m "feat(core): add SkillBlock and SkillBlockType to AST"
```

---

### Task 2: Parse @skill Container and Inner Blocks

**Files:**
- Modify: `crates/aif-parser/src/block.rs`
- Test: `crates/aif-parser/src/block.rs` (inline tests)

- [ ] **Step 1: Write failing tests for skill parsing**

Add to the `#[cfg(test)] mod tests` block in `crates/aif-parser/src/block.rs`:

```rust
#[test]
fn parse_skill_container_with_end() {
    let input = "@skill[name=debugging]\n  Some intro text.\n@end\n";
    let mut parser = BlockParser::new(input);
    let doc = parser.parse().unwrap();
    assert_eq!(doc.blocks.len(), 1);
    if let BlockKind::SkillBlock { skill_type, attrs, content, children, .. } = &doc.blocks[0].kind {
        assert!(matches!(skill_type, SkillBlockType::Skill));
        assert_eq!(attrs.get("name"), Some("debugging"));
        // Intro text becomes content
        assert!(!content.is_empty());
        assert!(children.is_empty());
    } else {
        panic!("expected SkillBlock, got {:?}", doc.blocks[0].kind);
    }
}

#[test]
fn parse_skill_with_inner_blocks() {
    let input = "\
@skill[name=debugging version=1.0]
@precondition
  User has reported a bug.
@end
@step[order=1]
  Reproduce the issue.
@end
@verify
  Fix resolves issue without regressions.
@end
@end
";
    let mut parser = BlockParser::new(input);
    let doc = parser.parse().unwrap();
    assert_eq!(doc.blocks.len(), 1);
    if let BlockKind::SkillBlock { children, .. } = &doc.blocks[0].kind {
        assert_eq!(children.len(), 3);
        // Check precondition
        if let BlockKind::SkillBlock { skill_type, .. } = &children[0].kind {
            assert!(matches!(skill_type, SkillBlockType::Precondition));
        } else {
            panic!("expected precondition");
        }
        // Check step
        if let BlockKind::SkillBlock { skill_type, attrs, .. } = &children[1].kind {
            assert!(matches!(skill_type, SkillBlockType::Step));
            assert_eq!(attrs.get("order"), Some("1"));
        } else {
            panic!("expected step");
        }
        // Check verify
        if let BlockKind::SkillBlock { skill_type, .. } = &children[2].kind {
            assert!(matches!(skill_type, SkillBlockType::Verify));
        } else {
            panic!("expected verify");
        }
    } else {
        panic!("expected SkillBlock");
    }
}

#[test]
fn parse_skill_with_free_text_and_blocks() {
    let input = "\
@skill[name=test]
Some free text intro.

@step[order=1]
  Do something.
@end
@end
";
    let mut parser = BlockParser::new(input);
    let doc = parser.parse().unwrap();
    assert_eq!(doc.blocks.len(), 1);
    if let BlockKind::SkillBlock { content, children, .. } = &doc.blocks[0].kind {
        // Free text in content
        assert!(!content.is_empty());
        // Inner block in children
        assert_eq!(children.len(), 1);
    } else {
        panic!("expected SkillBlock");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-parser`
Expected: FAIL — parser doesn't handle `@skill` or `@end` yet.

- [ ] **Step 3: Implement skill container parsing in BlockParser**

In `crates/aif-parser/src/block.rs`, modify `parse_directive` to handle skill block types. The key changes:

1. Add a helper to check if a directive type is a skill block:

```rust
fn is_skill_block_type(directive: &str) -> Option<SkillBlockType> {
    match directive {
        "skill" => Some(SkillBlockType::Skill),
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

2. In `parse_directive`, before the existing semantic block match, add skill block handling:

```rust
if let Some(skill_type) = is_skill_block_type(directive_type) {
    return self.parse_skill_block(skill_type, attrs, title_str, start);
}
```

3. Add the `parse_skill_block` method:

```rust
fn parse_skill_block(
    &mut self,
    skill_type: SkillBlockType,
    attrs: Attrs,
    title_str: &str,
    start: usize,
) -> Option<Block> {
    let is_container = matches!(skill_type, SkillBlockType::Skill);
    let title = if title_str.is_empty() {
        None
    } else {
        Some(parse_inline(title_str))
    };

    let mut content_lines: Vec<&str> = Vec::new();
    let mut children: Vec<Block> = Vec::new();
    let mut end = start;

    while self.pos < self.lines.len() {
        let line = self.current_line();

        // @end terminates this block
        if line.trim() == "@end" {
            end = self.current_offset() + line.len();
            self.pos += 1;
            break;
        }

        // Nested skill block directive inside a container
        if is_container && line.starts_with('@') {
            let rest = &line[1..];
            let type_end = rest
                .find(|c: char| c == '[' || c == ':' || c.is_whitespace())
                .unwrap_or(rest.len());
            let inner_type_str = &rest[..type_end];

            if let Some(inner_skill_type) = is_skill_block_type(inner_type_str) {
                // Parse the inner block's attrs and title from this line
                let after_type = &rest[type_end..];
                let (inner_attrs, after_attrs) = if after_type.starts_with('[') {
                    if let Some(close) = after_type.find(']') {
                        let attr_str = &after_type[1..close];
                        (parse_attrs(attr_str), &after_type[close + 1..])
                    } else {
                        (Attrs::new(), after_type)
                    }
                } else {
                    (Attrs::new(), after_type)
                };
                let inner_title = if let Some(rest) = after_attrs.strip_prefix(':') {
                    rest.trim()
                } else {
                    after_attrs.trim()
                };

                let inner_start = self.current_offset();
                self.pos += 1;

                if let Some(child) = self.parse_skill_block(inner_skill_type, inner_attrs, inner_title, inner_start) {
                    children.push(child);
                }
                continue;
            }
        }

        // Regular content line
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            content_lines.push(trimmed);
        }
        end = self.current_offset() + line.len();
        self.pos += 1;
    }

    let content_text = content_lines.join("\n");
    let content = if content_text.is_empty() {
        vec![]
    } else {
        parse_inline(&content_text)
    };

    Some(Block {
        kind: BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        },
        span: Span::new(start, end),
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-parser`
Expected: PASS — all 3 new skill tests + all existing tests.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-parser/src/block.rs
git commit -m "feat(parser): parse @skill container and inner skill blocks with @end"
```

---

### Task 3: Render Skill Blocks in HTML

**Files:**
- Modify: `crates/aif-html/src/emitter.rs`
- Test: `crates/aif-html/tests/render_html_extended.rs` (or new test file)

- [ ] **Step 1: Write failing test**

Create `crates/aif-html/tests/skill_html.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;

#[test]
fn render_skill_block_html() {
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Reproduce the bug.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "debugging".into());
                    a
                },
                title: None,
                content: vec![],
                children: vec![step],
            },
            span: Span::empty(),
        }],
    };
    let html = aif_html::render_html(&doc);
    assert!(html.contains("aif-skill"));
    assert!(html.contains("aif-step"));
    assert!(html.contains("Reproduce the bug."));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-html --test skill_html`
Expected: FAIL — `SkillBlock` not handled in emitter.

- [ ] **Step 3: Add SkillBlock rendering to HTML emitter**

Read `crates/aif-html/src/emitter.rs` and add a `BlockKind::SkillBlock` arm to the match in `emit_block`. Pattern follows `SemanticBlock`:

```rust
BlockKind::SkillBlock {
    skill_type,
    attrs,
    title,
    content,
    children,
} => {
    let class = skill_block_class(skill_type);
    out.push_str(&format!("<div class=\"{}\">", class));
    emit_attrs_html(out, attrs);
    if let Some(t) = title {
        out.push_str("<h3>");
        emit_inlines(out, t);
        out.push_str("</h3>");
    }
    if !content.is_empty() {
        out.push_str("<p>");
        emit_inlines(out, content);
        out.push_str("</p>");
    }
    for child in children {
        emit_block(out, child);
    }
    out.push_str("</div>\n");
}
```

Add the helper:

```rust
fn skill_block_class(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "aif-skill",
        SkillBlockType::Step => "aif-step",
        SkillBlockType::Verify => "aif-verify",
        SkillBlockType::Precondition => "aif-precondition",
        SkillBlockType::OutputContract => "aif-output-contract",
        SkillBlockType::Decision => "aif-decision",
        SkillBlockType::Tool => "aif-tool",
        SkillBlockType::Fallback => "aif-fallback",
        SkillBlockType::RedFlag => "aif-red-flag",
        SkillBlockType::Example => "aif-example",
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aif-html`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aif-html/src/emitter.rs crates/aif-html/tests/skill_html.rs
git commit -m "feat(html): render SkillBlock types with semantic CSS classes"
```

---

### Task 4: Render Skill Blocks in Markdown

**Files:**
- Modify: `crates/aif-markdown/src/emitter.rs`
- Test: `crates/aif-markdown/tests/skill_md.rs`

- [ ] **Step 1: Write failing test**

Create `crates/aif-markdown/tests/skill_md.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;

#[test]
fn render_skill_block_markdown() {
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Reproduce the bug.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "debugging".into());
                    a
                },
                title: None,
                content: vec![],
                children: vec![step],
            },
            span: Span::empty(),
        }],
    };
    let md = aif_markdown::render_markdown(&doc);
    assert!(md.contains("## Steps"));
    assert!(md.contains("1. Reproduce the bug."));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-markdown --test skill_md`
Expected: FAIL

- [ ] **Step 3: Add SkillBlock to Markdown emitter**

In `crates/aif-markdown/src/emitter.rs`, add `BlockKind::SkillBlock` arm to `emit_block`:

```rust
BlockKind::SkillBlock {
    skill_type,
    attrs,
    title,
    content,
    children,
} => {
    match skill_type {
        SkillBlockType::Skill => {
            // Emit skill name as H2
            if let Some(name) = attrs.get("name") {
                out.push_str(&format!("## {}\n\n", name));
            }
            if let Some(t) = title {
                out.push_str(&format!("{}\n\n", inlines_to_text(t)));
            }
            if !content.is_empty() {
                out.push_str(&inlines_to_text(content));
                out.push_str("\n\n");
            }
            // Group steps together under "## Steps"
            let steps: Vec<&Block> = children.iter().filter(|c| matches!(&c.kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Step, .. })).collect();
            let others: Vec<&Block> = children.iter().filter(|c| !matches!(&c.kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Step, .. })).collect();

            if !steps.is_empty() {
                out.push_str("## Steps\n\n");
                for step in &steps {
                    if let BlockKind::SkillBlock { attrs, content, .. } = &step.kind {
                        let order = attrs.get("order").unwrap_or("0");
                        out.push_str(&format!("{}. {}\n", order, inlines_to_text(content)));
                    }
                }
                out.push('\n');
            }

            for child in &others {
                emit_block(out, child, heading_level + 1);
            }
        }
        _ => {
            let heading = skill_type_heading(skill_type);
            out.push_str(&format!("{} {}\n\n", "#".repeat(heading_level), heading));
            if !content.is_empty() {
                out.push_str(&inlines_to_text(content));
                out.push_str("\n\n");
            }
            for child in children {
                emit_block(out, child, heading_level + 1);
            }
        }
    }
}
```

Add the helper:

```rust
fn skill_type_heading(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "Skill",
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

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aif-markdown`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aif-markdown/src/emitter.rs crates/aif-markdown/tests/skill_md.rs
git commit -m "feat(markdown): render SkillBlock types as structured Markdown"
```

---

### Task 5: Render Skill Blocks in LML (Standard Mode)

**Files:**
- Modify: `crates/aif-lml/src/emitter.rs`
- Test: `crates/aif-lml/tests/skill_lml.rs`

- [ ] **Step 1: Write failing test**

Create `crates/aif-lml/tests/skill_lml.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;

#[test]
fn render_skill_lml() {
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Reproduce the bug.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "debugging".into());
                    a
                },
                title: None,
                content: vec![],
                children: vec![step],
            },
            span: Span::empty(),
        }],
    };
    let lml = aif_lml::render_lml(&doc);
    assert!(lml.contains("[SKILL"));
    assert!(lml.contains("name=debugging"));
    assert!(lml.contains("[STEP"));
    assert!(lml.contains("Reproduce the bug."));
    assert!(lml.contains("[/SKILL]"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-lml --test skill_lml`
Expected: FAIL

- [ ] **Step 3: Add SkillBlock to LML emitter**

In `crates/aif-lml/src/emitter.rs`, add `BlockKind::SkillBlock` arm to `emit_block`:

```rust
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
    // Container blocks get closing tags
    if matches!(skill_type, SkillBlockType::Skill) || !children.is_empty() {
        out.push_str("[/");
        out.push_str(tag);
        out.push_str("]\n");
    }
    out.push('\n');
}
```

Add the helper:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-lml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aif-lml/src/emitter.rs crates/aif-lml/tests/skill_lml.rs
git commit -m "feat(lml): render SkillBlock types in LML format"
```

---

### Task 6: Scaffold aif-skill Crate

**Files:**
- Create: `crates/aif-skill/Cargo.toml`
- Create: `crates/aif-skill/src/lib.rs`
- Create: `crates/aif-skill/src/validate.rs`
- Create: `crates/aif-skill/src/hash.rs`
- Modify: `Cargo.toml` (workspace)

- [ ] **Step 1: Write failing test for skill validation**

Create `crates/aif-skill/src/validate.rs`:

```rust
use aif_core::ast::*;

#[derive(Debug, PartialEq)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}

pub fn validate_skill(_block: &Block) -> Vec<ValidationError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    fn make_skill(name: Option<&str>, children: Vec<Block>) -> Block {
        let mut attrs = Attrs::new();
        if let Some(n) = name {
            attrs.pairs.insert("name".into(), n.into());
        }
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

    fn make_step(order: &str) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("order".into(), order.into());
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text { text: format!("Step {}", order) }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn valid_skill_passes() {
        let skill = make_skill(Some("debugging"), vec![make_step("1"), make_step("2")]);
        let errors = validate_skill(&skill);
        assert!(errors.is_empty());
    }

    #[test]
    fn skill_missing_name_fails() {
        let skill = make_skill(None, vec![]);
        let errors = validate_skill(&skill);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("name"));
    }

    #[test]
    fn duplicate_step_order_fails() {
        let skill = make_skill(Some("test"), vec![make_step("1"), make_step("1")]);
        let errors = validate_skill(&skill);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("order"));
    }

    #[test]
    fn non_contiguous_step_order_fails() {
        let skill = make_skill(Some("test"), vec![make_step("1"), make_step("3")]);
        let errors = validate_skill(&skill);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("contiguous"));
    }
}
```

- [ ] **Step 2: Create crate scaffolding**

Create `crates/aif-skill/Cargo.toml`:

```toml
[package]
name = "aif-skill"
version.workspace = true
edition.workspace = true

[dependencies]
aif-core = { workspace = true }
sha2 = "0.10"
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
aif-core = { workspace = true }
aif-parser = { path = "../aif-parser" }
```

Create `crates/aif-skill/src/lib.rs`:

```rust
pub mod validate;
pub mod hash;
```

Create `crates/aif-skill/src/hash.rs` (placeholder):

```rust
// Hash computation — implemented in Task 7
```

Add to workspace `Cargo.toml` members:

```toml
"crates/aif-skill",
```

Add to workspace dependencies:

```toml
aif-skill = { path = "crates/aif-skill" }
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p aif-skill`
Expected: FAIL — `validate_skill` is `todo!()`.

- [ ] **Step 4: Implement validate_skill**

Replace the `todo!()` in `validate_skill` in `crates/aif-skill/src/validate.rs`:

```rust
pub fn validate_skill(block: &Block) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let BlockKind::SkillBlock { skill_type, attrs, children, .. } = &block.kind {
        if !matches!(skill_type, SkillBlockType::Skill) {
            errors.push(ValidationError::new("Top-level block must be @skill"));
            return errors;
        }

        // Rule 1: @skill must have a name attribute
        if attrs.get("name").is_none() {
            errors.push(ValidationError::new("@skill must have a 'name' attribute"));
        }

        // Rules 2-3: @step order validation
        let mut step_orders: Vec<u32> = Vec::new();
        for child in children {
            if let BlockKind::SkillBlock { skill_type: SkillBlockType::Step, attrs, .. } = &child.kind {
                if let Some(order_str) = attrs.get("order") {
                    if let Ok(order) = order_str.parse::<u32>() {
                        step_orders.push(order);
                    }
                }
            }
        }

        if !step_orders.is_empty() {
            // Check for duplicates
            let mut sorted = step_orders.clone();
            sorted.sort();
            let mut seen = std::collections::HashSet::new();
            for &o in &sorted {
                if !seen.insert(o) {
                    errors.push(ValidationError::new(
                        format!("Duplicate step order: {}", o),
                    ));
                    break;
                }
            }

            // Check contiguous from 1
            if seen.len() == sorted.len() {
                for (i, &o) in sorted.iter().enumerate() {
                    if o != (i as u32) + 1 {
                        errors.push(ValidationError::new(
                            format!("Step order values must be contiguous starting from 1, found gap at {}", o),
                        ));
                        break;
                    }
                }
            }
        }
    } else {
        errors.push(ValidationError::new("Expected SkillBlock"));
    }

    errors
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-skill`
Expected: PASS — all 4 validation tests.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-skill/ Cargo.toml
git commit -m "feat(skill): scaffold aif-skill crate with validation"
```

---

### Task 7: SHA-256 Hash Computation and Verification

**Files:**
- Modify: `crates/aif-skill/src/hash.rs`
- Test: inline in `hash.rs`

- [ ] **Step 1: Write failing tests**

Replace `crates/aif-skill/src/hash.rs` with:

```rust
use aif_core::ast::*;

/// Compute SHA-256 hash of a skill block's content.
/// Normalizes whitespace and line endings before hashing.
pub fn compute_skill_hash(_block: &Block) -> String {
    todo!()
}

/// Verify that a skill block's hash attribute matches its computed hash.
pub fn verify_skill_hash(_block: &Block) -> HashVerifyResult {
    todo!()
}

#[derive(Debug, PartialEq)]
pub enum HashVerifyResult {
    /// Hash matches
    Valid,
    /// Hash does not match (expected, actual)
    Mismatch { expected: String, actual: String },
    /// No hash attribute present
    NoHash,
    /// Not a skill block
    NotASkill,
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    fn make_skill_with_content(name: &str, text: &str) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), name.into());
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![Inline::Text { text: text.into() }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn compute_hash_deterministic() {
        let skill = make_skill_with_content("test", "Some content here.");
        let hash1 = compute_skill_hash(&skill);
        let hash2 = compute_skill_hash(&skill);
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));
        assert_eq!(hash1.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn different_content_different_hash() {
        let skill1 = make_skill_with_content("test", "Content A");
        let skill2 = make_skill_with_content("test", "Content B");
        assert_ne!(compute_skill_hash(&skill1), compute_skill_hash(&skill2));
    }

    #[test]
    fn verify_valid_hash() {
        let mut skill = make_skill_with_content("test", "Some content.");
        let hash = compute_skill_hash(&skill);
        // Set the hash attribute
        if let BlockKind::SkillBlock { ref mut attrs, .. } = skill.kind {
            attrs.pairs.insert("hash".into(), hash);
        }
        assert_eq!(verify_skill_hash(&skill), HashVerifyResult::Valid);
    }

    #[test]
    fn verify_tampered_content() {
        let mut skill = make_skill_with_content("test", "Original content.");
        let hash = compute_skill_hash(&skill);
        if let BlockKind::SkillBlock { ref mut attrs, ref mut content, .. } = skill.kind {
            attrs.pairs.insert("hash".into(), hash);
            // Tamper with content
            *content = vec![Inline::Text { text: "Tampered content.".into() }];
        }
        match verify_skill_hash(&skill) {
            HashVerifyResult::Mismatch { .. } => {} // expected
            other => panic!("expected Mismatch, got {:?}", other),
        }
    }

    #[test]
    fn verify_no_hash() {
        let skill = make_skill_with_content("test", "Content.");
        assert_eq!(verify_skill_hash(&skill), HashVerifyResult::NoHash);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill`
Expected: FAIL — functions are `todo!()`.

- [ ] **Step 3: Implement hash computation**

Replace the `todo!()` implementations in `crates/aif-skill/src/hash.rs`:

```rust
use sha2::{Sha256, Digest};

/// Normalize content for hashing: trim whitespace, normalize line endings.
fn normalize_for_hash(block: &Block) -> String {
    let mut out = String::new();
    if let BlockKind::SkillBlock { content, children, .. } = &block.kind {
        // Serialize content inlines to plain text
        for inline in content {
            inline_to_text(&mut out, inline);
        }
        // Serialize children recursively
        for child in children {
            out.push('\n');
            normalize_child(&mut out, child);
        }
    }
    // Normalize: trim, normalize line endings
    out.replace("\r\n", "\n").trim().to_string()
}

fn normalize_child(out: &mut String, block: &Block) {
    if let BlockKind::SkillBlock { skill_type, attrs, content, children, .. } = &block.kind {
        out.push_str(&format!("{:?}", skill_type));
        // Include non-hash attrs
        for (k, v) in &attrs.pairs {
            if k != "hash" {
                out.push_str(&format!(" {}={}", k, v));
            }
        }
        out.push('\n');
        for inline in content {
            inline_to_text(out, inline);
        }
        for child in children {
            out.push('\n');
            normalize_child(out, child);
        }
    }
}

fn inline_to_text(out: &mut String, inline: &Inline) {
    match inline {
        Inline::Text { text } => out.push_str(text),
        Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
            for i in content {
                inline_to_text(out, i);
            }
        }
        Inline::InlineCode { code } => out.push_str(code),
        Inline::Link { text, url } => {
            for i in text {
                inline_to_text(out, i);
            }
            out.push_str(url);
        }
        Inline::Reference { target } => out.push_str(target),
        Inline::SoftBreak => out.push(' '),
        Inline::HardBreak => out.push('\n'),
    }
}

pub fn compute_skill_hash(block: &Block) -> String {
    let normalized = normalize_for_hash(block);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}

pub fn verify_skill_hash(block: &Block) -> HashVerifyResult {
    if let BlockKind::SkillBlock { attrs, .. } = &block.kind {
        match attrs.get("hash") {
            Some(expected) => {
                let actual = compute_skill_hash(block);
                if expected == actual {
                    HashVerifyResult::Valid
                } else {
                    HashVerifyResult::Mismatch {
                        expected: expected.to_string(),
                        actual,
                    }
                }
            }
            None => HashVerifyResult::NoHash,
        }
    } else {
        HashVerifyResult::NotASkill
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-skill`
Expected: PASS — all hash + validation tests.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/hash.rs
git commit -m "feat(skill): add SHA-256 hash computation and verification"
```

---

### Task 8: SKILL.md Import (Markdown → AIF Skill)

**Files:**
- Create: `crates/aif-skill/src/import.rs`
- Modify: `crates/aif-skill/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/aif-skill/src/import.rs`:

```rust
use aif_core::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct ImportMapping {
    pub heading: String,
    pub mapped_to: SkillBlockType,
    pub confidence: Confidence,
}

#[derive(Debug)]
pub struct SkillImportResult {
    pub block: Block,
    pub mappings: Vec<ImportMapping>,
}

/// Import a SKILL.md file (with optional YAML frontmatter) into an AIF skill block.
pub fn import_skill_md(_input: &str) -> SkillImportResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_minimal_skill() {
        let input = "# My Skill\n\nSome description.\n";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { skill_type, attrs, content, .. } = &result.block.kind {
            assert!(matches!(skill_type, SkillBlockType::Skill));
            assert_eq!(attrs.get("name"), Some("My Skill"));
            assert!(!content.is_empty());
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_skill_with_frontmatter() {
        let input = "\
---
name: debugging
description: Use when encountering bugs
---

# Debugging

Debug stuff.
";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { attrs, .. } = &result.block.kind {
            assert_eq!(attrs.get("name"), Some("debugging"));
            assert_eq!(attrs.get("description"), Some("Use when encountering bugs"));
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_skill_with_steps_heading() {
        let input = "\
# Test Skill

## Steps

1. First step
2. Second step
3. Third step
";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { children, .. } = &result.block.kind {
            let steps: Vec<_> = children.iter().filter(|c| {
                matches!(&c.kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Step, .. })
            }).collect();
            assert_eq!(steps.len(), 3);
            // Check ordering
            if let BlockKind::SkillBlock { attrs, .. } = &steps[0].kind {
                assert_eq!(attrs.get("order"), Some("1"));
            }
        } else {
            panic!("expected SkillBlock");
        }
        // Check mapping confidence
        let step_mapping = result.mappings.iter().find(|m| m.heading == "Steps").unwrap();
        assert_eq!(step_mapping.confidence, Confidence::High);
    }

    #[test]
    fn import_skill_with_prerequisites() {
        let input = "\
# Test Skill

## Prerequisites

- Must have access to logs
- Must have a reproduction case
";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { children, .. } = &result.block.kind {
            let preconds: Vec<_> = children.iter().filter(|c| {
                matches!(&c.kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Precondition, .. })
            }).collect();
            assert_eq!(preconds.len(), 1);
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_skill_with_verification() {
        let input = "\
# Test Skill

## Verification

- All tests pass
- No regressions
";
        let result = import_skill_md(input);
        if let BlockKind::SkillBlock { children, .. } = &result.block.kind {
            let verifies: Vec<_> = children.iter().filter(|c| {
                matches!(&c.kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Verify, .. })
            }).collect();
            assert_eq!(verifies.len(), 1);
        } else {
            panic!("expected SkillBlock");
        }
    }

    #[test]
    fn import_reports_confidence_levels() {
        let input = "\
# Test Skill

## Steps

1. Do something

## Commands

Use these tools.

## Options

Pick one.
";
        let result = import_skill_md(input);
        assert!(result.mappings.iter().any(|m| m.confidence == Confidence::High));
        assert!(result.mappings.iter().any(|m| m.confidence == Confidence::Medium));
        assert!(result.mappings.iter().any(|m| m.confidence == Confidence::Low));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill`
Expected: FAIL — `import_skill_md` is `todo!()`.

- [ ] **Step 3: Implement SKILL.md importer**

Replace the `todo!()` in `import_skill_md`:

```rust
use aif_core::span::Span;

/// Match heading text to skill block type with confidence.
fn classify_heading(heading: &str) -> Option<(SkillBlockType, Confidence)> {
    let lower = heading.to_lowercase();
    match lower.as_str() {
        "steps" | "procedure" | "how to" | "instructions" | "the process" | "checklist" => {
            Some((SkillBlockType::Step, Confidence::High))
        }
        "prerequisites" | "requirements" | "when to use" | "preconditions" => {
            Some((SkillBlockType::Precondition, Confidence::High))
        }
        "verification" | "testing" | "acceptance" | "verify" | "verification checklist" => {
            Some((SkillBlockType::Verify, Confidence::High))
        }
        "examples" | "usage" | "example" => {
            Some((SkillBlockType::Example, Confidence::High))
        }
        "tools" | "commands" | "tool" => {
            Some((SkillBlockType::Tool, Confidence::Medium))
        }
        "fallback" | "recovery" | "if stuck" => {
            Some((SkillBlockType::Fallback, Confidence::Medium))
        }
        "anti-patterns" | "don't" | "avoid" | "red flags" | "common mistakes" => {
            Some((SkillBlockType::RedFlag, Confidence::Medium))
        }
        "output" | "expected output" | "returns" | "output contract" => {
            Some((SkillBlockType::OutputContract, Confidence::Medium))
        }
        "decision" | "choose" | "options" => {
            Some((SkillBlockType::Decision, Confidence::Low))
        }
        _ => None,
    }
}

/// Parse YAML frontmatter from beginning of input.
/// Returns (frontmatter pairs, remaining input).
fn parse_frontmatter(input: &str) -> (Vec<(String, String)>, &str) {
    if !input.starts_with("---") {
        return (vec![], input);
    }
    let after_first = &input[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        let fm_content = &after_first[..end_pos];
        let rest_start = 3 + end_pos + 4; // "---" + content + "\n---"
        let rest = if rest_start < input.len() {
            &input[rest_start..]
        } else {
            ""
        };
        let rest = rest.trim_start_matches('\n');

        let mut pairs = Vec::new();
        for line in fm_content.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !key.is_empty() {
                    pairs.push((key, value));
                }
            }
        }
        (pairs, rest)
    } else {
        (vec![], input)
    }
}

pub fn import_skill_md(input: &str) -> SkillImportResult {
    let (frontmatter, content) = parse_frontmatter(input);
    let mut attrs = Attrs::new();
    let mut mappings = Vec::new();

    // Apply frontmatter
    for (key, value) in &frontmatter {
        attrs.pairs.insert(key.clone(), value.clone());
    }

    // Parse content line by line to find H1 (skill name) and H2 (section headings)
    let mut skill_name: Option<String> = None;
    let mut current_heading: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();
    let mut sections: Vec<(Option<(SkillBlockType, Confidence)>, String, Vec<String>)> = Vec::new();
    let mut intro_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        if let Some(h1) = line.strip_prefix("# ") {
            let name = h1.trim().to_string();
            skill_name = Some(name);
        } else if let Some(h2) = line.strip_prefix("## ") {
            // Save previous section
            if let Some(heading) = current_heading.take() {
                let classification = classify_heading(&heading);
                sections.push((classification, heading, std::mem::take(&mut current_lines)));
            } else if !current_lines.is_empty() {
                intro_lines = std::mem::take(&mut current_lines);
            }
            current_heading = Some(h2.trim().to_string());
        } else {
            current_lines.push(line.to_string());
        }
    }
    // Save last section
    if let Some(heading) = current_heading.take() {
        let classification = classify_heading(&heading);
        sections.push((classification, heading, std::mem::take(&mut current_lines)));
    } else if !current_lines.is_empty() && intro_lines.is_empty() {
        intro_lines = current_lines;
    }

    // Set name from frontmatter or H1
    if attrs.get("name").is_none() {
        if let Some(name) = &skill_name {
            attrs.pairs.insert("name".into(), name.clone());
        }
    }

    // Build intro content
    let intro_text = intro_lines.join("\n").trim().to_string();
    let skill_content = if intro_text.is_empty() {
        vec![]
    } else {
        vec![Inline::Text { text: intro_text }]
    };

    // Build children from sections
    let mut children = Vec::new();
    for (classification, heading, lines) in sections {
        let body = lines.join("\n").trim().to_string();
        if body.is_empty() {
            continue;
        }

        match classification {
            Some((ref skill_type, ref confidence)) => {
                mappings.push(ImportMapping {
                    heading: heading.clone(),
                    mapped_to: skill_type.clone(),
                    confidence: confidence.clone(),
                });

                if matches!(skill_type, SkillBlockType::Step) {
                    // Split numbered items into individual steps
                    let mut step_order = 1u32;
                    for line in body.lines() {
                        let trimmed = line.trim();
                        // Match "N. text" or "- text"
                        let step_text = if let Some(dot_pos) = trimmed.find(". ") {
                            if trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                                Some(trimmed[dot_pos + 2..].to_string())
                            } else {
                                None
                            }
                        } else if let Some(rest) = trimmed.strip_prefix("- ") {
                            Some(rest.to_string())
                        } else {
                            None
                        };

                        if let Some(text) = step_text {
                            let mut step_attrs = Attrs::new();
                            step_attrs.pairs.insert("order".into(), step_order.to_string());
                            children.push(Block {
                                kind: BlockKind::SkillBlock {
                                    skill_type: SkillBlockType::Step,
                                    attrs: step_attrs,
                                    title: None,
                                    content: vec![Inline::Text { text }],
                                    children: vec![],
                                },
                                span: Span::empty(),
                            });
                            step_order += 1;
                        }
                    }
                } else {
                    let child_attrs = Attrs::new();
                    children.push(Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: skill_type.clone(),
                            attrs: child_attrs,
                            title: None,
                            content: vec![Inline::Text { text: body }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    });
                }
            }
            None => {
                // Unrecognized heading — keep as free text in a Section
                children.push(Block {
                    kind: BlockKind::Section {
                        attrs: Attrs::new(),
                        title: vec![Inline::Text { text: heading }],
                        children: vec![Block {
                            kind: BlockKind::Paragraph {
                                content: vec![Inline::Text { text: body }],
                            },
                            span: Span::empty(),
                        }],
                    },
                    span: Span::empty(),
                });
            }
        }
    }

    let block = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            title: None,
            content: skill_content,
            children,
        },
        span: Span::empty(),
    };

    SkillImportResult { block, mappings }
}
```

Update `crates/aif-skill/src/lib.rs`:

```rust
pub mod validate;
pub mod hash;
pub mod import;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-skill`
Expected: PASS — all import + validation + hash tests.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/import.rs crates/aif-skill/src/lib.rs
git commit -m "feat(skill): add SKILL.md importer with heading auto-detection and confidence"
```

---

### Task 9: SKILL.md Export (AIF Skill → Markdown)

**Files:**
- Create: `crates/aif-skill/src/export.rs`
- Modify: `crates/aif-skill/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/aif-skill/src/export.rs`:

```rust
use aif_core::ast::*;

/// Export an AIF skill block to SKILL.md format.
pub fn export_skill_md(_block: &Block) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    fn make_step(order: u32, text: &str) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("order".into(), order.to_string());
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text { text: text.into() }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn export_minimal_skill() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "test-skill".into());
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![Inline::Text { text: "A test skill.".into() }],
                children: vec![],
            },
            span: Span::empty(),
        };
        let md = export_skill_md(&skill);
        assert!(md.contains("---"));
        assert!(md.contains("name: test-skill"));
        assert!(md.contains("# test-skill"));
        assert!(md.contains("A test skill."));
    }

    #[test]
    fn export_skill_with_steps() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "debugging".into());
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children: vec![
                    make_step(1, "Reproduce the bug"),
                    make_step(2, "Find root cause"),
                ],
            },
            span: Span::empty(),
        };
        let md = export_skill_md(&skill);
        assert!(md.contains("## Steps"));
        assert!(md.contains("1. Reproduce the bug"));
        assert!(md.contains("2. Find root cause"));
    }

    #[test]
    fn export_skill_with_precondition_and_verify() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "test".into());
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children: vec![
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Precondition,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text { text: "Have a bug report.".into() }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Verify,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text { text: "All tests pass.".into() }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                ],
            },
            span: Span::empty(),
        };
        let md = export_skill_md(&skill);
        assert!(md.contains("## Prerequisites"));
        assert!(md.contains("Have a bug report."));
        assert!(md.contains("## Verification"));
        assert!(md.contains("All tests pass."));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill`
Expected: FAIL — `export_skill_md` is `todo!()`.

- [ ] **Step 3: Implement export**

Replace the `todo!()` in `export_skill_md`:

```rust
use crate::hash::compute_skill_hash;

fn skill_type_to_heading(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "Skill",
        SkillBlockType::Step => "Steps",
        SkillBlockType::Verify => "Verification",
        SkillBlockType::Precondition => "Prerequisites",
        SkillBlockType::OutputContract => "Expected Output",
        SkillBlockType::Decision => "Options",
        SkillBlockType::Tool => "Commands",
        SkillBlockType::Fallback => "Fallback",
        SkillBlockType::RedFlag => "Anti-patterns",
        SkillBlockType::Example => "Examples",
    }
}

fn inline_to_text(inline: &Inline) -> String {
    match inline {
        Inline::Text { text } => text.clone(),
        Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
            content.iter().map(inline_to_text).collect()
        }
        Inline::InlineCode { code } => format!("`{}`", code),
        Inline::Link { text, url } => {
            let t: String = text.iter().map(inline_to_text).collect();
            format!("[{}]({})", t, url)
        }
        Inline::Reference { target } => target.clone(),
        Inline::SoftBreak => " ".into(),
        Inline::HardBreak => "\n".into(),
    }
}

fn inlines_to_text(inlines: &[Inline]) -> String {
    inlines.iter().map(inline_to_text).collect()
}

pub fn export_skill_md(block: &Block) -> String {
    let mut out = String::new();

    if let BlockKind::SkillBlock { skill_type: SkillBlockType::Skill, attrs, content, children, .. } = &block.kind {
        let name = attrs.get("name").unwrap_or("unnamed");

        // YAML frontmatter
        out.push_str("---\n");
        out.push_str(&format!("name: {}\n", name));
        if let Some(desc) = attrs.get("description") {
            out.push_str(&format!("description: {}\n", desc));
        }
        if let Some(version) = attrs.get("version") {
            out.push_str(&format!("version: {}\n", version));
        }
        // Include hash for roundtrip
        let hash = compute_skill_hash(block);
        out.push_str(&format!("hash: {}\n", hash));
        out.push_str("---\n\n");

        // H1 title
        out.push_str(&format!("# {}\n\n", name));

        // Intro content
        if !content.is_empty() {
            out.push_str(&inlines_to_text(content));
            out.push_str("\n\n");
        }

        // Group steps together
        let steps: Vec<&Block> = children.iter().filter(|c| {
            matches!(&c.kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Step, .. })
        }).collect();
        let others: Vec<&Block> = children.iter().filter(|c| {
            !matches!(&c.kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Step, .. })
        }).collect();

        // Emit non-step children first (preconditions, etc.)
        for child in &others {
            if let BlockKind::SkillBlock { skill_type, content, .. } = &child.kind {
                let heading = skill_type_to_heading(skill_type);
                out.push_str(&format!("## {}\n\n", heading));
                if !content.is_empty() {
                    out.push_str(&inlines_to_text(content));
                    out.push_str("\n\n");
                }
            }
        }

        // Emit steps as numbered list
        if !steps.is_empty() {
            out.push_str("## Steps\n\n");
            for step in &steps {
                if let BlockKind::SkillBlock { attrs, content, .. } = &step.kind {
                    let order = attrs.get("order").unwrap_or("0");
                    out.push_str(&format!("{}. {}\n", order, inlines_to_text(content)));
                }
            }
            out.push('\n');
        }
    }

    out
}
```

Update `crates/aif-skill/src/lib.rs`:

```rust
pub mod validate;
pub mod hash;
pub mod import;
pub mod export;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-skill`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/export.rs crates/aif-skill/src/lib.rs
git commit -m "feat(skill): add SKILL.md exporter with frontmatter and hash"
```

---

### Task 10: Skill Manifest Generation

**Files:**
- Create: `crates/aif-skill/src/manifest.rs`
- Modify: `crates/aif-skill/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/aif-skill/src/manifest.rs`:

```rust
use aif_core::ast::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillManifest {
    pub skills: Vec<SkillEntry>,
    pub generated: String,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillEntry {
    pub name: String,
    pub version: Option<String>,
    pub hash: String,
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub blocks: Vec<String>,
    pub path: String,
}

/// Generate a manifest entry for a single skill block.
pub fn skill_to_entry(_block: &Block, _path: &str) -> Option<SkillEntry> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    #[test]
    fn generate_entry_from_skill() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "debugging".into());
        attrs.pairs.insert("version".into(), "1.0".into());
        attrs.pairs.insert("tags".into(), "process,troubleshooting".into());
        attrs.pairs.insert("priority".into(), "high".into());

        let step = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("order".into(), "1".into());
                    a
                },
                title: None,
                content: vec![Inline::Text { text: "Do something.".into() }],
                children: vec![],
            },
            span: Span::empty(),
        };

        let verify = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Verify,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text { text: "Check it.".into() }],
                children: vec![],
            },
            span: Span::empty(),
        };

        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children: vec![step, verify],
            },
            span: Span::empty(),
        };

        let entry = skill_to_entry(&skill, "skills/debugging.aif").unwrap();
        assert_eq!(entry.name, "debugging");
        assert_eq!(entry.version, Some("1.0".into()));
        assert_eq!(entry.tags, vec!["process", "troubleshooting"]);
        assert_eq!(entry.priority, Some("high".into()));
        assert_eq!(entry.blocks, vec!["step", "verify"]);
        assert_eq!(entry.path, "skills/debugging.aif");
        assert!(entry.hash.starts_with("sha256:"));
    }

    #[test]
    fn non_skill_block_returns_none() {
        let block = Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text { text: "Not a skill.".into() }],
            },
            span: Span::empty(),
        };
        assert!(skill_to_entry(&block, "foo.aif").is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill`
Expected: FAIL — `skill_to_entry` is `todo!()`.

- [ ] **Step 3: Implement manifest entry generation**

Replace the `todo!()`:

```rust
use crate::hash::compute_skill_hash;

fn skill_type_tag(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "skill",
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

pub fn skill_to_entry(block: &Block, path: &str) -> Option<SkillEntry> {
    if let BlockKind::SkillBlock { skill_type: SkillBlockType::Skill, attrs, children, .. } = &block.kind {
        let name = attrs.get("name")?.to_string();
        let version = attrs.get("version").map(|s| s.to_string());
        let hash = compute_skill_hash(block);
        let tags: Vec<String> = attrs
            .get("tags")
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        let priority = attrs.get("priority").map(|s| s.to_string());

        let mut blocks = Vec::new();
        for child in children {
            if let BlockKind::SkillBlock { skill_type, .. } = &child.kind {
                let tag = skill_type_tag(skill_type);
                if !blocks.contains(&tag.to_string()) {
                    blocks.push(tag.to_string());
                }
            }
        }

        Some(SkillEntry {
            name,
            version,
            hash,
            tags,
            priority,
            blocks,
            path: path.to_string(),
        })
    } else {
        None
    }
}
```

Update `crates/aif-skill/src/lib.rs`:

```rust
pub mod validate;
pub mod hash;
pub mod import;
pub mod export;
pub mod manifest;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-skill`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/manifest.rs crates/aif-skill/src/lib.rs
git commit -m "feat(skill): add manifest entry generation for lazy loading"
```

---

### Task 11: LML Skill-Compact Mode

**Files:**
- Modify: `crates/aif-lml/src/lib.rs`
- Modify: `crates/aif-lml/src/emitter.rs`
- Test: `crates/aif-lml/tests/skill_compact.rs`

- [ ] **Step 1: Write failing test**

Create `crates/aif-lml/tests/skill_compact.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;

#[test]
fn skill_compact_strips_examples() {
    let example = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Example,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "This is a long example that should be stripped.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Do the thing carefully and thoroughly.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let verify = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "Check results.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "test".into());
                    a
                },
                title: None,
                content: vec![],
                children: vec![step, verify, example],
            },
            span: Span::empty(),
        }],
    };

    let full = aif_lml::render_lml(&doc);
    let compact = aif_lml::render_lml_skill_compact(&doc);

    // Full mode includes example
    assert!(full.contains("long example"));
    // Compact mode strips example
    assert!(!compact.contains("long example"));
    // Compact preserves verify
    assert!(compact.contains("Check results."));
    // Compact is shorter
    assert!(compact.len() < full.len());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-lml --test skill_compact`
Expected: FAIL — `render_lml_skill_compact` doesn't exist.

- [ ] **Step 3: Add skill-compact mode to LML**

In `crates/aif-lml/src/lib.rs`, add:

```rust
pub fn render_lml_skill_compact(doc: &Document) -> String {
    emitter::emit_lml_skill_compact(doc)
}
```

In `crates/aif-lml/src/emitter.rs`, add a `skill_compact` parameter to the emit chain. The simplest approach: duplicate `emit_lml` with a flag.

Add at the top of the file:

```rust
pub fn emit_lml_skill_compact(doc: &Document) -> String {
    let mut out = String::new();
    emit_doc_inner(&mut out, doc, true);
    out
}
```

Refactor `emit_lml` and `emit_doc`:

```rust
pub fn emit_lml(doc: &Document) -> String {
    let mut out = String::new();
    emit_doc_inner(&mut out, doc, false);
    out
}

fn emit_doc_inner(out: &mut String, doc: &Document, skill_compact: bool) {
    out.push_str("[DOC");
    for (key, value) in &doc.metadata {
        out.push(' ');
        emit_attr_pair(out, key, value);
    }
    out.push_str("]\n");

    for block in &doc.blocks {
        emit_block_inner(out, block, 0, skill_compact);
    }

    out.push_str("[/DOC]\n");
}
```

Then rename `emit_block` to `emit_block_inner` adding the `skill_compact: bool` parameter, and in the `SkillBlock` arm:

```rust
BlockKind::SkillBlock {
    skill_type,
    attrs,
    title,
    content,
    children,
} => {
    // In compact mode, skip @example blocks entirely
    if skill_compact && matches!(skill_type, SkillBlockType::Example) {
        return;
    }

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
        if skill_compact && matches!(skill_type, SkillBlockType::Step) {
            // Compact: single-line summary (first sentence or first 80 chars)
            let full_text = inlines_to_plain(content);
            let summary = if let Some(dot) = full_text.find(". ") {
                &full_text[..=dot]
            } else if full_text.len() > 80 {
                &full_text[..80]
            } else {
                &full_text
            };
            out.push_str(summary);
            out.push('\n');
        } else {
            emit_inlines_plain(out, content);
            out.push('\n');
        }
    }
    for child in children {
        emit_block_inner(out, child, _depth + 1, skill_compact);
    }
    if matches!(skill_type, SkillBlockType::Skill) || !children.is_empty() {
        out.push_str("[/");
        out.push_str(tag);
        out.push_str("]\n");
    }
    out.push('\n');
}
```

Add helper:

```rust
fn inlines_to_plain(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        emit_inline_plain(&mut out, inline);
    }
    out
}
```

Also update all recursive calls from `emit_block` to `emit_block_inner` passing the `skill_compact` flag.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-lml`
Expected: PASS — all existing LML tests + new compact test.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-lml/src/lib.rs crates/aif-lml/src/emitter.rs crates/aif-lml/tests/skill_compact.rs
git commit -m "feat(lml): add skill-compact mode stripping examples and condensing steps"
```

---

### Task 12: CLI Skill Subcommands

**Files:**
- Modify: `crates/aif-cli/src/main.rs`
- Modify: `crates/aif-cli/Cargo.toml`
- Test: `crates/aif-cli/tests/skill_cli.rs`

- [ ] **Step 1: Write failing test**

Create `crates/aif-cli/tests/skill_cli.rs`:

```rust
use std::process::Command;

fn aif_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aif-cli"))
}

#[test]
fn skill_import_produces_json() {
    let input = "# Test Skill\n\n## Steps\n\n1. First step\n2. Second step\n";
    let tmp = std::env::temp_dir().join("test_skill.md");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["skill", "import", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SkillBlock"));
    assert!(stdout.contains("Step"));

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn skill_verify_no_hash() {
    let input = "@skill[name=test]\nSome content.\n@end\n";
    let tmp = std::env::temp_dir().join("test_verify.aif");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["skill", "verify", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("no hash") || combined.contains("NoHash"),
        "output: {}", combined);

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn skill_inspect_shows_metadata() {
    let input = "@skill[name=debugging version=1.0 tags=process,debug]\n@step[order=1]\n  Do it.\n@end\n@end\n";
    let tmp = std::env::temp_dir().join("test_inspect.aif");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["skill", "inspect", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("debugging"));
    assert!(stdout.contains("1.0"));

    std::fs::remove_file(&tmp).ok();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-cli --test skill_cli`
Expected: FAIL — no `skill` subcommand.

- [ ] **Step 3: Add skill subcommands to CLI**

In `crates/aif-cli/Cargo.toml`, add dependencies:

```toml
aif-skill = { path = "../aif-skill" }
```

And in dev-dependencies:

```toml
aif-skill = { path = "../aif-skill" }
```

In `crates/aif-cli/src/main.rs`, add the `Skill` subcommand:

```rust
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aif")]
#[command(about = "AIF: AI-native Interchange Format compiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an AIF document to an output format
    Compile {
        /// Input .aif file
        input: PathBuf,
        /// Output format: html, markdown, lml, json
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import a Markdown file to AIF IR (JSON)
    Import {
        /// Input Markdown file
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Dump the parsed IR as JSON
    DumpIr {
        /// Input .aif file
        input: PathBuf,
    },
    /// Skill-related operations
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
}

#[derive(Subcommand)]
enum SkillAction {
    /// Import a SKILL.md file to AIF IR (JSON)
    Import {
        /// Input .md file
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Export an AIF skill to SKILL.md format
    Export {
        /// Input .aif file containing a @skill block
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify integrity hash of a skill
    Verify {
        /// Input .aif file
        input: PathBuf,
    },
    /// Recompute and update hash for a skill
    Rehash {
        /// Input .aif file (modified in place)
        input: PathBuf,
    },
    /// Show skill metadata
    Inspect {
        /// Input .aif file
        input: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, format, output } => {
            let source = read_file(&input);
            let doc = parse_aif(&source);

            let result = match format.as_str() {
                "html" => aif_html::render_html(&doc),
                "markdown" | "md" => aif_markdown::render_markdown(&doc),
                "lml" => aif_lml::render_lml(&doc),
                "json" => serde_json::to_string_pretty(&doc).unwrap(),
                _ => {
                    eprintln!("Unknown format: {}. Supported: html, markdown, lml, json", format);
                    std::process::exit(1);
                }
            };

            write_output(result, output);
        }
        Commands::Import { input, output } => {
            let source = read_file(&input);
            let doc = aif_markdown::import_markdown(&source);
            let json = serde_json::to_string_pretty(&doc).unwrap();
            write_output(json, output);
        }
        Commands::DumpIr { input } => {
            let source = read_file(&input);
            let doc = parse_aif(&source);
            let json = serde_json::to_string_pretty(&doc).unwrap();
            println!("{}", json);
        }
        Commands::Skill { action } => handle_skill(action),
    }
}

fn handle_skill(action: SkillAction) {
    match action {
        SkillAction::Import { input, output } => {
            let source = read_file(&input);
            let result = aif_skill::import::import_skill_md(&source);
            // Print mappings to stderr
            for mapping in &result.mappings {
                let conf = match mapping.confidence {
                    aif_skill::import::Confidence::High => "high",
                    aif_skill::import::Confidence::Medium => "medium",
                    aif_skill::import::Confidence::Low => "low",
                };
                eprintln!("  {:?} ← \"{}\" ({} confidence)", mapping.mapped_to, mapping.heading, conf);
            }
            let json = serde_json::to_string_pretty(&result.block).unwrap();
            write_output(json, output);
        }
        SkillAction::Export { input, output } => {
            let source = read_file(&input);
            let doc = parse_aif(&source);
            // Find first skill block
            let skill_block = find_skill_block(&doc.blocks);
            match skill_block {
                Some(block) => {
                    let md = aif_skill::export::export_skill_md(block);
                    write_output(md, output);
                }
                None => {
                    eprintln!("No @skill block found in input.");
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Verify { input } => {
            let source = read_file(&input);
            let doc = parse_aif(&source);
            let skill_block = find_skill_block(&doc.blocks);
            match skill_block {
                Some(block) => {
                    let result = aif_skill::hash::verify_skill_hash(block);
                    match result {
                        aif_skill::hash::HashVerifyResult::Valid => {
                            println!("Valid: hash matches.");
                        }
                        aif_skill::hash::HashVerifyResult::Mismatch { expected, actual } => {
                            println!("MISMATCH!");
                            println!("  Expected: {}", expected);
                            println!("  Actual:   {}", actual);
                            std::process::exit(1);
                        }
                        aif_skill::hash::HashVerifyResult::NoHash => {
                            println!("No hash attribute found — no hash to verify.");
                        }
                        aif_skill::hash::HashVerifyResult::NotASkill => {
                            eprintln!("Not a skill block.");
                            std::process::exit(1);
                        }
                    }
                }
                None => {
                    eprintln!("No @skill block found in input.");
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Rehash { input } => {
            let source = read_file(&input);
            let mut doc = parse_aif(&source);
            // Find and update hash on first skill block
            let updated = rehash_skill_blocks(&mut doc.blocks);
            if updated {
                // Re-serialize to AIF source isn't trivial — output JSON instead
                let json = serde_json::to_string_pretty(&doc).unwrap();
                fs::write(&input, &json).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", input.display(), e);
                    std::process::exit(1);
                });
                eprintln!("Rehashed {}", input.display());
            } else {
                eprintln!("No @skill block found.");
                std::process::exit(1);
            }
        }
        SkillAction::Inspect { input } => {
            let source = read_file(&input);
            let doc = parse_aif(&source);
            let skill_block = find_skill_block(&doc.blocks);
            match skill_block {
                Some(block) => {
                    if let aif_core::ast::BlockKind::SkillBlock { attrs, children, .. } = &block.kind {
                        println!("Name:     {}", attrs.get("name").unwrap_or("(unnamed)"));
                        if let Some(v) = attrs.get("version") {
                            println!("Version:  {}", v);
                        }
                        if let Some(t) = attrs.get("tags") {
                            println!("Tags:     {}", t);
                        }
                        if let Some(p) = attrs.get("priority") {
                            println!("Priority: {}", p);
                        }
                        if let Some(h) = attrs.get("hash") {
                            println!("Hash:     {}", h);
                        }
                        println!("Children: {}", children.len());
                        for child in children {
                            if let aif_core::ast::BlockKind::SkillBlock { skill_type, .. } = &child.kind {
                                println!("  - {:?}", skill_type);
                            }
                        }
                    }
                }
                None => {
                    eprintln!("No @skill block found in input.");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn find_skill_block(blocks: &[aif_core::ast::Block]) -> Option<&aif_core::ast::Block> {
    blocks.iter().find(|b| {
        matches!(&b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    })
}

fn rehash_skill_blocks(blocks: &mut [aif_core::ast::Block]) -> bool {
    for block in blocks.iter_mut() {
        if let aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill,
            ref mut attrs,
            ..
        } = block.kind
        {
            let hash = aif_skill::hash::compute_skill_hash(block);
            attrs.pairs.insert("hash".into(), hash);
            return true;
        }
    }
    false
}

fn read_file(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path.display(), e);
        std::process::exit(1);
    })
}

fn parse_aif(source: &str) -> aif_core::ast::Document {
    aif_parser::parse(source).unwrap_or_else(|errors| {
        for e in &errors {
            eprintln!("{}", e);
        }
        std::process::exit(1);
    })
}

fn write_output(content: String, output: Option<PathBuf>) {
    if let Some(output_path) = output {
        fs::write(&output_path, &content).unwrap_or_else(|e| {
            eprintln!("Error writing {}: {}", output_path.display(), e);
            std::process::exit(1);
        });
        eprintln!("Wrote {}", output_path.display());
    } else {
        print!("{}", content);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-cli`
Expected: PASS — all CLI tests including new skill subcommand tests.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-cli/src/main.rs crates/aif-cli/Cargo.toml crates/aif-cli/tests/skill_cli.rs
git commit -m "feat(cli): add skill subcommands (import, export, verify, rehash, inspect)"
```

---

### Task 13: Test Fixture — Real Skill Roundtrip

**Files:**
- Create: `tests/fixtures/skills/debugging.md`
- Create: `crates/aif-skill/tests/roundtrip.rs`

- [ ] **Step 1: Create test fixture**

Create `tests/fixtures/skills/debugging.md`:

```markdown
---
name: debugging
description: Use when encountering any bug, test failure, or unexpected behavior
---

# Debugging

Systematic approach to finding and fixing bugs.

## Prerequisites

- A bug report or test failure exists
- You can reproduce the issue

## Steps

1. Read error messages carefully
2. Reproduce consistently
3. Check recent changes
4. Trace data flow
5. Form hypothesis and test minimally

## Verification

- Fix resolves original issue
- No regressions introduced
- All tests pass

## Anti-patterns

- Random fixes without understanding root cause
- "Quick fix for now, investigate later"
- Each fix reveals a new problem

## Fallback

If root cause is unclear after 3 attempts, escalate to user.

## Examples

A developer sees a failing test. They read the stack trace, reproduce it locally,
git blame the changed lines, find a missing null check, add a test, fix it.
```

- [ ] **Step 2: Write roundtrip test**

Create `crates/aif-skill/tests/roundtrip.rs`:

```rust
use std::fs;

#[test]
fn roundtrip_debugging_skill() {
    let md_input = fs::read_to_string("../../tests/fixtures/skills/debugging.md").unwrap();

    // Import
    let import_result = aif_skill::import::import_skill_md(&md_input);
    let block = &import_result.block;

    // Verify structure
    if let aif_core::ast::BlockKind::SkillBlock { attrs, children, .. } = &block.kind {
        assert_eq!(attrs.get("name"), Some("debugging"));
        assert_eq!(attrs.get("description"), Some("Use when encountering any bug, test failure, or unexpected behavior"));

        // Should have: steps, precondition, verify, red_flag, fallback, example
        assert!(children.len() >= 5, "expected at least 5 children, got {}", children.len());
    } else {
        panic!("expected SkillBlock");
    }

    // Check mappings
    assert!(!import_result.mappings.is_empty());

    // Export back to markdown
    let exported = aif_skill::export::export_skill_md(block);
    assert!(exported.contains("# debugging"));
    assert!(exported.contains("## Steps"));
    assert!(exported.contains("1. Read error messages carefully"));

    // Validate
    let errors = aif_skill::validate::validate_skill(block);
    assert!(errors.is_empty(), "validation errors: {:?}", errors);

    // Hash
    let hash = aif_skill::hash::compute_skill_hash(block);
    assert!(hash.starts_with("sha256:"));
}

#[test]
fn manifest_entry_from_imported_skill() {
    let md_input = fs::read_to_string("../../tests/fixtures/skills/debugging.md").unwrap();
    let import_result = aif_skill::import::import_skill_md(&md_input);
    let entry = aif_skill::manifest::skill_to_entry(&import_result.block, "skills/debugging.aif").unwrap();

    assert_eq!(entry.name, "debugging");
    assert!(entry.hash.starts_with("sha256:"));
    assert!(entry.blocks.contains(&"step".to_string()));
    assert!(entry.blocks.contains(&"verify".to_string()));
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p aif-skill --test roundtrip`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add tests/fixtures/skills/debugging.md crates/aif-skill/tests/roundtrip.rs
git commit -m "test(skill): add real skill roundtrip test with debugging.md fixture"
```

---

### Task 14: Token Benchmark for Skills

**Files:**
- Create: `benchmarks/skill_token_benchmark.py`
- Create: `tests/fixtures/skills/tdd.md`

- [ ] **Step 1: Create TDD skill fixture (for benchmark variety)**

Create `tests/fixtures/skills/tdd.md`:

```markdown
---
name: test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development

Write the test first. Watch it fail. Write minimal code to pass.

## Prerequisites

- A clear requirement or bug to fix
- Test framework is set up and working

## Steps

1. Write one minimal failing test
2. Run it to verify it fails for the right reason
3. Write the simplest code to make it pass
4. Run tests to verify all pass
5. Refactor if needed while keeping tests green
6. Commit

## Verification

- Every new function has a test
- Watched each test fail before implementing
- Each test failed for the expected reason
- Wrote minimal code to pass each test
- All tests pass

## Anti-patterns

- Writing production code before the test
- Test passes immediately (testing existing behavior)
- Can't explain why test failed
- "I already manually tested it"
- "Keep as reference" instead of deleting code-first code

## Examples

Developer needs to add a `validateEmail` function. They write `test_validates_correct_email`
and `test_rejects_invalid_email` first. Both fail because `validateEmail` doesn't exist.
They implement the minimal regex check. Tests pass. They refactor to use a proper email
validation library. Tests still pass. Commit.
```

- [ ] **Step 2: Create token benchmark script**

Create `benchmarks/skill_token_benchmark.py`:

```python
#!/usr/bin/env python3
"""
Skill Token Efficiency Benchmark

Compares token counts for skills in different formats:
- Original SKILL.md (Markdown)
- AIF JSON IR (imported)
- AIF LML (full)
- AIF LML skill-compact

Uses Claude's token counting API for accurate measurements.
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import anthropic

MODEL = "claude-opus-4-6"
PROJECT_ROOT = Path(__file__).resolve().parent.parent
AIF_CLI = PROJECT_ROOT / "target" / "release" / "aif-cli"
SKILLS_DIR = PROJECT_ROOT / "tests" / "fixtures" / "skills"


def count_tokens(client: anthropic.Anthropic, text: str) -> int:
    result = client.messages.count_tokens(
        model=MODEL,
        messages=[{"role": "user", "content": text}],
    )
    return result.input_tokens


def import_skill(md_path: str) -> str:
    """Import a SKILL.md via CLI, returns JSON IR."""
    result = subprocess.run(
        [str(AIF_CLI), "skill", "import", md_path],
        capture_output=True, text=True, timeout=30,
    )
    if result.returncode != 0:
        print(f"  Warning: import failed: {result.stderr}", file=sys.stderr)
        return ""
    return result.stdout


def format_size(n: int) -> str:
    if n >= 1_000:
        return f"{n/1_000:.1f}K"
    return str(n)


def main():
    if not AIF_CLI.exists():
        print(f"Error: AIF CLI not found at {AIF_CLI}")
        print("Run: cargo build --release")
        sys.exit(1)

    api_key = os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("claude_API")
    if not api_key:
        print("Error: Set ANTHROPIC_API_KEY environment variable")
        sys.exit(1)
    api_key = api_key.strip()

    try:
        client = anthropic.Anthropic(api_key=api_key)
        client.messages.count_tokens(
            model=MODEL, messages=[{"role": "user", "content": "test"}]
        )
    except anthropic.AuthenticationError:
        api_key = api_key[:-1]
        client = anthropic.Anthropic(api_key=api_key)

    print("=" * 70)
    print("Skill Token Efficiency Benchmark")
    print(f"Model: {MODEL}")
    print("=" * 70)
    print()

    skill_files = sorted(SKILLS_DIR.glob("*.md"))
    if not skill_files:
        print("No skill fixtures found in", SKILLS_DIR)
        sys.exit(1)

    results = []

    for skill_path in skill_files:
        name = skill_path.stem
        print(f"── {name} ", "─" * (50 - len(name)))

        md_text = skill_path.read_text()
        json_ir = import_skill(str(skill_path))
        if not json_ir:
            print("  SKIP: import failed")
            continue

        # Count tokens
        md_tokens = count_tokens(client, md_text)
        json_tokens = count_tokens(client, json_ir)

        md_bytes = len(md_text.encode("utf-8"))
        json_bytes = len(json_ir.encode("utf-8"))

        savings_json = (1 - json_tokens / md_tokens) * 100 if md_tokens > 0 else 0

        results.append({
            "skill": name,
            "md_tokens": md_tokens,
            "json_tokens": json_tokens,
            "md_bytes": md_bytes,
            "json_bytes": json_bytes,
            "savings_json_pct": savings_json,
        })

        print(f"  SKILL.md:     {format_size(md_tokens):>8} tokens ({format_size(md_bytes):>8} bytes)")
        print(f"  AIF JSON IR:  {format_size(json_tokens):>8} tokens ({format_size(json_bytes):>8} bytes) → {savings_json:+.1f}%")
        print()

        time.sleep(0.5)

    if not results:
        print("No results.")
        sys.exit(1)

    # Summary
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print()
    print(f"{'Skill':<30} {'SKILL.md':>10} {'AIF JSON':>10} {'Savings':>10}")
    print("─" * 62)

    total_md = total_json = 0
    for r in results:
        total_md += r["md_tokens"]
        total_json += r["json_tokens"]
        print(f"{r['skill']:<30} {format_size(r['md_tokens']):>10} {format_size(r['json_tokens']):>10} {r['savings_json_pct']:>+9.1f}%")

    print("─" * 62)
    avg_savings = (1 - total_json / total_md) * 100 if total_md > 0 else 0
    print(f"{'TOTAL':<30} {format_size(total_md):>10} {format_size(total_json):>10} {avg_savings:>+9.1f}%")
    print()

    # Save results
    output_path = PROJECT_ROOT / "benchmarks" / "skill_results.json"
    with open(output_path, "w") as f:
        json.dump({
            "model": MODEL,
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "skills": results,
        }, f, indent=2)
    print(f"Results saved to {output_path}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 3: Commit**

```bash
git add tests/fixtures/skills/tdd.md benchmarks/skill_token_benchmark.py
git commit -m "feat(bench): add skill token benchmark and TDD skill fixture"
```

---

### Task 15: Full Workspace Build and Integration Verification

- [ ] **Step 1: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All tests pass across all crates.

- [ ] **Step 2: Build release binary**

Run: `cargo build --release`
Expected: Compiles with no errors.

- [ ] **Step 3: End-to-end CLI test**

```bash
# Import a skill
./target/release/aif-cli skill import tests/fixtures/skills/debugging.md -o /tmp/debugging_skill.json

# Inspect a skill from AIF source
echo '@skill[name=test version=1.0]
@step[order=1]
  First step.
@end
@verify
  All tests pass.
@end
@end' > /tmp/test.aif

./target/release/aif-cli skill inspect /tmp/test.aif
./target/release/aif-cli skill verify /tmp/test.aif

# Compile skill-containing AIF to formats
./target/release/aif-cli compile /tmp/test.aif -f html
./target/release/aif-cli compile /tmp/test.aif -f lml
./target/release/aif-cli compile /tmp/test.aif -f markdown
```

Expected: All commands complete successfully with correct output.

- [ ] **Step 4: Commit any fixes**

If any issues found, fix and commit.
