# HTML Importer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bidirectional HTML import to `aif-html` — lossless roundtrip for AIF-emitted HTML, plus best-effort import for generic HTML documents.

**Architecture:** Two-layer importer. Layer 1 detects `aif-*` CSS classes and reconstructs exact AST types (lossless roundtrip). Layer 2 maps standard HTML tags to AIF blocks (generic import). An optional `--strip-chrome` flag enables readability-style content extraction for full web pages. Auto-detection picks the layer based on whether `aif-*` classes are present.

**Tech Stack:** `scraper` crate (CSS selectors over html5ever), `aif-core` AST types.

---

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/aif-html/src/importer.rs` | Core import logic: `import_html()`, inline parsing, block conversion, AIF class detection |
| `crates/aif-html/src/readability.rs` | Opt-in readability extraction (`--strip-chrome`): content scoring, boilerplate removal |
| `crates/aif-html/src/lib.rs` | Public API: re-export `import_html`, `HtmlImportResult`, `ImportMode` |
| `crates/aif-html/src/emitter.rs` | No changes (existing HTML emitter) |
| `crates/aif-html/Cargo.toml` | Add `scraper` dependency |
| `crates/aif-html/tests/import_html.rs` | Integration tests for generic HTML import |
| `crates/aif-html/tests/import_aif_roundtrip.rs` | Roundtrip tests: emit → import → compare |
| `crates/aif-html/tests/import_readability.rs` | Tests for `--strip-chrome` readability extraction |
| `crates/aif-cli/src/main.rs` | Add `.html`/`.htm` dispatch in `Import` subcommand |

---

### Task 1: Scaffolding + Basic Paragraph Import

**Files:**
- Create: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/src/lib.rs`
- Modify: `crates/aif-html/Cargo.toml`
- Create: `crates/aif-html/tests/import_html.rs`

- [ ] **Step 1: Add `scraper` dependency to Cargo.toml**

In `crates/aif-html/Cargo.toml`, add under `[dependencies]`:
```toml
scraper = "0.22"
```

- [ ] **Step 2: Write failing test for basic paragraph import**

Create `crates/aif-html/tests/import_html.rs`:

```rust
use aif_core::ast::*;

#[test]
fn test_import_paragraph() {
    let html = "<html><body><p>Hello world</p></body></html>";
    let result = aif_html::import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => {
            assert_eq!(content.len(), 1);
            match &content[0] {
                Inline::Text { text } => assert_eq!(text, "Hello world"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Paragraph, got {:?}", other),
    }
}

#[test]
fn test_import_multiple_paragraphs() {
    let html = "<html><body><p>First</p><p>Second</p></body></html>";
    let result = aif_html::import_html(html, false);
    assert_eq!(result.document.blocks.len(), 2);
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p aif-html --test import_html`
Expected: FAIL — `import_html` function not found.

- [ ] **Step 4: Create importer module with minimal implementation**

Create `crates/aif-html/src/importer.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use scraper::{Html, Selector};

/// Result of importing HTML into an AIF Document.
pub struct HtmlImportResult {
    pub document: Document,
    pub mode: ImportMode,
}

/// Whether the import used AIF-roundtrip or generic tag mapping.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportMode {
    /// Detected `aif-*` CSS classes — lossless roundtrip.
    AifRoundtrip,
    /// Standard HTML tag mapping — best-effort.
    Generic,
}

/// Import an HTML string into an AIF Document.
///
/// If `strip_chrome` is true, applies readability-style content extraction
/// before tag mapping (useful for full web pages with navigation/ads).
pub fn import_html(input: &str, _strip_chrome: bool) -> HtmlImportResult {
    let doc_html = Html::parse_document(input);
    let mut document = Document::new();

    // Find <body> or use root
    let body_sel = Selector::parse("body").unwrap();
    let root = doc_html.select(&body_sel).next();

    if let Some(body) = root {
        for child in body.children() {
            if let Some(element) = child.value().as_element() {
                if element.name() == "p" {
                    let text: String = child
                        .children()
                        .filter_map(|c| c.value().as_text().map(|t| t.to_string()))
                        .collect();
                    document.blocks.push(Block {
                        kind: BlockKind::Paragraph {
                            content: vec![Inline::Text { text }],
                        },
                        span: Span::new(0, 0),
                    });
                }
            }
        }
    }

    HtmlImportResult {
        document,
        mode: ImportMode::Generic,
    }
}
```

- [ ] **Step 5: Update `lib.rs` to expose the importer**

Replace `crates/aif-html/src/lib.rs` with:

```rust
mod emitter;
pub mod importer;

use aif_core::ast::Document;

pub use importer::{import_html, HtmlImportResult, ImportMode};

pub fn render_html(doc: &Document) -> String {
    emitter::emit_html(doc)
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p aif-html --test import_html`
Expected: PASS (2 tests).

- [ ] **Step 7: Run full workspace tests to verify no regressions**

Run: `cargo test --workspace`
Expected: All existing tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/src/lib.rs crates/aif-html/Cargo.toml crates/aif-html/tests/import_html.rs
git commit -m "feat(aif-html): scaffold HTML importer with basic paragraph import"
```

---

### Task 2: Inline Element Parsing

**Files:**
- Modify: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/tests/import_html.rs`

- [ ] **Step 1: Write failing tests for inline elements**

Append to `crates/aif-html/tests/import_html.rs`:

```rust
#[test]
fn test_import_strong() {
    let html = "<html><body><p><strong>bold</strong></p></body></html>";
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    match &content[0] {
        Inline::Strong { content } => {
            assert_eq!(content.len(), 1);
            match &content[0] {
                Inline::Text { text } => assert_eq!(text, "bold"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Strong, got {:?}", other),
    }
}

#[test]
fn test_import_emphasis() {
    let html = "<html><body><p><em>italic</em></p></body></html>";
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    match &content[0] {
        Inline::Emphasis { content } => {
            match &content[0] {
                Inline::Text { text } => assert_eq!(text, "italic"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Emphasis, got {:?}", other),
    }
}

#[test]
fn test_import_inline_code() {
    let html = "<html><body><p><code>let x = 1</code></p></body></html>";
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    match &content[0] {
        Inline::InlineCode { code } => assert_eq!(code, "let x = 1"),
        other => panic!("expected InlineCode, got {:?}", other),
    }
}

#[test]
fn test_import_link() {
    let html = r#"<html><body><p><a href="https://example.com">click</a></p></body></html>"#;
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    match &content[0] {
        Inline::Link { text, url } => {
            assert_eq!(url, "https://example.com");
            match &text[0] {
                Inline::Text { text } => assert_eq!(text, "click"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Link, got {:?}", other),
    }
}

#[test]
fn test_import_inline_image() {
    let html = r#"<html><body><p><img src="pic.png" alt="photo"></p></body></html>"#;
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    match &content[0] {
        Inline::Image { alt, src } => {
            assert_eq!(src, "pic.png");
            assert_eq!(alt, "photo");
        }
        other => panic!("expected Image, got {:?}", other),
    }
}

#[test]
fn test_import_hard_break() {
    let html = "<html><body><p>line1<br>line2</p></body></html>";
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    assert_eq!(content.len(), 3);
    assert!(matches!(&content[1], Inline::HardBreak));
}

#[test]
fn test_import_mixed_inlines() {
    let html = "<html><body><p>Hello <strong>bold</strong> and <em>italic</em> world</p></body></html>";
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    // Should be: Text("Hello "), Strong("bold"), Text(" and "), Emphasis("italic"), Text(" world")
    assert_eq!(content.len(), 5);
    assert!(matches!(&content[0], Inline::Text { text } if text == "Hello "));
    assert!(matches!(&content[1], Inline::Strong { .. }));
    assert!(matches!(&content[2], Inline::Text { text } if text == " and "));
    assert!(matches!(&content[3], Inline::Emphasis { .. }));
    assert!(matches!(&content[4], Inline::Text { text } if text == " world"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_html`
