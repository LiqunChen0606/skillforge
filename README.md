# AIF: AI-native Interchange Format

A semantic document format and toolchain for humans and LLMs: concise like Markdown, typed like XML/JATS, renderable like HTML. Written in Rust.

## Why AIF?

LLMs consume and produce structured documents, but existing formats force a trade-off:

- **Markdown** — concise but untyped. No semantic blocks, no machine-checkable structure.
- **HTML/XML** — typed but verbose. Tags inflate token counts 12-80%+.
- **JSON** — machine-friendly but bloated. 81% more tokens than plain text.

AIF eliminates the trade-off: typed semantic blocks with token counts matching or beating Markdown, plus multiple output formats optimized for different consumers.

## Features

- **Semantic blocks** — typed content like `@claim`, `@evidence`, `@definition`, `@theorem`
- **Skill profiles** — structured AI skill representation with `@skill`, `@step`, `@verify`, `@precondition`
- **12+ output formats** — HTML, Markdown, LML (5 modes), JSON IR, binary wire, binary token-optimized, PDF
- **Markdown & PDF import** — convert existing `.md` and `.pdf` files to AIF semantic IR
- **Integrity verification** — SHA-256 content hashing for skill blocks
- **Skill versioning** — semver parsing, semantic diff, auto-bump with change classification
- **Skill chaining** — dependency declaration with semver constraints, topological sort, cycle detection
- **Skill marketplace** — remote registry client with local/cache/remote resolution
- **Binary serialization** — wire format (postcard) for tool-to-tool transfer, token-optimized for compact storage
- **Token efficient** — LML Aggressive saves **82.2%** vs raw HTML with 100% semantic compliance
- **Bidirectional LML** — parse LML aggressive-mode back to AST for full roundtrip
- **JSON Schema** — cross-language SDK support via generated JSON Schema
- **Cross-language SDKs** — Python (Pydantic v2) and TypeScript (Zod) models generated from JSON Schema
- **Skill registry** — local file-based registry for skill lookup by name, version, or hash
- **Delta transport** — incremental binary diff encoding for efficient skill updates
- **Document chunking** — 4 strategies (section, token-budget, semantic, fixed-blocks) with cross-document chunk graphs
- **Format recommender** — analyzes document structure to suggest optimal output format
- **Semantic compression** — text deduplication dictionary for repeated content
- **Hybrid format** — LML text with base64-encoded binary content blocks
- **Skill eval pipeline** — 3-stage quality assessment: structural lint, behavioral compliance, effectiveness testing
- **Migration engine** — chunked codebase migration with typed skills, static+LLM verification, repair loops, and AIF report generation
- **HTML import** — two-layer importer: lossless AIF roundtrip via CSS class detection + generic HTML-to-AIF mapping with readability extraction
- **LLM configuration** — multi-provider support (Anthropic, OpenAI, Google, local) with `~/.aif/config.toml`

## Quick Start

```bash
# Build from source
cargo build --workspace

# Compile an AIF document to HTML
cargo run -p aif-cli -- compile doc.aif --format html

# Compile to LLM-optimized format (5 verbosity modes)
cargo run -p aif-cli -- compile doc.aif --format lml-aggressive

# Compile to PDF
cargo run -p aif-cli -- compile doc.aif --format pdf

# Compile to compact binary
cargo run -p aif-cli -- compile doc.aif --format binary-wire

# Import Markdown, HTML, or PDF to AIF IR
cargo run -p aif-cli -- import doc.md
cargo run -p aif-cli -- import doc.html
cargo run -p aif-cli -- import doc.html --strip-chrome  # Extract article content
cargo run -p aif-cli -- import doc.pdf

# Dump semantic IR as JSON
cargo run -p aif-cli -- dump-ir doc.aif

# Generate JSON Schema
cargo run -p aif-cli -- schema

# Validate and run migrations
cargo run -p aif-cli -- migrate validate skill.aif
cargo run -p aif-cli -- migrate run --skill skill.aif --source ./src --output ./migrated

# Eval pipeline for skills
cargo run -p aif-cli -- skill eval skill.aif --stage 1 --report text
```

