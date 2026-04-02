# Migration Skill System — Design Document

## Vision

A migration engine built into AIF that lets users author typed migration skills, run them against codebases in a chunked pipeline with repair loops, and get structured migration reports with per-chunk verification results and confidence scores.

## Design Decisions

| Question | Decision |
|----------|----------|
| Skill format | Attribute convention on existing `@skill` blocks — no new AST types |
| Engine model | Chunked pipeline: chunk → migrate → verify → repair → reconcile |
| Verification | Hybrid: static analysis first, LLM-as-judge for semantic checks |
| Output | Migrated files to output directory + structured AIF migration report |
| Chunking | Reuse `aif-pdf` chunking infrastructure |
| Pipeline pattern | Reuse `aif-eval` deterministic → LLM stage pattern |

## Architecture

```
                    ┌──────────────────────────────────┐
                    │         MIGRATION SKILL           │
                    │  @skill[profile=migration]        │
                    │                                    │
                    │  @precondition  — when to apply    │
                    │  @step          — migration steps  │
                    │  @verify        — verification     │
                    │  @fallback      — recovery plans   │
                    │  @output_contract — success def    │
                    └──────────────┬───────────────────┘
                                   │
                    ┌──────────────▼───────────────────┐
                    │      SOURCE CODEBASE INTAKE       │
                    │                                    │
                    │  Files, configs, dependencies      │
                    │  → Chunk using aif-pdf strategies  │
                    └──────────────┬───────────────────┘
                                   │
                    ┌──────────────▼───────────────────┐
                    │     PER-CHUNK MIGRATION LOOP      │
                    │                                    │
                    │  1. Apply skill steps to chunk     │
                    │  2. Static verification            │
                    │     - syntax check                 │
                    │     - type check (if available)    │
                    │     - pattern grep                 │
                    │  3. LLM semantic verification      │
                    │     - behavioral preservation?     │
                    │     - @verify block criteria met?  │
                    │  4. If failed → repair loop        │
                    │     (max N iterations)             │
                    │  5. Record chunk result            │
                    └──────────────┬───────────────────┘
                                   │
                    ┌──────────────▼───────────────────┐
                    │    CROSS-CHUNK RECONCILIATION     │
                    │                                    │
                    │  - Resolve import/dependency refs  │
                    │  - Check interface consistency     │
                    │  - Verify no broken cross-refs     │
                    └──────────────┬───────────────────┘
                                   │
                    ┌──────────────▼───────────────────┐
                    │       MIGRATION REPORT (.aif)     │
                    │                                    │
                    │  Per-chunk: status, confidence,    │
                    │    verification results, repairs   │
                    │  Overall: summary, unresolved,     │
                    │    manual review points            │
                    │                                    │
                    │  Compiles to HTML/MD/LML for free  │
                    └──────────────────────────────────┘
```

## Migration Skill Format

Migration skills are standard `@skill` blocks with a `profile=migration` attribute. No new AST types needed — we use the existing `SkillBlockType` variants plus attribute conventions.

```aif
@skill[name="jest-to-vitest", version="1.0", profile=migration]
  @precondition
    Repository uses Jest as test runner.
    package.json contains "jest" in devDependencies.
    Test files use `describe`, `it`, `expect` patterns.
  @end

  @step[order=1]
    Replace Jest imports with Vitest equivalents.
    `import { describe, it, expect } from 'vitest'`
  @end

  @step[order=2]
    Update jest.config.js → vitest.config.ts.
    Map Jest config options to Vitest equivalents.
  @end

  @step[order=3]
    Replace Jest-specific APIs:
    - `jest.fn()` → `vi.fn()`
    - `jest.mock()` → `vi.mock()`
    - `jest.spyOn()` → `vi.spyOn()`
  @end

  @verify
    All test files import from 'vitest', not 'jest'.
    No remaining `jest.` calls in test files.
    `vitest run` passes with same test count as original.
    No `@jest` or `jest.config` files remain.
  @end

  @fallback
    If a test file uses Jest-specific timer mocking,
    preserve the original and flag for manual review.
  @end

  @output_contract
    All test files migrated to Vitest API.
    vitest.config.ts exists and is valid.
    Test pass rate >= original pass rate.
  @end
@end
```

### Validation

`aif skill verify` gains a migration profile check:
- `@precondition` present (applicability conditions)
- At least one `@step` block
- `@verify` present (verification criteria)
- `@output_contract` present (success definition)
- Optional: `@fallback` for recovery strategies

