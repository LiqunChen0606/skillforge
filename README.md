# SkillForge

### Typed Semantic Documents for LLMs

> **SkillForge** is a Rust toolchain that compiles typed, structured documents into 12+ output formats. Import HTML, PDF, or Markdown. Author verifiable skills for Claude, Codex, and Gemini. Clean documents for LLM consumption with 22% fewer tokens than Markdown.

![Language](https://img.shields.io/badge/Language-Rust-orange)
![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
![Formats](https://img.shields.io/badge/Output_Formats-12+-green)
![License](https://img.shields.io/badge/License-Apache--2.0%20%7C%20MIT-lightgrey)
![Skills](https://img.shields.io/badge/Skills-Claude%20%7C%20Codex%20%7C%20Gemini-blueviolet)

---

## What SkillForge Does

### 1. Clean documents for LLMs

Import messy HTML, PDF, and Markdown into a typed semantic IR — then compile to the most token-efficient format that preserves structure.

```bash
aif import page.html --strip-chrome       # Strip chrome, keep content
aif import paper.pdf                      # PDF → structured blocks
aif import doc.md --infer-semantics       # Auto-detect claims, evidence, definitions
aif compile doc.aif --format lml-aggressive  # Compile to LLM-optimal format
aif lint doc.aif                          # 10 structural quality checks
```

**Token comparison (10 Wikipedia articles, Anthropic API token counting):**

| Format | Tokens | Structure | Use case |
|--------|--------|-----------|----------|
| Cleaned HTML text | 544K | None | Cheapest — no structure |
| Raw PDF text | 561K | None | Cheapest — no structure |
| **AIF LML Aggressive** | **981K** | **Typed semantic blocks** | **Best structure-per-token** |
| Raw Markdown | 1,263K | Basic (headings, lists) | General documents |
| Raw HTML | 5,500K | Full + presentational bloat | Browser rendering |

LML Aggressive is **22% cheaper than Markdown** with typed claims, evidence, definitions, tables. It costs 80% more than flat text extraction — that's the price of structure.

### 2. Author verifiable AI skills

Write skills with typed blocks (`@step`, `@verify`, `@red_flag`) that are lintable, hashable, signable, and versionable.

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
@end
```

**Skill lifecycle:** `import` (MD→AIF) → `lint` (10 structural checks) → `verify` (SHA-256 hash) → `sign` (Ed25519) → `eval` (3-stage pipeline) → `diff` (semantic diff) → `bump` (auto-version) → `export` (AIF→MD) → deploy.

### 3. Migrate codebases with typed skills

```bash
aif migrate run --skill nextjs-13-to-15.aif --source ./src --output ./migrated
```

Pipeline: validate → chunk source → apply per-chunk (LLM) → verify (regex + semantic) → repair loop → report. Three production examples: Next.js 13→15, ESLint flat config, TypeScript strict mode.

---

## Try It (60 seconds)

```bash
# Install
cargo install --path crates/aif-cli

# Run the demo
bash scripts/demo.sh
```

The demo creates an AIF document, compiles to 4 formats, lints it, imports Markdown with semantic inference, and signs a skill — all in one script.

Or use the Python SDK:

```python
import skillforge

# Clean HTML for an LLM
ir = skillforge.clean_html("<html><body><nav>skip</nav><p>Content</p></body></html>")

# Compile AIF to LML
lml = skillforge.compile("@claim\nThis is a claim.\n", "lml-aggressive")

# Lint a document
results = skillforge.lint("#title: Test\n\n@claim\nHello.\n")
```

---

## Benchmark Results

We ran 7 benchmark suites. Here's what we found — honestly.

### Skill Execution Quality (126 runs, 6 formats, claude-sonnet-4-6)

Does the format affect how well the LLM follows a skill?

| Format | Tokens | Overall | Multi-Step | Constraints |
|--------|--------|---------|------------|-------------|
| **LML Aggressive** | **861** | **0.88** | **0.81** | **0.89** |
| JSON IR | 3,838 | 0.87 | 0.74 | 0.87 |
| LML Standard | 928 | 0.86 | 0.74 | 0.88 |
| Raw Markdown | 901 | 0.84 | 0.72 | 0.86 |
| HTML | 1,217 | 0.84 | 0.73 | 0.87 |
| AIF Source | 1,024 | 0.82 | 0.72 | 0.85 |

**Findings:** LML Aggressive scores +4pp overall vs Markdown at 4% fewer tokens. The gap is largest on multi-step workflows (+9pp). On easy/standard scenarios, all formats score ~0.95 — format only matters when the task requires judgment. 15 of 21 scenarios were ties.

**Caveats:** Single run per scenario×format (no confidence intervals). Judge model (Sonnet) may have biases. 21 scenarios is a moderate sample. An adversarial resistance advantage seen in a 3-scenario pilot was NOT reproduced in the full 60-run adversarial benchmark.

### Document Token Efficiency (10 Wikipedia articles, Anthropic API)

| Format | Total Tokens | vs Markdown | Structure |
|--------|-------------|-------------|-----------|
| Cleaned HTML | 544K | +57% cheaper | None |
| Raw PDF text | 561K | +56% cheaper | None |
| **LML Aggressive** | **981K** | **+22% cheaper** | **Full semantic** |
| Raw Markdown | 1,263K | baseline | Basic |
| Raw HTML | 5,500K | 4.4× more | Full + chrome |

### Roundtrip Fidelity (40 files × 3 formats)

| Path | Overall | Semantic types preserved |
|------|---------|------------------------|
| AIF → JSON → AIF | 1.00 | 100% (lossless) |
| AIF → Markdown → AIF | 0.84 | 93% |
| AIF → HTML → AIF | 0.50 | 93% (block types lost in generic mode) |

Full results: [benchmarks/RESULTS.md](benchmarks/RESULTS.md) | [Visual dashboard](benchmarks/index.html)

---

## Architecture

```
Import (HTML, PDF, MD) ──→ Parser ──→ Typed AST ──→ 12+ output formats
                                         │
                               Lint / Infer / Sign / Eval
```

13 Rust crates: `aif-core` (AST, lint, inference), `aif-parser`, `aif-html`, `aif-markdown`, `aif-lml` (5 modes), `aif-binary`, `aif-skill` (hash, sign, version, diff, chain, inherit), `aif-pdf`, `aif-eval`, `aif-migrate`, `aif-lsp`, `aif-python` (PyO3), `aif-cli`.

## Project Structure

```
crates/          # 13 Rust crates
examples/
├── documents/   # General AIF documents
├── skills/      # 8 AI agent skills with authoring guide
├── migrations/  # 3 migration skills with detailed guide
└── rich-content/# Tables, SVG, audio/video metadata, cross-references
artifacts/       # Artifact generators (Excel workbook, Mermaid diagrams)
benchmarks/      # 7 suites: execution, adversarial, tokens, chunking, roundtrip, citation
case-studies/    # 3 superpowers skills converted to AIF
plugins/         # Claude Code plugin (4 slash commands)
editors/         # VS Code extension (syntax highlighting, LSP)
security/        # Formal security analysis of skill signing
scripts/         # Demo script, SDK generators
```

## Roadmap

### Done

- [x] 13 Rust crates — full compiler pipeline with 12+ output formats
- [x] Skill lifecycle — lint, hash, sign (Ed25519), version, diff, eval, inherit
- [x] Document cleaning — HTML/PDF/MD import with readability and semantic inference
- [x] Migration engine — typed skills with LLM apply, verification, repair loops
- [x] Python bindings — 7 functions via PyO3
- [x] 7 benchmark suites with 126+ LLM-evaluated runs
- [x] VS Code extension + Claude Code plugin
- [x] Artifact skills — workbook and diagram generators

### Next

- [ ] Publish to PyPI and crates.io
- [ ] LangChain / LlamaIndex integration
- [ ] Multi-run variance analysis with confidence intervals
- [ ] Artifact skills platform (spreadsheets, decks, diagrams from templates)

## Citation

If you find SkillForge useful in your research or workflows, please cite:

```bibtex
@software{skillforge2026,
  author       = {Liqun Chen},
  title        = {{SkillForge}: Typed Semantic Documents for {LLMs}},
  year         = {2026},
  url          = {https://github.com/LiqunChen0606/skillforge},
  note         = {Semantic document compiler and AI skill toolkit. LML Aggressive format
                  scores +4pp on LLM skill compliance vs raw Markdown (0.88 vs 0.84,
                  126 runs). 22\% fewer tokens than Markdown with full semantic types.
                  Built on the AIF (AI-native Interchange Format).}
}
```

## Built With

This project was built entirely through AI-assisted development using [**ClawTerminal**](https://github.com/LiqunChen0606/clawterminal-docs) — an iOS SSH terminal + AI chatroom that connects your iPhone, iPad, or Apple Watch to Claude, Codex, Gemini, and Aider on remote servers.

[![Download on the App Store](https://img.shields.io/badge/Download-App_Store-black?style=for-the-badge&logo=apple&logoColor=white)](https://apps.apple.com/app/claw-ssh-ai-terminal/id6740686929)

150+ commits, 13 Rust crates, 30K lines of Rust, 7 benchmark suites, 590+ tests — authored, reviewed, and shipped via ClawTerminal.

## License

Apache-2.0 OR MIT (dual-licensed)