## Architecture

```
.aif source ──→ Parser ──→ Semantic IR (typed AST) ──→ Output Formats
                                  │
                    ┌─────────────┼─────────────────┐
                    │             │                  │
                    ▼             ▼                  ▼
              Human-readable   LLM-optimized    Machine-optimized
              (HTML, Markdown,  (LML modes)     (JSON, Binary)
               PDF)
```

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `aif-core` | AST types, spans, errors, JSON Schema generation, shared `inlines_to_text` utility — shared IR |
| `aif-parser` | Logos-based lexer + parser (`.aif` → AST) |
| `aif-html` | HTML compiler (AST → HTML) + importer (HTML → AST) with AIF-roundtrip and generic modes, readability extraction |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | LML compiler — 5 prose modes, bidirectional parser, hybrid format, semantic compression |
| `aif-binary` | Binary serialization — wire (postcard) and token-optimized with full encode/decode roundtrip |
| `aif-skill` | Skill profiles — validation, hashing, versioning, diff, registry, delta transport, format recommender, chaining, marketplace |
| `aif-pdf` | PDF export (krilla) + import (pdf_oxide) + document chunking (4 strategies) + chunk graphs |
| `aif-eval` | Eval pipeline — Anthropic LLM client, behavioral compliance, scenario tests, pipeline orchestrator |
| `aif-migrate` | Migration engine — chunked pipeline, repair loops, static+LLM verification, AIF report generation |
| `aif-cli` | CLI tool: `compile`, `import`, `dump-ir`, `skill`, `schema`, `chunk`, `config`, `migrate` subcommands |

## Benchmark Results

### Document Token Efficiency (10 Wikipedia Articles)

Benchmarked with Claude API token counting (claude-opus-4-6, 2026-04-01). Baseline: Raw HTML (5.5M tokens total across 10 articles).

| Format | Total Tokens | vs Raw HTML | Bytes |
|--------|-------------|-------------|-------|
| Raw HTML (baseline) | 5,515,206 | — | 13.6M |
| Raw PDF (file) | 1,351,997 | +75.5% saved | 23.7M |
| Raw PDF (text) | 560,950 | +89.8% saved | 1.7M |
| Raw Markdown | 1,262,755 | +77.1% saved | 3.5M |
| AIF JSON IR | 4,483,743 | +18.7% saved | 18.4M |
| AIF HTML | 1,268,768 | +77.0% saved | 3.5M |
| AIF Markdown (RT) | 1,009,181 | +81.7% saved | 2.9M |
| AIF LML Standard | 985,090 | +82.1% saved | 2.8M |
| **AIF LML Aggressive** | **979,838** | **+82.2% saved** | **2.8M** |

> Full HTML report: `benchmarks/results.html` | Raw data: `benchmarks/results.json`

### Skill Token Efficiency (10 AI Skills)

Benchmarked with Claude API token counting (claude-opus-4-6, 2026-04-01). Baseline: SKILL.md (39.5K tokens total).

| Format | Total Tokens | vs SKILL.md | Compliance | TNO |
|--------|-------------|-------------|------------|-----|
| **Markdown (roundtrip)** | **38.8K** | **+1.9% saved** | 100% | 1.05 |
| SKILL.md (baseline) | 39.5K | — | — | — |
| LML Aggressive | 39.5K | ~0% | 100% | 0.99 |
| LML Compact | 40.6K | -2.7% | 100% | 0.98 |
| LML Standard | 40.8K | -3.3% | 100% | 0.94 |
| HTML | 44.4K | -12.5% | 100% | 0.82 |
| JSON IR | 71.6K | -81.3% | 100% | 0.49 |
| Binary Wire | 179.3K | -353.9%\* | — | — |
| Binary Token | 179.3K | -353.8%\* | — | — |

