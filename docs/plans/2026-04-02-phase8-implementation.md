# Phase 8: Semantic Inference, Roundtrip Benchmark, Chunk Graph Lint — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add pattern-based semantic inference on imported documents, a roundtrip quality benchmark, and chunk graph structural lint checks.

**Architecture:** Three independent modules — `aif-core::infer` (pattern rules that upgrade untyped blocks to semantic blocks), `benchmarks/roundtrip_benchmark.py` (AIF → format → AIF fidelity measurement), and `lint_chunk_graph()` in `aif-core::lint` (orphaned chunks, missing continuation, dependency cycles). All follow TDD.

**Tech Stack:** Rust (aif-core, aif-cli), Python (benchmark script), regex for pattern matching

---

## Task 1: Inference Engine — Core Types and Trait

**Files:**
- Create: `crates/aif-core/src/infer.rs`
- Modify: `crates/aif-core/src/lib.rs`

- [ ] **Step 1: Write failing test for InferConfig default**

Add to `crates/aif-core/src/infer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = InferConfig::default();
        assert_eq!(config.min_confidence, 0.5);
        assert!(matches!(config.strategy, InferStrategy::Pattern));
    }
}
```

- [ ] **Step 2: Implement core types**

Create `crates/aif-core/src/infer.rs`:

```rust
//! Semantic inference engine.
//!
//! Upgrades untyped blocks (Paragraph, BlockQuote, Callout) to typed SemanticBlocks
//! based on pattern rules. Each inference carries a confidence score and rule name
//! in the block's `_aif_*` attributes.

use crate::ast::*;
use crate::text::{inlines_to_text, TextMode};

/// Configuration for the semantic inference engine.
pub struct InferConfig {
    /// Only apply inferences with confidence >= this threshold.
    pub min_confidence: f64,
    /// Which inference strategy to use.
    pub strategy: InferStrategy,
}

pub enum InferStrategy {
    /// Deterministic pattern-based rules only.
    Pattern,
    // Future: Llm(LlmConfig) for LLM-assisted classification
}

impl Default for InferConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            strategy: InferStrategy::Pattern,
        }
    }
}

/// A single inference rule that examines a block and optionally returns
/// a semantic type + confidence score.
pub trait InferRule: Send + Sync {
    fn name(&self) -> &str;
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)>;
}
```

- [ ] **Step 3: Register module in lib.rs**

Add to `crates/aif-core/src/lib.rs`:

```rust
pub mod infer;
```

- [ ] **Step 4: Run test**

