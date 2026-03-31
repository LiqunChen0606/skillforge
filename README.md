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
- **8 output formats** — HTML, Markdown, LML (5 modes), JSON IR, binary wire, binary token-optimized
- **Markdown import** — convert existing `.md` files to AIF semantic IR
- **Integrity verification** — SHA-256 content hashing for skill blocks
- **Skill versioning** — semver parsing, semantic diff, auto-bump with change classification
- **Binary serialization** — wire format (postcard) for tool-to-tool transfer, token-optimized for compact storage
- **Token efficient** — LML Aggressive matches SKILL.md baseline with 100% semantic compliance
- **Bidirectional LML** — parse LML aggressive-mode back to AST for full roundtrip
- **JSON Schema** — cross-language SDK support via generated JSON Schema
- **Skill registry** — local file-based registry for skill lookup by name, version, or hash
- **Delta transport** — incremental binary diff encoding for efficient skill updates
- **Format recommender** — analyzes document structure to suggest optimal output format
- **Semantic compression** — text deduplication dictionary for repeated content
- **Hybrid format** — LML text with base64-encoded binary content blocks

## Quick Start

```bash
# Build from source
cargo build --workspace

# Compile an AIF document to HTML
cargo run -p aif-cli -- compile doc.aif --format html

# Compile to LLM-optimized format (5 verbosity modes)
cargo run -p aif-cli -- compile doc.aif --format lml-aggressive

# Compile to compact binary
cargo run -p aif-cli -- compile doc.aif --format binary-wire

# Import Markdown to AIF IR
cargo run -p aif-cli -- import doc.md

# Dump semantic IR as JSON
cargo run -p aif-cli -- dump-ir doc.aif
```

## Architecture

```
.aif source ──→ Parser ──→ Semantic IR (typed AST) ──→ Output Formats
                                  │
                    ┌─────────────┼─────────────────┐
                    │             │                  │
                    ▼             ▼                  ▼
              Human-readable   LLM-optimized    Machine-optimized
              (HTML, Markdown)  (LML modes)     (JSON, Binary)
```

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `aif-core` | Shared AST types, spans, errors |
| `aif-parser` | Logos-based lexer + parser (`.aif` → AST) |
| `aif-html` | HTML compiler |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | LML compiler — 5 prose modes, bidirectional parser, hybrid format, semantic compression |
| `aif-binary` | Binary serialization — wire (postcard) and token-optimized with full encode/decode roundtrip |
| `aif-skill` | Skill profiles — validation, hashing, versioning, diff, registry, delta transport, format recommender |
| `aif-cli` | CLI tool: `compile`, `import`, `dump-ir`, `skill`, `schema` subcommands |

## Benchmark Results

Benchmarked across 10 real-world AI skill documents using Claude API token counting (claude-opus-4-6, 2026-03-31).

### Token Efficiency (LLM Context Windows)

| Format | Total Tokens | vs SKILL.md Baseline | Compliance | TNO |
|--------|-------------|---------------------|------------|-----|
| **Markdown (roundtrip)** | **38.8K** | **+1.9% saved** | — | — |
| SKILL.md (baseline) | 39.5K | — | — | — |
| **LML Aggressive** | **39.5K** | **~0%** | **100%** | **0.99** |
| LML Compact | 40.6K | -2.7% | 100% | 0.98 |
| LML Standard | 40.8K | -3.3% | 100% | 0.94 |
| HTML | 44.4K | -12.5% | — | — |
| JSON IR | 71.6K | -81.3% | — | — |

> **TNO** = Token-Normalized Output quality (1.0 = perfect). Measures semantic compliance per token spent.

### Byte Efficiency (Wire Transport)

| Format | Bytes | vs JSON IR |
|--------|-------|-----------|
| Binary wire (postcard) | ~1.4 KB | **-82%** |
| Binary token-optimized | ~1.4 KB | **-82%** |
| JSON IR | ~7.7 KB | baseline |

> Binary formats are compact in bytes but inflate under base64 encoding for LLM token counting. Use binary for tool-to-tool transfer, not LLM context windows.

### Key Takeaways

1. **For LLM context windows:** LML Aggressive is the sweet spot — matches Markdown token count with 100% semantic compliance and 0.99 TNO. Markdown roundtrip saves 1.9% more tokens but loses type information.

2. **For wire transport:** Binary wire format (postcard) is 82% smaller than JSON, ideal for tool-to-tool pipelines and bulk storage.

3. **Avoid for LLM context:** HTML (+12.5%), JSON (+81%), and binary formats (base64 inflation) all waste tokens.

4. **Semantic tags are nearly free:** LML Aggressive proves that semantic structure doesn't have to cost tokens — the right tag design (`@step:`, `@verify:`) adds negligible overhead versus unstructured Markdown.

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
```

### Change Classification

The diff engine classifies changes for automatic semver bumping:

| Classification | Examples | Semver Impact |
|---------------|----------|---------------|
| **Breaking** | Removed step, changed precondition | Major |
| **Additive** | New step, new example, new fallback | Minor |
| **Cosmetic** | Rewording text, reordering within block | Patch |

## LML Prose Modes

Five verbosity levels for different token budgets:

| Mode | Tag Style | Use Case |
|------|-----------|----------|
| Standard | `[STEP]...[/STEP]` | Full semantic tags, maximum clarity |
| Compact | Standard minus `@example` | Reduced token count |
| Conservative | `[ST]`, `[VER]` + legend | Abbreviated tags |
| Moderate | Conservative + flatten | Fewer structural wrappers |
| Aggressive | `@step:`, `@verify:` | Minimal delimiters, best token efficiency |

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
cargo test --workspace         # Run all ~230 tests
cargo run -p aif-cli -- --help # CLI usage
```

## License

Apache-2.0 OR MIT (dual-licensed)