> \* Binary formats are compact in bytes (~82% smaller than JSON) but inflate when base64-encoded for token counting. Use binary for wire transport, not LLM context.
>
> **TNO** = Token-Normalized Output quality (1.0 = perfect). Measures semantic compliance per token spent.
>
> Full HTML report: `benchmarks/skill_benchmark_report.html` | Raw data: `benchmarks/skill_results.json`

### Key Findings

1. **Raw PDF text is cheapest — but lossy.** Raw PDF text extraction (89.8% savings) beats every format on token count, but produces flat unstructured text with no headings, sections, tables, or semantic blocks. Fine for simple Q&A; unsuitable when you need the model to reason about document structure.

2. **AIF LML Aggressive is the best structured format.** At 82.2% savings vs HTML, it's only ~75% more tokens than raw PDF text, but carries full semantic structure: typed sections, headings, claims, callouts, code blocks, tables, and figures. The LLM can navigate and reason about the document as a structured object, not a flat string.

3. **AIF beats raw Markdown by 5+ percentage points.** Raw Markdown saves 77.1%, AIF LML Aggressive pushes to 82.2%. Across 10 Wikipedia articles, that's **283K fewer tokens** (1.26M → 0.98M).

4. **Semantic tags are nearly free.** LML Aggressive proves structure doesn't cost tokens — `@step:`, `@verify:` add negligible overhead vs unstructured Markdown.

5. **For wire transport:** Binary wire format is 82% smaller in bytes than JSON — use for storage/transport, not LLM context (base64 inflates tokens).

### Structure-per-Token Comparison

| Format | Tokens | Structure | Roundtrip | Best For |
|--------|--------|-----------|-----------|----------|
| Raw PDF text | 561K | None | No | Cheap Q&A, summarization |
| Raw Markdown | 1.26M | Basic (headings, lists) | Partial | General documents |
| **AIF LML Aggressive** | **980K** | **Full semantic** | **Yes** | **Structured reasoning, agents** |
| Raw HTML | 5.5M | Full + presentational | Yes | Browser rendering |

## Skill Profiles

AIF supports structured skill documents for AI agents:

```aif
@skill[name="debugging", version="1.0", priority="high"]
  @precondition
    User has reported a bug or test failure.
  @end

  @step[order=1]
    Reproduce the issue with a minimal test case.
  @end

  @verify
    Fix resolves the issue without regressions.
  @end

  @fallback
    Escalate to user after 3 attempts.
  @end
@end
```

### Skill CLI Commands

```bash
# Import a SKILL.md into AIF (supports all output formats)
aif skill import skill.md -f lml-aggressive -o skill.lml

# Export AIF skill back to Markdown
aif skill export skill.aif -o skill.md

# Verify integrity hash
aif skill verify skill.aif

# Semantic diff between skill versions
aif skill diff old.aif new.aif --format json

# Auto-bump version based on change classification
aif skill bump skill.aif --dry-run

# Inspect skill metadata
aif skill inspect skill.aif

# Skill chaining & dependencies
aif skill deps skill.aif          # Show dependencies
aif skill chain skill.aif         # Resolve execution order
aif skill compose skill.aif       # Compose dependency chain

# Skill marketplace
aif skill search "query" --tags t1,t2   # Search remote registry
aif skill publish skill.aif             # Publish to remote
aif skill install name --version v      # Install from remote
aif skill info name                     # Show remote metadata
```

### Change Classification

The diff engine classifies changes for automatic semver bumping:

| Classification | Examples | Semver Impact |
|---------------|----------|---------------|
| **Breaking** | Removed step, changed precondition | Major |
| **Additive** | New step, new example, new fallback | Minor |
| **Cosmetic** | Rewording text, reordering within block | Patch |

## Document Chunking

Split documents into addressable chunks for RAG pipelines and sub-document referencing:

```bash
# Split by section boundaries
aif chunk split doc.aif --strategy section -o chunks/

# Split by token budget (for LLM context windows)
aif chunk split doc.aif --strategy token-budget --max-tokens 4096 -o chunks/

# Build cross-document chunk graph
aif chunk graph doc1.aif doc2.aif -o graph.json
```

