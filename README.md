# SkillForge

### Semantic Document Compiler & AI Skill Toolkit

> **SkillForge** is a Rust toolchain built on the **AIF** (AI-native Interchange Format) that compiles typed, structured documents into 12+ output formats — giving LLMs full semantic structure at fewer tokens than raw Markdown. Author skills for Claude, Codex, and Gemini. Clean and normalize HTML, PDF, and Markdown for LLM consumption. Migrate codebases with typed, verifiable migration skills.

![Language](https://img.shields.io/badge/Language-Rust-orange)
![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
![Formats](https://img.shields.io/badge/Output_Formats-12+-green)
![License](https://img.shields.io/badge/License-Apache--2.0%20%7C%20MIT-lightgrey)
![Skills](https://img.shields.io/badge/Skills-Claude%20%7C%20Codex%20%7C%20Gemini-blueviolet)

**Latest:** v0.1.0 — Semantic inference engine, LLM-assisted classification, async migration with Anthropic API, skill execution quality benchmark (LML +10pp vs Markdown), cleaned HTML baseline benchmarks.

---

## Why SkillForge?

Every document format forces a trade-off between **structure** and **token cost**:

| What you send | Tokens (10 articles) | Structure | Roundtrip |
|---------------|---------------------|-----------|-----------|
| Cleaned HTML text | **544K** | None — stripped to text | No |
| Raw PDF text | 561K | None — flat string | No |
| **AIF LML Aggressive** | **981K** | **Full semantic types** | **Yes** |
| Raw Markdown | 1,263K | Basic (headings, lists) | Partial |
| Raw HTML | 5,500K | Full + presentational bloat | Yes |

**SkillForge's LML Aggressive is the only format that preserves full semantic types at fewer tokens than Markdown.** Plain text extraction is cheapest but gives the LLM zero structure. LML costs 80% more tokens than flat text, but the LLM can reason about typed sections, claims, evidence, tables — not just content. It's **22% cheaper than Markdown** with far richer semantics.

### Does Format Actually Affect LLM Behavior?

Yes. We benchmarked skill execution across 4 formats — same skill, same task, same model:

| Format | Tokens | Overall Compliance |
|--------|--------|--------------------|
| **LML Aggressive** | **1,012** | **0.97** |
| JSON IR | 4,732 | 0.95 |
| HTML | 1,485 | 0.91 |
| Raw Markdown | 1,067 | 0.87 |

**LML Aggressive: +10 percentage points** over raw Markdown at 5% fewer tokens. The explicit typed tags (`@step:`, `@verify:`, `@red_flag:`) help the LLM identify and follow each instruction block. See [full benchmark](benchmarks/skill-execution/).

---

## Install

```bash
# From source
cargo install --path crates/aif-cli

# Then use anywhere
aif compile doc.aif --format html
aif import doc.md --infer-semantics
aif lint doc.aif
```

## Quick Start

```bash
# Compile to any format
aif compile doc.aif --format lml-aggressive   # Best for LLM context
aif compile doc.aif --format html             # Web publishing
aif compile doc.aif --format pdf              # Print-ready

# Import and clean documents for LLMs
aif import doc.md                             # Markdown → typed IR
aif import page.html --strip-chrome           # HTML → clean semantic IR
aif import paper.pdf                          # PDF → structured blocks
aif import doc.md --infer-semantics           # + auto-detect semantic types
aif import doc.md --infer-llm                 # + LLM-assisted classification

# Validate documents
aif lint doc.aif                              # 9 structural quality checks
aif skill eval skill.aif --stage 1            # Skill-specific lint (7 checks)

# Migration skills
aif migrate validate migration.aif
aif migrate run --skill migration.aif --source ./src --output ./migrated
```

---

## Key Features

### Semantic Document Authoring

```aif
@claim[id=c1, refs=e1]
  AIF preserves document meaning, not just appearance.
@end

@evidence[id=e1]
  Benchmark: 22% fewer tokens than Markdown with full semantic types.
@end

@table[id=results, refs=c1]: Token Comparison
| Format | Tokens | Structure |
| Cleaned HTML | 544K | None |
| AIF LML | 981K | Full semantic |
| Raw Markdown | 1,263K | Basic |
```

Typed blocks (`@claim`, `@evidence`, `@definition`, `@table`, `@figure`) are preserved across all 12 output formats. `aif lint` validates cross-references, evidence chains, and structural integrity. See [rich content examples](examples/rich-content/) for tables, SVG figures, audio/video metadata, and cross-reference demos.

### Skill Authoring for AI Agents

Write structured, verifiable skills for Claude Code, Codex, and Gemini:

```aif
@skill[name="code-review", version="1.0"]
  @step[order=1]
    Understand the PR context before reviewing code.
  @end
  @verify
    Every blocking issue includes a concrete fix.
  @end
  @red_flag
    Don't bikeshed on style while missing logic bugs.
  @end
@end
```

Full lifecycle: `aif skill import` (MD→AIF), `aif skill export` (AIF→MD), `aif skill verify` (integrity hash), `aif skill diff` (semantic diff), `aif skill bump` (auto-version), `aif skill eval` (3-stage quality pipeline). See [skills guide](examples/skills/) for authoring, validation, and deployment to Claude Code / Codex.

### Migration Engine

Typed migration skills with automated verification and repair:

```bash
aif migrate run --skill nextjs-13-to-15.aif --source ./src --output ./migrated
```

Pipeline: validate skill → chunk source files → apply per-chunk (LLM) → verify (static regex + semantic) → repair loop → generate AIF report. Three production examples included. See [migration guide](examples/migrations/).

---

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

| Crate | Purpose |
|-------|---------|
| `aif-core` | AST, document lint (9 checks), semantic inference, chunk graphs, JSON Schema |
| `aif-parser` | Logos-based lexer + block/inline parser |
| `aif-html` | HTML compiler + two-layer importer (roundtrip + generic + readability) |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | 5 LML prose modes, bidirectional parser, hybrid format, compression |
| `aif-binary` | Wire (postcard) and token-optimized binary with full roundtrip |
| `aif-skill` | Validation, hashing, versioning, diff, registry, chaining, marketplace |
| `aif-pdf` | PDF export + import + 4 chunking strategies + chunk graphs |
| `aif-eval` | 3-stage eval pipeline: structural lint → behavioral compliance → effectiveness |
| `aif-migrate` | Chunked migration engine with LLM apply, verification, repair loops |
| `aif-cli` | CLI: `compile`, `import`, `lint`, `skill`, `chunk`, `migrate`, `config`, `schema` |

## Benchmarks

| Benchmark | Key Finding |
|-----------|-------------|
| [Document tokens](benchmarks/document-tokens/) | Cleaned HTML 544K, LML Aggressive 981K, Raw MD 1,263K, Raw HTML 5.5M |
| [Skill tokens](benchmarks/skill-tokens/) | 100% semantic compliance across all formats, TNO 1.05 for Markdown RT |
| [Skill execution](benchmarks/skill-execution/) | LML Aggressive 0.97 vs Raw Markdown 0.87 overall compliance |
| [Chunking quality](benchmarks/chunking/) | 4 strategies compared: self-containment, size variance, budget compliance |
| [Roundtrip fidelity](benchmarks/roundtrip/) | JSON 1.00 (lossless), HTML 0.93, Markdown 0.57 |

Open [benchmarks/index.html](benchmarks/index.html) for the visual dashboard.

## Examples

```
examples/
├── documents/       # General documents and format conversions
├── skills/          # AI agent skills + Claude Code plugins (with authoring guide)
├── migrations/      # Codebase migration skills + reports (with detailed guide)
└── rich-content/    # Tables, SVG figures, audio/video metadata, cross-references
```

## Roadmap

- [ ] Multi-view compilation — `aif compile --view author`, `--view llm`, `--view api`
- [ ] Undefined terms lint — detect terms in claims not defined in `@definition` blocks
- [ ] Reusable skill templates — `@skill[extends="base-debugging"]` inheritance
- [ ] Citation precision benchmark — chunked retrieval accuracy with ground-truth Q&A
- [ ] LSP / editor integration — syntax highlighting, lint-on-save, go-to-definition
- [ ] crates.io publish — `cargo install aif-cli` from the registry

## Citation

If you find SkillForge useful in your research or workflows, please cite:

```bibtex
@software{skillforge2026,
  author       = {Liqun Chen},
  title        = {{SkillForge}: Semantic Document Compiler and {AI} Skill Toolkit},
  year         = {2026},
  url          = {https://github.com/LiqunChen0606/skillforge},
  note         = {Built on the AIF (AI-native Interchange Format). Best structure-per-token
                  ratio: full semantic types at 22\% fewer tokens than Markdown. Skills in
                  LML format improve LLM compliance by 10 percentage points vs raw Markdown.}
}
```

## Built With

This project was built almost entirely through AI-assisted development using [**ClawTerminal**](https://github.com/LiqunChen0606/clawterminal-docs) — an iOS SSH terminal + AI chatroom that connects your iPhone, iPad, or Apple Watch to Claude, Codex, Gemini, and Aider on remote servers.

[![Download on the App Store](https://img.shields.io/badge/Download-App_Store-black?style=for-the-badge&logo=apple&logoColor=white)](https://apps.apple.com/app/claw-ssh-ai-terminal/id6740686929)

From initial design through 8 implementation phases, 120+ commits, 11 Rust crates, 5 benchmark suites, and 210+ tests — all authored, reviewed, and shipped from an iPhone via ClawTerminal's AI agent integration.

## License

Apache-2.0 OR MIT (dual-licensed)
