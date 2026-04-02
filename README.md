# AIF: AI-native Interchange Format

> Write once in concise semantic syntax. Compile to 12+ formats. **Save 82% of tokens.** Migrate codebases with typed skills.

AIF is a semantic document format and Rust toolchain that gives LLMs structured documents at a fraction of the token cost — and gives humans a typed authoring syntax that compiles to HTML, PDF, Markdown, JSON, binary, and 5 levels of LLM-optimized output.

## The Problem

Every time you feed a document to an LLM, you're paying a **token tax**:

| What you send | Tokens (10 articles) | What the LLM gets |
|---------------|---------------------|-------------------|
| Raw HTML | **5,515,206** | Structure + presentational bloat |
| Raw Markdown | 1,262,755 | Basic formatting, no types |
| JSON IR | 4,483,743 | Typed but 81% overhead |
| **AIF LML Aggressive** | **979,838** | **Full semantic structure** |

AIF LML Aggressive: **82.2% fewer tokens than HTML**, with typed sections, claims, evidence, callouts, tables, figures — and lossless roundtrip.

## Quick Start

```bash
cargo build --workspace

# Compile to any format
aif compile doc.aif --format html
aif compile doc.aif --format lml-aggressive
aif compile doc.aif --format pdf

# Import from Markdown, HTML, or PDF
aif import doc.md
aif import page.html --strip-chrome
aif import paper.pdf

# Lint a document for structural issues
aif lint doc.aif

# Run the skill eval pipeline
aif skill eval skill.aif --stage 1

# Run a codebase migration
aif migrate run --skill migration.aif --source ./src --output ./migrated
```

## Migration Skills: Automate Codebase Transformations

This is AIF's most distinctive feature. Write a **typed migration skill** in AIF syntax. The engine chunks your codebase, applies the skill per-chunk with LLM verification, auto-repairs failures, and generates a rich report.

### How It Works

```
migration.aif ──→ Validate ──→ Chunk source ──→ Apply per-chunk ──→ Verify ──→ Repair loop ──→ Report
                    │              │                  │               │            │
                    │         file/dir/token      LLM callback    static regex   up to N
                    │           strategy                          + semantic     iterations
                    ▼                                              checks
              Skill has required
              @precondition, @step,
              @verify, @output_contract
```

### Example: Next.js 13 → 15 Migration

