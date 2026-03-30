# AIF — AI-native Interchange Format

## Project Overview

AIF is a semantic document format and toolchain for humans and LLMs. Concise like Markdown, typed like XML/JATS, renderable like HTML. Written in Rust.

## Architecture

**Two-layer design:** Surface syntax (`.aif` files) → Semantic IR (typed AST) → Output formats (HTML, Markdown, LML, JSON).

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `aif-core` | AST types, spans, errors — shared IR |
| `aif-parser` | Logos-based lexer + block/inline parser (`.aif` → AST) |
| `aif-html` | HTML compiler (AST → HTML) |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | LML compiler — LLM-optimized tagged format |
| `aif-skill` | Skill profiles — validation, SHA-256 hashing, SKILL.md import/export, manifest |
| `aif-cli` | CLI tool: `compile`, `import`, `dump-ir`, `skill` subcommands |

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

## CLI Commands

```bash
# Document compilation
aif compile input.aif -f html|markdown|lml|json [-o output]
aif import input.md [-o output]
aif dump-ir input.aif

# Skill operations
aif skill import input.md [-o output.json]
aif skill export input.aif [-o output.md]
aif skill verify input.aif
aif skill rehash input.aif
aif skill inspect input.aif
```