Run: `cargo test -p aif-core --lib infer`
Expected: PASS — `default_config` passes.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-core/src/infer.rs crates/aif-core/src/lib.rs
git commit -m "feat(aif-core): add semantic inference types and InferRule trait"
```

---

## Task 2: Inference Engine — Pattern Rules

**Files:**
- Modify: `crates/aif-core/src/infer.rs`

- [ ] **Step 1: Write failing test for blockquote-with-citation rule**

```rust
#[test]
fn blockquote_with_link_inferred_as_evidence() {
    let block = Block {
        kind: BlockKind::BlockQuote {
            content: vec![Block {
                kind: BlockKind::Paragraph {
                    content: vec![
                        Inline::Text { text: "According to ".into() },
                        Inline::Link {
                            text: vec![Inline::Text { text: "Smith 2024".into() }],
                            url: "https://example.com".into(),
                        },
                    ],
                },
                span: Span::new(0, 0),
            }],
        },
        span: Span::new(0, 0),
    };
    let rule = BlockquoteWithCitation;
    let result = rule.try_infer(&block);
    assert!(result.is_some());
    let (stype, conf) = result.unwrap();
    assert_eq!(stype, SemanticBlockType::Evidence);
    assert!((conf - 0.70).abs() < 0.01);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-core --lib infer::tests::blockquote_with_link`
Expected: FAIL — `BlockquoteWithCitation` not found.

- [ ] **Step 3: Implement all 8 pattern rules**

Add to `crates/aif-core/src/infer.rs`:

```rust
use crate::span::Span;

// --- Helper: extract plain text from a block's content inlines ---
fn block_text(block: &Block) -> String {
    match &block.kind {
        BlockKind::Paragraph { content } => inlines_to_text(content, TextMode::Plain),
        BlockKind::BlockQuote { content } => {
            content.iter().map(block_text).collect::<Vec<_>>().join(" ")
        }
        BlockKind::Callout { content, .. } => inlines_to_text(content, TextMode::Plain),
        _ => String::new(),
    }
}

fn has_link_inline(block: &Block) -> bool {
    fn check_inlines(inlines: &[Inline]) -> bool {
        inlines.iter().any(|i| match i {
            Inline::Link { .. } => true,
            Inline::Strong { content } | Inline::Emphasis { content } => check_inlines(content),
            _ => false,
        })
    }
    match &block.kind {
        BlockKind::Paragraph { content } => check_inlines(content),
        BlockKind::BlockQuote { content } => content.iter().any(has_link_inline),
        _ => false,
    }
}

fn sentence_count(text: &str) -> usize {
    text.split(|c| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .count()
}

fn text_starts_with_any(text: &str, prefixes: &[&str]) -> bool {
    let lower = text.trim_start().to_lowercase();
    prefixes.iter().any(|p| lower.starts_with(p))
}

// --- Rule 1: BlockQuote with citation link → Evidence ---
pub struct BlockquoteWithCitation;
impl InferRule for BlockquoteWithCitation {
    fn name(&self) -> &str { "blockquote_with_citation" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if !matches!(&block.kind, BlockKind::BlockQuote { .. }) { return None; }
        if has_link_inline(block) {
            Some((SemanticBlockType::Evidence, 0.70))
        } else {
            None
        }
    }
}

// --- Rule 2: Short BlockQuote without link → Claim ---
pub struct BlockquoteShortClaim;
impl InferRule for BlockquoteShortClaim {
    fn name(&self) -> &str { "blockquote_short_claim" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if !matches!(&block.kind, BlockKind::BlockQuote { .. }) { return None; }
        if has_link_inline(block) { return None; }
        let text = block_text(block);
        if sentence_count(&text) < 3 && !text.is_empty() {
            Some((SemanticBlockType::Claim, 0.55))
        } else {
            None
        }
    }
}

// --- Rule 3: Paragraph starting with definition language → Definition ---
pub struct ParagraphDefinition;
impl InferRule for ParagraphDefinition {
    fn name(&self) -> &str { "paragraph_definition" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if !matches!(&block.kind, BlockKind::Paragraph { .. }) { return None; }
        let text = block_text(block);
        if text_starts_with_any(&text, &["we define", "is defined as", "definition:"]) {
            Some((SemanticBlockType::Definition, 0.80))
        } else {
            None
        }
    }
}

// --- Rule 4: Paragraph starting with recommendation language → Recommendation ---
pub struct ParagraphRecommendation;
impl InferRule for ParagraphRecommendation {
    fn name(&self) -> &str { "paragraph_recommendation" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if !matches!(&block.kind, BlockKind::Paragraph { .. }) { return None; }
        let text = block_text(block);
        if text_starts_with_any(&text, &["we recommend", "it is recommended"]) {
            Some((SemanticBlockType::Recommendation, 0.75))
        } else {
            None
        }
    }
}

// --- Rule 5: Paragraph starting with conclusion language → Conclusion ---
pub struct ParagraphConclusion;
impl InferRule for ParagraphConclusion {
    fn name(&self) -> &str { "paragraph_conclusion" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if !matches!(&block.kind, BlockKind::Paragraph { .. }) { return None; }
        let text = block_text(block);
        if text_starts_with_any(&text, &["we conclude", "in conclusion", "therefore"]) {
            Some((SemanticBlockType::Conclusion, 0.75))
        } else {
            None
        }
    }
}

// --- Rule 6: Paragraph starting with assumption language → Assumption ---
pub struct ParagraphAssumption;
impl InferRule for ParagraphAssumption {
    fn name(&self) -> &str { "paragraph_assumption" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if !matches!(&block.kind, BlockKind::Paragraph { .. }) { return None; }
        let text = block_text(block);
        if text_starts_with_any(&text, &["we assume", "assuming that"]) {
            Some((SemanticBlockType::Assumption, 0.70))
        } else {
            None
        }
    }
}

// --- Rule 7: Paragraph starting with result language → Result ---
pub struct ParagraphResult;
impl InferRule for ParagraphResult {
    fn name(&self) -> &str { "paragraph_result" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if !matches!(&block.kind, BlockKind::Paragraph { .. }) { return None; }
        let text = block_text(block);
        if text_starts_with_any(&text, &["the result shows", "results indicate"]) {
            Some((SemanticBlockType::Result, 0.65))
        } else {
            None
        }
    }
}

// --- Rule 8: Warning callout with requirement language → Requirement ---
pub struct CalloutRequirement;
impl InferRule for CalloutRequirement {
    fn name(&self) -> &str { "callout_requirement" }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        match &block.kind {
            BlockKind::Callout { callout_type: CalloutType::Warning, .. } => {
                let text = block_text(block).to_lowercase();
                if text.contains("requirement") || text.contains("must") || text.contains("shall") {
                    Some((SemanticBlockType::Requirement, 0.60))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Build the default set of pattern rules.
pub fn default_rules() -> Vec<Box<dyn InferRule>> {
    vec![
        Box::new(BlockquoteWithCitation),
        Box::new(BlockquoteShortClaim),
        Box::new(ParagraphDefinition),
        Box::new(ParagraphRecommendation),
        Box::new(ParagraphConclusion),
        Box::new(ParagraphAssumption),
        Box::new(ParagraphResult),
        Box::new(CalloutRequirement),
    ]
}
```

- [ ] **Step 4: Add tests for remaining rules**

```rust
#[test]
fn short_blockquote_inferred_as_claim() {
    let block = Block {
        kind: BlockKind::BlockQuote {
            content: vec![Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Text { text: "AIF is the future.".into() }],
                },
                span: Span::new(0, 0),
            }],
        },
        span: Span::new(0, 0),
    };
    let rule = BlockquoteShortClaim;
    let result = rule.try_infer(&block);
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, SemanticBlockType::Claim);
}

#[test]
fn paragraph_definition_detected() {
    let block = Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: "We define TNO as Token-Normalized Outcome.".into() }],
        },
        span: Span::new(0, 0),
    };
    let rule = ParagraphDefinition;
    assert!(rule.try_infer(&block).is_some());
}

#[test]
fn paragraph_conclusion_detected() {
    let block = Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: "Therefore, AIF is more efficient.".into() }],
        },
        span: Span::new(0, 0),
    };
    let rule = ParagraphConclusion;
    assert!(rule.try_infer(&block).is_some());
}

