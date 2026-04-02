# Phase 8: Semantic Inference, Roundtrip Benchmark, Chunk Graph Lint

> **Goal:** Strengthen the "import → normalize → validate" pipeline with automatic semantic type inference, format roundtrip quality measurement, and chunk graph structural validation.

## 1. Architecture

```
Import pipeline (Phase 8):
  HTML/MD/PDF → Parser → raw AST → [Semantic Inferrer] → enriched AST → output
                                         │
                                    pattern rules
                                    + confidence scores
                                    + _aif_inferred attrs

Validation pipeline (Phase 8):
  enriched AST → [Roundtrip Benchmark] → fidelity report
  chunk graph  → [Chunk Graph Lint]    → graph diagnostics
```

### Module Placement

| Feature | Module | Location |
|---------|--------|----------|
| Semantic inference | `aif-core::infer` | `crates/aif-core/src/infer.rs` |
| Roundtrip benchmark | Python script | `benchmarks/roundtrip_benchmark.py` |
| Chunk graph lint | `aif-core::lint` extension | `crates/aif-core/src/lint.rs` + new `lint_chunk_graph()` |

### Why `aif-core::infer`?

Both HTML and Markdown importers produce the same `Document` AST. Inference runs on the AST, not on source format. One module serves all importers. When LLM inference (option C) is added later, there is one integration point.

---

## 2. Semantic Inference Engine

### API

```rust
// crates/aif-core/src/infer.rs

pub struct InferConfig {
    pub min_confidence: f64,      // Only apply inferences >= this (default 0.5)
    pub strategy: InferStrategy,
}

pub enum InferStrategy {
    Pattern,       // Deterministic heuristic rules
    // Llm(LlmConfig),  // Future: LLM-assisted classification
}

impl Default for InferConfig {
    fn default() -> Self {
        Self { min_confidence: 0.5, strategy: InferStrategy::Pattern }
    }
}

/// Walk the AST and upgrade untyped blocks to semantic blocks where patterns match.
/// Blocks already typed as SemanticBlock or SkillBlock are skipped.
/// Inferred blocks get _aif_inferred, _aif_confidence, _aif_infer_rule attrs.
pub fn annotate_semantics(doc: &mut Document, config: &InferConfig)
```

### Inference Metadata

When a rule fires, the block is transformed in-place and stamped with:

- `_aif_inferred: "true"` — machine-inferred, not author-declared
- `_aif_confidence: "0.75"` — rule confidence (0.0–1.0)
- `_aif_infer_rule: "blockquote_with_citation"` — which rule matched

### Pattern Rules (Initial Set)

| # | Input Block | Pattern | Output Type | Confidence |
|---|-------------|---------|-------------|------------|
| 1 | `BlockQuote` | Contains a `Link` inline (citation) | Evidence | 0.70 |
| 2 | `BlockQuote` | No link, short (< 3 sentences) | Claim | 0.55 |
| 3 | `Paragraph` | Starts with "We define..." / "X is defined as..." / "Definition:" | Definition | 0.80 |
| 4 | `Paragraph` | Starts with "We recommend..." / "It is recommended..." | Recommendation | 0.75 |
| 5 | `Paragraph` | Starts with "We conclude..." / "In conclusion..." / "Therefore..." | Conclusion | 0.75 |
| 6 | `Paragraph` | Starts with "We assume..." / "Assuming that..." | Assumption | 0.70 |
| 7 | `Paragraph` | Starts with "The result shows..." / "Results indicate..." | Result | 0.65 |
| 8 | `Callout[warning]` | Content matches "requirement" / "must" / "shall" | Requirement | 0.60 |

### Rule Trait (extensible for future LLM rules)

```rust
pub trait InferRule: Send + Sync {
    fn name(&self) -> &str;
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)>;
}
```

Rules are collected in a `Vec<Box<dyn InferRule>>`. The `annotate_semantics` function iterates blocks, applies the first matching rule (highest confidence wins if multiple match), and transforms the block. This trait makes adding an `LlmInferRule` trivial later.

### What Is NOT Inferred

- Blocks already typed as `SemanticBlock` or `SkillBlock` — skip
- `Section`, `Table`, `Figure`, `CodeBlock` — structural blocks, not semantic candidates
- Blocks where all rules return `None` or below `min_confidence`

### CLI Integration

```bash
# Import with semantic inference
aif import doc.md --infer-semantics
aif import doc.html --infer-semantics --strip-chrome

# Inference is opt-in. Without the flag, behavior is unchanged.
```

### Interaction with Lint

The existing `ClaimsWithoutEvidence` lint check benefits: after inference, a document that had only blockquotes and paragraphs may now have typed claims and evidence, making the check meaningful on imported documents.

---

## 3. Roundtrip Quality Benchmark

### Script

`benchmarks/roundtrip_benchmark.py` — self-contained Python script, no LLM calls.

### Roundtrip Paths

| Path | Compile Step | Re-import Step |
|------|-------------|----------------|
| AIF → HTML → AIF | `aif compile --format html` | `aif import result.html` |
| AIF → Markdown → AIF | `aif compile --format markdown` | `aif import result.md` |
| AIF → JSON → AIF | `aif compile --format json` | `aif compile --input-format json result.json --format json` |

