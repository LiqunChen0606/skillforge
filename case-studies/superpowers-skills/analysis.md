# Analysis: Superpowers Skills -- Markdown vs AIF

## Source Material

Three production Claude Code skills from the [superpowers plugin](https://github.com/anthropics/claude-plugins-official/tree/main/superpowers) (v5.0.7):

| Skill | Original Location |
|-------|-------------------|
| systematic-debugging | `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/systematic-debugging/SKILL.md` |
| test-driven-development | `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/test-driven-development/SKILL.md` |
| writing-plans | `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/writing-plans/SKILL.md` |

## 1. Token Count Comparison

Token estimates use BPE heuristic: words x 1.3.

| Skill | MD Words | MD Tokens (est.) | AIF Words | AIF Tokens (est.) | Reduction |
|-------|----------|-------------------|-----------|---------------------|-----------|
| systematic-debugging | 1,504 | ~1,955 | 1,092 | ~1,420 | 27.4% |
| test-driven-development | 1,496 | ~1,945 | 1,080 | ~1,404 | 27.8% |
| writing-plans | 914 | ~1,188 | 758 | ~985 | 17.1% |
| **Total** | **3,914** | **~5,089** | **2,930** | **~3,809** | **25.1%** |

**Key insight:** AIF versions are ~25% more compact while preserving all actionable content. The reduction comes from:
- Eliminating Markdown structural repetition (repeated `##` headers, `###` subheaders, `---` separators)
- Consolidating prose into typed blocks that carry semantic meaning via their type rather than surrounding prose
- Removing meta-commentary that AIF's structure makes implicit (e.g., "You MUST complete each phase" is implicit in `@step[order=N]`)

The writing-plans skill shows less reduction (17%) because its original MD is already tightly written with minimal structural overhead.

## 2. Structural Comparison: Typed Blocks

AIF adds typed semantic blocks that Markdown encodes only through conventions and prose.

### systematic-debugging.aif -- 17 typed blocks

| Block Type | Count | What it captures |
|------------|-------|------------------|
| `@precondition` | 1 | When to invoke the skill |
| `@step[order=N]` | 4 | The four debugging phases |
| `@red_flag` | 8 | Anti-patterns that signal process violation |
| `@example` | 1 | Multi-layer diagnostic instrumentation |
| `@decision` | 1 | Architecture-questioning threshold (3+ failed fixes) |
| `@verify` | 1 | Success criteria for the debugging process |
| `@output_contract` | 1 | What the debugger must produce |

### test-driven-development.aif -- 22 typed blocks

| Block Type | Count | What it captures |
|------------|-------|------------------|
| `@precondition` | 1 | When to use TDD |
| `@step[order=N]` | 6 | Red-Green-Refactor cycle + verification steps |
| `@red_flag` | 9 | Anti-patterns (code before test, tests after, etc.) |
| `@example` | 3 | Good/bad test, good/bad implementation, bug fix workflow |
| `@decision` | 1 | What to do when stuck |
| `@verify` | 1 | Completion checklist |
| `@output_contract` | 1 | Required deliverables |

### writing-plans.aif -- 13 typed blocks

| Block Type | Count | What it captures |
|------------|-------|------------------|
| `@precondition` | 1 | When to write a plan |
| `@step[order=N]` | 5 | Plan creation workflow |
| `@red_flag` | 4 | Placeholder and no-code-block anti-patterns |
| `@example` | 1 | Properly structured task template |
| `@verify` | 1 | Plan quality criteria |
| `@output_contract` | 1 | Plan deliverables |

### What Markdown lacks

In raw Markdown, all of these are just sections with headers. There's no way to:

1. **Programmatically identify red flags.** AIF's `@red_flag` blocks can be extracted, counted, and validated. In Markdown, red flags are mixed into prose under a "Red Flags" section header.

2. **Enforce step ordering.** AIF's `@step[order=N]` attributes create an explicit ordering. Markdown uses `### Phase N` headers, but nothing prevents misordering.

3. **Validate completeness.** AIF lint can check that a skill has `@precondition`, `@step`, `@verify`, and `@output_contract` blocks. Markdown has no structural requirements.

4. **Extract verification criteria.** AIF's `@verify` block is machine-parseable. Markdown verification checklists are free-text under arbitrary headers.

5. **Distinguish examples from instructions.** AIF's `@example` blocks are typed. In Markdown, examples are interspersed with instructions using code blocks and prose.

## 3. AIF Lint Findings

Running `aif skill eval --stage 1` (structural lint) on the AIF versions:

```
systematic-debugging:    PASS (7 checks)
test-driven-development: PASS (7 checks)
writing-plans:           PASS (7 checks)
```

All three skills pass the 7 structural lint checks:
- Frontmatter (name + "Use when" description)
- RequiredSections (at least one @step and @verify)
- StepOrdering (sequential order attributes)
- NameFormat (alphanumeric + hyphens)
- NoEmptyBlocks (all blocks have content)
- DescriptionLength (not too long)
- VersionFormat (semver)

### What lint catches that raw Markdown misses

The original Markdown skills would **fail** several of these checks if converted naively:

1. **No "Use when" description convention.** The MD frontmatter `description` field doesn't follow a standard prefix. The lint enforces `"Use when..."` for consistent tool-use matching.

2. **No required sections.** A Markdown skill could omit verification criteria entirely. AIF lint requires `@verify`.

3. **No step ordering validation.** Markdown skills use `### Phase N` naming but nothing verifies N is sequential.

4. **No empty-block detection.** AIF lint catches `@step` blocks with no content. Markdown has no equivalent check.

### Parser quirk discovered

During conversion, we found that the AIF parser retains literal quotes in attribute values when using `name="value"` syntax with unquoted name keys (`name=value` works correctly). This is a minor parser inconsistency where quoted attribute values include the quote characters in the stored string. The lint `starts_with("Use when")` check fails on `"\"Use when..."`. Workaround: use unquoted values for names and short descriptions.

## 4. Qualitative Comparison

### Navigability

**Markdown strengths:**
- Familiar to all developers
- Renders nicely in GitHub, editors, wikis
- Free-form prose is natural for nuanced advice

**AIF strengths:**
- Typed blocks create a scannable outline (`@precondition`, `@step` x4, `@verify`, `@output_contract`)
- Red flags are visually distinct from instructions
- Examples are clearly demarcated from the steps they illustrate
- `aif skill inspect` gives instant structural overview without reading content

### Maintainability

**Markdown challenges:**
- Adding a step requires renumbering subsequent phases manually
- Moving red flags between sections requires understanding surrounding context
- No validation that the skill is complete (could delete @verify without warning)

**AIF advantages:**
- `@step[order=N]` makes reordering explicit
- `@red_flag` blocks can be added/removed without affecting surrounding structure
- `aif skill eval --stage 1` validates completeness on every change
- `aif skill diff` can compare structural changes between versions

### LLM Consumption

**For feeding skills to an LLM agent:**
- AIF LML-aggressive format compresses the typed structure into minimal delimiters (`@step:`, `@verify:`, `@red_flag:`)
- Estimated ~25% fewer tokens for the same semantic content
- Typed blocks provide clear context boundaries for the LLM to distinguish instructions from examples

## 5. Summary

| Dimension | Markdown | AIF |
|-----------|----------|-----|
| Token efficiency | Baseline | ~25% fewer tokens |
| Typed blocks | 0 (everything is prose) | 17-22 per skill |
| Lint/validation | None built-in | 7 structural checks, all pass |
| Programmatic access | Header-based only | Full typed AST |
| Human readability | Excellent | Good (slightly more verbose syntax) |
| Ecosystem support | Universal | AIF toolchain only |
| Roundtrip fidelity | N/A | Full (AIF -> LML/HTML/Binary -> AIF) |

**Bottom line:** AIF adds structural guarantees and validation that Markdown cannot provide, at a ~25% token savings. The tradeoff is ecosystem lock-in -- AIF files need the AIF toolchain to validate and compile, while Markdown works everywhere. For skills consumed by LLM agents (the primary superpowers use case), the typed structure and token efficiency make AIF a measurable improvement.
