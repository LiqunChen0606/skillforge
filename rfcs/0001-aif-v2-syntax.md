# AIF v2 Syntax — Remove `@end`, Use Container Closers

**Status:** Design approved, ready for implementation planning
**Date:** 2026-04-04
**Author:** SkillForge / AIF

---

## Problem

AIF currently uses two termination models in one language:

| Block family | How it closes | Examples |
|---|---|---|
| Document blocks | blank line | `@section`, `@callout`, `@claim`, `@evidence`, `@table` |
| Skill blocks | explicit `@end` | `@skill`, `@step`, `@verify`, `@precondition`, `@red_flag`, `@output_contract`, `@tool`, `@example`, `@decision`, `@fallback` |

Skill files pay a heavy `@end` tax:

| File | Lines | `@end` lines | % |
|---|---|---|---|
| [examples/skills/code_review.aif](examples/skills/code_review.aif) | 108 | 14 | 13% |
| [examples/skills/base-debugging.aif](examples/skills/base-debugging.aif) | 38 | 9 | 24% |
| [examples/migrations/migration_nextjs_13_to_15.aif](examples/migrations/migration_nextjs_13_to_15.aif) | 163 | 15 | 9% |

The anonymous `@end` is the worst of both worlds: it costs tokens like a close-tag but doesn't name the block it closes, so mismatched nesting isn't caught by the parser. LLM-generated skills frequently miss or mismatch `@end`, producing parse errors.

The skill-execution benchmark already hints at this: LML Aggressive (closer-free) scores 0.88 vs AIF Source syntax (`@end`-based) at 0.85.

## Goals

1. Eliminate `@end` from the surface syntax.
2. Keep the grammar verifiable — the parser should catch unclosed / misnested containers.
3. Zero-day back-compat: every existing v1 `.aif` file continues to parse without changes.
4. Ship a one-command migration tool.

## Non-Goals

- Indentation-based syntax (rejected: fragile under copy/paste and LLM generation).
- Touching inline syntax (bold, emphasis, code spans) — only block-level termination changes.
- Changing output formats (HTML, Markdown, LML) or their emitters' behavior beyond source-emit.

---

## Design

### Termination Rules

**Containers** require an explicit named closer `@/<name>`. There are exactly three container directives:

- `@skill` → closes with `@/skill`
- `@section` → closes with `@/section`
- `@artifact_skill` → closes with `@/artifact_skill`

**Leaf blocks** auto-close when the parser sees the next line starting at column 0 with `@` (another directive or a container closer), or EOF. Leaf blocks are every other `@`-directive:

- Skill leaves: `@step`, `@verify`, `@precondition`, `@output_contract`, `@decision`, `@tool`, `@fallback`, `@red_flag`, `@example`, `@input_schema`, `@template`, `@binding`, `@generate`, `@export`
- Doc leaves: `@claim`, `@evidence`, `@definition`, `@assumption`, `@requirement`, `@result`, `@recommendation`, `@conclusion`, `@callout`, `@table`, `@figure`, `@audio`, `@video`, `@code`, `@list`, `@blockquote`, `@footnote`

**Leaf termination trigger (precise):** A leaf block's body continues until a line whose character at column 0 is `@`, followed by a letter or `/`. Indented `@` (e.g. `  @foo`) is treated as prose content. EOF terminates.

**Escape hatch:** To include a line that literally starts with `@` inside a leaf body, either indent it by one or more spaces (recommended — preserves visual prose), or prefix with backslash (`\@foo`). The parser strips a leading backslash before `@` during inline processing, so `\@step` renders as `@step` in output.

### Closer Verification

When the parser encounters `@/<name>`, it must match the name of the innermost open container. Mismatch is a parse error with both names in the message:

```
error: expected `@/skill` (opened at line 3), got `@/section` at line 42
```

### Grammar (EBNF sketch)