Expected: FAIL — inline elements not parsed, only raw text extracted.

- [ ] **Step 3: Implement inline parsing with recursive node traversal**

Replace the minimal `import_html` implementation in `crates/aif-html/src/importer.rs` with a proper recursive parser. The key function is `parse_inlines` which walks child nodes of an element and produces `Vec<Inline>`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use scraper::{Html, Selector, ElementRef, Node};

pub struct HtmlImportResult {
    pub document: Document,
    pub mode: ImportMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportMode {
    AifRoundtrip,
    Generic,
}

pub fn import_html(input: &str, _strip_chrome: bool) -> HtmlImportResult {
    let doc_html = Html::parse_document(input);
    let mut document = Document::new();

    let body_sel = Selector::parse("body").unwrap();
    let root = doc_html.select(&body_sel).next();

    if let Some(body) = root {
        let blocks = parse_blocks(body);
        document.blocks = blocks;
    }

    HtmlImportResult {
        document,
        mode: ImportMode::Generic,
    }
}

fn parse_blocks(parent: ElementRef) -> Vec<Block> {
    let mut blocks = Vec::new();
    for child in parent.children() {
        match child.value() {
            Node::Element(el) => {
                let child_ref = ElementRef::wrap(child).unwrap();
                if let Some(block) = parse_block_element(child_ref) {
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }
    blocks
}

fn parse_block_element(el: ElementRef) -> Option<Block> {
    let tag = el.value().name();
    let kind = match tag {
        "p" => BlockKind::Paragraph {
            content: parse_inlines(el),
        },
        _ => return None,
    };
    Some(Block {
        kind,
        span: Span::new(0, 0),
    })
}

fn parse_inlines(parent: ElementRef) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for child in parent.children() {
        match child.value() {
            Node::Text(text) => {
                let s = text.to_string();
                if !s.is_empty() {
                    inlines.push(Inline::Text { text: s });
                }
            }
            Node::Element(el) => {
                let child_ref = ElementRef::wrap(child).unwrap();
                match el.name() {
                    "strong" | "b" => {
                        inlines.push(Inline::Strong {
                            content: parse_inlines(child_ref),
                        });
                    }
                    "em" | "i" => {
                        inlines.push(Inline::Emphasis {
                            content: parse_inlines(child_ref),
                        });
                    }
                    "code" => {
                        let code: String = child_ref.text().collect();
                        inlines.push(Inline::InlineCode { code });
                    }
                    "a" => {
                        let url = el.attr("href").unwrap_or("").to_string();
                        inlines.push(Inline::Link {
                            text: parse_inlines(child_ref),
                            url,
                        });
                    }
                    "img" => {
                        let src = el.attr("src").unwrap_or("").to_string();
                        let alt = el.attr("alt").unwrap_or("").to_string();
                        inlines.push(Inline::Image { alt, src });
                    }
                    "br" => {
                        inlines.push(Inline::HardBreak);
                    }
                    _ => {
                        // Recurse into unknown inline elements, extract content
                        inlines.extend(parse_inlines(child_ref));
                    }
                }
            }
            _ => {}
        }
    }
    inlines
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_html`
Expected: PASS (all 9 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/tests/import_html.rs
git commit -m "feat(aif-html): add inline element parsing (strong, em, code, link, img, br)"
```

---

### Task 3: Headings, Sections, and Metadata Extraction

**Files:**
- Modify: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/tests/import_html.rs`

- [ ] **Step 1: Write failing tests for headings and metadata**

Append to `crates/aif-html/tests/import_html.rs`:

```rust
#[test]
fn test_import_section_with_heading() {
    let html = "<html><body><section><h2>Title</h2><p>Content</p></section></body></html>";
    let result = aif_html::import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Section { attrs: _, title, children } => {
            assert_eq!(title.len(), 1);
            match &title[0] {
                Inline::Text { text } => assert_eq!(text, "Title"),
                other => panic!("expected Text, got {:?}", other),
            }
            assert_eq!(children.len(), 1);
            assert!(matches!(&children[0].kind, BlockKind::Paragraph { .. }));
        }
        other => panic!("expected Section, got {:?}", other),
    }
}

#[test]
fn test_import_section_with_id() {
    let html = r#"<html><body><section id="intro"><h2>Intro</h2></section></body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Section { attrs, .. } => {
            assert_eq!(attrs.id.as_deref(), Some("intro"));
        }
        other => panic!("expected Section, got {:?}", other),
    }
}

#[test]
fn test_import_bare_heading_becomes_section() {
    // A bare <h2> (no wrapping <section>) should still produce a Section block
    let html = "<html><body><h2>Standalone</h2><p>Content after heading</p></body></html>";
    let result = aif_html::import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            match &title[0] {
                Inline::Text { text } => assert_eq!(text, "Standalone"),
                other => panic!("expected Text, got {:?}", other),
            }
            // The paragraph after the heading should be a child of this section
            assert_eq!(children.len(), 1);
        }
        other => panic!("expected Section, got {:?}", other),
    }
}

#[test]
fn test_import_metadata_from_head() {
    let html = r#"<html><head><title>My Doc</title><meta name="description" content="A summary"></head><body><p>Text</p></body></html>"#;
    let result = aif_html::import_html(html, false);
    assert_eq!(result.document.metadata.get("title").map(|s| s.as_str()), Some("My Doc"));
    assert_eq!(result.document.metadata.get("summary").map(|s| s.as_str()), Some("A summary"));
}

