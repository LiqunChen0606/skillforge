# AIF — AI-native Interchange Format

## Project Overview

AIF is a semantic document format and toolchain (**SkillForge**) for humans and LLMs. Concise like Markdown, typed like XML/JATS, renderable like HTML. Written in Rust. Two core capabilities: (1) rigorous skill formatting/generation with typed blocks, versioning, and eval pipelines; (2) document cleaning and normalization (HTML, PDF, Markdown → typed semantic IR) for token-efficient LLM consumption.

## Architecture

**Two-layer design:** Surface syntax (`.aif` files) → Semantic IR (typed AST) → Output formats (HTML, Markdown, LML, JSON).

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `aif-core` | AST types, spans, errors, JSON Schema generation, shared `inlines_to_text` utility — shared IR |
| `aif-parser` | Logos-based lexer + block/inline parser (`.aif` → AST) |
| `aif-html` | HTML compiler (AST → HTML) + importer (HTML → AST) with AIF-roundtrip and generic modes, readability extraction |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | LML compiler — 5 prose modes, bidirectional parser, hybrid LML+binary, semantic compression |
| `aif-binary` | Binary serialization — wire (postcard) and token-optimized formats with full encode/decode roundtrip, media metadata, semantic/callout type preservation |
| `aif-skill` | Skill profiles — validation, hashing, versioning, diff, registry, delta transport, format recommender, chaining, marketplace |
| `aif-pdf` | PDF export (krilla) + import (pdf_oxide) + document chunking (4 strategies) + chunk graphs |
| `aif-eval` | Eval pipeline — Anthropic LLM client, behavioral compliance, scenario tests, pipeline orchestrator |
| `aif-migrate` | Migration engine — chunked pipeline, repair loops, static+LLM verification, AIF report generation |
| `aif-cli` | CLI tool: `compile`, `import`, `dump-ir`, `skill`, `schema`, `chunk`, `config`, `migrate` subcommands |

### Key Types

- `Document` — top-level: metadata + blocks
- `Block` / `BlockKind` — paragraphs, sections, semantic blocks, callouts, tables, figures, code, lists, skills
- `Inline` — text, emphasis, strong, code, links, references, footnotes
- `SkillBlockType` — step, verify, precondition, output_contract, decision, tool, fallback, red_flag, example
- `MediaMeta` — optional metadata on Figure/Audio/Video: alt, width, height, duration, mime, poster
- `Attrs` — id + key-value pairs on any block
- `TextMode` — Plain, Markdown, Render modes for `inlines_to_text` (in `aif-core::text`)
- `ChunkGraph` / `Chunk` / `ChunkId` — sub-document addressing and cross-document linking
- `EvalReport` / `StageResult` / `ScenarioResult` — eval pipeline results (in `aif-skill::eval`)
- `LintCheck` / `LintResult` — structural lint checks (in `aif-skill::lint`)
- `LlmConfig` / `AifConfig` — LLM provider and project configuration (in `aif-core::config`)
- `MigrationConfig` / `ChunkResult` / `MigrationReport` — migration pipeline types (in `aif-migrate::types`); `MigrationConfig` includes `chunk_strategy` and `dry_run` fields (unified from former `EngineConfig`)
- `SourceChunk` / `ChunkStrategy` — source file chunking (in `aif-migrate::chunk`); `SourceChunk` includes `warnings: Vec<String>` for oversized-chunk diagnostics
- `MigrationEngine` — pipeline orchestrator with `run()` method: validate → chunk → apply → verify → repair → report (in `aif-migrate::engine`)
- `StaticCheckSpec` — pattern-based static verification with regex matching for both presence and absence checks (in `aif-migrate::verify`)
- `HtmlImportResult` / `ImportMode` — HTML import result with AIF-roundtrip vs generic mode detection (in `aif-html::importer`)

## Build & Test

```bash
cargo build --workspace        # Build all
cargo test --workspace         # Run all tests
cargo run -p aif-cli -- --help # CLI usage
```

## Conventions

- MIT + Apache 2.0 dual license (Rust ecosystem convention)
- TDD: write failing test → implement → verify
- Each crate has `tests/` directory with integration tests
- Test fixtures live in `tests/fixtures/`
- Benchmarks in `benchmarks/`
- Example documents in `examples/`