```
document        := metadata* block*
block           := container | leaf
container       := container_open content* container_close
container_open  := "@" container_name attrs? (":" inline)? NEWLINE
container_close := "@/" container_name NEWLINE
container_name  := "skill" | "section" | "artifact_skill"
leaf            := "@" leaf_name attrs? (":" inline)? NEWLINE leaf_body?
leaf_body       := line*  ; stops at next column-0 "@" or EOF
```

### Before/After

```aif
# Before (v1)                          # After (v2)
@skill[name="code_review"]             @skill[name="code_review"]
  @precondition                        @precondition
    PR with passing CI exists.           PR with passing CI exists.
  @end                                 @step[order=1]
  @step[order=1]                         Read PR description first.
    Read PR description first.         @step[order=2]
  @end                                   Check tests cover changes.
  @step[order=2]                       @verify
    Check tests cover changes.           All blocking issues have fixes.
  @end                                 @/skill
  @verify
    All blocking issues have fixes.
  @end
@end
```

14 lines → 9 lines. 4 `@end` → 1 `@/skill`. Container name verified.

---

## Dual-Syntax Parser

### Version Detection

The parser runs a one-pass pre-scan on the input:

- If the input contains any line matching `^\s*@/[a-z_]+\s*$` → **v2**
- Else if the input contains any line matching `^\s*@end\s*$` → **v1**
- Else (no closers at all, e.g. only leaf blocks) → **v2** (new default)
- If the input contains **both** `@end` and `@/name` → **error**: `"mixed v1/v2 syntax not supported; run 'aif migrate-syntax'"`

Detection runs before tokenization and chooses the terminator strategy for the block parser.

### Parser Architecture

- `crates/aif-parser/src/lexer.rs` — unchanged, already emits `AtDirective` and text tokens. Add one new token: `AtSlashDirective` (`@/name`).
- `crates/aif-parser/src/block.rs` — add `SyntaxVersion` enum, thread through `BlockParser`. Branch on version when reading child blocks:
  - **v1 path:** existing `@end`-terminated logic (unchanged).
  - **v2 path:** leaf bodies terminate on column-0 `@`; containers terminate on matching `@/name`.
- Shared code: attribute parsing, inline parsing, block construction — all version-agnostic.

The two paths share ~95% of the parser. Only the "where does this block end?" decision differs.

### Emitter Default

All emitters that produce AIF source syntax (currently: `aif compile --format aif` if exposed, `aif skill export` if it emits `.aif`) default to v2 output. A `--syntax=v1` flag is provided for one release as an escape hatch.

Other output formats (HTML, Markdown, LML×5, JSON, binary) are unaffected — they don't emit AIF source.

---

## Migration Tool

### CLI

```
aif migrate-syntax <path> [--in-place] [--dry-run]
```

- `<path>` can be a file or directory (recursive `.aif` scan).
- Default: writes to stdout for files, refuses directories without `--in-place`.
- `--in-place`: rewrites files, no backup (git is the backup).
- `--dry-run`: prints what would change, makes no writes.
- Exit codes: 0 = success, 1 = parse errors, 2 = no changes needed.

### Algorithm

1. Parse input as v1 (force version).
2. Walk the AST, emit via the v2 emitter.
3. Diff: if output differs, write (or print).

The round-trip proves correctness — if `parse_v1 → emit_v2 → parse_v2` produces the same AST as the v1 parse, the migration is lossless.

### In-Tree Migration

A single commit migrates:
- All 48 `examples/**/*.aif` files
- All `tests/fixtures/**/*.aif` files
- All documentation snippets in `CLAUDE.md`, `README.md`, and skill READMEs

Migration is mechanical; review is checking the diff for the expected shape (remove `@end`, add `@/skill` or `@/section` or `@/artifact_skill`).

---

## Back-Compat Matrix