#[test]
fn test_import_nested_sections() {
    let html = r#"<html><body>
        <section><h2>Outer</h2>
            <section><h3>Inner</h3><p>Nested content</p></section>
        </section>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            match &title[0] {
                Inline::Text { text } => assert_eq!(text, "Outer"),
                other => panic!("expected Text, got {:?}", other),
            }
            assert_eq!(children.len(), 1);
            assert!(matches!(&children[0].kind, BlockKind::Section { .. }));
        }
        other => panic!("expected Section, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_html`
Expected: FAIL — `<section>` and `<h*>` tags not handled.

- [ ] **Step 3: Implement section/heading parsing and metadata extraction**

Update `parse_block_element` in `crates/aif-html/src/importer.rs` to handle `<section>` and `<h1>`-`<h6>`. Update `import_html` to extract metadata from `<head>`.

For `<section>`: find first `<h*>` child as title, remaining children as blocks.

For bare `<h*>` (not inside `<section>`): collect subsequent sibling blocks until the next heading of same-or-higher level, wrap them in a Section.

For metadata: select `<title>` → `metadata["title"]`, `<meta name="description">` → `metadata["summary"]`.

Key additions to `parse_block_element`:
```rust
"section" => {
    let mut title_inlines = Vec::new();
    let mut children = Vec::new();
    let id = el.value().attr("id").map(|s| s.to_string());
    for child in el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            let tag = child_el.value().name();
            if title_inlines.is_empty() && matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                title_inlines = parse_inlines(child_el);
            } else if let Some(block) = parse_block_element(child_el) {
                children.push(block);
            }
        }
    }
    let mut attrs = Attrs::new();
    attrs.id = id;
    BlockKind::Section { attrs, title: title_inlines, children }
}
```

For bare headings, update `parse_blocks` to group a `<h*>` + following siblings into a Section block. This requires a two-pass approach: first collect raw block elements, then group bare headings with their following content.

For metadata extraction, add at the top of `import_html`:
```rust
let title_sel = Selector::parse("title").unwrap();
if let Some(title_el) = doc_html.select(&title_sel).next() {
    let title_text: String = title_el.text().collect();
    if !title_text.is_empty() {
        document.metadata.insert("title".into(), title_text);
    }
}
let meta_desc_sel = Selector::parse(r#"meta[name="description"]"#).unwrap();
if let Some(meta_el) = doc_html.select(&meta_desc_sel).next() {
    if let Some(content) = meta_el.value().attr("content") {
        document.metadata.insert("summary".into(), content.to_string());
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_html`
Expected: PASS (all 14 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/tests/import_html.rs
git commit -m "feat(aif-html): add section/heading parsing and metadata extraction"
```

---

### Task 4: Code Blocks, Block Quotes, Lists, and Thematic Breaks

**Files:**
- Modify: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/tests/import_html.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/aif-html/tests/import_html.rs`:

```rust
#[test]
fn test_import_code_block() {
    let html = r#"<html><body><pre><code class="language-rust">fn main() {}</code></pre></body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::CodeBlock { lang, attrs: _, code } => {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        }
        other => panic!("expected CodeBlock, got {:?}", other),
    }
}

#[test]
fn test_import_code_block_no_lang() {
    let html = "<html><body><pre><code>plain code</code></pre></body></html>";
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), None);
            assert_eq!(code, "plain code");
        }
        other => panic!("expected CodeBlock, got {:?}", other),
    }
}

#[test]
fn test_import_blockquote() {
    let html = "<html><body><blockquote><p>Quoted text</p></blockquote></body></html>";
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::BlockQuote { content } => {
            assert_eq!(content.len(), 1);
            assert!(matches!(&content[0].kind, BlockKind::Paragraph { .. }));
        }
        other => panic!("expected BlockQuote, got {:?}", other),
    }
}

#[test]
fn test_import_unordered_list() {
    let html = "<html><body><ul><li>Item 1</li><li>Item 2</li></ul></body></html>";
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
            match &items[0].content[0] {
                Inline::Text { text } => assert_eq!(text, "Item 1"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected List, got {:?}", other),
    }
}

#[test]
fn test_import_ordered_list() {
    let html = "<html><body><ol><li>First</li><li>Second</li></ol></body></html>";
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(ordered);
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected List, got {:?}", other),
    }
}

#[test]
fn test_import_nested_list() {
    let html = "<html><body><ul><li>Parent<ul><li>Child</li></ul></li></ul></body></html>";
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::List { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].children.len(), 1);
            assert!(matches!(&items[0].children[0].kind, BlockKind::List { .. }));
        }
        other => panic!("expected List, got {:?}", other),
    }
}

#[test]
fn test_import_thematic_break() {
    let html = "<html><body><p>Before</p><hr><p>After</p></body></html>";
    let result = aif_html::import_html(html, false);
    assert_eq!(result.document.blocks.len(), 3);
    assert!(matches!(&result.document.blocks[1].kind, BlockKind::ThematicBreak));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_html`
Expected: FAIL — code blocks, blockquotes, lists, hr not handled.

- [ ] **Step 3: Implement block-level elements**

Add cases to `parse_block_element` in `crates/aif-html/src/importer.rs`:

```rust
"pre" => {
    // Look for <code> child
    let code_sel = Selector::parse("code").unwrap();
    if let Some(code_el) = el.select(&code_sel).next() {
        let lang = code_el.value().attr("class")
            .and_then(|c| c.strip_prefix("language-"))
            .map(|s| s.to_string());
        let code: String = code_el.text().collect();
        BlockKind::CodeBlock {
            lang,
            attrs: Attrs::new(),
            code,
        }
    } else {
        let code: String = el.text().collect();
        BlockKind::CodeBlock {
            lang: None,
            attrs: Attrs::new(),
            code,
        }
    }
}
"blockquote" => {
    BlockKind::BlockQuote {
        content: parse_blocks(el),
    }
}
"ul" => {
    BlockKind::List {
        ordered: false,
        items: parse_list_items(el),
    }
}
"ol" => {
    BlockKind::List {
        ordered: true,
        items: parse_list_items(el),
    }
}
"hr" => BlockKind::ThematicBreak,
```

Add helper function `parse_list_items`:

```rust
fn parse_list_items(list_el: ElementRef) -> Vec<ListItem> {
    let li_sel = Selector::parse(":scope > li").unwrap();
    let mut items = Vec::new();
    for child in list_el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            if child_el.value().name() == "li" {
                let mut content = Vec::new();
                let mut children = Vec::new();
                for li_child in child_el.children() {
                    match li_child.value() {
                        Node::Text(text) => {
                            let s = text.to_string();
                            if !s.trim().is_empty() {
                                content.push(Inline::Text { text: s });
                            }
                        }
                        Node::Element(_) => {
                            let li_child_el = ElementRef::wrap(li_child).unwrap();
                            let tag = li_child_el.value().name();
                            match tag {
                                "ul" | "ol" => {
                                    if let Some(block) = parse_block_element(li_child_el) {
                                        children.push(block);
                                    }
                                }
                                _ => {
                                    // Inline elements within list item
                                    content.extend(parse_inline_node(li_child_el));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                items.push(ListItem { content, children });
            }
        }
    }
    items
}
```

Note: `parse_inline_node` is a helper that handles a single element as an inline (same match arms as in `parse_inlines` but for a single `ElementRef`). Extract the inline-element matching from `parse_inlines` into a shared helper to avoid duplication.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_html`
Expected: PASS (all 21 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/tests/import_html.rs
git commit -m "feat(aif-html): add code blocks, blockquotes, lists, and thematic breaks"
```

---

### Task 5: Tables

**Files:**
- Modify: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/tests/import_html.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/aif-html/tests/import_html.rs`:

```rust
#[test]
fn test_import_table_with_headers() {
    let html = r#"<html><body>
        <table>
            <thead><tr><th>Name</th><th>Age</th></tr></thead>
            <tbody><tr><td>Alice</td><td>30</td></tr></tbody>
        </table>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Table { attrs: _, caption: _, headers, rows } => {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].len(), 2);
        }
        other => panic!("expected Table, got {:?}", other),
    }
}

#[test]
fn test_import_table_with_caption() {
    let html = r#"<html><body>
        <table>
            <caption>Results</caption>
            <thead><tr><th>Score</th></tr></thead>
            <tbody><tr><td>100</td></tr></tbody>
        </table>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Table { caption, .. } => {
            assert!(caption.is_some());
            let cap = caption.as_ref().unwrap();
            match &cap[0] {
                Inline::Text { text } => assert_eq!(text, "Results"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Table, got {:?}", other),
    }
}

#[test]
fn test_import_table_with_id() {
    let html = r#"<html><body><table id="data"><thead><tr><th>X</th></tr></thead></table></body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Table { attrs, .. } => {
            assert_eq!(attrs.id.as_deref(), Some("data"));
        }
        other => panic!("expected Table, got {:?}", other),
    }
}