## AIF Syntax Quick Reference

```aif
#title: Document Title
#author: Name

@section[id=intro]: Introduction
  Paragraph with **bold**, *italic*, `code`.

  @claim
    A typed semantic block.
  @end

  @callout[type=note]
    An informational callout.
  @end
@end

@skill[name="debugging", version="1.0"]
  @precondition
    When to apply this skill.
  @end

  @step[order=1]
    First step.
  @end

  @verify
    How to validate.
  @end
@end
```

## LML Prose Modes

| Mode | Tag Style | Use Case |
|------|-----------|----------|
| Standard | `[STEP]...[/STEP]` | Full semantic tags |
| Compact | Standard minus `@example` blocks | Reduced token count |
| Conservative | `[ST]`, `[VER]`, `[PRE]` + legend | Abbreviated tags |
| Moderate | Conservative + drop single-child wrappers | Fewer structural tags |
| Aggressive | `@step:`, `@verify:`, `@pre:` | Markdown-like, minimal delimiters |

## CLI Commands

```bash
# Document compilation
aif compile input.aif -f html|markdown|lml|lml-compact|lml-conservative|lml-moderate|lml-aggressive|json|binary-wire|binary-token|pdf [-o output]
aif compile --input-format json input.json -f html|lml-aggressive|...  # Compile from JSON IR
aif import input.md|input.html|input.pdf [-o output] [--strip-chrome]
aif dump-ir input.aif
aif schema                     # Generate JSON Schema for AIF Document type

# Skill operations
aif skill import input.md [-f json|html|markdown|lml|lml-compact|lml-conservative|lml-moderate|lml-aggressive|binary-wire|binary-token] [-o output]
aif skill export input.aif [-o output.md]
aif skill verify input.aif
aif skill rehash input.aif
aif skill inspect input.aif
aif skill diff old.aif new.aif [--format text|json]
aif skill bump input.aif [--dry-run]

# Skill chaining & marketplace
aif skill deps input.aif                    # Show skill dependencies
aif skill chain input.aif                   # Resolve execution order
aif skill compose input.aif [-o output]     # Compose dependency chain
aif skill search "query" [--tags t1,t2]     # Search remote registry
aif skill publish input.aif                 # Publish to remote registry
aif skill install name [--version v]        # Install from remote
aif skill info name [--version v]           # Show remote metadata

# Document chunking
aif chunk split input.aif --strategy section|token-budget|semantic|fixed-blocks [--max-tokens N] [-o dir]
aif chunk graph input1.aif input2.aif [-o graph.json]

# Eval pipeline
aif skill eval <skill.aif> [--stage 1|2|3] [--report text|json]

# Migration
aif migrate validate input.aif                # Validate migration skill profile
aif migrate run --skill s.aif --source ./src --output ./migrated [--strategy file|directory|token-budget] [--max-repairs 3] [--report text|json]

# Configuration
aif config set llm.provider <provider>   # anthropic, openai, google, local
aif config set llm.api-key <key>
aif config set llm.model <model>
aif config list

# Benchmarks
python benchmarks/skill_token_benchmark.py  # Skill format comparison (requires ANTHROPIC_API_KEY)
python benchmarks/token_benchmark.py        # Document format comparison: raw HTML/PDF/MD vs AIF formats (Wikipedia articles)
```

## Phase 2 Features

### Token-Optimized Binary Roundtrip (Task 1)
Full `encode` + `decode` for the custom binary format. All block/inline types roundtrip correctly.

### LML+Binary Hybrid (Task 2)
`crates/aif-lml/src/hybrid.rs` — LML text with base64-encoded binary content blocks for mixed human/machine consumption.

### Bidirectional LML Parser (Task 3)
`crates/aif-lml/src/parser.rs` — Parses LML aggressive-mode back into the AST. Enables full LML roundtrip: AST → LML → AST.

### JSON Schema / Cross-Language SDK (Task 4)
`crates/aif-core/src/schema.rs` — Generates JSON Schema from AST types via `schemars`. CLI: `aif schema`.

### Incremental Diff Transport (Task 5)
`crates/aif-skill/src/delta.rs` — Binary delta encoding for skill updates. Encodes only changed blocks for efficient wire transport.

