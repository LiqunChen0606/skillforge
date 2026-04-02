# AIF Migration Skills

Migration skills are typed AIF documents that automate codebase transformations. Unlike ad-hoc scripts or manual migration guides, AIF migration skills are **machine-verifiable**: the engine validates preconditions, applies changes per-chunk, runs static + semantic verification, auto-repairs failures, and generates structured reports.

## How Migration Skills Work

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

### Required Blocks

Every migration skill must have `profile="migration"` and include:

| Block | Purpose |
|-------|---------|
| `@precondition` | When this migration applies (framework version, file patterns) |
| `@step[order=N]` | Ordered transformation steps |
| `@verify` | Success criteria — checked after each chunk |
| `@output_contract` | What the codebase looks like when done |

### Optional Blocks

| Block | Purpose |
|-------|---------|
| `@example` | Before/after code showing the transformation |
| `@decision` | Guidance for ambiguous situations |
| `@red_flag` | Anti-patterns to avoid during migration |
| `@fallback` | What to do when edge cases arise |

## Running a Migration

```bash
# 1. Validate the skill is well-formed
aif migrate validate migrations/migration_nextjs_13_to_15.aif

# 2. Run the migration
aif migrate run \
  --skill migrations/migration_nextjs_13_to_15.aif \
  --source ./my-nextjs-app/src \
  --output ./migrated \
  --strategy file \
  --max-repairs 3 \
  --report text

# 3. Review the generated report
# Reports include: executive summary, risk assessment, per-chunk pass/fail,
# failure analysis, and actionable recommendations.
```

### Chunking Strategies

| Strategy | Description | Best For |
|----------|-------------|----------|
| `file` | One chunk per source file | Small-medium codebases |
| `directory` | One chunk per directory | Monorepos with module boundaries |
| `token-budget` | Split at token limit | Large files that exceed context windows |

### Report Output

The engine generates a structured report with:

- **Executive Summary** — success/partial/failed/skipped counts, overall confidence
- **Risk Assessment** — Low/Medium/High/Critical based on failure rate
- **Verification Analysis** — per-check pass rates (static regex + semantic)
- **Results by Chunk** — individual file-level pass/fail
- **Failure Analysis** — recurring patterns, repair exhaustion warnings
- **Recommendations** — actionable next steps tiered by success rate

See [reports/](reports/) for example HTML reports from each migration.

## Included Examples

### Next.js 13 → 15 (`migration_nextjs_13_to_15.aif`)

7-step migration covering:
1. Dependency upgrade to Next.js 15
2. Async request APIs (`cookies()`, `headers()`, `params`, `searchParams`)
3. Next.config.js → next.config.ts
4. Caching semantics (implicit → explicit `force-cache`)
5. Middleware updates
6. React 19 patterns
7. Test suite updates

**Key decision:** Whether to migrate dynamic route params in-place or create wrapper functions.

**Red flag:** Don't migrate files partially — async/sync mixing causes runtime errors.

### ESLint Legacy → Flat Config (`migration_eslint_flat_config.aif`)

7-step migration from `.eslintrc.*` to `eslint.config.js`:
1. Inventory existing config files
2. Create base `eslint.config.js`
3. Migrate plugins to flat format
4. Convert rules and overrides
5. Handle TypeScript-specific config
6. Update npm scripts and CI
7. Remove legacy files

Includes `@example` blocks with concrete before/after config transformations.

### TypeScript Strict Mode (`migration_typescript_strict.aif`)

8-step phased rollout of `strict: true`:
1. Baseline audit of current strictness
2. Enable `noImplicitAny` (highest impact)
3. Enable `strictNullChecks`
4. Enable `strictFunctionTypes`
5. Enable `strictBindCallApply`
6. Enable `noImplicitThis`
7. Enable remaining strict flags
8. Enable top-level `strict: true` and remove individual flags

**Key decision:** Whether to use `// @ts-expect-error` for deferred fixes or fix everything immediately.

## Writing Your Own Migration Skill

```aif
@skill[name="my-migration", version="1.0", profile="migration"]

  @precondition
    Describe when this migration applies.
  @end

  @step[order=1]
    First transformation step with clear instructions.
  @end

  @step[order=2]
    Second step. Include code patterns to find and replace.
  @end

  @verify
    - Pattern X no longer appears in source files
    - All files compile/pass linting
    - Tests pass
  @end

  @example
    Before:
    ```typescript
    // old pattern
    ```
    After:
    ```typescript
    // new pattern
    ```
  @end

  @output_contract
    The codebase has been fully migrated. All verification checks pass.
  @end

@end
```

Validate with: `aif migrate validate my-migration.aif`
