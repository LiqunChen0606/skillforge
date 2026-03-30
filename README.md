# AIF: AI-native Interchange Format

A semantic document format and toolchain for humans and LLMs: concise like Markdown, typed like XML/JATS, renderable like HTML.

## Features

- **Semantic blocks** — typed content like `@claim`, `@evidence`, `@definition`, `@theorem`
- **Skill profiles** — structured AI skill representation with `@skill`, `@step`, `@verify`, `@precondition`
- **Multiple outputs** — compile to HTML, Markdown, LML (LLM-optimized), or JSON IR
- **Markdown import** — convert existing `.md` files to AIF semantic IR
- **Integrity verification** — SHA-256 content hashing for skill blocks
- **Token efficient** — ~31% fewer tokens than HTML for equivalent content

## Quick Start

```bash
# Build from source
cargo build --workspace

# Compile an AIF document to HTML
cargo run -p aif-cli -- compile doc.aif --format html

# Compile to Markdown
cargo run -p aif-cli -- compile doc.aif --format markdown

# Compile to LLM-optimized view
cargo run -p aif-cli -- compile doc.aif --format lml

# Dump semantic IR as JSON
cargo run -p aif-cli -- dump-ir doc.aif

# Import Markdown to AIF IR
cargo run -p aif-cli -- import doc.md
```

## Skill Profiles

AIF supports structured skill documents for AI agents with validation, hashing, and lazy loading:

```aif
@skill[name="debugging", version="1.0", priority="high"]
  @precondition
    User has reported a bug or test failure.
  @end

  @step[order=1]
    Reproduce the issue with a minimal test case.
  @end

  @step[order=2]
    Identify the root cause.
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
# Import a SKILL.md into AIF
cargo run -p aif-cli -- skill import skill.md -o skill.json

# Export AIF skill back to Markdown
cargo run -p aif-cli -- skill export skill.aif -o skill.md

# Verify integrity hash
cargo run -p aif-cli -- skill verify skill.aif

# Recompute hash
cargo run -p aif-cli -- skill rehash skill.aif

# Inspect skill metadata
cargo run -p aif-cli -- skill inspect skill.aif
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

## Architecture

| Crate | Purpose |
|-------|---------|
| `aif-core` | Shared AST types, spans, errors |
| `aif-parser` | Lexer + parser (`.aif` → AST) |
| `aif-html` | HTML compiler |
| `aif-markdown` | Markdown compiler + importer |
| `aif-lml` | LML compiler (LLM-optimized) |
| `aif-skill` | Skill validation, hashing, import/export, manifest |
| `aif-cli` | Command-line interface |

## Token Efficiency

Benchmarked across 10 Wikipedia articles (Claude API token counting):

| Format | vs HTML Baseline |
|--------|-----------------|
| AIF Source | -31.3% |
| LML | -35.4% |
| Markdown | -77.1% |
| AIF JSON IR | -18.7% |

## License

Apache-2.0 OR MIT (dual-licensed)