### Skill Registry (Task 6)
`crates/aif-skill/src/registry.rs` — Local file-based registry for skill lookup by name, version, or content hash.

### Compliance Benchmarks (Task 7)
Extended `benchmarks/skill_token_benchmark.py` with HTML, Markdown, and JSON compliance patterns.

### Format Recommender (Task 8)
`crates/aif-skill/src/recommend.rs` — Analyzes document structure to recommend optimal output format.

### Semantic Compression (Task 9)
`crates/aif-lml/src/compress.rs` — Text deduplication dictionary for repeated content across blocks.

## Phase 3 Features

### PDF Support
`crates/aif-pdf/` — PDF export via krilla (text rendering, word-wrap, pagination) and import via pdf_oxide (text extraction, paragraph splitting, heading/code classification). Confidence scores are documented constants: HEADING=0.65, CODE=0.55, PARAGRAPH=0.80. Feature-flagged: `import`, `export`.

### Document Chunking
`crates/aif-pdf/src/chunk/` — 4 chunking strategies (Section, TokenBudget, Semantic, FixedBlocks) with deterministic content-addressable ChunkIds. Cross-document chunk graphs with typed edges (Evidence, Dependency, Continuation, CrossReference, Refutation). Token estimation uses BPE_TOKENS_PER_WORD constant (1.3x multiplier).

### Skill Chaining
`crates/aif-skill/src/chain.rs` — Dependency declaration via `requires` attribute with semver version constraints. Kahn's algorithm for topological sort with cycle detection and version conflict reporting. `resolve_chain()` loads skill files from registry to extract transitive dependencies. CLI `chain` and `compose` commands resolve against local registry (~/.aif/registry.json).

### Skill Marketplace
`crates/aif-skill/src/remote.rs` + `resolver.rs` — Remote registry client with REST API protocol (stub). Unified resolver: local → cache → remote with version-aware matching. Download validation ensures fetched skills are valid AIF documents before caching. Version parsing errors are surfaced (not silently defaulted).

### Rich Media Metadata
`MediaMeta` struct on Figure, Audio, and Video blocks with 6 optional fields: `alt`, `width`, `height`, `duration`, `mime`, `poster`. Propagated across all emitters (HTML, Markdown, LML standard/aggressive, binary wire/token-opt, PDF, skill export/hash, compression, hybrid). Binary uses presence-flags byte (6 bits) encoding only non-None fields. LML aggressive uses abbreviated keys (`w=`, `h=`). JSON uses `skip_serializing_if` to omit null fields.

### Binary Type Preservation
Token-optimized binary format now encodes and decodes `SemanticBlockType` (9 variants) and `CalloutType` (4 variants) as single-byte IDs, enabling lossless roundtrip for all semantic block and callout types.

### Cross-Language SDKs
`sdks/python/` — Pydantic v2 models with Literal discriminators and StrEnum for tagged unions.
`sdks/typescript/` — TypeScript interfaces + Zod schemas (z.discriminatedUnion, z.lazy for recursive types).
`scripts/generate_sdks.py` — Codegen from JSON Schema with `--check` mode for CI validation.

## Phase 4 Features

### Skill Eval Pipeline
`crates/aif-eval/` — Three-stage quality pipeline for coding-agent skills. Stage 1: Structural lint (7 deterministic checks, no LLM). Stage 2: Behavioral compliance (LLM simulates agent with skill, checks 3 default rules). Stage 3: Effectiveness eval (scenario tests extracted from @verify blocks). Pipeline orchestrator stops on first stage failure. MVP supports Anthropic as LLM provider.

### LLM Configuration
`~/.aif/config.toml` with `[llm]` section: provider, api_key, model, base_url. Environment variable overrides: AIF_LLM_PROVIDER, AIF_LLM_API_KEY, AIF_LLM_MODEL. CLI: `aif config set/list`.

## Phase 5 Features