This is a lint extension, not a new stage — fits into the existing `aif-skill::lint` module.

## Crate: `aif-migrate`

New workspace crate housing the migration engine.

### Module Structure

```
crates/aif-migrate/
├── src/
│   ├── lib.rs           — public API
│   ├── engine.rs        — pipeline orchestrator
│   ├── chunk.rs         — source chunking (delegates to aif-pdf infra)
│   ├── apply.rs         — apply skill steps to a chunk via LLM
│   ├── verify.rs        — hybrid verification (static + LLM)
│   ├── repair.rs        — repair loop for failed chunks
│   ├── reconcile.rs     — cross-chunk dependency reconciliation
│   ├── report.rs        — generate AIF migration report
│   └── types.rs         — MigrationConfig, ChunkResult, MigrationReport
└── tests/
    ├── fixtures/        — sample skills + source files
    └── integration.rs
```

### Key Types

```rust
/// Configuration for a migration run
pub struct MigrationConfig {
    pub skill_path: PathBuf,        // Path to migration skill .aif file
    pub source_dir: PathBuf,        // Source codebase to migrate
    pub output_dir: PathBuf,        // Where migrated files go
    pub max_repair_iterations: u32, // Max repair attempts per chunk (default: 3)
    pub chunk_strategy: ChunkStrategy, // How to chunk source files
    pub llm_config: LlmConfig,     // LLM provider config (from aif-core)
}

/// Result for a single chunk
pub struct ChunkResult {
    pub chunk_id: String,
    pub files: Vec<PathBuf>,
    pub status: ChunkStatus,
    pub confidence: f64,            // 0.0 - 1.0
    pub verification: VerificationResult,
    pub repair_iterations: u32,
    pub notes: Vec<String>,
}

pub enum ChunkStatus {
    Success,
    PartialSuccess,  // Some verifications passed
    Failed,          // Repair loop exhausted
    Skipped,         // Precondition not met for this chunk
}

pub struct VerificationResult {
    pub static_checks: Vec<StaticCheck>,
    pub semantic_checks: Vec<SemanticCheck>,
    pub passed: bool,
}

pub struct StaticCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub struct SemanticCheck {
    pub criterion: String,   // From @verify block
    pub passed: bool,
    pub reasoning: String,   // LLM explanation
    pub confidence: f64,
}

/// Full migration report — generates as AIF Document
pub struct MigrationReport {
    pub skill_name: String,
    pub source_dir: PathBuf,
    pub chunks: Vec<ChunkResult>,
    pub overall_confidence: f64,
    pub unresolved: Vec<String>,
    pub manual_review: Vec<String>,
    pub duration: Duration,
}
```

### Pipeline Flow

```rust
pub async fn run_migration(config: MigrationConfig) -> Result<MigrationReport> {
    // 1. Parse and validate migration skill
    let skill = parse_migration_skill(&config.skill_path)?;
    validate_migration_profile(&skill)?;

    // 2. Chunk source codebase
    let chunks = chunk_source(&config.source_dir, &config.chunk_strategy)?;

    // 3. Per-chunk migration with repair loop
    let mut results = Vec::new();
    for chunk in &chunks {
        let result = migrate_chunk(chunk, &skill, &config).await?;
        results.push(result);
    }

    // 4. Cross-chunk reconciliation
    reconcile_chunks(&mut results, &config.output_dir)?;

    // 5. Generate report
    let report = generate_report(&skill, &results, &config)?;
    Ok(report)
}
```

## Chunking Strategy

Reuse `aif-pdf`'s `ChunkStrategy` concept but adapted for source code:

| Strategy | Description | Best For |
|----------|-------------|----------|
| `FilePerChunk` | Each source file is one chunk | Small files, independent transforms |
| `DirectoryChunk` | Group by directory | Monorepos, component-based projects |
| `DependencyGraph` | Group by import graph clusters | Tightly coupled codebases |
| `TokenBudget` | Group files until token limit | LLM context window management |

Default: `FilePerChunk` — simplest, most predictable. Users can override.

## Verification Pipeline

### Stage 1: Static Checks (no LLM)

Run deterministic checks derived from `@verify` block patterns:

- **Syntax check** — Parse output files (language-aware if possible, or basic bracket matching)
- **Pattern grep** — Extract grep-able assertions from `@verify` (e.g., "No remaining `jest.` calls" → `grep -r "jest\."`)
- **File existence** — Check referenced files exist
- **Import check** — Verify import statements reference valid modules

