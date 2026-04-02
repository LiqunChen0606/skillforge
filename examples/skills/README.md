# AIF Skills & Plugins

Example skills and Claude Code plugins in AIF format — demonstrating how typed semantic blocks create more rigorous, verifiable, and LLM-effective skills than plain SKILL.md files.

**Benchmark result:** LLMs follow AIF LML skills **10 percentage points better** than raw Markdown (0.97 vs 0.87 overall compliance) at 5% fewer tokens. See [skill execution benchmark](../../benchmarks/skill-execution/results.json).

## Why AIF for Skills?

| Feature | SKILL.md (Markdown) | AIF Skill |
|---------|-------------------|-----------|
| Typed blocks | No — plain prose | Yes — `@step`, `@verify`, `@example`, `@red_flag`... |
| Integrity hash | No | SHA-256 per skill, detects unauthorized edits |
| Version management | Manual | Semver with auto-bump based on change classification |
| Semantic diff | No | Structural diff: breaking/additive/cosmetic changes |
| Quality checks | No | 7 structural lint checks + 3-stage eval pipeline |
| Multi-format output | Markdown only | HTML, LML (5 modes), JSON, binary, PDF |
| Bidirectional | One-way | AIF ↔ Markdown roundtrip |
| LLM compliance | Baseline | +10 percentage points (benchmark verified) |

## Skills Included

| Skill | Description |
|-------|-------------|
| [code_review.aif](code_review.aif) | Automated PR code review with confidence scoring and @example blocks |
| [frontend-design.aif](frontend-design.aif) | Distinctive UI design that avoids generic AI aesthetics |
| [feature-dev.aif](feature-dev.aif) | 7-phase guided feature development workflow |
| [commit-commands.aif](commit-commands.aif) | Git workflow automation (commit, push, PR) |
| [security-guidance.aif](security-guidance.aif) | Detect 6 categories of security vulnerabilities |
| [claude-opus-4-5-migration.aif](claude-opus-4-5-migration.aif) | Model migration from Sonnet/Opus 4.x to Opus 4.5 |

---

## Guide: Creating AIF Skills

### 1. Write a New Skill from Scratch

Every AIF skill follows this structure:

```aif
#title: My Skill Name
#summary: One-line description

@skill[name="my-skill", version="1.0", description="Use when ..."]

  @precondition
    When should this skill activate? What context is required?
  @end

  @step[order=1]
    First thing the agent should do.
  @end

  @step[order=2]
    Second step. Be specific — include commands, patterns, file paths.
  @end

  @verify
    - How to confirm the skill was applied correctly
    - Concrete, checkable criteria
  @end

  @output_contract
    What the world looks like when this skill completes successfully.
  @end

@end
```

**Optional blocks** (add as needed):

| Block | When to use |
|-------|-------------|
| `@example` | Show concrete before/after code or usage scenarios |
| `@decision` | Guide the agent through ambiguous situations |
| `@red_flag` | Warn about anti-patterns and common mistakes |
| `@fallback` | What to do when the primary approach fails |
| `@tool` | List required tools (`git`, `cargo test`, etc.) |

### 2. Validate Your Skill

```bash
# Structural lint (7 checks: frontmatter, required sections, block types, etc.)
aif skill eval my-skill.aif --stage 1

# Output:
# STAGE 1: StructuralLint [....] PASS (2ms)
#   [+] Frontmatter
#   [+] RequiredSections
#   [+] BlockTypes
#   [+] VersionHash
#   [+] DescriptionLength
#   [+] NameFormat
#   [+] NoEmptyBlocks
```

### 3. Version and Hash Your Skill

```bash
# Compute integrity hash
aif skill rehash my-skill.aif

# Inspect metadata
aif skill inspect my-skill.aif

# Compare versions
aif skill diff old-skill.aif new-skill.aif --format json
# Output: { "classification": "Additive", "changes": [...] }

# Auto-bump version based on changes
aif skill bump my-skill.aif --dry-run
# Output: "1.0.0 → 1.1.0 (Additive: new @example block)"
```

### 4. Compile to Any Format