### Migration Skill System
`crates/aif-migrate/` — Chunked migration engine that applies typed migration skills to codebases. Migration skills use `profile=migration` attribute on `@skill` blocks with required `@precondition`, `@step`, `@verify`, and `@output_contract` blocks. Full `MigrationEngine::run()` orchestration: validate skill → chunk source files → apply per-chunk with LLM callback → verify (static regex + semantic) → repair loop → generate AIF report. Three chunking strategies: FilePerChunk, DirectoryChunk, TokenBudget (with oversized-chunk warnings). Unified `MigrationConfig` (no separate `EngineConfig`). Multi-code-block extraction from LLM responses. Regex-based pattern matching for both presence and absence verification checks with explicit invalid-regex error reporting. CLI: `aif migrate validate` and `aif migrate run`.

### Enhanced Migration Reports
AIF report generation includes 7 rich sections: Executive Summary (success/partial/failed/skipped counts, confidence level), Risk Assessment (Low/Medium/High/Critical with interpretation), Verification Analysis (static + semantic check pass rates with per-check details), Results by Chunk (individual `[PASS]`/`[FAIL]` per check), Failure Analysis (recurring patterns, repair exhaustion warnings), Manual Review/Unresolved, and Recommendations (actionable next steps tiered by success rate). Helper methods on `MigrationReport`: `status_counts()`, `failed_static_checks()`, `failed_semantic_checks()`, `total_repair_iterations()`, `average_confidence()`, `confidence_label()`, `risk_level()`.

### Example Migration Skills
Three production-quality examples in `examples/`: `migration_nextjs_13_to_15.aif` (Next.js 13→15, 7 steps — async request APIs, caching, React 19), `migration_eslint_flat_config.aif` (ESLint legacy→flat config, 7 steps — plugin migration, FlatCompat), `migration_typescript_strict.aif` (TypeScript strict mode, 8 steps — phased rollout). All include `@red_flag`, `@example`, and `@decision` blocks for common pitfalls and migration choices.

## Phase 6 Features

### HTML Import
`crates/aif-html/src/importer.rs` — Two-layer HTML importer. Layer 1 (AIF roundtrip): detects `aif-*` CSS classes on `<div>`, `<aside>`, `<a>`, `<sup>` elements to reconstruct exact AST types — lossless roundtrip for AIF-emitted HTML. Layer 2 (generic): maps standard HTML tags to AIF blocks (`<p>` → Paragraph, `<section>/<h*>` → Section, `<pre><code>` → CodeBlock, `<table>` → Table, `<figure>/<audio>/<video>` → media blocks with MediaMeta, `<ul>/<ol>` → List, `<blockquote>` → BlockQuote). Auto-detects mode based on presence of `aif-*` classes. Extracts `<title>` and `<meta description>` as document metadata. Bare headings are grouped with following siblings into synthetic Section blocks.

### Readability Extraction
`crates/aif-html/src/readability.rs` — Opt-in `--strip-chrome` flag for importing full web pages. Prioritizes content roots: `<article>` → `<main>` → `[role="main"]` → `<body>` with chrome tag filtering. Chrome tags (nav, header, footer, non-AIF aside) are stripped. Tag-based heuristic, not full Mozilla Readability.

## Table Support — AIF Advantage

AIF provides **full semantic table roundtrip** across all major formats — a key differentiator vs. raw Markdown/HTML:

| Format | Headers | Rows | Caption | Inline Formatting | Roundtrip |
|--------|---------|------|---------|-------------------|-----------|
| AIF Parser | ✓ | ✓ | ✓ | ✓ | ✓ |
| HTML Emit/Import | ✓ `<thead>`/`<tbody>` | ✓ | ✓ `<caption>` | ✓ | ✓ Lossless |
| Markdown Emit/Import | ✓ GFM | ✓ | — (no MD standard) | ✓ | ✓ Structure preserved |
| LML Standard | ✓ `[TABLE]...[/TABLE]` | ✓ | ✓ | ✓ | ✓ |
| LML Aggressive | ✓ `@table:` | ✓ | ✓ | ✓ | ✓ Bidirectional |
| LML Compressed | ✓ + dedup | ✓ + dedup | ✓ | ✓ | ✓ |
| LML Hybrid | ✓ | ✓ | ✓ | ✓ | ✓ |
| Binary Wire | ✓ | ✓ | ✓ | ✓ | ✓ Lossless |
| Binary Token-Opt | ✓ | ✓ | ✓ | ✓ | ✓ Lossless |
| PDF Export | Text-only | Text-only | — | ✓ | — |