```aif
@skill[name="nextjs-13-to-15", version="1.0", profile="migration"]

  @precondition
    Next.js 13.x project with app/ router using synchronous request APIs.
  @end

  @step[order=1]
    Convert synchronous `cookies()`, `headers()`, `params`, `searchParams`
    to async: `const cookieStore = await cookies()`.
  @end

  @step[order=2]
    Replace implicit caching with explicit `force-cache` directives.
    Add `export const fetchCache = 'force-cache'` where needed.
  @end

  @verify
    - No remaining synchronous cookies()/headers() calls
    - All fetch() calls have explicit cache directive
    - TypeScript compilation succeeds with Next.js 15 types
  @end

  @red_flag
    Don't migrate dynamic route params without checking all usage sites.
    Partial async migration causes runtime errors — migrate entire files.
  @end

  @example
    Before:
    ```typescript
    export default function Page({ params }: { params: { id: string } }) {
      const { id } = params;
    ```
    After:
    ```typescript
    export default async function Page({ params }: { params: Promise<{ id: string }> }) {
      const { id } = await params;
    ```
  @end

  @output_contract
    All files compile with Next.js 15. No synchronous request API usage remains.
  @end
@end
```

### Run It

```bash
# Validate your migration skill is well-formed
aif migrate validate migration.aif

# Execute the migration
aif migrate run \
  --skill migration.aif \
  --source ./src \
  --output ./migrated \
  --strategy file \
  --max-repairs 3 \
  --report text
```

The engine generates a rich AIF report with: executive summary, risk assessment (Low/Medium/High/Critical), verification analysis, per-chunk pass/fail, failure patterns, and actionable recommendations.

**Three production-quality examples included:** Next.js 13→15 (7 steps), ESLint flat config (7 steps), TypeScript strict mode (8 steps). See [examples/](examples/).

## Semantic Blocks

Unlike Markdown, AIF natively supports typed semantic content:

```aif
@claim[id=c1]
  AIF preserves document meaning, not just appearance.
@end

@evidence[id=e1]
  Benchmark shows 82.2% token savings with 100% semantic compliance.
@end

@definition[id=def1]
  Token-Normalized Outcome (TNO) measures semantic fidelity per token spent.
@end

@callout[type=warning]
  Excessive use of JSON IR inflates token counts by 81%.
@end
```

These types are preserved across all output formats — the LLM knows a `@claim` is a claim, not just a paragraph. This enables document-level semantic linting: `aif lint` checks for broken references, claims without evidence, duplicate IDs, orphaned media, and malformed tables.

## Skill Profiles

Structured AI skill documents with versioning, integrity hashing, and a 3-stage eval pipeline:

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
@end
```

```bash
# Skill lifecycle
aif skill import skill.md -f lml-aggressive    # Import from Markdown
aif skill verify skill.aif                      # Check integrity hash
aif skill diff old.aif new.aif                  # Semantic diff
aif skill bump skill.aif                        # Auto-version bump
aif skill eval skill.aif --stage 1              # Structural lint
aif skill eval skill.aif                        # Full 3-stage eval

# Skill chaining & marketplace
aif skill chain skill.aif                       # Resolve dependency order
aif skill search "debugging" --tags rust        # Search remote registry
aif skill publish skill.aif                     # Publish to registry
```

### Change Classification

| Classification | Examples | Semver Impact |
|---------------|----------|---------------|
| **Breaking** | Removed step, changed precondition | Major |
| **Additive** | New step, new example, new fallback | Minor |
| **Cosmetic** | Rewording text, reordering within block | Patch |

## Benchmark Results

### Structure-per-Token: Why AIF Matters

| Format | Tokens | Structure | Roundtrip | Best For |
|--------|--------|-----------|-----------|----------|
| Raw PDF text | 561K | None | No | Cheap Q&A, summarization |
| Raw Markdown | 1.26M | Basic (headings, lists) | Partial | General documents |
| **AIF LML Aggressive** | **980K** | **Full semantic** | **Yes** | **Structured reasoning, agents** |
| Raw HTML | 5.5M | Full + presentational | Yes | Browser rendering |

### Document Token Efficiency (10 Wikipedia Articles)

| Format | Total Tokens | vs Raw HTML |
|--------|-------------|-------------|
| Raw HTML (baseline) | 5,515,206 | — |
| Raw PDF (text) | 560,950 | +89.8% saved |
| Raw Markdown | 1,262,755 | +77.1% saved |
| AIF Markdown (RT) | 1,009,181 | +81.7% saved |
| AIF LML Standard | 985,090 | +82.1% saved |
| **AIF LML Aggressive** | **979,838** | **+82.2% saved** |

### Skill Token Efficiency (10 AI Skills)

| Format | Total Tokens | vs SKILL.md | Compliance | TNO |
|--------|-------------|-------------|------------|-----|
| **Markdown (RT)** | **38,755** | **+1.9% saved** | **100%** | **1.05** |
| SKILL.md (baseline) | 39,506 | — | — | — |
| LML Aggressive | 39,514 | ~0% | 100% | 0.99 |
| HTML | 44,427 | -12.5% | 100% | 0.82 |
| JSON IR | 71,640 | -81.3% | 100% | 0.49 |

> Full reports: [Benchmark Dashboard](benchmarks/index.html) | [Document Report](benchmarks/results.html) | [Skill Report](benchmarks/skill_benchmark_report.html)

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
| `aif-core` | AST types, spans, errors, document lint, JSON Schema, shared utilities |
| `aif-parser` | Logos-based lexer + parser (`.aif` → AST) |
| `aif-html` | HTML compiler + importer (roundtrip + generic modes, readability extraction) |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer |
| `aif-lml` | LML compiler — 5 prose modes, bidirectional parser, hybrid format, semantic compression |
| `aif-binary` | Binary serialization — wire (postcard) and token-optimized with full roundtrip |
| `aif-skill` | Skill profiles — validation, hashing, versioning, diff, registry, chaining, marketplace, eval |
| `aif-pdf` | PDF export + import + document chunking (4 strategies) + chunk graphs |
| `aif-eval` | Eval pipeline — structural lint, behavioral compliance, scenario tests |
| `aif-migrate` | Migration engine — chunked pipeline, repair loops, verification, AIF reports |
| `aif-cli` | CLI: `compile`, `import`, `lint`, `dump-ir`, `skill`, `schema`, `chunk`, `config`, `migrate` |

## All Output Formats

| Format | Command Flag | Use Case |
|--------|-------------|----------|
| HTML | `--format html` | Browser rendering, web publishing |
| Markdown | `--format markdown` | Human editing, GitHub display |
| LML Standard | `--format lml` | Full semantic tags for LLM context |
| LML Compact | `--format lml-compact` | Standard minus @example blocks |
| LML Conservative | `--format lml-conservative` | Abbreviated tags with legend |
| LML Moderate | `--format lml-moderate` | Conservative + flatten wrappers |
| LML Aggressive | `--format lml-aggressive` | Minimal delimiters, best efficiency |
| LML Hybrid | `--format lml-hybrid` | LML text + base64 binary blocks |
| JSON IR | `--format json` | Typed AST for SDK/tooling |
| Binary Wire | `--format binary-wire` | Compact postcard for transport |
| Binary Token | `--format binary-token` | Token-aware binary encoding |
| PDF | `--format pdf` | Print-ready document export |

## Document Chunking

Split documents into addressable chunks for RAG pipelines:

```bash
aif chunk split doc.aif --strategy section -o chunks/
aif chunk split doc.aif --strategy token-budget --max-tokens 4096 -o chunks/
aif chunk graph doc1.aif doc2.aif -o graph.json
```

Four strategies: Section, TokenBudget, Semantic, FixedBlocks. Chunk graphs support typed edges: Evidence, Dependency, Continuation, CrossReference, Refutation.

## Cross-Language SDKs

Generated from JSON Schema for type-safe AIF document handling:

- **Python** (`sdks/python/`) — Pydantic v2 models
- **TypeScript** (`sdks/typescript/`) — TypeScript interfaces + Zod schemas

```bash
python scripts/generate_sdks.py          # Generate
python scripts/generate_sdks.py --check  # CI validation
```

## HTML Import

Two-layer importer: AIF-roundtrip mode reconstructs exact AST types from `aif-*` CSS classes; generic mode maps standard HTML tags. Readability extraction strips page chrome.

```bash
aif import page.html                # Auto-detect mode
aif import article.html --strip-chrome  # Strip nav/header/footer
```

## Build & Test

```bash
cargo build --workspace        # Build all crates
cargo test --workspace         # Run all tests
cargo run -p aif-cli -- --help # CLI usage
```

## Design Documents

- [Original Proposal](docs/proposal.md) — full rationale and architecture vision
- [Skill Profile Design](docs/plans/2026-03-30-skill-profile-design.md)
- [Binary IR & Versioning](docs/plans/2026-03-31-binary-ir-versioning-design.md)
- [PDF & Document Chunking](docs/plans/2026-03-31-pdf-chunking-design.md)
- [Skill Chaining & Marketplace](docs/plans/2026-03-31-skill-chaining-marketplace-design.md)
- [Cross-Language SDK](docs/plans/2026-03-31-multi-language-sdk-design.md)
- [Skill Eval Pipeline](docs/plans/2026-04-01-skill-eval-pipeline-design.md)
- [Migration Skill System](docs/plans/2026-04-02-migration-skill-system-design.md)
- [HTML Importer](docs/plans/2026-04-02-html-importer.md)

## License

Apache-2.0 OR MIT (dual-licensed)