PDF is excluded — PDF import is lossy by design (confidence-scored text extraction), not a meaningful roundtrip target.

### Metrics (per document, per path)

1. **Block count preservation** — `roundtripped_blocks / original_blocks` ratio
2. **Block type preservation** — fraction of blocks that kept their exact `BlockKind` type
3. **Semantic type preservation** — fraction of `SemanticBlock` types that survived (Claim stayed Claim, not flattened to Paragraph). Weighted 2x in overall score.
4. **Metadata preservation** — fraction of metadata keys present in both
5. **Inline fidelity** — fraction of inline elements (bold, italic, code, links) preserved
6. **Overall fidelity score** — weighted average: `(block_type * 1 + semantic * 2 + metadata * 1 + inline * 1) / 5`

### Implementation

```python
def roundtrip(aif_file, format):
    # 1. Dump original IR as JSON
    original = run_cli(["dump-ir", aif_file])
    
    # 2. Compile to target format
    compiled = run_cli(["compile", aif_file, "--format", format, "-o", tmp_file])
    
    # 3. Re-import back to IR
    reimported = run_cli(["import", tmp_file])
    
    # 4. Compare original vs reimported JSON
    return compute_metrics(original, reimported)
```

### Output

- Text summary table to stdout
- JSON results to `benchmarks/roundtrip_results.json`
- One row per document per path

### Test Corpus

All `.aif` files in `examples/`.

---

## 4. Chunk Graph Lint

### Graph-Level Checks

New function: `pub fn lint_chunk_graph(graph: &ChunkGraph) -> Vec<DocLintResult>`

| Check | Enum Variant | Severity | What It Catches |
|-------|-------------|----------|-----------------|
| Orphaned chunks | `OrphanedChunks` | Warning | Chunks with no links (isolated nodes). Single-chunk documents are exempt. |
| Missing continuation | `MissingContinuation` | Warning | Sequential chunks from the same document without a `Continuation` link between chunk N and chunk N+1. |
| Dependency cycle | `DependencyCycle` | Error | Circular `Dependency` or `ParentContext` links. Uses DFS with visited set. |

### API

```rust
// Added to crates/aif-core/src/lint.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkLintCheck {
    OrphanedChunks,
    MissingContinuation,
    DependencyCycle,
}

pub fn lint_chunk_graph(graph: &ChunkGraph) -> Vec<DocLintResult>
```

Returns the same `DocLintResult` type for CLI compatibility. The `block_id` field carries the `ChunkId` string for graph-level issues.

### Cycle Detection

```rust
fn detect_cycles(graph: &ChunkGraph) -> Vec<Vec<ChunkId>> {
    // DFS with 3-color marking (white/gray/black)
    // Only follows Dependency and ParentContext edges
    // Returns list of cycles found (each cycle = vec of ChunkIds)
}
```

### CLI Integration

```bash
# Lint a chunk graph file
aif chunk lint graph.json

# Output: same text/json format as `aif lint`
aif chunk lint graph.json --format json
```

This requires adding a `Lint` variant to the existing `ChunkAction` enum.

---

## 5. Files Modified / Created

### New Files
- `crates/aif-core/src/infer.rs` — semantic inference engine + 8 pattern rules + tests
- `benchmarks/roundtrip_benchmark.py` — roundtrip quality benchmark script

### Modified Files
- `crates/aif-core/src/lib.rs` — add `pub mod infer;`
- `crates/aif-core/src/lint.rs` — add `ChunkLintCheck` enum, `lint_chunk_graph()`, 3 graph checks
- `crates/aif-cli/src/main.rs` — add `--infer-semantics` flag to `import`, add `chunk lint` subcommand
- `CLAUDE.md` — document Phase 8 features

### Unchanged
- All emitters (HTML, MD, LML, binary, PDF) — inference runs on the AST before compilation, emitters don't need changes
- `aif-html/src/importer.rs`, `aif-markdown/src/importer.rs` — importers are unchanged; inference is called after import in the CLI

---

## 6. Future Extension: LLM Inference (Option C)

The `InferStrategy::Llm` variant and `LlmInferRule` trait impl are intentionally left as stubs. When activated:

1. Pattern rules run first (cheap, fast)
2. Blocks that matched no pattern OR matched with confidence < 0.5 are batched
3. Batched blocks are sent to the configured LLM (via existing `aif-core::config::LlmConfig`)
4. LLM returns `(SemanticBlockType, confidence)` per block
5. Results are merged with pattern results, highest confidence wins

This requires no architectural changes — just a new `InferRule` implementation.

---

## 7. Success Criteria

- `aif import doc.md --infer-semantics` upgrades blockquotes and paragraphs to typed semantic blocks with confidence metadata
- `aif lint` validates inferred `refs` targets and flags low-confidence inferences
- Roundtrip benchmark shows >=90% block type preservation for HTML and JSON paths, >=80% for Markdown
- Chunk graph lint catches orphaned chunks, missing continuation links, and dependency cycles
- All existing tests continue to pass (inference is opt-in, no behavior change without flag)