Four chunking strategies: Section, TokenBudget, Semantic, FixedBlocks. Chunk graphs support typed edges: Evidence, Dependency, Continuation, CrossReference, Refutation.

## LML Prose Modes

Five verbosity levels for different token budgets:

| Mode | Tag Style | Use Case |
|------|-----------|----------|
| Standard | `[STEP]...[/STEP]` | Full semantic tags, maximum clarity |
| Compact | Standard minus `@example` | Reduced token count |
| Conservative | `[ST]`, `[VER]` + legend | Abbreviated tags |
| Moderate | Conservative + flatten | Fewer structural wrappers |
| Aggressive | `@step:`, `@verify:` | Minimal delimiters, best token efficiency |

## Cross-Language SDKs

Generated from JSON Schema for type-safe AIF document handling in any language:

- **Python** (`sdks/python/`) — Pydantic v2 models with Literal discriminators and StrEnum
- **TypeScript** (`sdks/typescript/`) — TypeScript interfaces + Zod schemas with discriminated unions

```bash
# Generate SDKs from JSON Schema
python scripts/generate_sdks.py

# Validate SDKs match current schema (CI mode)
python scripts/generate_sdks.py --check
```

## Syntax Overview

```aif
#title: My Document
#author: Author Name

@section[id=intro]: Introduction
  Paragraph with **bold**, *italic*, `code`, and [links](url).

  @claim
    A typed semantic assertion.
  @end

  @callout[type=warning]
    Important notice.
  @end
@end
```

See [docs/proposal.md](docs/proposal.md) for the full specification.

## Build & Test

```bash
cargo build --workspace        # Build all crates
cargo test --workspace         # Run all ~334 tests
cargo run -p aif-cli -- --help # CLI usage
```

## Migration Engine

AIF supports typed migration skills for automated codebase transformations:

```aif
@skill[name="nextjs-upgrade", version="1.0", profile="migration"]
  @precondition
    Next.js 13.x project with pages/ or app/ router.
  @end

  @step[order=1]
    Convert synchronous request APIs to async.
  @end

  @verify
    No remaining synchronous cookies()/headers() calls.
  @end

  @output_contract
    All files compile with Next.js 15 types.
  @end

  @red_flag
    Don't migrate dynamic route params without checking usage.
  @end
@end
```

Pipeline: validate skill → chunk source files → apply per-chunk → verify (static regex + semantic) → repair loop → generate rich AIF report with risk assessment and recommendations.

Three example migration skills included: Next.js 13→15, ESLint flat config, TypeScript strict mode.

## HTML Import

Import HTML documents to AIF semantic IR:

```bash
# Import AIF-emitted HTML (lossless roundtrip)
cargo run -p aif-cli -- import page.html

# Import generic web pages with chrome stripping
cargo run -p aif-cli -- import article.html --strip-chrome
```

Two-layer detection: AIF-roundtrip mode (via `aif-*` CSS classes) reconstructs exact AST types; generic mode maps standard HTML tags to AIF blocks. Readability extraction strips navigation, headers, footers, and sidebars.

## Design Documents

- [Skill Profile Design](docs/plans/2026-03-30-skill-profile-design.md)
- [Binary IR Versioning](docs/plans/2026-03-31-binary-ir-versioning-design.md)
- [Phase 2 Implementation Plan](docs/plans/2026-03-31-phase2-all-tasks.md)
- [PDF & Document Chunking](docs/plans/2026-03-31-pdf-chunking-design.md)
- [Skill Chaining & Marketplace](docs/plans/2026-03-31-skill-chaining-marketplace-design.md)
- [Cross-Language SDK](docs/plans/2026-03-31-multi-language-sdk-design.md)
- [Skill Eval Pipeline](docs/plans/2026-04-01-skill-eval-pipeline-design.md)
- [Migration Skill System](docs/plans/2026-04-02-migration-skill-system-design.md)
- [HTML Importer](docs/plans/2026-04-02-html-importer.md)

## License

Apache-2.0 OR MIT (dual-licensed)
