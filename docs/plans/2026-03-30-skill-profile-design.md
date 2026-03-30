# Phase 3: Skill Profile Design

## Overview

The Skill Profile extends AIF with structured skill representation — enabling AI agents to store, discover, verify, and lazily load skills in a semantic, token-efficient format. Skills are first-class document types with integrity verification and adaptive block granularity.

## 1. Skill Block Model

### Container Block

```aif
@skill[name="debugging" version="1.0" hash="sha256:abc123..." hash-scope="whole"]
  @precondition
    User has reported a bug or test failure.
  @end

  @step[order=1]
    Reproduce the issue with a minimal test case.
  @end

  @verify
    The fix resolves the original issue without introducing regressions.
  @end

  @fallback
    If root cause is unclear after 3 attempts, escalate to user.
  @end
@end
```

### Block Types

| Block | Required | Purpose |
|-------|----------|---------|
| `@skill` | Container | Top-level skill wrapper with metadata attributes |
| `@step` | No | Ordered procedure step (`order` attr) |
| `@verify` | No | Validation/acceptance criteria |
| `@precondition` | No | When this skill applies |
| `@output_contract` | No | Expected output shape/format |
| `@decision` | No | Decision point with options |
| `@tool` | No | Tool/command reference |
| `@fallback` | No | Recovery strategy |
| `@red_flag` | No | Anti-patterns / things to avoid |
| `@example` | No | Concrete usage example |

### Attributes

**`@skill` attributes:**
- `name` (required): Skill identifier
- `version` (optional): Semver string
- `hash` (optional): `sha256:<hex>` content hash
- `hash-scope` (optional): `whole` (default) or `sections`
- `tags` (optional): Comma-separated classification tags
- `priority` (optional): `critical`, `high`, `normal`, `low`

**`@step` attributes:**
- `order` (required): Integer step number
- `label` (optional): Human-readable step name

**`@tool` attributes:**
- `name` (required): Tool identifier
- `args` (optional): Expected arguments pattern

**`@decision` attributes:**
- `condition` (optional): When this decision point triggers

### Validation Rules

1. `@skill` must have a `name` attribute
2. `@step` blocks, if present, must have unique `order` values
3. `@step` order values must form a contiguous sequence starting from 1
4. Inner blocks can only appear inside `@skill` containers
5. All inner blocks are optional — a `@skill` with only free text is valid
6. When `hash-scope="sections"`, each inner block gets its own `hash` attribute

### Integrity Verification

**Whole-skill hash (default):**
- Hash covers all content between `@skill` and its closing `@end`, excluding the `hash` attribute itself
- Content is normalized (trim whitespace, normalize line endings) before hashing
- SHA-256 produces a hex-encoded digest

**Section-level hash (opt-in via `hash-scope="sections"`):**
- Each inner block gets its own `hash` attribute
- Enables partial update detection — know exactly which section changed
- The `@skill` level hash becomes the hash of concatenated section hashes

## 2. SKILL.md Import Pipeline

### Heading Auto-Detection

The importer maps common Markdown headings to AIF skill blocks:

| Heading Pattern | Maps To | Confidence |
|----------------|---------|------------|
| `## Steps`, `## Procedure`, `## How to`, `## Instructions` | `@step` (split by numbered items) | High |
| `## Prerequisites`, `## Requirements`, `## When to use` | `@precondition` | High |
| `## Verification`, `## Testing`, `## Acceptance` | `@verify` | High |
| `## Examples`, `## Usage` | `@example` | High |
| `## Tools`, `## Commands` | `@tool` | Medium |
| `## Fallback`, `## Recovery`, `## If stuck` | `@fallback` | Medium |
| `## Anti-patterns`, `## Don't`, `## Avoid` | `@red_flag` | Medium |
| `## Output`, `## Expected output`, `## Returns` | `@output_contract` | Medium |
| `## Decision`, `## Choose`, `## Options` | `@decision` | Low |
| (unrecognized headings) | Free text in `@skill` | — |

### Confidence Scores

Each mapping carries a confidence level reported in the import output:
- **High**: Direct semantic match, no ambiguity
- **Medium**: Reasonable inference, may need review
- **Low**: Best-guess mapping, user should verify

The CLI reports confidence during import:
```
Imported skill "debugging" from debugging.md
  @precondition ← "Prerequisites" (high confidence)
  @step[1..5]   ← "Steps" (high confidence)
  @tool         ← "Commands" (medium confidence)
  @decision     ← "Options" (low confidence — verify mapping)
```

### Import Process

1. Extract skill name from `# Title` (H1) or filename
2. Scan H2 headings and match against pattern table
3. For `@step` mappings, split numbered/bulleted lists into individual steps
4. Compute SHA-256 hash of imported content
5. Generate `.aif` output with confidence annotations as comments

### Export (AIF → SKILL.md)

Reverse mapping: `@step` blocks become numbered lists under `## Steps`, etc. Metadata attributes become YAML frontmatter. Hash is preserved in frontmatter for roundtrip verification.

## 3. CLI Commands

### New Commands

```
aif skill import <input.md> [-o output.aif]    # Import SKILL.md → AIF
aif skill export <input.aif> [-o output.md]     # Export AIF → SKILL.md
aif skill verify <input.aif>                     # Verify integrity hash
aif skill rehash <input.aif>                     # Recompute and update hash
aif skill manifest <dir>                         # Generate skill manifest
aif skill inspect <input.aif>                    # Show skill metadata
```

### Skill Manifest

The manifest enables lazy loading — agents discover skills without loading full content.

```json
{
  "skills": [
    {
      "name": "debugging",
      "version": "1.0",
      "hash": "sha256:abc123...",
      "tags": ["process", "troubleshooting"],
      "priority": "high",
      "token_count": 715,
      "blocks": ["precondition", "step", "verify", "fallback"],
      "path": "skills/debugging.aif"
    }
  ],
  "generated": "2026-03-30T16:00:00Z",
  "total_tokens": 4200
}
```

**Manifest token budget target:** ~20 tokens per skill entry, ~200 tokens for a 10-skill index.

### Lazy Loading Flow

1. Agent loads manifest (~200 tokens for 10 skills)
2. Agent selects relevant skills by name/tags/priority
3. Selected skills loaded in LML-compact view (stripped of examples, verbose text)
4. Full skill loaded only when actively executing

### LML-Compact for Skills

The LML compiler gains a `--skill-compact` flag that:
- Strips `@example` blocks (loaded on demand)
- Condenses `@step` blocks to single-line summaries
- Preserves `@precondition`, `@verify`, `@red_flag` in full
- Typical reduction: 40-60% fewer tokens vs full skill

## 4. Implementation Scope

### New Crate: `aif-skill`

Responsible for:
- Skill-specific AST validation
- Hash computation and verification
- SKILL.md import/export
- Manifest generation

### Modified Crates

- **aif-core**: Add skill block types to AST (`Skill`, `Step`, `Verify`, etc.)
- **aif-parser**: Parse `@skill` container and inner blocks
- **aif-lml**: Add `--skill-compact` rendering mode
- **aif-cli**: Add `skill` subcommand group

### Test Strategy

- Unit tests for each block type parsing
- Roundtrip tests: SKILL.md → AIF → SKILL.md
- Hash verification tests (detect tampering)
- Manifest generation tests
- LML-compact token count tests