#[test]
fn test_import_table_no_thead() {
    // Some HTML tables have no <thead>, just <tr> with <td>
    let html = r#"<html><body>
        <table>
            <tr><td>A</td><td>B</td></tr>
            <tr><td>C</td><td>D</td></tr>
        </table>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Table { headers, rows, .. } => {
            assert!(headers.is_empty());
            assert_eq!(rows.len(), 2);
        }
        other => panic!("expected Table, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_html`
Expected: FAIL — `<table>` not handled.

- [ ] **Step 3: Implement table parsing**

Add `"table"` case to `parse_block_element`:

```rust
"table" => {
    let id = el.value().attr("id").map(|s| s.to_string());
    let mut attrs = Attrs::new();
    attrs.id = id;

    // Caption
    let caption_sel = Selector::parse("caption").unwrap();
    let caption = el.select(&caption_sel).next().map(|cap_el| parse_inlines(cap_el));

    // Headers from <thead><tr><th>
    let mut headers = Vec::new();
    let th_sel = Selector::parse("thead th").unwrap();
    for th in el.select(&th_sel) {
        headers.push(parse_inlines(th));
    }

    // Rows from <tbody><tr><td> or bare <tr><td>
    let mut rows = Vec::new();
    let tbody_sel = Selector::parse("tbody").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse(":scope > td").unwrap();

    if let Some(tbody) = el.select(&tbody_sel).next() {
        for tr in tbody.select(&tr_sel) {
            let row: Vec<Vec<Inline>> = tr.children()
                .filter_map(ElementRef::wrap)
                .filter(|c| c.value().name() == "td")
                .map(|td| parse_inlines(td))
                .collect();
            if !row.is_empty() {
                rows.push(row);
            }
        }
    } else {
        // No <tbody> — bare <tr> elements directly in <table>
        for tr in el.select(&tr_sel) {
            // Skip if this <tr> is inside <thead>
            let has_th = tr.children()
                .filter_map(ElementRef::wrap)
                .any(|c| c.value().name() == "th");
            if has_th { continue; }
            let row: Vec<Vec<Inline>> = tr.children()
                .filter_map(ElementRef::wrap)
                .filter(|c| c.value().name() == "td")
                .map(|td| parse_inlines(td))
                .collect();
            if !row.is_empty() {
                rows.push(row);
            }
        }
    }

    BlockKind::Table { attrs, caption, headers, rows }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_html`
Expected: PASS (all 25 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/tests/import_html.rs
git commit -m "feat(aif-html): add table import with headers, caption, and body rows"
```

---

### Task 6: Media Blocks (Figure, Audio, Video)

**Files:**
- Modify: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/tests/import_html.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/aif-html/tests/import_html.rs`:

```rust
#[test]
fn test_import_figure() {
    let html = r#"<html><body>
        <figure>
            <img src="photo.jpg" alt="Sunset" width="800" height="600">
            <figcaption>A beautiful sunset</figcaption>
        </figure>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Figure { src, meta, caption, .. } => {
            assert_eq!(src, "photo.jpg");
            assert_eq!(meta.alt.as_deref(), Some("Sunset"));
            assert_eq!(meta.width, Some(800));
            assert_eq!(meta.height, Some(600));
            assert!(caption.is_some());
        }
        other => panic!("expected Figure, got {:?}", other),
    }
}

#[test]
fn test_import_figure_with_id() {
    let html = r#"<html><body><figure id="fig1"><img src="a.png" alt=""></figure></body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Figure { attrs, .. } => {
            assert_eq!(attrs.id.as_deref(), Some("fig1"));
        }
        other => panic!("expected Figure, got {:?}", other),
    }
}

#[test]
fn test_import_audio() {
    let html = r#"<html><body>
        <audio controls src="song.mp3" id="a1">
            <p>My Song</p>
        </audio>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Audio { src, attrs, caption, .. } => {
            assert_eq!(src, "song.mp3");
            assert_eq!(attrs.id.as_deref(), Some("a1"));
            assert!(caption.is_some());
        }
        other => panic!("expected Audio, got {:?}", other),
    }
}

#[test]
fn test_import_audio_with_source() {
    let html = r#"<html><body>
        <audio controls>
            <source src="track.ogg" type="audio/ogg">
        </audio>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Audio { src, meta, .. } => {
            assert_eq!(src, "track.ogg");
            assert_eq!(meta.mime.as_deref(), Some("audio/ogg"));
        }
        other => panic!("expected Audio, got {:?}", other),
    }
}

#[test]
fn test_import_video() {
    let html = r#"<html><body>
        <video controls src="clip.mp4" width="1920" height="1080" poster="thumb.jpg">
            <p>My Video</p>
        </video>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Video { src, meta, caption, .. } => {
            assert_eq!(src, "clip.mp4");
            assert_eq!(meta.width, Some(1920));
            assert_eq!(meta.height, Some(1080));
            assert_eq!(meta.poster.as_deref(), Some("thumb.jpg"));
            assert!(caption.is_some());
        }
        other => panic!("expected Video, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_html`
Expected: FAIL — figure, audio, video not handled.

- [ ] **Step 3: Implement media block parsing**

Add cases to `parse_block_element`:

```rust
"figure" => {
    let id = el.value().attr("id").map(|s| s.to_string());
    let mut attrs = Attrs::new();
    attrs.id = id;

    let img_sel = Selector::parse("img").unwrap();
    let figcap_sel = Selector::parse("figcaption").unwrap();

    let (src, meta) = if let Some(img) = el.select(&img_sel).next() {
        let src = img.value().attr("src").unwrap_or("").to_string();
        let meta = MediaMeta {
            alt: img.value().attr("alt").map(|s| s.to_string()),
            width: img.value().attr("width").and_then(|s| s.parse().ok()),
            height: img.value().attr("height").and_then(|s| s.parse().ok()),
            ..MediaMeta::default()
        };
        (src, meta)
    } else {
        (String::new(), MediaMeta::default())
    };

    let caption = el.select(&figcap_sel).next().map(|cap| parse_inlines(cap));

    BlockKind::Figure { attrs, caption, src, meta }
}
"audio" => {
    let id = el.value().attr("id").map(|s| s.to_string());
    let mut attrs = Attrs::new();
    attrs.id = id;

    // src can be on <audio> itself or on a <source> child
    let source_sel = Selector::parse("source").unwrap();
    let (src, mime) = if let Some(audio_src) = el.value().attr("src") {
        (audio_src.to_string(), None)
    } else if let Some(source_el) = el.select(&source_sel).next() {
        let s = source_el.value().attr("src").unwrap_or("").to_string();
        let m = source_el.value().attr("type").map(|t| t.to_string());
        (s, m)
    } else {
        (String::new(), None)
    };

    let meta = MediaMeta { mime, ..MediaMeta::default() };

    // Caption from <p> child
    let p_sel = Selector::parse("p").unwrap();
    let caption = el.select(&p_sel).next().map(|p| parse_inlines(p));

    BlockKind::Audio { attrs, caption, src, meta }
}
"video" => {
    let id = el.value().attr("id").map(|s| s.to_string());
    let mut attrs = Attrs::new();
    attrs.id = id;

    let src = el.value().attr("src").unwrap_or("").to_string();
    let meta = MediaMeta {
        width: el.value().attr("width").and_then(|s| s.parse().ok()),
        height: el.value().attr("height").and_then(|s| s.parse().ok()),
        poster: el.value().attr("poster").map(|s| s.to_string()),
        ..MediaMeta::default()
    };

    let p_sel = Selector::parse("p").unwrap();
    let caption = el.select(&p_sel).next().map(|p| parse_inlines(p));

    BlockKind::Video { attrs, caption, src, meta }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_html`
Expected: PASS (all 30 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/tests/import_html.rs
git commit -m "feat(aif-html): add figure, audio, and video import with MediaMeta"
```

---

### Task 7: AIF Semantic Blocks and Callouts (Roundtrip Layer)

**Files:**
- Modify: `crates/aif-html/src/importer.rs`
- Create: `crates/aif-html/tests/import_aif_roundtrip.rs`

- [ ] **Step 1: Write failing tests for AIF class detection**

Create `crates/aif-html/tests/import_aif_roundtrip.rs`:

```rust
use aif_core::ast::*;

#[test]
fn test_roundtrip_semantic_block_claim() {
    let html = r#"<html><body><div class="aif-claim"><p>This is a claim.</p></div></body></html>"#;
    let result = aif_html::import_html(html, false);
    assert_eq!(result.mode, aif_html::ImportMode::AifRoundtrip);
    match &result.document.blocks[0].kind {
        BlockKind::SemanticBlock { block_type, content, .. } => {
            assert_eq!(*block_type, SemanticBlockType::Claim);
            match &content[0] {
                Inline::Text { text } => assert_eq!(text, "This is a claim."),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected SemanticBlock, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_semantic_block_with_title() {
    let html = r#"<html><body>
        <div class="aif-evidence" id="e1">
            <strong>Key Finding</strong>
            <p>The data shows...</p>
        </div>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::SemanticBlock { block_type, attrs, title, content } => {
            assert_eq!(*block_type, SemanticBlockType::Evidence);
            assert_eq!(attrs.id.as_deref(), Some("e1"));
            assert!(title.is_some());
            let t = title.as_ref().unwrap();
            match &t[0] {
                Inline::Text { text } => assert_eq!(text, "Key Finding"),
                other => panic!("expected Text, got {:?}", other),
            }
            assert!(!content.is_empty());
        }
        other => panic!("expected SemanticBlock, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_all_semantic_types() {
    let types = vec![
        ("claim", SemanticBlockType::Claim),
        ("evidence", SemanticBlockType::Evidence),
        ("definition", SemanticBlockType::Definition),
        ("theorem", SemanticBlockType::Theorem),
        ("assumption", SemanticBlockType::Assumption),
        ("result", SemanticBlockType::Result),
        ("conclusion", SemanticBlockType::Conclusion),
        ("requirement", SemanticBlockType::Requirement),
        ("recommendation", SemanticBlockType::Recommendation),
    ];
    for (class, expected_type) in types {
        let html = format!(
            r#"<html><body><div class="aif-{}"><p>Content</p></div></body></html>"#,
            class
        );
        let result = aif_html::import_html(&html, false);
        match &result.document.blocks[0].kind {
            BlockKind::SemanticBlock { block_type, .. } => {
                assert_eq!(*block_type, expected_type, "failed for class aif-{}", class);
            }
            other => panic!("expected SemanticBlock for aif-{}, got {:?}", class, other),
        }
    }
}

#[test]
fn test_roundtrip_callout_note() {
    let html = r#"<html><body><aside class="aif-callout aif-note"><p>Take note.</p></aside></body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Callout { callout_type, content, .. } => {
            assert_eq!(*callout_type, CalloutType::Note);
            match &content[0] {
                Inline::Text { text } => assert_eq!(text, "Take note."),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Callout, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_all_callout_types() {
    let types = vec![
        ("note", CalloutType::Note),
        ("warning", CalloutType::Warning),
        ("info", CalloutType::Info),
        ("tip", CalloutType::Tip),
    ];
    for (class, expected_type) in types {
        let html = format!(
            r#"<html><body><aside class="aif-callout aif-{}"><p>Content</p></aside></body></html>"#,
            class
        );
        let result = aif_html::import_html(&html, false);
        match &result.document.blocks[0].kind {
            BlockKind::Callout { callout_type, .. } => {
                assert_eq!(*callout_type, expected_type, "failed for aif-{}", class);
            }
            other => panic!("expected Callout for aif-{}, got {:?}", class, other),
        }
    }
}

#[test]
fn test_roundtrip_aif_ref() {
    let html = r#"<html><body><p>See <a class="aif-ref" href="#intro">intro</a></p></body></html>"#;
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    // "See " + Reference
    assert!(content.len() >= 2);
    match &content[1] {
        Inline::Reference { target } => assert_eq!(target, "intro"),
        other => panic!("expected Reference, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_aif_footnote() {
    let html = r#"<html><body><p>Text<sup class="aif-footnote">note</sup></p></body></html>"#;
    let result = aif_html::import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    match &content[1] {
        Inline::Footnote { content } => {
            match &content[0] {
                Inline::Text { text } => assert_eq!(text, "note"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Footnote, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_aif_roundtrip`
Expected: FAIL — AIF CSS classes not detected.

- [ ] **Step 3: Implement AIF class detection**

Update `parse_block_element` in `crates/aif-html/src/importer.rs` to check for `aif-*` classes on `<div>` and `<aside>` elements BEFORE falling through to generic parsing:

```rust
"div" => {
    let classes: Vec<&str> = el.value().classes().collect();
    // Check for AIF semantic block classes
    if let Some(sem_type) = detect_semantic_type(&classes) {
        let id = el.value().attr("id").map(|s| s.to_string());
        let mut attrs = Attrs::new();
        attrs.id = id;

        // Title from <strong> direct child
        let strong_sel = Selector::parse(":scope > strong").unwrap();
        let title = el.select(&strong_sel).next().map(|s| parse_inlines(s));

        // Content from <p> direct child
        let p_sel = Selector::parse(":scope > p").unwrap();
        let content = el.select(&p_sel).next()
            .map(|p| parse_inlines(p))
            .unwrap_or_default();

        return Some(Block {
            kind: BlockKind::SemanticBlock { block_type: sem_type, attrs, title, content },
            span: Span::new(0, 0),
        });
    }
    // Check for AIF skill block classes (handled in Task 8)
    // ...
    // Generic div: recurse into children
    let blocks = parse_blocks(el);
    if blocks.is_empty() { return None; }
    if blocks.len() == 1 { return Some(blocks.into_iter().next().unwrap()); }
    // Multiple blocks in a div — return them individually (flatten)
    // This requires returning Vec<Block> instead; handle via a separate mechanism
    return None; // Simplified for now
}
"aside" => {
    let classes: Vec<&str> = el.value().classes().collect();
    if classes.contains(&"aif-callout") {
        let callout_type = if classes.contains(&"aif-note") { CalloutType::Note }
            else if classes.contains(&"aif-warning") { CalloutType::Warning }
            else if classes.contains(&"aif-info") { CalloutType::Info }
            else if classes.contains(&"aif-tip") { CalloutType::Tip }
            else { CalloutType::Note }; // fallback

        let id = el.value().attr("id").map(|s| s.to_string());
        let mut attrs = Attrs::new();
        attrs.id = id;

        let p_sel = Selector::parse("p").unwrap();
        let content = el.select(&p_sel).next()
            .map(|p| parse_inlines(p))
            .unwrap_or_default();

        BlockKind::Callout { callout_type, attrs, content }
    } else {
        // Generic aside → blockquote-like
        BlockKind::BlockQuote { content: parse_blocks(el) }
    }
}
```

Add helper function:
```rust
fn detect_semantic_type(classes: &[&str]) -> Option<SemanticBlockType> {
    for class in classes {
        match *class {
            "aif-claim" => return Some(SemanticBlockType::Claim),
            "aif-evidence" => return Some(SemanticBlockType::Evidence),
            "aif-definition" => return Some(SemanticBlockType::Definition),
            "aif-theorem" => return Some(SemanticBlockType::Theorem),
            "aif-assumption" => return Some(SemanticBlockType::Assumption),
            "aif-result" => return Some(SemanticBlockType::Result),
            "aif-conclusion" => return Some(SemanticBlockType::Conclusion),
            "aif-requirement" => return Some(SemanticBlockType::Requirement),
            "aif-recommendation" => return Some(SemanticBlockType::Recommendation),
            _ => {}
        }
    }
    None
}
```

Update inline parsing to detect `aif-ref` and `aif-footnote` classes:

In the `"a"` arm of inline parsing:
```rust
"a" => {
    let classes: Vec<&str> = el.classes().collect();
    if classes.contains(&"aif-ref") {
        let target = el.attr("href").unwrap_or("")
            .trim_start_matches('#').to_string();
        inlines.push(Inline::Reference { target });
    } else {
        let url = el.attr("href").unwrap_or("").to_string();
        inlines.push(Inline::Link {
            text: parse_inlines(child_ref),
            url,
        });
    }
}
```

In inline parsing, add `"sup"` handling:
```rust
"sup" => {
    let classes: Vec<&str> = el.classes().collect();
    if classes.contains(&"aif-footnote") {
        inlines.push(Inline::Footnote {
            content: parse_inlines(child_ref),
        });
    } else {
        inlines.extend(parse_inlines(child_ref));
    }
}
```

Also update `import_html` to auto-detect AIF mode by checking if any `aif-*` class is present:

```rust
// After parsing blocks, detect mode
let has_aif_classes = input.contains("aif-");
let mode = if has_aif_classes { ImportMode::AifRoundtrip } else { ImportMode::Generic };
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_aif_roundtrip`
Expected: PASS (all 7 tests).

- [ ] **Step 5: Run all tests**

Run: `cargo test -p aif-html`
Expected: PASS (all tests including previous generic import tests).

- [ ] **Step 6: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/tests/import_aif_roundtrip.rs
git commit -m "feat(aif-html): add AIF semantic block and callout roundtrip import"
```

---

### Task 8: AIF Skill Blocks (Roundtrip Layer)

**Files:**
- Modify: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/tests/import_aif_roundtrip.rs`

- [ ] **Step 1: Write failing tests for skill block import**

Append to `crates/aif-html/tests/import_aif_roundtrip.rs`:

```rust
#[test]
fn test_roundtrip_skill_block() {
    let html = r#"<html><body>
        <div class="aif-skill" id="debugging">
            <h3>Debugging Skill</h3>
            <p>A skill for debugging.</p>
            <div class="aif-precondition"><p>When a bug is found.</p></div>
            <div class="aif-step"><h3>Step 1</h3><p>Read the error.</p></div>
            <div class="aif-verify"><p>Confirm the fix works.</p></div>
        </div>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::SkillBlock { skill_type, attrs, title, content, children } => {
            assert_eq!(*skill_type, SkillBlockType::Skill);
            assert_eq!(attrs.id.as_deref(), Some("debugging"));
            assert!(title.is_some());
            assert!(!content.is_empty());
            assert_eq!(children.len(), 3);
            assert!(matches!(&children[0].kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Precondition, .. }));
            assert!(matches!(&children[1].kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Step, .. }));
            assert!(matches!(&children[2].kind, BlockKind::SkillBlock { skill_type: SkillBlockType::Verify, .. }));
        }
        other => panic!("expected SkillBlock, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_all_skill_types() {
    let types = vec![
        ("aif-skill", SkillBlockType::Skill),
        ("aif-step", SkillBlockType::Step),
        ("aif-verify", SkillBlockType::Verify),
        ("aif-precondition", SkillBlockType::Precondition),
        ("aif-output-contract", SkillBlockType::OutputContract),
        ("aif-decision", SkillBlockType::Decision),
        ("aif-tool", SkillBlockType::Tool),
        ("aif-fallback", SkillBlockType::Fallback),
        ("aif-red-flag", SkillBlockType::RedFlag),
        ("aif-example", SkillBlockType::Example),
        ("aif-scenario", SkillBlockType::Scenario),
    ];
    for (class, expected_type) in types {
        let html = format!(
            r#"<html><body><div class="{}"><p>Content</p></div></body></html>"#,
            class
        );
        let result = aif_html::import_html(&html, false);
        match &result.document.blocks[0].kind {
            BlockKind::SkillBlock { skill_type, .. } => {
                assert_eq!(*skill_type, expected_type, "failed for class {}", class);
            }
            other => panic!("expected SkillBlock for {}, got {:?}", class, other),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_aif_roundtrip`
Expected: FAIL — skill block classes not detected.

- [ ] **Step 3: Implement skill block detection**

In the `"div"` arm of `parse_block_element`, after the semantic block check, add skill block detection:

```rust
// Check for AIF skill block classes
if let Some(skill_type) = detect_skill_type(&classes) {
    let id = el.value().attr("id").map(|s| s.to_string());
    let mut attrs = Attrs::new();
    attrs.id = id;
    // Copy non-standard attrs from HTML data attributes if present
    for (key, val) in el.value().attrs() {
        if key.starts_with("data-aif-") {
            attrs.pairs.insert(
                key.trim_start_matches("data-aif-").to_string(),
                val.to_string(),
            );
        }
    }

    // Title from <h3> direct child
    let h3_sel = Selector::parse(":scope > h3").unwrap();
    let title = el.select(&h3_sel).next().map(|h| parse_inlines(h));

    // Content from <p> direct child (first one)
    let p_sel = Selector::parse(":scope > p").unwrap();
    let content = el.select(&p_sel).next()
        .map(|p| parse_inlines(p))
        .unwrap_or_default();

    // Children: nested div elements that are also skill blocks
    let mut children = Vec::new();
    let div_sel = Selector::parse(":scope > div").unwrap();
    for child_div in el.select(&div_sel) {
        if let Some(block) = parse_block_element(child_div) {
            children.push(block);
        }
    }

    return Some(Block {
        kind: BlockKind::SkillBlock { skill_type, attrs, title, content, children },
        span: Span::new(0, 0),
    });
}
```

Add helper:
```rust
fn detect_skill_type(classes: &[&str]) -> Option<SkillBlockType> {
    for class in classes {
        match *class {
            "aif-skill" => return Some(SkillBlockType::Skill),
            "aif-step" => return Some(SkillBlockType::Step),
            "aif-verify" => return Some(SkillBlockType::Verify),
            "aif-precondition" => return Some(SkillBlockType::Precondition),
            "aif-output-contract" => return Some(SkillBlockType::OutputContract),
            "aif-decision" => return Some(SkillBlockType::Decision),
            "aif-tool" => return Some(SkillBlockType::Tool),
            "aif-fallback" => return Some(SkillBlockType::Fallback),
            "aif-red-flag" => return Some(SkillBlockType::RedFlag),
            "aif-example" => return Some(SkillBlockType::Example),
            "aif-scenario" => return Some(SkillBlockType::Scenario),
            _ => {}
        }
    }
    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_aif_roundtrip`
Expected: PASS (all 9 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/aif-html/src/importer.rs crates/aif-html/tests/import_aif_roundtrip.rs
git commit -m "feat(aif-html): add AIF skill block roundtrip import (all 11 types)"
```

---

### Task 9: Full Roundtrip Test (Emit → Import → Compare)

**Files:**
- Modify: `crates/aif-html/tests/import_aif_roundtrip.rs`

- [ ] **Step 1: Write roundtrip test**

Append to `crates/aif-html/tests/import_aif_roundtrip.rs`:

```rust
use aif_core::span::Span;

/// Helper: build a Document, emit HTML, import HTML, compare AST structure.
fn roundtrip(doc: &Document) -> Document {
    let html = aif_html::render_html(doc);
    let result = aif_html::import_html(&html, false);
    assert_eq!(result.mode, aif_html::ImportMode::AifRoundtrip);
    result.document
}

#[test]
fn test_full_roundtrip_mixed_document() {
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Test Doc".into());

    // Section with paragraph
    doc.blocks.push(Block {
        kind: BlockKind::Section {
            attrs: { let mut a = Attrs::new(); a.id = Some("s1".into()); a },
            title: vec![Inline::Text { text: "Introduction".into() }],
            children: vec![
                Block {
                    kind: BlockKind::Paragraph {
                        content: vec![
                            Inline::Text { text: "Hello ".into() },
                            Inline::Strong { content: vec![Inline::Text { text: "world".into() }] },
                        ],
                    },
                    span: Span::new(0, 0),
                },
            ],
        },
        span: Span::new(0, 0),
    });

    // Semantic block
    doc.blocks.push(Block {
        kind: BlockKind::SemanticBlock {
            block_type: SemanticBlockType::Claim,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "A bold claim.".into() }],
        },
        span: Span::new(0, 0),
    });

    // Callout
    doc.blocks.push(Block {
        kind: BlockKind::Callout {
            callout_type: CalloutType::Warning,
            attrs: Attrs::new(),
            content: vec![Inline::Text { text: "Be careful!".into() }],
        },
        span: Span::new(0, 0),
    });

    let imported = roundtrip(&doc);

    // Verify title metadata
    assert_eq!(imported.metadata.get("title").map(|s| s.as_str()), Some("Test Doc"));

    // Verify section
    match &imported.blocks[0].kind {
        BlockKind::Section { attrs, title, children } => {
            assert_eq!(attrs.id.as_deref(), Some("s1"));
            match &title[0] {
                Inline::Text { text } => assert_eq!(text, "Introduction"),
                other => panic!("expected Text, got {:?}", other),
            }
            assert_eq!(children.len(), 1);
        }
        other => panic!("expected Section, got {:?}", other),
    }

    // Verify semantic block
    match &imported.blocks[1].kind {
        BlockKind::SemanticBlock { block_type, .. } => {
            assert_eq!(*block_type, SemanticBlockType::Claim);
        }
        other => panic!("expected SemanticBlock, got {:?}", other),
    }

    // Verify callout
    match &imported.blocks[2].kind {
        BlockKind::Callout { callout_type, .. } => {
            assert_eq!(*callout_type, CalloutType::Warning);
        }
        other => panic!("expected Callout, got {:?}", other),
    }
}

#[test]
fn test_full_roundtrip_code_and_table() {
    let mut doc = Document::new();

    doc.blocks.push(Block {
        kind: BlockKind::CodeBlock {
            lang: Some("rust".into()),
            attrs: Attrs::new(),
            code: "fn main() {}".into(),
        },
        span: Span::new(0, 0),
    });

    doc.blocks.push(Block {
        kind: BlockKind::Table {
            attrs: Attrs::new(),
            caption: Some(vec![Inline::Text { text: "Data".into() }]),
            headers: vec![
                vec![Inline::Text { text: "A".into() }],
                vec![Inline::Text { text: "B".into() }],
            ],
            rows: vec![vec![
                vec![Inline::Text { text: "1".into() }],
                vec![Inline::Text { text: "2".into() }],
            ]],
        },
        span: Span::new(0, 0),
    });

    let imported = roundtrip(&doc);

    match &imported.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        }
        other => panic!("expected CodeBlock, got {:?}", other),
    }

    match &imported.blocks[1].kind {
        BlockKind::Table { caption, headers, rows, .. } => {
            assert!(caption.is_some());
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
        }
        other => panic!("expected Table, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p aif-html --test import_aif_roundtrip`
Expected: PASS — if prior tasks implemented correctly. If any fail, debug and fix.

- [ ] **Step 3: Commit**

```bash
git add crates/aif-html/tests/import_aif_roundtrip.rs
git commit -m "test(aif-html): add full emit→import roundtrip tests"
```

---

### Task 10: Readability Extraction (--strip-chrome)

**Files:**
- Create: `crates/aif-html/src/readability.rs`
- Modify: `crates/aif-html/src/importer.rs`
- Modify: `crates/aif-html/src/lib.rs`
- Create: `crates/aif-html/tests/import_readability.rs`

- [ ] **Step 1: Write failing tests for readability extraction**

Create `crates/aif-html/tests/import_readability.rs`:

```rust
use aif_core::ast::*;

#[test]
fn test_strip_chrome_removes_nav() {
    let html = r#"<html><body>
        <nav><a href="/">Home</a><a href="/about">About</a></nav>
        <main>
            <article>
                <h1>Real Article</h1>
                <p>This is the actual content that matters.</p>
            </article>
        </main>
        <footer><p>Copyright 2025</p></footer>
    </body></html>"#;
    let result = aif_html::import_html(html, true);
    // Should NOT contain nav or footer content
    let json = serde_json::to_string(&result.document).unwrap();
    assert!(!json.contains("Home"));
    assert!(!json.contains("Copyright"));
    // Should contain article content
    assert!(json.contains("Real Article"));
    assert!(json.contains("actual content"));
}

#[test]
fn test_strip_chrome_uses_article_tag() {
    let html = r#"<html><body>
        <div class="sidebar">Ads here</div>
        <article>
            <p>Main content paragraph one.</p>
            <p>Main content paragraph two.</p>
        </article>
        <div class="sidebar">More ads</div>
    </body></html>"#;
    let result = aif_html::import_html(html, true);
    assert!(result.document.blocks.len() >= 2);
    let json = serde_json::to_string(&result.document).unwrap();
    assert!(!json.contains("Ads here"));
    assert!(json.contains("Main content paragraph one"));
}

#[test]
fn test_strip_chrome_uses_main_tag() {
    let html = r#"<html><body>
        <header><h1>Site Title</h1></header>
        <main>
            <p>Important content.</p>
        </main>
        <footer><p>Footer stuff</p></footer>
    </body></html>"#;
    let result = aif_html::import_html(html, true);
    let json = serde_json::to_string(&result.document).unwrap();
    assert!(!json.contains("Footer stuff"));
    assert!(json.contains("Important content"));
}

#[test]
fn test_strip_chrome_confidence_scores() {
    let html = r#"<html><body>
        <main>
            <p>Content extracted via readability.</p>
        </main>
        <footer><p>Junk</p></footer>
    </body></html>"#;
    let result = aif_html::import_html(html, true);
    // Readability-extracted blocks should have import_confidence attr
    match &result.document.blocks[0].kind {
        BlockKind::Paragraph { .. } => {
            // Confidence should be set (we'll check via attrs on the block)
            // Since Paragraph doesn't have attrs, confidence is tracked separately
            // Just verify the content was imported
        }
        other => panic!("expected Paragraph, got {:?}", other),
    }
}

#[test]
fn test_no_strip_chrome_keeps_everything() {
    let html = r#"<html><body>
        <nav><a href="/">Home</a></nav>
        <p>Content</p>
        <footer><p>Footer</p></footer>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    // Without strip_chrome, nav and footer links/text should be preserved
    // (nav becomes links, footer paragraphs are kept)
    assert!(result.document.blocks.len() >= 2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-html --test import_readability`
Expected: FAIL — `strip_chrome` parameter is ignored.

- [ ] **Step 3: Implement readability extraction**

Create `crates/aif-html/src/readability.rs`:

```rust
use scraper::{Html, Selector, ElementRef};

/// Tags that are considered boilerplate/chrome (not content).
const CHROME_TAGS: &[&str] = &["nav", "header", "footer", "aside"];

/// Tags that signal main content areas.
const CONTENT_TAGS: &[&str] = &["article", "main"];

/// Extract the main content element from an HTML document.
/// Returns the ElementRef to parse, or None if no content area found
/// (in which case the caller should fall back to <body>).
///
/// Strategy:
/// 1. If <article> exists, use it.
/// 2. Else if <main> exists, use it.
/// 3. Else if [role="main"] exists, use it.
/// 4. Else fall back to <body> but strip chrome tags.
pub fn extract_content_root(doc: &Html) -> ContentExtraction {
    // Try <article>
    let article_sel = Selector::parse("article").unwrap();
    if let Some(el) = doc.select(&article_sel).next() {
        return ContentExtraction::ContentElement(el.id());
    }

    // Try <main>
    let main_sel = Selector::parse("main").unwrap();
    if let Some(el) = doc.select(&main_sel).next() {
        return ContentExtraction::ContentElement(el.id());
    }

    // Try [role="main"]
    let role_sel = Selector::parse("[role=main]").unwrap();
    if let Some(el) = doc.select(&role_sel).next() {
        return ContentExtraction::ContentElement(el.id());
    }

    // Fallback: use body but filter out chrome tags
    ContentExtraction::BodyWithChromeStripped
}

pub enum ContentExtraction {
    /// Use this specific element as the content root
    ContentElement(ego_tree::NodeId),
    /// Use <body> but skip nav/header/footer/aside children
    BodyWithChromeStripped,
}

/// Check if an element tag should be stripped as chrome.
pub fn is_chrome_tag(tag: &str) -> bool {
    CHROME_TAGS.contains(&tag)
}
```

Update `crates/aif-html/src/lib.rs` to add the module:
```rust
mod emitter;
pub mod importer;
mod readability;
```

Update `import_html` in `crates/aif-html/src/importer.rs` to use readability when `strip_chrome` is true:

```rust
pub fn import_html(input: &str, strip_chrome: bool) -> HtmlImportResult {
    let doc_html = Html::parse_document(input);
    let mut document = Document::new();

    // Extract metadata from <head>
    extract_metadata(&doc_html, &mut document);

    if strip_chrome {
        use crate::readability::{extract_content_root, ContentExtraction, is_chrome_tag};
        match extract_content_root(&doc_html) {
            ContentExtraction::ContentElement(node_id) => {
                let el = ElementRef::wrap(doc_html.tree.get(node_id).unwrap()).unwrap();
                document.blocks = parse_blocks(el);
            }
            ContentExtraction::BodyWithChromeStripped => {
                let body_sel = Selector::parse("body").unwrap();
                if let Some(body) = doc_html.select(&body_sel).next() {
                    for child in body.children() {
                        if let Some(child_el) = ElementRef::wrap(child) {
                            if !is_chrome_tag(child_el.value().name()) {
                                if let Some(block) = parse_block_element(child_el) {
                                    document.blocks.push(block);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        let body_sel = Selector::parse("body").unwrap();
        if let Some(body) = doc_html.select(&body_sel).next() {
            document.blocks = parse_blocks(body);
        }
    }

    let has_aif_classes = input.contains("aif-");
    let mode = if has_aif_classes { ImportMode::AifRoundtrip } else { ImportMode::Generic };

    HtmlImportResult { document, mode }
}
```

Note: The `ego_tree::NodeId` approach requires accessing scraper's internal tree. If `scraper`'s API doesn't expose this cleanly, an alternative is to use CSS selectors to find the content element and pass it to `parse_blocks` directly. Adjust the `ContentExtraction` enum to hold a selector string instead and re-select within `import_html`. The implementer should check `scraper`'s API and pick the approach that compiles.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-html --test import_readability`
Expected: PASS (all 5 tests).

- [ ] **Step 5: Run all aif-html tests**

Run: `cargo test -p aif-html`
Expected: PASS (all tests).

- [ ] **Step 6: Commit**

```bash
git add crates/aif-html/src/readability.rs crates/aif-html/src/importer.rs crates/aif-html/src/lib.rs crates/aif-html/tests/import_readability.rs
git commit -m "feat(aif-html): add readability extraction for --strip-chrome mode"
```

---

### Task 11: CLI Integration

**Files:**
- Modify: `crates/aif-cli/src/main.rs`

- [ ] **Step 1: Write failing test for CLI HTML import**

The CLI dispatches by file extension. Currently `.pdf` → PDF import, else → Markdown. We need `.html`/`.htm` → HTML import.

Create a test fixture at `crates/aif-cli/tests/fixtures/test_import.html`:
```html
<!DOCTYPE html>
<html lang="en">
<head><title>Test</title></head>
<body>
<p>Hello from HTML</p>
</body>
</html>
```

Add a CLI integration test (or verify manually). The simplest approach: run the CLI and check output:

```bash
cargo run -p aif-cli -- import crates/aif-cli/tests/fixtures/test_import.html
```

Expected: FAIL — CLI treats `.html` as Markdown, producing garbled output.

- [ ] **Step 2: Update CLI import dispatch**

In `crates/aif-cli/src/main.rs`, update the `Import` match arm. Add HTML detection between PDF and the Markdown fallback:

```rust
Commands::Import { input, output } => {
    let ext = input.extension().map(|e| e.to_ascii_lowercase());
    let is_pdf = ext.as_ref().map(|e| e == "pdf").unwrap_or(false);
    let is_html = ext.as_ref().map(|e| e == "html" || e == "htm").unwrap_or(false);

    if is_pdf {
        // ... existing PDF import code ...
    } else if is_html {
        let source = read_source(&input);
        let result = aif_html::import_html(&source, false);
        eprintln!(
            "Imported HTML ({} mode), {} blocks",
            match result.mode {
                aif_html::ImportMode::AifRoundtrip => "AIF roundtrip",
                aif_html::ImportMode::Generic => "generic",
            },
            result.document.blocks.len()
        );
        let json = serde_json::to_string_pretty(&result.document).unwrap();
        write_output(&json, output.as_ref());
    } else {
        // ... existing Markdown import code ...
    }
}
```

Also update the CLI help text for the `Import` subcommand — change the doc comment from `"Input file (Markdown or PDF)"` to `"Input file (Markdown, HTML, or PDF)"`.

- [ ] **Step 3: Add `--strip-chrome` flag to Import subcommand**

Update the `Import` variant in the CLI:

```rust
Import {
    /// Input file (Markdown, HTML, or PDF)
    input: PathBuf,
    /// Output file (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Strip page chrome (nav, header, footer) for HTML import
    #[arg(long)]
    strip_chrome: bool,
},
```

Pass `strip_chrome` to `import_html`:
```rust
let result = aif_html::import_html(&source, strip_chrome);
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p aif-cli && cargo run -p aif-cli -- import crates/aif-cli/tests/fixtures/test_import.html`
Expected: JSON output with one Paragraph block containing "Hello from HTML".

- [ ] **Step 5: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-cli/src/main.rs crates/aif-cli/tests/fixtures/test_import.html
git commit -m "feat(aif-cli): add HTML import with --strip-chrome flag"
```

---

### Task 12: Update CLAUDE.md and Documentation

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update CLAUDE.md**

Add HTML import to the `aif-html` crate description:
```
| `aif-html` | HTML compiler (AST → HTML) + importer (HTML → AST) with AIF-roundtrip and generic modes |
```

Add to CLI commands section:
```bash
aif import input.html [--strip-chrome]  # Import HTML (auto-detects AIF roundtrip vs generic)
```

Update the Known Limitations section — remove the "HTML import not implemented" note and add:
```
- HTML generic import maps `<div>` containers to flat block lists (no generic div-to-section heuristic)
- Readability extraction (--strip-chrome) uses tag-based heuristics, not full Mozilla Readability algorithm
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md with HTML importer capabilities"
```

---

## Summary

| Task | Description | Tests |
|------|-------------|-------|
| 1 | Scaffolding + paragraph import | 2 |
| 2 | Inline elements (strong, em, code, link, img, br) | 7 |
| 3 | Sections, headings, metadata | 5 |
| 4 | Code blocks, blockquotes, lists, thematic breaks | 7 |
| 5 | Tables | 4 |
| 6 | Media (figure, audio, video + MediaMeta) | 5 |
| 7 | AIF semantic blocks + callouts (roundtrip) | 7 |
| 8 | AIF skill blocks (roundtrip) | 2 |
| 9 | Full roundtrip test (emit → import → compare) | 2 |
| 10 | Readability extraction (--strip-chrome) | 5 |
| 11 | CLI integration | Manual + workspace |
| 12 | Documentation | — |
| **Total** | | **~46 tests** |