```bash
# LML Aggressive — optimal for LLM context windows
aif compile my-skill.aif --format lml-aggressive

# HTML — for documentation sites
aif compile my-skill.aif --format html -o my-skill.html

# JSON IR — for programmatic access
aif compile my-skill.aif --format json

# Binary — for wire transport (45% smaller than JSON)
aif compile my-skill.aif --format binary-wire -o my-skill.bin
```

---

## Guide: Converting Between AIF and Markdown

AIF skills and SKILL.md files are **bidirectionally convertible**. This means you can:
- Import existing SKILL.md files into AIF for typed validation
- Export AIF skills back to SKILL.md for use in Claude Code / Codex

### Markdown → AIF (Import)

```bash
# Import a SKILL.md into AIF JSON IR
aif skill import my-skill.md -f json -o my-skill.json

# Import and compile directly to LML Aggressive
aif skill import my-skill.md -f lml-aggressive -o my-skill.lml

# Import to AIF HTML for documentation
aif skill import my-skill.md -f html -o my-skill.html
```

**What happens during import:**
1. Markdown headings map to `@section` blocks
2. YAML frontmatter (`name`, `description`, `version`) becomes `@skill` attributes
3. Numbered lists become `@step` blocks (if inside a skill context)
4. Blockquotes and callouts map to their AIF equivalents
5. Code blocks preserve language tags

**After import, you can lint:**
```bash
# Check structural quality of the imported skill
aif skill eval my-skill.aif --stage 1
```

### AIF → Markdown (Export)

```bash
# Export an AIF skill back to SKILL.md format
aif skill export my-skill.aif -o SKILL.md
```

**What happens during export:**
1. `@skill` attributes become YAML frontmatter
2. `@step[order=N]` blocks become numbered sections
3. `@verify` becomes a checklist
4. `@example` blocks become fenced code blocks
5. `@red_flag`, `@decision`, `@fallback` become labeled sections

### Roundtrip Workflow

```bash
# 1. Start with an existing SKILL.md
aif skill import SKILL.md -f json -o skill.json

# 2. Compile to AIF source for editing
#    (or edit the JSON IR directly)

# 3. Lint and validate
aif skill eval skill.aif --stage 1

# 4. Export back to Markdown
aif skill export skill.aif -o SKILL.md

# 5. Verify roundtrip quality
aif skill diff original.aif roundtripped.aif
```

---

## Guide: Building Skills for Claude Code and Codex

### For Claude Code

Claude Code skills live in `.claude/commands/` or plugin `skills/` directories as SKILL.md files. To create a rigorous skill:

```bash
# 1. Author in AIF (typed, validated)
cat > my-skill.aif << 'EOF'
@skill[name="my-skill", version="1.0", description="Use when user asks to ..."]
  @precondition
    ...
  @end
  @step[order=1]
    ...
  @end
  @verify
    ...
  @end
@end
EOF

# 2. Validate structure
aif skill eval my-skill.aif --stage 1

# 3. Export to SKILL.md for Claude Code
aif skill export my-skill.aif -o SKILL.md

# 4. Place in your plugin
cp SKILL.md my-plugin/skills/my-skill/SKILL.md
```

### For Codex (AGENTS.md)

Codex uses `AGENTS.md` files for agent guidance. AIF skills can be exported to Markdown and placed in `AGENTS.md`:

```bash
# Export as Markdown, append to AGENTS.md
aif skill export my-skill.aif -o skill-section.md
cat skill-section.md >> AGENTS.md
```

### Skill Development Lifecycle

```
Author in AIF → Lint (stage 1) → Eval (stages 2-3) → Export to MD → Deploy
     ↑                                                       │
     └───────── Import updated MD back ←──────────────────────┘
```

1. **Author** in AIF for typed blocks and structural guarantees
2. **Lint** catches missing sections, empty blocks, bad names, hash mismatches
3. **Eval** (if LLM configured) tests behavioral compliance and effectiveness
4. **Export** to SKILL.md for deployment in Claude Code / Codex
5. **Re-import** after manual edits to re-validate and re-hash

### Skill Quality Checklist

Before deploying a skill, verify:

```bash
# All 7 structural checks pass
aif skill eval my-skill.aif --stage 1

# Integrity hash is set
aif skill verify my-skill.aif

# Version is bumped if changed
aif skill bump my-skill.aif --dry-run

# Diff shows expected changes
aif skill diff previous.aif my-skill.aif
```
