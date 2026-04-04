# SkillForge

### Quality layer for [Agent Skills](https://agentskills.io) + Structured document format for LLMs

> **SkillForge** does two things: (1) adds lint, hashing, signing, versioning, and eval to your SKILL.md files — then exports back to Markdown for deployment to 30+ platforms; (2) provides a **typed semantic format (AIF)** that LLMs follow better than plain Markdown — typed `@step`, `@verify`, `@red_flag` blocks that survive compilation to 12+ output formats including an LLM-optimized mode 22% cheaper than Markdown.

![Language](https://img.shields.io/badge/Language-Rust-orange)
![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)
![Agent Skills](https://img.shields.io/badge/Compatible-agentskills.io-blueviolet)
![License](https://img.shields.io/badge/License-Apache--2.0%20%7C%20MIT-lightgrey)
[![PyPI](https://img.shields.io/pypi/v/aif-skillforge?label=PyPI&color=blue)](https://pypi.org/project/aif-skillforge/)

---

## Two Problems

### Problem 1: SKILL.md has no quality tooling

The [Agent Skills standard](https://agentskills.io) is adopted by 30+ platforms. But SKILL.md is plain Markdown — you can't lint it, hash it, sign it, version it, or evaluate whether the LLM actually follows it. SkillForge adds all five.

### Problem 2: Markdown is structurally flat

When you send a skill or document as Markdown, the LLM can't distinguish a `@step` from a `@red_flag`, a `@claim` from `@evidence`. Everything is just paragraphs and headings. AIF's typed blocks give the LLM explicit structure — and our benchmark shows LLMs follow typed formats **+4pp better** overall (+9pp on multi-step workflows) at **4% fewer tokens**.

## Quick Start

```bash
# Install
cargo install --path crates/aif-cli

# Import your existing SKILL.md
aif skill import SKILL.md -f json -o skill.json

# Lint (10 structural checks)
aif skill eval my-skill.aif --stage 1

# Verify integrity hash
aif skill verify my-skill.aif

# Sign with Ed25519
aif skill keygen                                    # Generate keypair
aif skill sign my-skill.aif --key private.key       # Sign
aif skill verify-signature my-skill.aif --signature <sig> --pubkey public.key

# Semantic diff between versions
aif skill diff old.aif new.aif --format json

# Auto-bump version based on change classification
aif skill bump my-skill.aif

# Export back to SKILL.md for deployment
aif skill export my-skill.aif -o SKILL.md
```

Or use Python — [`pip install aif-skillforge`](https://pypi.org/project/aif-skillforge/):

```python
import skillforge

# Lint a skill
results = skillforge.lint(open("skill.aif").read())

# Convert between formats
ir = skillforge.import_markdown(open("SKILL.md").read())
html = skillforge.compile(open("skill.aif").read(), "html")
```

## What SkillForge Checks

### 10 Structural Lint Checks

| Check | What it catches |
|-------|----------------|
| Frontmatter | Missing name or description |
| RequiredSections | No `@step` or `@verify` block |
| BlockTypes | Non-skill blocks mixed with skill blocks |
| VersionHash | Hash doesn't match content (tampered?) |
| DescriptionLength | Description > 1024 chars |
| NameFormat | Invalid characters in skill name |
| NoEmptyBlocks | Empty `@step` or `@verify` (placeholder left in) |
| ClaimsWithoutEvidence | `@claim` with no supporting `@evidence` |
| BrokenReferences | `refs=` pointing to nonexistent IDs |
| UndefinedTerms | Terms in claims not defined in `@definition` blocks |

### Ed25519 Signing

```
Author signs skill → SHA-256 hash → Ed25519 signature
Consumer verifies  → re-hash content → check signature against public key
Tampered skill     → hash mismatch → INVALID
```

For teams sharing skills via registries, marketplaces, or git repos — signing proves the skill hasn't been modified since it was reviewed. See [security analysis](security/analysis.md).

### 3-Stage Eval Pipeline

| Stage | What it does | LLM needed? |
|-------|-------------|-------------|
| 1. Structural lint | 10 deterministic checks (above) | No |
| 2. Behavioral compliance | LLM simulates agent with skill, checks rules | Yes |
| 3. Effectiveness | Scenario tests extracted from `@verify` blocks | Yes |

### Semantic Versioning

```bash
aif skill diff old.aif new.aif
# Output: { "classification": "Additive", "changes": [...] }

aif skill bump my-skill.aif --dry-run
# Output: "1.0.0 → 1.1.0 (Additive: new @example block)"
```

| Change type | Examples | Semver |
|-------------|----------|--------|
| Breaking | Removed step, changed precondition | Major |
| Additive | New step, new example, new fallback | Minor |
| Cosmetic | Rewording, reordering within block | Patch |

---

## The Format Advantage: Why Typed Blocks Beat Markdown

SkillForge isn't just quality tooling — it's also a **better format** for LLM consumption. AIF typed blocks (`@step`, `@verify`, `@claim`, `@evidence`) compile to 12+ output formats, including LML Aggressive which is 22% cheaper than Markdown with full semantic structure.

### For skills: LLMs follow typed blocks better

| What the LLM sees | Markdown | AIF LML Aggressive |
|-------------------|----------|-------------------|
| Step instructions | `## Step 1` (heading) | `@step:` (typed tag) |
| Constraints | Buried in prose | `@red_flag:` (explicit) |
| Verification | Just a paragraph | `@verify:` (typed) |
| **LLM compliance** | **0.84** | **0.88 (+4pp)** |
| **Multi-step workflows** | **0.72** | **0.81 (+9pp)** |

### For documents: Clean structure at fewer tokens

```bash
# Clean HTML for LLM consumption
aif import page.html --strip-chrome --infer-semantics

# Compile to token-efficient format (22% cheaper than Markdown, full structure)
aif compile doc.aif --format lml-aggressive

# 12+ output formats
aif compile doc.aif --format html|markdown|pdf|json|lml|lml-aggressive|binary-wire|...
```

| Format | Tokens (10 articles) | Structure |
|--------|---------------------|-----------|
| Cleaned HTML text | 544K | None |
| **LML Aggressive** | **981K** | **Typed semantic blocks** |
| Raw Markdown | 1,263K | Basic |

---

## Benchmark: Does Format Affect Skill Execution?

126 runs, 21 scenarios, 6 formats (claude-sonnet-4-6):

| Format | Tokens | Overall Compliance | Multi-Step Workflows |
|--------|--------|--------------------|---------------------|
| **LML Aggressive** | **861** | **0.88** | **0.81** |
| Raw Markdown | 901 | 0.84 | 0.72 |

+4pp overall, +9pp on multi-step workflows. Gap appears only on hard tasks — easy scenarios all score ~0.95. Full results: [benchmarks/RESULTS.md](benchmarks/RESULTS.md)

---

## Architecture

```
SKILL.md ──→ aif skill import ──→ Typed AST ──→ lint / hash / sign / eval
                                       │
                                       ├──→ aif skill export ──→ SKILL.md (deploy)
                                       ├──→ aif compile ──→ 12+ output formats
                                       └──→ aif skill diff ──→ change classification
```

13 Rust crates. 590+ tests. Full list: `aif-core`, `aif-parser`, `aif-html`, `aif-markdown`, `aif-lml`, `aif-binary`, `aif-skill`, `aif-pdf`, `aif-eval`, `aif-migrate`, `aif-lsp`, `aif-python`, `aif-cli`.

## Try It (60 seconds)

```bash
cargo install --path crates/aif-cli
bash scripts/demo.sh
```

## Project Structure

```
crates/          # 13 Rust crates — the product
examples/
├── skills/      # 10 AI skills + authoring guide
├── documents/   # Format conversion demos
├── migrations/  # 3 migration skills + reports
├── rich-content/# Tables, SVG, media metadata
└── case-studies/# Superpowers skills converted to AIF
benchmarks/      # 7 suites + consolidated RESULTS.md
plugins/         # Claude Code plugin (/lint-skill, /sign-skill)
editors/         # VS Code extension (LSP, syntax highlighting)
scripts/         # demo.sh, workbook/diagram generators
```

## Roadmap

- [x] Bidirectional SKILL.md ↔ AIF conversion
- [x] 10 structural lint checks + 3-stage eval pipeline
- [x] Ed25519 signing with tamper detection
- [x] Semantic versioning with auto-bump
- [x] Skill inheritance (`@skill[extends="base"]`)
- [x] Python bindings (7 functions via PyO3)
- [x] 12+ output format compiler
- [x] 7 benchmark suites (126+ LLM-evaluated runs)
- [ ] Publish to PyPI and crates.io
- [ ] LangChain / LlamaIndex integration
- [ ] Skill registry with signed publishing
- [ ] GitHub Action for CI skill linting

## Citation

```bibtex
@software{skillforge2026,
  author       = {Liqun Chen},
  title        = {{SkillForge}: Quality Layer for Agent Skills},
  year         = {2026},
  url          = {https://github.com/LiqunChen0606/skillforge},
  note         = {Lint, hash, sign, version, and eval for SKILL.md files.
                  Compatible with the Agent Skills standard (agentskills.io).
                  Built on the AIF (AI-native Interchange Format).}
}
```

## Built With

Built entirely through AI-assisted development using [**ClawTerminal**](https://github.com/LiqunChen0606/clawterminal-docs) — an iOS SSH terminal + AI chatroom for Claude, Codex, Gemini, and Aider.

[![Download on the App Store](https://img.shields.io/badge/Download-App_Store-black?style=for-the-badge&logo=apple&logoColor=white)](https://apps.apple.com/app/claw-ssh-ai-terminal/id6740686929)

150+ commits, 13 crates, 30K lines Rust, 590+ tests — authored and shipped from an iPhone.

## License

Apache-2.0 OR MIT (dual-licensed)
