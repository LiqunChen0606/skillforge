# Case Study: Superpowers Skills in AIF

## What We Did

Converted 3 production Claude Code skills from the [superpowers plugin](https://github.com/anthropics/claude-plugins-official/tree/main/superpowers) (v5.0.7) into AIF format and analyzed the structural and token-efficiency differences.

## Skills Converted

| Skill | Original (MD) | AIF Version | MD Lines | AIF Lines | Typed Blocks |
|-------|---------------|-------------|----------|-----------|--------------|
| systematic-debugging | 296 lines, 1504 words | systematic-debugging.aif | 296 | 154 | 17 blocks |
| test-driven-development | 371 lines, 1496 words | test-driven-development.aif | 371 | 187 | 22 blocks |
| writing-plans | 152 lines, 914 words | writing-plans.aif | 152 | 112 | 13 blocks |

## Key Findings

1. **25% fewer tokens** -- AIF versions use ~3,809 estimated tokens vs ~5,089 for the Markdown originals (BPE estimate: words x 1.3). The reduction comes from typed blocks carrying semantic meaning that Markdown encodes through repetitive prose and headers.

2. **52 typed blocks total** across 3 skills (17 + 22 + 13). Each `@red_flag`, `@step`, `@verify`, `@example`, and `@decision` block is machine-parseable. The Markdown originals have 0 typed blocks -- everything is free-text under headings.

3. **All 3 skills pass AIF structural lint** (7 checks each: frontmatter, required sections, step ordering, name format, no empty blocks, description length, version format). Raw Markdown has no equivalent validation.

4. **Parser quirk discovered** -- AIF parser retains literal quotes in attribute values when keys use `name="value"` syntax. Workaround: use unquoted values (`name=value`). Documented in analysis.md.

5. **Red flags are first-class** -- systematic-debugging has 8 `@red_flag` blocks, TDD has 9. In Markdown these are buried in prose. In AIF they can be extracted, counted, and used programmatically.

## How to Reproduce

```bash
# Inspect skill structure
aif skill inspect case-studies/superpowers-skills/systematic-debugging.aif
aif skill inspect case-studies/superpowers-skills/test-driven-development.aif
aif skill inspect case-studies/superpowers-skills/writing-plans.aif

# Run structural lint (stage 1 eval)
aif skill eval case-studies/superpowers-skills/systematic-debugging.aif --stage 1
aif skill eval case-studies/superpowers-skills/test-driven-development.aif --stage 1
aif skill eval case-studies/superpowers-skills/writing-plans.aif --stage 1

# Compile to LML-aggressive (most token-efficient structured format)
aif compile case-studies/superpowers-skills/systematic-debugging.aif -f lml-aggressive
aif compile case-studies/superpowers-skills/test-driven-development.aif -f lml-aggressive
aif compile case-studies/superpowers-skills/writing-plans.aif -f lml-aggressive

# Compare token counts (words x 1.3 for BPE estimate)
aif compile case-studies/superpowers-skills/systematic-debugging.aif -f lml-aggressive | wc -w
```

## Files

- `systematic-debugging.aif` -- 4-phase debugging process with 8 red flags and multi-layer diagnostic example
- `test-driven-development.aif` -- Red-Green-Refactor cycle with 9 red flags, good/bad code examples, and bug fix workflow
- `writing-plans.aif` -- Plan creation workflow with no-placeholder enforcement and task template example
- `analysis.md` -- Detailed comparison of token counts, typed blocks, lint findings, and qualitative differences