**vs. competitors:** Raw Markdown loses captions and can't roundtrip complex tables. Raw HTML carries presentational bloat. JSON IR inflates token count 2-4x. AIF LML Aggressive preserves full table semantics at ~82% fewer tokens than HTML.

## Phase 7 Features

### Document-Level Semantic Linting
`crates/aif-core/src/lint.rs` — 9 structural checks for any AIF document (not just skills): ClaimsWithoutEvidence, BrokenReferences, BrokenEvidenceLinks, OrphanedMedia, DuplicateIds, EmptySections (nested only), MissingMetadata, EmptyFootnotes, MalformedTables. `DocLintResult` includes optional `block_id` for precise error location. CLI: `aif lint doc.aif [--format text|json]`.

### Evidence Linkage
Claim-to-evidence linking via `refs` attribute on any block: `@claim[id=c1, refs=e1,e2]` declares that claim `c1` is supported by evidence blocks `e1` and `e2`. Comma-separated target IDs. The `BrokenEvidenceLinks` lint check validates all ref targets exist. Works on any block type with `Attrs` (semantic blocks, figures, tables, sections).

### Import Provenance
All importers now set `_aif_source_format` ("html", "markdown", "pdf") and `_aif_import_mode` ("aif-roundtrip", "generic", "readability") in document metadata. CLI additionally sets `_aif_source_file` (original path) and `_aif_import_confidence` (PDF only, avg confidence score). Metadata keys prefixed with `_aif_` are reserved for provenance.

### Chunk Graph Enrichment
`ChunkMetadata` extended with: `summary` (optional auto-generated text), `requires_parent_context` (bool), `semantic_types` (list of semantic block types in chunk). New `LinkType::ParentContext` variant for "must read before" relationships. `ChunkGraph::required_context(id)` follows ParentContext + Dependency links transitively. `ChunkGraph::chunks_for_doc(path)` returns ordered chunks for a document.

### Chunking Quality Benchmark
`benchmarks/chunking_quality_benchmark.py` — evaluates 4 chunking strategies across all example documents. Metrics: self-containment (% chunks with titles), token budget compliance, size coefficient of variation, blocks per chunk. Outputs text summary + JSON.

### Cross-Benchmark Dashboard
`benchmarks/index.html` — unified landing page linking both document and skill benchmark reports with executive summary, side-by-side comparison, AIF advantage visualization, and format decision matrix.

### General-Purpose Skill Example
`examples/code_review.aif` — code review skill showcasing all AIF skill block types: @precondition, @step (4), @verify, @fallback, @example (2 with before/after code), @decision, @red_flag, @output_contract, @tool.

## Phase 8 Features

### Semantic Inference Engine
`crates/aif-core/src/infer.rs` — Pattern-based inference upgrades untyped blocks (Paragraph, BlockQuote, Callout) to typed SemanticBlocks with confidence scoring. 8 rules: blockquote-with-citation→Evidence (0.70), short-blockquote→Claim (0.55), paragraph-definition (0.80), recommendation (0.75), conclusion (0.75), assumption (0.70), result (0.65), callout-requirement (0.60). `InferRule` trait extensible for future LLM inference. Inferred blocks get `_aif_inferred`, `_aif_confidence`, `_aif_infer_rule` metadata. CLI: `aif import doc.md --infer-semantics`.

### Roundtrip Quality Benchmark
`benchmarks/roundtrip_benchmark.py` — Measures AIF→HTML→AIF, AIF→Markdown→AIF, AIF→JSON→AIF fidelity. 5 metrics: block count ratio, block type preservation, semantic type preservation (2x weight), metadata preservation, inline fidelity. Results: JSON path 1.00 (lossless), HTML 0.93, Markdown 0.57.

### Chunk Graph Lint
`lint_chunk_graph()` in `aif-core::lint` — 3 structural checks on chunk graphs: OrphanedChunks (isolated nodes in multi-chunk docs), MissingContinuation (sequential chunks without Continuation link), DependencyCycle (circular Dependency/ParentContext links via 3-color DFS). CLI: `aif chunk lint graph.json [--format text|json]`.

