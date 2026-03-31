# AIF Architecture Reference

## Overview

AIF (AI-native Interchange Format) is a semantic document format and toolchain for humans and LLMs.
Two-layer design: Surface syntax (`.aif` files) → Semantic IR (typed AST) → Output formats.

## Crate Map

| Crate | Responsibility | Key Types |
|-------|---------------|-----------|
| `aif-core` | AST types, spans, errors — shared IR | `Document`, `Block`, `BlockKind`, `Inline`, `Attrs` |
| `aif-parser` | Logos-based lexer + block/inline parser | `.aif` → `Document` |
| `aif-html` | HTML compiler | `Document` → semantic HTML |
| `aif-markdown` | Markdown compiler + pulldown-cmark importer | `Document` ↔ Markdown |
| `aif-lml` | LML compiler — LLM-optimized tagged format | `Document` → LML (multiple modes) |
| `aif-skill` | Skill profiles — validation, SHA-256 hashing, SKILL.md import/export | `SkillImportResult`, `SkillManifest` |
| `aif-cli` | CLI tool | `compile`, `import`, `dump-ir`, `skill` subcommands |

## AST Design

### Document
Top-level container: `metadata: BTreeMap<String, String>` + `blocks: Vec<Block>`.

### Block / BlockKind
Each `Block` has a `kind: BlockKind` and `span: Span`. BlockKind variants:
- **Paragraph** — inline content
- **Section** — attrs, title (inlines), children (blocks)
- **SemanticBlock** — typed (Claim, Evidence, Definition, Theorem, Assumption, Result, Conclusion, Requirement, Recommendation), attrs, optional title, content
- **Callout** — typed (Note, Warning, Info, Tip), attrs, content
- **SkillBlock** — typed (Skill, Step, Verify, Precondition, OutputContract, Decision, Tool, Fallback, RedFlag, Example), attrs, optional title, content, children
- **Table** — attrs, optional caption, headers, rows
- **Figure** — attrs, optional caption, src
- **CodeBlock** — optional lang, attrs, code string
- **BlockQuote** — children blocks
- **List** — ordered flag, items (content + children)
- **ThematicBreak** — separator

### Inline
- Text, Emphasis, Strong, InlineCode, Link (text + url), Reference (target), Footnote, SoftBreak, HardBreak

## AIF Surface Syntax

```aif
#title: Document Title
#author: Name

@section[id=intro]: Introduction
  Paragraph with **bold**, *italic*, `code`.

  @claim
    A typed semantic block.
  @end

  @callout[type=note]
    An informational callout.
  @end
@end

@skill[name="debugging", version="1.0"]
  @precondition
    When to apply this skill.
  @end

  @step[order=1]
    First step.
  @end

  @verify
    How to validate.
  @end
@end
```

## LML Output Format

Tagged format optimized for LLM consumption. Uses `[TAG]...[/TAG]` delimiters with uppercase semantic tags.

Modes:
- **Standard** — Full tags: `[SECTION]`, `[STEP]`, `[PRECONDITION]`
- **Skill-Compact** — Standard but strips `@example` blocks
- **Conservative** — Abbreviated tags: `[S]`, `[ST]`, `[PRE]` with legend
- **Moderate** — Drops wrapper tags for single-child blocks, abbreviated tags
- **Aggressive** — Markdown-like with `@step:` prefixes, minimal delimiters

## Skill Profile System

Skills are imported from SKILL.md (Markdown with YAML frontmatter), mapped to semantic `SkillBlock` AST nodes. Features:
- SHA-256 integrity hashing
- Heading → SkillBlockType mapping (e.g., "## When to Use" → Precondition)
- Roundtrip: SKILL.md → AST → SKILL.md
- Manifest generation for skill registries

## Roadmap (Future Work)

- **PDF support** — Phase 2: PDF import via layout analysis
- **Skill versioning** — Semantic versioning with diff support
- **Skill chaining** — Compose skills via dependency graph
- **Chunk graphs** — Cross-document evidence linking
- **Marketplace** — Skill registry and discovery
- **Binary IR** — Compressed AST for bulk LLM ingestion
