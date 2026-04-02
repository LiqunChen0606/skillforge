# AIF — AI-native Interchange Format

## Project Overview

AIF is a semantic document format and toolchain for humans and LLMs. Concise like Markdown, typed like XML/JATS, renderable like HTML. Written in Rust.

## Architecture

**Two-layer design:** Surface syntax (`.aif` files) → Semantic IR (typed AST) → Output formats (HTML, Markdown, LML, JSON).

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `aif-core` | AST types, spans, errors, JSON Schema generation, shared `inlines_to_text` utility — shared IR |
| `aif-parser` | Logos-based lexer + block/inline parser (`.aif` → AST) |
| `aif-html` | HTML compiler (AST → HTML) |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | LML compiler — 5 prose modes, bidirectional parser, hybrid LML+binary, semantic compression |
| `aif-binary` | Binary serialization — wire (postcard) and token-optimized formats with full encode/decode roundtrip, media metadata, semantic/callout type preservation |
| `aif-skill` | Skill profiles — validation, hashing, versioning, diff, registry, delta transport, format recommender, chaining, marketplace |
| `aif-pdf` | PDF export (krilla) + import (pdf_oxide) + document chunking (4 strategies) + chunk graphs |
| `aif-eval` | Eval pipeline — Anthropic LLM client, behavioral compliance, scenario tests, pipeline orchestrator |
| `aif-cli` | CLI tool: `compile`, `import`, `dump-ir`, `skill`, `schema`, `chunk` subcommands |

### Key Types

- `Document` — top-level: metadata + blocks
- `Block` / `BlockKind` — paragraphs, sections, semantic blocks, callouts, tables, figures, code, lists, skills
- `Inline` — text, emphasis, strong, code, links, references, footnotes
- `SkillBlockType` — step, verify, precondition, output_contract, decision, tool, fallback, red_flag, example
- `MediaMeta` — optional metadata on Figure/Audio/Video: alt, width, height, duration, mime, poster
- `Attrs` — id + key-value pairs on any block
- `TextMode` — Plain, Markdown, Render modes for `inlines_to_text` (in `aif-core::text`)
- `ChunkGraph` / `Chunk` / `ChunkId` — sub-document addressing and cross-document linking

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
aif import input.md|input.pdf [-o output]
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

## Known Limitations

- Markdown importer does not detect audio/video links for roundtrip fidelity
- LML aggressive mode does not emit `mime` for media blocks (derivable from `src` extension); parser handles it if present

## Recent Fixes

- **`inlines_to_text` consolidation** — unified into `aif-core::text` with `TextMode` enum (Plain, Markdown, Render); all 3 former call sites delegate to the shared implementation
- **LML media roundtrip** — bidirectional parser now round-trips Figure/Audio/Video blocks with full MediaMeta attributes
- **Binary type roundtrip coverage** — all 9 `SemanticBlockType` and 4 `CalloutType` variants now have explicit roundtrip tests

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
Baseline: Raw HTML (5.5M tokens total).

| Format | Total Tokens | vs Raw HTML | Bytes |
|--------|-------------|-------------|-------|
| Raw HTML (baseline) | 5.5M | — | 13.6M |
| Raw PDF (file) | 1.4M | +75.5% saved | 23.7M |
| Raw PDF (text) | 561.0K | +89.8% saved | 1.7M |
| Raw Markdown | 1.3M | +77.1% saved | 3.5M |
| AIF JSON IR | 4.5M | +18.7% saved | 18.4M |
| AIF HTML | 1.3M | +77.0% saved | 3.5M |
| AIF Markdown (RT) | 1.0M | +81.7% saved | 2.9M |
| AIF LML Aggressive | 979.8K | **+82.2% saved** | 2.8M |
| AIF LML Standard | 985.1K | +82.1% saved | 2.8M |

Key findings: AIF LML Aggressive is the most token-efficient format, saving **82.2%** vs raw HTML. Even raw Markdown saves 77.1%, but AIF LML adds another 5+ percentage points on top.

Full HTML report: `benchmarks/results.html`
Raw data: `benchmarks/results.json`
