# AIF Plugin Skills

Claude Code plugins re-expressed in AIF format — demonstrating how AIF's typed semantic blocks capture the same structured knowledge as SKILL.md files, but with explicit block types, versioning, integrity hashing, and multi-format compilation.

## What This Is

Each subdirectory corresponds to a [claude-code plugin](https://github.com/anthropics/claude-code/tree/main/plugins). The `.aif` files contain the same skill knowledge as the original `SKILL.md` files, rewritten in AIF syntax with:

- `@skill` blocks with name, version, and description
- `@precondition` for when to activate
- `@step` blocks with ordered instructions
- `@verify` for validation criteria
- `@example` blocks with concrete before/after code
- `@decision` for ambiguous situations
- `@red_flag` for anti-patterns
- `@fallback` for edge cases
- `@tool` for required tools
- `@output_contract` for success criteria

## Why AIF?

| Feature | SKILL.md | AIF |
|---------|----------|-----|
| Typed blocks | No (plain Markdown) | Yes (@claim, @step, @verify...) |
| Integrity hash | No | SHA-256 per skill |
| Version management | Manual | Semver with auto-bump |
| Semantic diff | No | Structural change classification |
| Multi-format output | Markdown only | HTML, LML (5 modes), JSON, binary, PDF |
| Linting | No | 9 document + 7 skill lint checks |
| Token efficiency | Baseline | LML Aggressive: ~same tokens, explicit types |

## Compiling

```bash
# Compile to any format
aif compile plugins/code-review/code-review.aif --format lml-aggressive
aif compile plugins/code-review/code-review.aif --format html

# Verify integrity
aif skill verify plugins/code-review/code-review.aif

# Run eval pipeline
aif skill eval plugins/code-review/code-review.aif --stage 1
```

## Plugins Included

| Plugin | Description | Source |
|--------|-------------|--------|
| code-review | Automated PR code review with specialized agents | [code-review](code-review/) |
| frontend-design | Distinctive UI design guidance for AI-generated code | [frontend-design](frontend-design/) |
| feature-dev | 7-phase feature development workflow | [feature-dev](feature-dev/) |
| commit-commands | Git workflow automation (commit, push, PR) | [commit-commands](commit-commands/) |
| security-guidance | Security pattern detection for common vulnerabilities | [security-guidance](security-guidance/) |
| claude-opus-4-5-migration | Model migration from Sonnet/Opus 4.x to Opus 4.5 | [claude-opus-4-5-migration](claude-opus-4-5-migration/) |