| Input | Parser behavior |
|---|---|
| v1 file (has `@end`) | parses as v1, no warning (initial release) |
| v2 file (has `@/name`) | parses as v2 |
| File with neither (leaf-only doc) | parses as v2 |
| Mixed `@end` + `@/name` | parse error with migration hint |

**Deprecation timeline:**
- **v0.3.0 (this change):** Dual-syntax parser ships. v2 is emit default. No deprecation warnings.
- **v0.4.0:** v1 parser emits a deprecation warning on stderr when parsing `@end`-style files.
- **v1.0.0:** v1 parser removed. `aif migrate-syntax` becomes a standalone migration utility.

---

## Testing Strategy

### Parser (aif-parser)

- **Round-trip tests:** for every example file, assert `parse_v1(file) == parse_v2(migrate(file))`.
- **Column-0 termination:** fixtures with `@` at column 0 vs indented `@` in prose.
- **Mismatched closer:** `@skill ... @/section` produces a specific error with both names.
- **EOF termination:** leaf block at EOF (no trailing newline) closes cleanly.
- **Mixed-syntax detection:** file with both `@end` and `@/name` is rejected.
- **Version auto-detection:** each of the 4 detection cases has a test.

### Migration Tool (aif-cli)

- **Round-trip fidelity:** for every `examples/**/*.aif`, `migrate → parse → emit` produces identical AST to the original parse.
- **Idempotence:** `migrate(migrate(file)) == migrate(file)`.
- **Dry-run correctness:** `--dry-run` never writes.
- **Exit codes:** no-change file exits 2, migrated file exits 0.

### Emitter

- **v2 output is parseable:** every emitted file round-trips through the v2 parser.
- **Container closer matches opener:** `@skill[name=x]` closes with `@/skill`, never `@/section`.

### Regression

- The existing test suite runs against unchanged v1 fixtures — the v1 parser path must not regress.
- Skill-execution benchmark re-run with v2 fixtures to confirm no compliance loss.

---

## Error Handling

| Scenario | Error | Location |
|---|---|---|
| Unclosed container | `"unclosed @skill opened at line N"` | at EOF |
| Wrong closer name | `"expected @/skill (opened at line N), got @/section"` | at mismatched closer |
| Orphan closer | `"unexpected @/skill: no open container"` | at closer line |
| Mixed syntax | `"file mixes v1 (@end) and v2 (@/name) syntax; run 'aif migrate-syntax'"` | before parsing |
| Invalid closer name | `"unknown container: @/foo; containers are @/skill, @/section, @/artifact_skill"` | at closer line |

All errors include line numbers. Migration tool failures print the offending file path.

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Column-0 rule surprises users when `@` appears in prose | Document escape convention (`\@` or indent); add lint warning for column-0 `@` inside prose that doesn't match any known directive |
| Dual parser path rots | Shared AST + single emitter-output contract forces both paths to stay in sync; round-trip tests catch divergence |
| Users hand-edit v1 files after migration, mixing syntaxes | Mixed-syntax error stops them immediately with a clear fix |
| `@/name` looks like a URL path in diffs | Visual, not functional; acceptable |
| LSP/syntax-highlighting breaks | tower-lsp semantic tokens regenerated; VS Code TextMate grammar updated (one regex addition) |

---

## Out of Scope

- Changing `@end` to anything in v1 files at read time (no rewriting on parse).
- Adding new block types.
- Changing attribute syntax `[key=value]`.
- Compatibility shims in output formats — HTML/Markdown/LML emitters don't change.
- LSP-based auto-migration on save (future work).

---

## Success Criteria

1. All 48 example `.aif` files migrate cleanly (zero round-trip diffs).
2. Full test suite passes with both v1 and v2 fixtures.
3. `wc -l` shrinks by 8-20% across skill files; `@end` occurrences drop to 0 in v2 files.
4. Skill-execution benchmark: v2 fixtures score within ±1pp of v1 on compliance (no regression).
5. Parse error quality: mismatched closer test prints both names + both line numbers.