#[test]
fn plain_paragraph_not_inferred() {
    let block = Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text { text: "The weather is nice today.".into() }],
        },
        span: Span::new(0, 0),
    };
    let rules = default_rules();
    let matched = rules.iter().any(|r| r.try_infer(&block).is_some());
    assert!(!matched, "Plain paragraph should not match any rule");
}

#[test]
fn callout_requirement_detected() {
    let block = Block {
        kind: BlockKind::Callout {
            callout_type: CalloutType::Warning,
            attrs: Attrs::default(),
            content: vec![Inline::Text { text: "The system must handle 10K requests/s.".into() }],
        },
        span: Span::new(0, 0),
    };
    let rule = CalloutRequirement;
    assert!(rule.try_infer(&block).is_some());
}

#[test]
fn callout_note_not_inferred_as_requirement() {
    let block = Block {
        kind: BlockKind::Callout {
            callout_type: CalloutType::Note,
            attrs: Attrs::default(),
            content: vec![Inline::Text { text: "You must restart the server.".into() }],
        },
        span: Span::new(0, 0),
    };
    let rule = CalloutRequirement;
    assert!(rule.try_infer(&block).is_none(), "Only Warning callouts trigger requirement rule");
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test -p aif-core --lib infer`
Expected: PASS — all 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-core/src/infer.rs
git commit -m "feat(aif-core): implement 8 pattern-based semantic inference rules"
```

---

## Task 3: Inference Engine — annotate_semantics()

**Files:**
- Modify: `crates/aif-core/src/infer.rs`

- [ ] **Step 1: Write failing test for annotate_semantics**

```rust
#[test]
fn annotate_upgrades_blockquote_to_claim() {
    let mut doc = Document {
        metadata: [("title".into(), "T".into())].into(),
        blocks: vec![Block {
            kind: BlockKind::BlockQuote {
                content: vec![Block {
                    kind: BlockKind::Paragraph {
                        content: vec![Inline::Text { text: "AIF is the future.".into() }],
                    },
                    span: Span::new(0, 0),
                }],
            },
            span: Span::new(0, 0),
        }],
    };
    annotate_semantics(&mut doc, &InferConfig::default());
    match &doc.blocks[0].kind {
        BlockKind::SemanticBlock { block_type, attrs, .. } => {
            assert_eq!(*block_type, SemanticBlockType::Claim);
            assert_eq!(attrs.pairs.get("_aif_inferred").map(|s| s.as_str()), Some("true"));
            assert!(attrs.pairs.contains_key("_aif_confidence"));
            assert_eq!(attrs.pairs.get("_aif_infer_rule").map(|s| s.as_str()), Some("blockquote_short_claim"));
        }
        other => panic!("Expected SemanticBlock, got {:?}", other),
    }
}

#[test]
fn annotate_skips_existing_semantic_blocks() {
    let mut doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SemanticBlock {
                block_type: SemanticBlockType::Claim,
                attrs: Attrs { id: Some("c1".into()), ..Default::default() },
                title: None,
                content: vec![Inline::Text { text: "Already typed.".into() }],
            },
            span: Span::new(0, 0),
        }],
    };
    annotate_semantics(&mut doc, &InferConfig::default());
    // Should still be a SemanticBlock with no _aif_inferred attr
    match &doc.blocks[0].kind {
        BlockKind::SemanticBlock { attrs, .. } => {
            assert!(!attrs.pairs.contains_key("_aif_inferred"));
        }
        _ => panic!("Should still be SemanticBlock"),
    }
}

#[test]
fn annotate_respects_min_confidence() {
    let mut doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::BlockQuote {
                content: vec![Block {
                    kind: BlockKind::Paragraph {
                        content: vec![Inline::Text { text: "Short claim.".into() }],
                    },
                    span: Span::new(0, 0),
                }],
            },
            span: Span::new(0, 0),
        }],
    };
    let config = InferConfig { min_confidence: 0.90, ..Default::default() };
    annotate_semantics(&mut doc, &config);
    // Claim rule has confidence 0.55, should NOT be applied
    assert!(matches!(&doc.blocks[0].kind, BlockKind::BlockQuote { .. }));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p aif-core --lib infer::tests::annotate`
Expected: FAIL — `annotate_semantics` not found.

- [ ] **Step 3: Implement annotate_semantics**

Add to `crates/aif-core/src/infer.rs`:

```rust
use std::collections::BTreeMap;

/// Walk the document and upgrade untyped blocks to semantic blocks.
pub fn annotate_semantics(doc: &mut Document, config: &InferConfig) {
    let rules = default_rules();
    annotate_blocks(&mut doc.blocks, &rules, config);
}

fn annotate_blocks(blocks: &mut Vec<Block>, rules: &[Box<dyn InferRule>], config: &InferConfig) {
    for block in blocks.iter_mut() {
        // Skip blocks that are already typed
        match &block.kind {
            BlockKind::SemanticBlock { .. } | BlockKind::SkillBlock { .. } => continue,
            BlockKind::Section { .. } | BlockKind::Table { .. } | BlockKind::Figure { .. }
            | BlockKind::Audio { .. } | BlockKind::Video { .. } | BlockKind::CodeBlock { .. }
            | BlockKind::ThematicBreak | BlockKind::List { .. } => continue,
            _ => {}
        }

        // Try each rule, keep the highest confidence match
        let mut best: Option<(&str, SemanticBlockType, f64)> = None;
        for rule in rules {
            if let Some((stype, conf)) = rule.try_infer(block) {
                if conf >= config.min_confidence {
                    if best.as_ref().map_or(true, |b| conf > b.2) {
                        best = Some((rule.name(), stype, conf));
                    }
                }
            }
        }

        // Transform the block if a rule matched
        if let Some((rule_name, stype, confidence)) = best {
            let content = extract_content(block);
            let mut attrs = Attrs::new();
            attrs.pairs.insert("_aif_inferred".into(), "true".into());
            attrs.pairs.insert("_aif_confidence".into(), format!("{:.2}", confidence));
            attrs.pairs.insert("_aif_infer_rule".into(), rule_name.to_string());

            block.kind = BlockKind::SemanticBlock {
                block_type: stype,
                attrs,
                title: None,
                content,
            };
        }

        // Recurse into children
        match &mut block.kind {
            BlockKind::Section { children, .. } => annotate_blocks(children, rules, config),
            BlockKind::BlockQuote { content } => annotate_blocks(content, rules, config),
            _ => {}
        }
    }
}

/// Extract inline content from a block being transformed.
fn extract_content(block: &Block) -> Vec<Inline> {
    match &block.kind {
        BlockKind::Paragraph { content } => content.clone(),
        BlockKind::BlockQuote { content } => {
            // Flatten child paragraphs into inline content
            content.iter().flat_map(|b| match &b.kind {
                BlockKind::Paragraph { content } => content.clone(),
                _ => vec![Inline::Text { text: block_text(b) }],
            }).collect()
        }
        BlockKind::Callout { content, .. } => content.clone(),
        _ => vec![],
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p aif-core --lib infer`
Expected: PASS — all 10 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-core/src/infer.rs
git commit -m "feat(aif-core): implement annotate_semantics with confidence scoring"
```

---

## Task 4: CLI — Wire --infer-semantics Flag

**Files:**
- Modify: `crates/aif-cli/src/main.rs`

- [ ] **Step 1: Add flag to Import command**

In the `Commands::Import` variant, add:

```rust
    Import {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        strip_chrome: bool,
        /// Run semantic inference on imported document
        #[arg(long)]
        infer_semantics: bool,
    },
```

- [ ] **Step 2: Wire inference into import handler**

In the `Commands::Import` match arm, after each import path (PDF, HTML, Markdown) and before the `write_output` call, add:

```rust
// After: let mut doc = ...
// Before: let json = serde_json::to_string_pretty(&doc)...

if infer_semantics {
    aif_core::infer::annotate_semantics(&mut doc, &aif_core::infer::InferConfig::default());
    let inferred_count = doc.blocks.iter()
        .filter(|b| matches!(&b.kind, aif_core::ast::BlockKind::SemanticBlock { attrs, .. } if attrs.pairs.contains_key("_aif_inferred")))
        .count();
    if inferred_count > 0 {
        eprintln!("Inferred {} semantic block(s)", inferred_count);
    }
}
```

Apply this to all 3 branches (PDF, HTML, Markdown).

- [ ] **Step 3: Build and test**

Run: `cargo build -p aif-cli`
Expected: compiles without errors.

Run: `echo '> AIF is the future of documents.\n\nWe recommend adopting AIF for all LLM pipelines.' | cargo run -p aif-cli -- import /dev/stdin --infer-semantics 2>&1`
Expected: stderr shows "Inferred 2 semantic block(s)", stdout JSON contains `SemanticBlock` with `_aif_inferred: "true"`.

- [ ] **Step 4: Commit**

```bash
git add crates/aif-cli/src/main.rs
git commit -m "feat(aif-cli): add --infer-semantics flag to import command"
```

---

## Task 5: Chunk Graph Lint — OrphanedChunks and MissingContinuation

**Files:**
- Modify: `crates/aif-core/src/lint.rs`

- [ ] **Step 1: Write failing tests**

Add to tests module in `crates/aif-core/src/lint.rs`:

```rust
use crate::chunk::*;

#[test]
fn orphaned_chunk_detected() {
    let mut graph = ChunkGraph::new();
    let a = ChunkId::new("doc", &[0]);
    let b = ChunkId::new("doc", &[1]);
    // Add chunks but no links
    graph.add_chunk(Chunk {
        id: a.clone(), source_doc: "doc.aif".into(), block_path: vec![0],
        blocks: vec![], metadata: ChunkMetadata {
            title: None, block_types: vec![], estimated_tokens: 100,
            depth: 0, sequence: 0, total_chunks: 2,
            summary: None, requires_parent_context: false, semantic_types: vec![],
        },
    });
    graph.add_chunk(Chunk {
        id: b.clone(), source_doc: "doc.aif".into(), block_path: vec![1],
        blocks: vec![], metadata: ChunkMetadata {
            title: None, block_types: vec![], estimated_tokens: 100,
            depth: 0, sequence: 1, total_chunks: 2,
            summary: None, requires_parent_context: false, semantic_types: vec![],
        },
    });
    let results = lint_chunk_graph(&graph);
    let orphaned: Vec<_> = results.iter()
        .filter(|r| r.check == DocLintCheck::OrphanedChunks && !r.passed)
        .collect();
    assert_eq!(orphaned.len(), 2);
}

#[test]
fn single_chunk_not_orphaned() {
    let mut graph = ChunkGraph::new();
    graph.add_chunk(Chunk {
        id: ChunkId::new("doc", &[0]), source_doc: "doc.aif".into(), block_path: vec![0],
        blocks: vec![], metadata: ChunkMetadata {
            title: None, block_types: vec![], estimated_tokens: 100,
            depth: 0, sequence: 0, total_chunks: 1,
            summary: None, requires_parent_context: false, semantic_types: vec![],
        },
    });
    let results = lint_chunk_graph(&graph);
    assert!(results.iter().all(|r| r.passed));
}

#[test]
fn missing_continuation_detected() {
    let mut graph = ChunkGraph::new();
    let a = ChunkId::new("doc", &[0]);
    let b = ChunkId::new("doc", &[1]);
    graph.add_chunk(Chunk {
        id: a.clone(), source_doc: "doc.aif".into(), block_path: vec![0],
        blocks: vec![], metadata: ChunkMetadata {
            title: None, block_types: vec![], estimated_tokens: 100,
            depth: 0, sequence: 0, total_chunks: 2,
            summary: None, requires_parent_context: false, semantic_types: vec![],
        },
    });
    graph.add_chunk(Chunk {
        id: b.clone(), source_doc: "doc.aif".into(), block_path: vec![1],
        blocks: vec![], metadata: ChunkMetadata {
            title: None, block_types: vec![], estimated_tokens: 100,
            depth: 0, sequence: 1, total_chunks: 2,
            summary: None, requires_parent_context: false, semantic_types: vec![],
        },
    });
    // No continuation link between a→b
    let results = lint_chunk_graph(&graph);
    let missing: Vec<_> = results.iter()
        .filter(|r| r.check == DocLintCheck::MissingContinuation && !r.passed)
        .collect();
    assert_eq!(missing.len(), 1);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p aif-core --lib lint::tests::orphaned`
Expected: FAIL — `lint_chunk_graph` not found, `DocLintCheck::OrphanedChunks` not found.

- [ ] **Step 3: Add new check variants and implement**

Add to `DocLintCheck` enum:

```rust
    /// Chunk has no incoming or outgoing links (isolated node).
    OrphanedChunks,
    /// Sequential chunks from the same document lack a Continuation link.
    MissingContinuation,
    /// Circular Dependency or ParentContext links detected.
    DependencyCycle,
```

Add `use crate::chunk::*;` to imports.

Implement:

```rust
/// Run structural lint checks on a chunk graph.
pub fn lint_chunk_graph(graph: &ChunkGraph) -> Vec<DocLintResult> {
    let mut results = Vec::new();

    // 1. Orphaned chunks (skip single-chunk documents)
    let mut doc_chunk_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for chunk in graph.chunks.values() {
        *doc_chunk_counts.entry(&chunk.source_doc).or_insert(0) += 1;
    }

    let mut orphan_found = false;
    for (id, chunk) in &graph.chunks {
        let doc_count = doc_chunk_counts.get(chunk.source_doc.as_str()).copied().unwrap_or(0);
        if doc_count <= 1 { continue; }
        let has_outgoing = graph.links.iter().any(|l| &l.source == id);
        let has_incoming = graph.links.iter().any(|l| &l.target == id);
        if !has_outgoing && !has_incoming {
            results.push(DocLintResult::fail(
                DocLintCheck::OrphanedChunks,
                DocLintSeverity::Warning,
                format!("Chunk {} has no links (isolated)", id),
                Some(id.0.clone()),
            ));
            orphan_found = true;
        }
    }
    if !orphan_found {
        results.push(DocLintResult::pass(DocLintCheck::OrphanedChunks));
    }

    // 2. Missing continuation links
    let mut missing_cont = false;
    for (doc_path, _) in &doc_chunk_counts {
        let mut doc_chunks: Vec<_> = graph.chunks.values()
            .filter(|c| c.source_doc == *doc_path)
            .collect();
        doc_chunks.sort_by_key(|c| c.metadata.sequence);

        for window in doc_chunks.windows(2) {
            let a_id = &window[0].id;
            let b_id = &window[1].id;
            let has_continuation = graph.links.iter().any(|l| {
                &l.source == a_id && &l.target == b_id
                    && l.link_type == LinkType::Continuation
            });
            if !has_continuation {
                results.push(DocLintResult::fail(
                    DocLintCheck::MissingContinuation,
                    DocLintSeverity::Warning,
                    format!("No Continuation link from {} to {}", a_id, b_id),
                    Some(a_id.0.clone()),
                ));
                missing_cont = true;
            }
        }
    }
    if !missing_cont {
        results.push(DocLintResult::pass(DocLintCheck::MissingContinuation));
    }

    // 3. Dependency cycles
    let cycles = detect_dependency_cycles(graph);
    if cycles.is_empty() {
        results.push(DocLintResult::pass(DocLintCheck::DependencyCycle));
    } else {
        for cycle in &cycles {
            let cycle_str = cycle.iter().map(|id| id.0.as_str()).collect::<Vec<_>>().join(" → ");
            results.push(DocLintResult::fail(
                DocLintCheck::DependencyCycle,
                DocLintSeverity::Error,
                format!("Dependency cycle: {}", cycle_str),
                cycle.first().map(|id| id.0.clone()),
            ));
        }
    }

    results
}

fn detect_dependency_cycles(graph: &ChunkGraph) -> Vec<Vec<ChunkId>> {
    // 3-color DFS: 0=white, 1=gray, 2=black
    let mut color: BTreeMap<&ChunkId, u8> = BTreeMap::new();
    let mut cycles = Vec::new();
    let mut path: Vec<ChunkId> = Vec::new();

    for id in graph.chunks.keys() {
        if color.get(id).copied().unwrap_or(0) == 0 {
            dfs_cycle(graph, id, &mut color, &mut path, &mut cycles);
        }
    }
    cycles
}

fn dfs_cycle<'a>(
    graph: &'a ChunkGraph,
    node: &'a ChunkId,
    color: &mut BTreeMap<&'a ChunkId, u8>,
    path: &mut Vec<ChunkId>,
    cycles: &mut Vec<Vec<ChunkId>>,
) {
    color.insert(node, 1); // gray
    path.push(node.clone());

    for link in &graph.links {
        if &link.source != node { continue; }
        if !matches!(link.link_type, LinkType::Dependency | LinkType::ParentContext) { continue; }

        match color.get(&link.target).copied().unwrap_or(0) {
            0 => dfs_cycle(graph, &link.target, color, path, cycles),
            1 => {
                // Found a cycle — extract from the target position in path
                if let Some(pos) = path.iter().position(|p| p == &link.target) {
                    cycles.push(path[pos..].to_vec());
                }
            }
            _ => {} // black, already finished
        }
    }

    path.pop();
    color.insert(node, 2); // black
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p aif-core --lib lint`
Expected: PASS — all tests including new chunk graph lint tests.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-core/src/lint.rs
git commit -m "feat(aif-core): add chunk graph lint — orphaned, missing continuation, cycles"
```

---

## Task 6: Chunk Graph Lint — Cycle Detection Test + CLI Wiring

**Files:**
- Modify: `crates/aif-core/src/lint.rs` (test)
- Modify: `crates/aif-cli/src/main.rs`

- [ ] **Step 1: Add cycle detection test**

```rust
#[test]
fn dependency_cycle_detected() {
    let mut graph = ChunkGraph::new();
    let a = ChunkId::new("doc", &[0]);
    let b = ChunkId::new("doc", &[1]);
    graph.add_chunk(Chunk {
        id: a.clone(), source_doc: "doc.aif".into(), block_path: vec![0],
        blocks: vec![], metadata: ChunkMetadata {
            title: None, block_types: vec![], estimated_tokens: 100,
            depth: 0, sequence: 0, total_chunks: 2,
            summary: None, requires_parent_context: false, semantic_types: vec![],
        },
    });
    graph.add_chunk(Chunk {
        id: b.clone(), source_doc: "doc.aif".into(), block_path: vec![1],
        blocks: vec![], metadata: ChunkMetadata {
            title: None, block_types: vec![], estimated_tokens: 100,
            depth: 0, sequence: 1, total_chunks: 2,
            summary: None, requires_parent_context: false, semantic_types: vec![],
        },
    });
    graph.add_link(ChunkLink { source: a.clone(), target: b.clone(), link_type: LinkType::Dependency, label: None });
    graph.add_link(ChunkLink { source: b.clone(), target: a.clone(), link_type: LinkType::Dependency, label: None });
    let results = lint_chunk_graph(&graph);
    let cycles: Vec<_> = results.iter()
        .filter(|r| r.check == DocLintCheck::DependencyCycle && !r.passed)
        .collect();
    assert!(!cycles.is_empty());
}
```

- [ ] **Step 2: Add `chunk lint` CLI subcommand**

Add to `ChunkAction` enum:

```rust
    /// Lint a chunk graph for structural issues
    Lint {
        /// Chunk graph JSON file
        input: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
```

Add handler in `handle_chunk`:

```rust
ChunkAction::Lint { input, format } => {
    let source = read_source(&input);
    let graph: aif_core::chunk::ChunkGraph = serde_json::from_str(&source)
        .unwrap_or_else(|e| {
            eprintln!("Error parsing chunk graph JSON: {}", e);
            std::process::exit(1);
        });
    let results = aif_core::lint::lint_chunk_graph(&graph);
    let (total, passed, failed) = aif_core::lint::lint_summary(&results);

    if format == "json" {
        let json_results: Vec<_> = results.iter().map(|r| serde_json::json!({
            "check": format!("{:?}", r.check),
            "passed": r.passed,
            "severity": format!("{:?}", r.severity),
            "message": r.message,
            "block_id": r.block_id,
        })).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "file": input.display().to_string(),
            "total": total, "passed": passed, "failed": failed,
            "results": json_results,
        })).unwrap());
    } else {
        println!("Chunk Graph Lint: {}", input.display());
        println!("{}", "=".repeat(60));
        for r in &results {
            if r.passed {
                println!("  [+] {:?}", r.check);
            } else {
                let sev = match r.severity {
                    aif_core::lint::DocLintSeverity::Error => "ERROR",
                    aif_core::lint::DocLintSeverity::Warning => "WARN",
                };
                let loc = r.block_id.as_ref().map(|id| format!(" ({})", id)).unwrap_or_default();
                println!("  [x] {:?} [{}]{}: {}", r.check, sev, loc, r.message);
            }
        }
        println!("{}", "-".repeat(60));
        println!("{} checks: {} passed, {} failed", total, passed, failed);
        if failed > 0 { std::process::exit(1); }
    }
}
```

- [ ] **Step 3: Build and test**

Run: `cargo build -p aif-cli`
Expected: compiles.

Run: `cargo test -p aif-core --lib lint`
Expected: PASS — all tests including cycle detection.

- [ ] **Step 4: Commit**

```bash
git add crates/aif-core/src/lint.rs crates/aif-cli/src/main.rs
git commit -m "feat(aif-cli): add 'chunk lint' subcommand for graph validation"
```

---

## Task 7: Roundtrip Quality Benchmark

**Files:**
- Create: `benchmarks/roundtrip_benchmark.py`

- [ ] **Step 1: Write the benchmark script**

Create `benchmarks/roundtrip_benchmark.py`:

```python
#!/usr/bin/env python3
"""
AIF Roundtrip Quality Benchmark

Measures how well documents survive format conversion round-trips:
AIF → HTML → AIF, AIF → Markdown → AIF, AIF → JSON → AIF.

Metrics: block count, block type, semantic type, metadata, inline fidelity.
"""

import json
import subprocess
import sys
import tempfile
import os
from pathlib import Path


def run_cli(args: list[str]) -> str:
    result = subprocess.run(
        ["cargo", "run", "-p", "aif-cli", "--"] + args,
        capture_output=True, text=True, cwd=Path(__file__).parent.parent,
    )
    return result.stdout


def dump_ir(aif_file: str) -> dict:
    out = run_cli(["dump-ir", aif_file])
    return json.loads(out) if out.strip() else {}


def count_blocks(ir: dict) -> int:
    count = 0
    def walk(blocks):
        nonlocal count
        for b in blocks:
            count += 1
            kind = b.get("kind", {})
            for key in ("children", "content"):
                if isinstance(kind.get(key), list) and kind[key] and isinstance(kind[key][0], dict) and "kind" in kind[key][0]:
                    walk(kind[key])
    walk(ir.get("blocks", []))
    return count


def collect_block_types(ir: dict) -> list[str]:
    types = []
    def walk(blocks):
        for b in blocks:
            kind = b.get("kind", {})
            types.append(kind.get("type", "Unknown"))
            for key in ("children", "content"):
                if isinstance(kind.get(key), list) and kind[key] and isinstance(kind[key][0], dict) and "kind" in kind[key][0]:
                    walk(kind[key])
    walk(ir.get("blocks", []))
    return types


def collect_semantic_types(ir: dict) -> list[str]:
    types = []
    def walk(blocks):
        for b in blocks:
            kind = b.get("kind", {})
            if kind.get("type") == "SemanticBlock":
                types.append(kind.get("block_type", "Unknown"))
            for key in ("children", "content"):
                if isinstance(kind.get(key), list) and kind[key] and isinstance(kind[key][0], dict) and "kind" in kind[key][0]:
                    walk(kind[key])
    walk(ir.get("blocks", []))
    return types


def count_inlines(ir: dict) -> dict[str, int]:
    counts: dict[str, int] = {}
    def walk_inlines(inlines):
        for i in inlines:
            if isinstance(i, dict):
                itype = i.get("type", "")
                counts[itype] = counts.get(itype, 0) + 1
                for key in ("content", "text"):
                    if isinstance(i.get(key), list):
                        walk_inlines(i[key])
    def walk_blocks(blocks):
        for b in blocks:
            kind = b.get("kind", {})
            for key in ("content", "title", "caption"):
                if isinstance(kind.get(key), list) and kind[key] and isinstance(kind[key][0], dict):
                    if "type" in kind[key][0] and "kind" not in kind[key][0]:
                        walk_inlines(kind[key])
            for key in ("children",):
                if isinstance(kind.get(key), list) and kind[key] and isinstance(kind[key][0], dict) and "kind" in kind[key][0]:
                    walk_blocks(kind[key])
    walk_blocks(ir.get("blocks", []))
    return counts


def compute_metrics(original: dict, roundtripped: dict) -> dict:
    orig_count = count_blocks(original)
    rt_count = count_blocks(roundtripped)
    block_count_ratio = rt_count / orig_count if orig_count > 0 else 1.0

    orig_types = collect_block_types(original)
    rt_types = collect_block_types(roundtripped)
    type_match = sum(1 for a, b in zip(orig_types, rt_types) if a == b)
    block_type_pres = type_match / len(orig_types) if orig_types else 1.0

    orig_sem = collect_semantic_types(original)
    rt_sem = collect_semantic_types(roundtripped)
    sem_match = sum(1 for a, b in zip(orig_sem, rt_sem) if a == b)
    semantic_pres = sem_match / len(orig_sem) if orig_sem else 1.0

    orig_meta = set(k for k in original.get("metadata", {}) if not k.startswith("_aif_"))
    rt_meta = set(k for k in roundtripped.get("metadata", {}) if not k.startswith("_aif_"))
    meta_pres = len(orig_meta & rt_meta) / len(orig_meta) if orig_meta else 1.0

    orig_inlines = count_inlines(original)
    rt_inlines = count_inlines(roundtripped)
    all_types = set(orig_inlines) | set(rt_inlines)
    if all_types:
        inline_match = sum(min(orig_inlines.get(t, 0), rt_inlines.get(t, 0)) for t in all_types)
        inline_total = sum(orig_inlines.values())
        inline_pres = inline_match / inline_total if inline_total > 0 else 1.0
    else:
        inline_pres = 1.0

    overall = (block_type_pres + semantic_pres * 2 + meta_pres + inline_pres) / 5.0

    return {
        "block_count_ratio": round(block_count_ratio, 3),
        "block_type_preservation": round(block_type_pres, 3),
        "semantic_type_preservation": round(semantic_pres, 3),
        "metadata_preservation": round(meta_pres, 3),
        "inline_fidelity": round(inline_pres, 3),
        "overall_fidelity": round(overall, 3),
    }


def roundtrip(aif_file: str, fmt: str) -> dict:
    original = dump_ir(aif_file)
    if not original:
        return {"error": "failed to parse original"}

    with tempfile.NamedTemporaryFile(suffix=f".{fmt}", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        ext = {"html": "html", "markdown": "md", "json": "json"}[fmt]
        tmp_path_ext = tmp_path.rsplit(".", 1)[0] + "." + ext
        os.rename(tmp_path, tmp_path_ext)
        tmp_path = tmp_path_ext

        run_cli(["compile", aif_file, "--format", fmt, "-o", tmp_path])

        if fmt == "json":
            reimported_json = run_cli(["compile", tmp_path, "--input-format", "json", "--format", "json"])
            reimported = json.loads(reimported_json) if reimported_json.strip() else {}
        else:
            reimported_json = run_cli(["import", tmp_path])
            reimported = json.loads(reimported_json) if reimported_json.strip() else {}

        if not reimported:
            return {"error": "failed to reimport"}

        return compute_metrics(original, reimported)
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


def main():
    examples_dir = Path(__file__).parent.parent / "examples"
    aif_files = sorted(examples_dir.glob("*.aif"))
    formats = ["html", "markdown", "json"]
    all_results = []

    print("=" * 70)
    print("AIF Roundtrip Quality Benchmark")
    print("=" * 70)
    print()
    print(f"{'File':<35s} {'Format':<12s} {'Blocks':>7s} {'Types':>7s} {'Semantic':>9s} {'Meta':>6s} {'Inline':>7s} {'Overall':>8s}")
    print("-" * 95)

    for aif_file in aif_files:
        for fmt in formats:
            result = roundtrip(str(aif_file), fmt)
            result["file"] = aif_file.name
            result["format"] = fmt
            all_results.append(result)

            if "error" in result:
                print(f"{aif_file.name:<35s} {fmt:<12s} ERROR: {result['error']}")
            else:
                print(
                    f"{aif_file.name:<35s} {fmt:<12s} "
                    f"{result['block_count_ratio']:>7.3f} "
                    f"{result['block_type_preservation']:>7.3f} "
                    f"{result['semantic_type_preservation']:>9.3f} "
                    f"{result['metadata_preservation']:>6.3f} "
                    f"{result['inline_fidelity']:>7.3f} "
                    f"{result['overall_fidelity']:>8.3f}"
                )

    # Summary per format
    print()
    print("=" * 70)
    print("Summary by Format")
    print("=" * 70)
    print(f"{'Format':<12s} {'Avg Overall':>12s} {'Avg Semantic':>13s} {'Avg Block Type':>15s}")
    print("-" * 55)
    for fmt in formats:
        fmt_results = [r for r in all_results if r.get("format") == fmt and "error" not in r]
        if not fmt_results:
            continue
        avg_overall = sum(r["overall_fidelity"] for r in fmt_results) / len(fmt_results)
        avg_sem = sum(r["semantic_type_preservation"] for r in fmt_results) / len(fmt_results)
        avg_bt = sum(r["block_type_preservation"] for r in fmt_results) / len(fmt_results)
        print(f"{fmt:<12s} {avg_overall:>12.3f} {avg_sem:>13.3f} {avg_bt:>15.3f}")

    output_path = Path(__file__).parent / "roundtrip_results.json"
    with open(output_path, "w") as f:
        json.dump(all_results, f, indent=2)
    print(f"\nResults saved to {output_path}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run the benchmark**

Run: `python3 benchmarks/roundtrip_benchmark.py`
Expected: table output with fidelity scores. JSON path should show ~1.0 (lossless). HTML should be >=0.9. Markdown >=0.8.

- [ ] **Step 3: Commit**

```bash
git add benchmarks/roundtrip_benchmark.py benchmarks/roundtrip_results.json
git commit -m "feat: add roundtrip quality benchmark for AIF format fidelity"
```

---

## Task 8: Update CLAUDE.md + Final Test

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add Phase 8 section to CLAUDE.md**

After the Phase 7 section, add:

```markdown
## Phase 8 Features

### Semantic Inference Engine
`crates/aif-core/src/infer.rs` — Pattern-based inference upgrades untyped blocks to semantic types. 8 rules: blockquote-with-citation→Evidence, short-blockquote→Claim, paragraph-definition/recommendation/conclusion/assumption/result, callout-requirement. Each inferred block gets `_aif_inferred`, `_aif_confidence`, `_aif_infer_rule` attrs. `InferRule` trait extensible for future LLM inference. CLI: `aif import doc.md --infer-semantics`.

### Roundtrip Quality Benchmark
`benchmarks/roundtrip_benchmark.py` — Measures AIF→HTML→AIF, AIF→Markdown→AIF, AIF→JSON→AIF fidelity. 5 metrics: block count ratio, block type preservation, semantic type preservation (2x weight), metadata preservation, inline fidelity. Overall fidelity score per document per path.

### Chunk Graph Lint
`lint_chunk_graph()` in `aif-core::lint` — 3 structural checks: OrphanedChunks (isolated nodes), MissingContinuation (sequential chunks without link), DependencyCycle (circular Dependency/ParentContext links via 3-color DFS). CLI: `aif chunk lint graph.json`.
```

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass, 0 failures.

- [ ] **Step 3: Commit and push**

```bash
git add CLAUDE.md
git commit -m "feat: Phase 8 — semantic inference, roundtrip benchmark, chunk graph lint"
git push origin main
```
