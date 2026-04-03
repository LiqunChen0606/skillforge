# SkillForge

### Typed Semantic Documents for LLMs

> **SkillForge** is a Rust toolchain that gives LLMs **typed semantic structure** — claims, evidence, definitions, steps, verification blocks — in formats they demonstrably follow better than raw Markdown. Import HTML, PDF, or Markdown. Author verifiable skills for Claude, Codex, and Gemini. Migrate codebases with typed migration skills.

![Language](https://img.shields.io/badge/Language-Rust-orange)
![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
![Formats](https://img.shields.io/badge/Output_Formats-12+-green)
![License](https://img.shields.io/badge/License-Apache--2.0%20%7C%20MIT-lightgrey)
![Skills](https://img.shields.io/badge/Skills-Claude%20%7C%20Codex%20%7C%20Gemini-blueviolet)

---

## The Problem

LLMs treat all text as flat content. When you send raw Markdown or HTML, the model has no way to distinguish a **claim** from **evidence**, a **step** from a **verification check**, or a **precondition** from an **output contract**. This matters:

- **Agents skip steps** because nothing marks instruction blocks as typed steps
- **RAG pipelines lose structure** because chunking splits documents at arbitrary token boundaries
- **Skills degrade** because the model can't tell which blocks are constraints vs. examples

## The Evidence

We benchmarked the same skill, same task, same model across 4 formats:

| Format | Tokens | LLM Compliance |
|--------|--------|----------------|
| **AIF LML Aggressive** | **1,012** | **0.97** |
| JSON IR | 4,732 | 0.95 |
| HTML | 1,485 | 0.91 |
| Raw Markdown | 1,067 | **0.87** |

**+10 percentage points** over raw Markdown at 5% fewer tokens. Explicit typed tags (`@step:`, `@verify:`, `@red_flag:`) help the LLM identify and follow each instruction block. See [full benchmark](benchmarks/skill-execution/).

---

## Two Core Capabilities

### 1. Document Cleaning & Semantic Enrichment

**For data pipeline engineers** — normalize messy HTML, PDF, and Markdown into a typed semantic IR that LLMs can reason about, not just read.

```bash
# Import and enrich documents
aif import page.html --strip-chrome           # Strip nav/footer chrome, keep content
aif import paper.pdf                          # PDF → structured blocks with confidence scores
aif import doc.md --infer-semantics           # Auto-detect claims, evidence, definitions
aif import doc.md --infer-llm                 # LLM-assisted semantic classification

# Compile for LLM consumption
aif compile doc.aif --format lml-aggressive   # Typed tags, minimal tokens
aif compile doc.aif --format lml-aggressive --view llm  # Strip examples, truncate code

# Validate structure
aif lint doc.aif                              # 10 structural checks including broken refs
```

**What you get vs. plain text extraction:**

| What you send the LLM | Structure | Roundtrip | Use case |
|------------------------|-----------|-----------|----------|
| Cleaned HTML text | None — flat string | No | Cheap Q&A, summarization |
| Raw Markdown | Basic (headings, lists) | Partial | General documents |
| **AIF LML Aggressive** | **Typed blocks: claims, evidence, tables, definitions** | **Yes** | **Structured reasoning, agents, RAG** |

### 2. Skill Authoring & Verification

**For AI tool builders** — write skills with typed instruction blocks that LLMs follow more reliably, with built-in versioning, eval, and deployment.

```aif
@skill[name="code-review", version="1.0"]
  @precondition
    PR has passing CI and at least one approval.
  @end

  @step[order=1]
    Understand the PR context before reviewing code.
  @end

  @verify
    Every blocking issue includes a concrete fix suggestion.
  @end

  @red_flag
    Don't bikeshed on style while missing logic bugs.
  @end

  @output_contract
    Return structured review: summary, blocking issues, suggestions.
  @end
@end
```

Full lifecycle: `import` (MD→AIF) → `verify` (integrity hash) → `eval` (3-stage quality pipeline) → `diff` (semantic diff) → `bump` (auto-version) → `resolve` (inheritance chains) → deploy to Claude Code / Codex / Gemini.

See [skills guide](examples/skills/) for authoring, validation, and deployment.

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

## AIF Syntax

```aif
#title: Research Summary
#author: Team

@section[id=findings]: Key Findings
  @claim[id=c1, refs=e1]
    Typed semantic blocks improve LLM instruction-following by 10pp.
  @end

  @evidence[id=e1]
    Benchmark: 0.97 compliance (LML) vs 0.87 (raw Markdown), same model.
  @end

  @definition[id=d1]
    **Compliance** means step coverage + constraint respect + output contract adherence.
  @end

  @table[id=results, refs=c1]: Format Comparison
  | Format | Compliance | Tokens |
  | LML Aggressive | 0.97 | 1,012 |
  | Raw Markdown | 0.87 | 1,067 |
@end
```

Typed blocks (`@claim`, `@evidence`, `@definition`, `@table`, `@figure`) are preserved across all 12 output formats. `aif lint` validates cross-references, evidence chains, undefined terms, and structural integrity.

---

## Migration Engine

Typed migration skills with automated verification and repair:

```bash
aif migrate run --skill nextjs-13-to-15.aif --source ./src --output ./migrated
```

Pipeline: validate skill → chunk source files → apply per-chunk (LLM) → verify (static regex + semantic) → repair loop → generate AIF report. Three production examples included. See [migration examples](examples/).

---

## Architecture

```
Import sources           Semantic IR              Output formats
(HTML, PDF, MD) ──→  Parser ──→ Typed AST ──→  ┌── Human (HTML, MD, PDF)
                                    │           ├── LLM (5 LML modes)
                          Lint / Infer /        └── Machine (JSON, Binary)
                          View filtering
```

12 Rust crates:

| Crate | Purpose |
|-------|---------|
| `aif-core` | AST, lint (10 checks), semantic inference, view filtering, chunk graphs, JSON Schema |
| `aif-parser` | Logos lexer + block/inline parser |
| `aif-html` | HTML compiler + two-layer importer (roundtrip + generic + readability) |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | 5 prose modes, bidirectional parser, hybrid format, compression |
| `aif-binary` | Wire + token-optimized binary with full roundtrip |
| `aif-skill` | Validation, hashing, versioning, diff, registry, chaining, inheritance, marketplace |
| `aif-pdf` | PDF export + import + 4 chunking strategies + chunk graphs |
| `aif-eval` | 3-stage eval: structural lint → behavioral compliance → effectiveness |
| `aif-migrate` | Chunked migration engine with LLM apply, verification, repair loops |
| `aif-lsp` | LSP server: diagnostics, semantic tokens, go-to-definition |
| `aif-cli` | CLI: `compile`, `import`, `lint`, `skill`, `chunk`, `migrate`, `config`, `schema` |

## Benchmarks

| Benchmark | Key Finding |
|-----------|-------------|
| [Skill execution](benchmarks/skill-execution/) | LML 0.97 vs Markdown 0.87 compliance (+10pp, same model) |
| [Document tokens](benchmarks/document-tokens/) | LML Aggressive: full semantic types at 22% fewer tokens than Markdown |
| [Skill tokens](benchmarks/skill-tokens/) | 100% semantic compliance, TNO 1.05 for Markdown roundtrip |
| [Chunking quality](benchmarks/chunking/) | 4 strategies: self-containment, size variance, budget compliance |
| [Roundtrip fidelity](benchmarks/roundtrip/) | JSON 1.00 (lossless), HTML 0.93, Markdown 0.57 |
| [Citation precision](benchmarks/citation-precision/) | Chunked retrieval accuracy with ground-truth Q&A |

Open [benchmarks/index.html](benchmarks/index.html) for the visual dashboard.

## Examples

```
examples/
├── documents/       # General documents and format conversions
├── skills/          # AI agent skills + Claude Code plugins (with authoring guide)
├── migrations/      # Codebase migration skills + reports
└── rich-content/    # Tables, SVG figures, audio/video metadata, cross-references
```

## Roadmap

All planned features for v0.1.0 are complete:

- [x] Multi-view compilation — `aif compile --view author|llm|api`
- [x] Undefined terms lint — detect terms in claims not defined in `@definition` blocks
- [x] Skill inheritance — `@skill[extends="base-debugging"]`
- [x] Citation precision benchmark — chunked retrieval accuracy
- [x] LSP server — syntax highlighting, diagnostics, semantic tokens
- [x] crates.io publish prep — all 12 crates with metadata

## Citation

If you find SkillForge useful in your research or workflows, please cite:

```bibtex
@software{skillforge2026,
  author       = {Liqun Chen},
  title        = {{SkillForge}: Typed Semantic Documents for {LLMs}},
  year         = {2026},
  url          = {https://github.com/LiqunChen0606/skillforge},
  note         = {Typed semantic blocks improve LLM compliance by 10 percentage points
                  vs raw Markdown (0.97 vs 0.87). 12 output formats with full roundtrip.
                  Built on the AIF (AI-native Interchange Format).}
}
```

## Built With

This project was built almost entirely through AI-assisted development using [**ClawTerminal**](https://github.com/LiqunChen0606/clawterminal-docs) — an iOS SSH terminal + AI chatroom that connects your iPhone, iPad, or Apple Watch to Claude, Codex, Gemini, and Aider on remote servers.

[![Download on the App Store](https://img.shields.io/badge/Download-App_Store-black?style=for-the-badge&logo=apple&logoColor=white)](https://apps.apple.com/app/claw-ssh-ai-terminal/id6740686929)

From initial design through 9 implementation phases, 120+ commits, 12 Rust crates, 6 benchmark suites, and 220+ tests — all authored, reviewed, and shipped from an iPhone via ClawTerminal's AI agent integration.

## License

Apache-2.0 OR MIT (dual-licensed)