### AIF Plugin Skills
`plugins/` — 6 claude-code plugins re-expressed in AIF format: code-review, security-guidance, feature-dev, frontend-design, commit-commands, claude-opus-4-5-migration. Demonstrates AIF skill syntax with typed blocks (@step, @verify, @example, @red_flag, @decision, @fallback, @output_contract).

## Known Limitations

- Markdown emit drops table captions (no GFM standard for captions)
- LML aggressive mode does not emit `mime` for media blocks (derivable from `src` extension); parser handles it if present
- HTML generic import maps `<div>` containers to flat block lists (no generic div-to-section heuristic)
- Readability extraction (`--strip-chrome`) uses tag-based heuristics, not full Mozilla Readability algorithm
- No column alignment, colspan, or rowspan support (tables must be rectangular grids)
- PDF table export renders as text-only (no grid/borders)

## Recent Fixes

- **`inlines_to_text` consolidation** — unified into `aif-core::text` with `TextMode` enum (Plain, Markdown, Render); all 3 former call sites delegate to the shared implementation
- **LML media roundtrip** — bidirectional parser now round-trips Figure/Audio/Video blocks with full MediaMeta attributes
- **Binary type roundtrip coverage** — all 9 `SemanticBlockType` and 4 `CalloutType` variants now have explicit roundtrip tests
- **Migration engine hardening** (Phase 5):
  - Symmetric regex matching — both `PatternPresence` and `PatternAbsence` use `Regex::is_match()`
  - Explicit invalid-regex errors — bad patterns fail the check with clear message instead of silent fallback
  - Robust negation heuristic — word-boundary regex (`\bno\s`, `\bnot\s`, `must not`, `cannot`, `never`, `removed`, `forbidden`, `absent`, `without`, `eliminated`) replaces naive substring check
  - Oversized-chunk warnings — `SourceChunk` emits warning when a single file exceeds token budget
  - Full `MigrationEngine::run()` — orchestrates validate → chunk → apply → verify → repair → report with pluggable `apply_fn` callback
  - Multi-code-block extraction — `parse_migration_response` joins all code blocks, not just first
  - Unified config — merged `EngineConfig` into `MigrationConfig`
  - Distinct risk callouts — Low→Note, Medium→Info, High→Warning, Critical→Tip
  - Removed unused `reqwest`/`tokio` dependencies
- **Code quality sweep** (Phase 6):
  - HTML importer: AIF roundtrip detection uses `class="aif-` prefix instead of bare `aif-` substring to prevent false positives
  - Readability: `OnceLock`-cached static CSS selectors replace scattered `Selector::parse().unwrap()` calls
  - Removed dead code: unused `MdSection.level` field, unreachable `"aif"` match arm in CLI
  - 6 Clippy fixes: redundant closures → method references, derivable Default, useless `format!`, `vec![]` macro, `.to_vec()`
  - Renamed `LlmProvider::from_str` → `parse_provider` to avoid `FromStr` trait conflict
  - Replaced `write!().unwrap()` with `push_str(&format!())` in hybrid LML emitter
- **Enhanced benchmark reports**:
  - Skill report: executive summary cards, cost impact at 3 Claude pricing tiers, statistical analysis (mean/min/max/stddev), compliance/TNO heatmap, format recommendation matrix, binary formats separated to fix chart scale
  - Document report: executive summary (82.2% savings, $68 saved per 10 articles), cost & latency impact table, scale impact projections, Information Density Score analysis, variance analysis
  - Benchmark Python script: `compute_statistics()` and `compute_cost_impact()` functions, extended JSON output
- **Example migration reports**: 3 HTML templates in `examples/` — NextJS (success), ESLint (clean success), TypeScript strict (partial success with deferred files)
- **Full table support across all LML modes** — emitter now outputs pipe-delimited headers, separator, and data rows for Standard, Aggressive, Compressed, and Hybrid modes (previously only caption was preserved)
- **LML table parser** — bidirectional parser handles `@table:` blocks with caption, headers, separator, and data rows
- **Markdown table import** — enabled `ENABLE_TABLES` in pulldown_cmark, added `TableAccumulator` for header/row/cell tracking; GFM tables now import with full structure
- **Semantic compression includes table cells** — `collect_text_occurrences()` and `emit_block_compressed()` now process header and row cell content, not just captions
- **Hybrid LML table data** — hybrid emitter includes full pipe-delimited table rows instead of caption-only
- **CLI migrate run wired to engine** — reads skill file, chunks source directory, runs `MigrationEngine::run()`, outputs text/JSON reports; placeholder apply_fn when no LLM key configured
- **Migration examples enriched** — all 3 examples now include `@example` and `@decision` blocks with concrete before/after code and migration decision criteria