### Stage 2: LLM Semantic Check

For `@verify` criteria that can't be checked statically:

- Feed: original chunk + migrated chunk + `@verify` criteria
- Ask: "Does the migrated code satisfy each criterion? For each, explain why or why not."
- Parse structured response into `SemanticCheck` results

This mirrors `aif-eval` Stage 2 (behavioral compliance).

## Repair Loop

When verification fails:

```
for iteration in 0..max_repair_iterations:
    result = apply_skill_to_chunk(chunk, skill)
    verification = verify_chunk(result)
    if verification.passed:
        return Success(result)

    # Build repair prompt with failure context
    repair_context = format_failures(verification)
    result = repair_chunk(chunk, result, repair_context, skill)
    verification = verify_chunk(result)
    if verification.passed:
        return Success(result)

return Failed(best_result)
```

The repair prompt includes:
- Original chunk
- Current (failed) migration attempt
- Specific verification failures
- `@fallback` block guidance from the skill

## Cross-Chunk Reconciliation

After all chunks are migrated independently:

1. **Import resolution** — Check that cross-chunk imports still resolve
2. **Interface consistency** — Types/interfaces used across chunks match
3. **Dependency graph** — No circular dependencies introduced
4. **Config coherence** — Shared config files are consistent

This is a final pass, not a loop — if reconciliation finds issues, they go into the report as manual review items.

## Migration Report

The report is a native AIF document:

```aif
#title: Migration Report — jest-to-vitest
#author: aif-migrate

@section: Summary
  Migrated 47 files across 12 chunks.
  Overall confidence: 0.92.
  Duration: 3m 42s.
@end

@section: Results by Chunk
  @callout[type=success]
    Chunk 1 (src/components/): 8 files, confidence 0.98
    All static and semantic checks passed.
  @end

  @callout[type=warning]
    Chunk 5 (src/utils/): 3 files, confidence 0.75
    Static checks passed. Semantic check flagged possible
    behavior change in timer mocking (2 repair iterations).
  @end

  @callout[type=error]
    Chunk 9 (src/legacy/): 2 files, confidence 0.40
    Repair loop exhausted. Manual review required.
  @end
@end

@section: Manual Review Required
  - src/legacy/old-timer-tests.ts — Jest timer faking pattern not auto-convertible
  - src/legacy/custom-runner.ts — Custom test runner integration
@end

@section: Unresolved Issues
  - 2 snapshot files may need regeneration after migration
  - Custom jest.config transform plugins have no Vitest equivalent
@end
```

Because it's AIF, users can compile it to HTML, Markdown, or LML.

## CLI Integration

```bash
# Run a migration
aif migrate run --skill migration.aif --source ./src --output ./migrated [--strategy file|directory|dependency|token-budget] [--max-repairs 3]

# Validate a migration skill
aif skill verify migration.aif   # existing command, extended with migration profile checks

# View migration report
aif compile migration-report.aif -f html -o report.html
```

## Dependencies

| Dependency | Source | What We Reuse |
|------------|--------|---------------|
| Chunking infrastructure | `aif-pdf` | `ChunkStrategy`, token estimation |
| LLM client | `aif-eval` | Anthropic API client |
| Skill parsing/validation | `aif-skill` | Lint checks, `@skill` block parsing |
| Config | `aif-core` | `LlmConfig`, `AifConfig` |
| Report generation | `aif-core` | AST types for building AIF documents |

## Non-Goals (MVP)

- **Multi-provider LLM** — MVP supports Anthropic only (same as `aif-eval`)
- **Candidate branching** — No multi-candidate frontier search; single-path with repair loop
- **Repo scanning** — No automatic framework/version detection; user specifies the skill
- **Remote skill registry** — Use local skills only
- **Parallel chunk migration** — Sequential for MVP; parallelism is a future optimization
- **Git integration** — No automatic branch creation; output goes to a directory

## Future Extensions

1. **Candidate branching** — Try multiple migration strategies, keep best
2. **Repo scanner** — Auto-detect frameworks, versions, and applicable skills
3. **Parallel chunks** — Migrate independent chunks concurrently
4. **Git integration** — Create branches, PRs automatically
5. **Portfolio view** — Dashboard for multi-repo migration campaigns
6. **Skill marketplace** — Publish/install migration skills from remote registry
