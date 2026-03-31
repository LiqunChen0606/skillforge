# AIF — AI-native Interchange Format

## Project Overview

AIF is a semantic document format and toolchain for humans and LLMs. Concise like Markdown, typed like XML/JATS, renderable like HTML. Written in Rust.

## Architecture

**Two-layer design:** Surface syntax (`.aif` files) → Semantic IR (typed AST) → Output formats (HTML, Markdown, LML, JSON).

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `aif-core` | AST types, spans, errors, JSON Schema generation — shared IR |
| `aif-parser` | Logos-based lexer + block/inline parser (`.aif` → AST) |
| `aif-html` | HTML compiler (AST → HTML) |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | LML compiler — 5 prose modes, bidirectional parser, hybrid LML+binary, semantic compression |
| `aif-binary` | Binary serialization — wire (postcard) and token-optimized formats with full encode/decode roundtrip |
| `aif-skill` | Skill profiles — validation, hashing, versioning, diff, registry, delta transport, format recommender |
| `aif-cli` | CLI tool: `compile`, `import`, `dump-ir`, `skill`, `schema` subcommands |

### Key Types

- `Document` — top-level: metadata + blocks
- `Block` / `BlockKind` — paragraphs, sections, semantic blocks, callouts, tables, figures, code, lists, skills
- `Inline` — text, emphasis, strong, code, links, references, footnotes
- `SkillBlockType` — step, verify, precondition, output_contract, decision, tool, fallback, red_flag, example
- `Attrs` — id + key-value pairs on any block

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
aif compile input.aif -f html|markdown|lml|lml-compact|lml-conservative|lml-moderate|lml-aggressive|json|binary-wire|binary-token [-o output]
aif import input.md [-o output]
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

# Benchmarks
python benchmarks/skill_token_benchmark.py  # Requires ANTHROPIC_API_KEY
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