## Benchmark Results (2026-03-31, claude-opus-4-6, 10 skills)

| Format | Total Tokens | vs SKILL.md | Compliance | TNO |
|--------|-------------|-------------|------------|-----|
| SKILL.md (baseline) | 39.5K | — | — | — |
| Markdown (roundtrip) | 38.8K | +1.9% saved | — | — |
| LML Aggressive | 39.5K | ~0% | 100% | 0.99 |
| LML Compact | 40.6K | -2.7% | 100% | 0.98 |
| LML Standard | 40.8K | -3.3% | 100% | 0.94 |
| HTML | 44.4K | -12.5% | — | — |
| JSON IR | 71.6K | -81.3% | — | — |
| Binary Wire | 179.3K | -353.9%* | 100% | — |
| Binary Token | 179.3K | -353.8%* | 100% | — |

\* Binary formats are compact in bytes (~82% smaller than JSON) but inflate when base64-encoded for token counting. Use binary for wire transport, not LLM context.

Full HTML report: `benchmarks/skill_benchmark_report.html`
Raw data: `benchmarks/skill_results.json`

## Document Benchmark Results (2026-04-01, claude-opus-4-6, 10 Wikipedia articles)

Compares raw formats (HTML, PDF, Markdown) vs AIF output formats for general documents.
Baseline: Raw HTML (5.5M tokens total). Cleaned HTML text added for fair comparison.

| Format | Total Tokens | vs Raw HTML | Structure | Bytes |
|--------|-------------|-------------|-----------|-------|
| Raw HTML (baseline) | 5.5M | — | Full + chrome | 13.6M |
| Cleaned HTML text | ~600K* | ~89% saved | None | ~2M |
| Raw PDF (text) | 561.0K | +89.8% saved | None | 1.7M |
| Raw Markdown | 1.3M | +77.1% saved | Basic | 3.5M |
| **AIF LML Aggressive** | **979.8K** | **+82.2% saved** | **Full semantic** | **2.8M** |
| AIF LML Standard | 985.1K | +82.1% saved | Full semantic | 2.8M |
| AIF Markdown (RT) | 1.0M | +81.7% saved | Basic | 2.9M |
| AIF JSON IR | 4.5M | +18.7% saved | Full semantic | 18.4M |

\* Estimated; run `python benchmarks/token_benchmark.py` for exact cleaned HTML numbers.

### Key Findings

1. **Plain text extraction is cheapest but structureless** — PDF-text (561K) and cleaned HTML (~600K) strip everything to flat text. Fine for Q&A; unsuitable when the LLM needs to reason about document structure.
2. **AIF LML Aggressive is the best structured format** — 22% fewer tokens than raw Markdown with full semantic types (claims, evidence, definitions, tables). The only format that preserves typed blocks at fewer tokens than Markdown.
3. **AIF costs ~75% more than flat text extraction** — but carries typed sections, headings, claims, evidence, tables, figures, and lossless roundtrip. This is the price of structure.
4. **The "82% vs Raw HTML" stat is real but context-dependent** — raw HTML includes presentational markup, CSS classes, navigation chrome. Comparing vs cleaned HTML text or PDF-text is fairer for cost analysis. Comparing vs Markdown is fairer for structure analysis.

### Structure-per-Token (the real comparison)

| Format | Tokens | Structure | Roundtrip | Best For |
|--------|--------|-----------|-----------|----------|
| Raw PDF text | 561K | None | No | Cheap Q&A, summarization |
| Cleaned HTML text | ~600K | None | No | Fair text-only baseline |
| Raw Markdown | 1.26M | Basic | Partial | General documents |
| **AIF LML Aggressive** | **980K** | **Full semantic** | **Yes** | **Structured reasoning, agents** |
| Raw HTML | 5.5M | Full + presentational | Yes | Browser rendering |

Full HTML report: `benchmarks/results.html`
Raw data: `benchmarks/results.json`
