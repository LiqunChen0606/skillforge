# Multi-Language SDK Design

## Goal

Generate Python and TypeScript SDKs from the AIF JSON Schema so that non-Rust consumers can read, create, validate, and serialize AIF documents with full type safety. SDKs stay in sync automatically — Rust types are the single source of truth.

## Current State

- `aif-core` derives `schemars::JsonSchema` on all AST types
- `aif schema` CLI command emits Draft-07 JSON Schema (655 lines)
- Schema covers 10 definitions: `Document`, `Block`, `BlockKind`, `Inline`, `Attrs`, `Span`, `ListItem`, `SemanticBlockType`, `CalloutType`, `SkillBlockType`
- All types also derive `serde::Serialize` + `serde::Deserialize` with `#[serde(tag = "type")]` on enums (internally tagged)
- JSON IR roundtrip already works via serde_json

## Codegen Approach Evaluation

### Option A: quicktype (JSON Schema → code)

- **Pros**: Mature, supports Python + TS + many languages, reads JSON Schema directly, handles tagged unions
- **Cons**: Generated code is verbose, limited customization, no built-in validation beyond types, Python output uses plain dicts (not idiomatic)

### Option B: typeshare (Rust → code)

- **Pros**: Reads Rust source directly (no schema intermediary), generates idiomatic TypeScript interfaces
- **Cons**: Python support is immature/missing, limited enum variant handling, requires `#[typeshare]` annotations on every type

### Option C: Custom codegen from JSON Schema (recommended)

- **Pros**: Full control over output idiom (pydantic for Python, zod for TS), schema is already generated, can emit validation + parse/serialize helpers, handles `#[serde(tag = "type")]` correctly
- **Cons**: More upfront work, must maintain codegen script

### Option D: Hand-written SDKs

- **Pros**: Maximum idiom, can add convenience methods
- **Cons**: Drift risk, maintenance burden, no single source of truth

### Recommendation: **Option C — Custom codegen from JSON Schema**

Rationale:
1. Schema already exists and is authoritative
2. `serde(tag = "type")` tagged unions need special handling that quicktype gets wrong for Python pydantic
3. Custom codegen lets us emit pydantic v2 models (Python) and zod schemas (TypeScript) — the idiomatic choices
4. A single `generate_sdks.py` script (~300-500 lines) is straightforward and fully automatable
5. Generated code can include `parse_document()` / `serialize_document()` helpers

## Architecture

```
┌─────────────┐     cargo run -- schema     ┌──────────────┐
│  Rust AST   │ ──────────────────────────► │ JSON Schema  │
│  (aif-core) │                             │  (Draft-07)  │
└─────────────┘                             └──────┬───────┘
                                                   │
                                         generate_sdks.py
                                                   │
                                    ┌──────────────┼──────────────┐
                                    ▼                             ▼
                            ┌──────────────┐            ┌──────────────┐
                            │  sdks/python/ │            │  sdks/ts/    │
                            │  pydantic v2  │            │  zod + types │
                            └──────────────┘            └──────────────┘
```

### Pipeline Steps

1. `cargo run -p aif-cli -- schema > schema.json`
2. `python scripts/generate_sdks.py schema.json`
3. Script reads schema, walks definitions, emits:
   - `sdks/python/aif/types.py` — pydantic models
   - `sdks/python/aif/parser.py` — `parse_document(json_str) → Document`, `serialize_document(doc) → json_str`
   - `sdks/ts/src/types.ts` — TypeScript interfaces + zod schemas
   - `sdks/ts/src/parser.ts` — `parseDocument(json: string): Document`, `serializeDocument(doc: Document): string`

## SDK Structure

### Python SDK — `sdks/python/`

```
sdks/python/
├── pyproject.toml          # Package config (aif-sdk)
├── aif/
│   ├── __init__.py         # Re-exports Document, parse_document, serialize_document
│   ├── types.py            # Generated pydantic v2 models
│   └── parser.py           # Generated parse/serialize helpers
└── tests/
    └── test_roundtrip.py   # Roundtrip: JSON → Document → JSON
```

**Key design decisions:**
- Pydantic v2 `BaseModel` for all struct types
- `Literal` discriminator for tagged unions (`BlockKind`, `Inline`)
- `Annotated[Union[...], Field(discriminator="type")]` for tagged enum fields
- Enums as `StrEnum` for `SemanticBlockType`, `CalloutType`, `SkillBlockType`
- `BTreeMap<String, String>` → `dict[str, str]`
- `Option<T>` → `T | None = None`

### TypeScript SDK — `sdks/ts/`

```
sdks/ts/
├── package.json            # Package config (@aif/sdk)
├── tsconfig.json
├── src/
│   ├── index.ts            # Re-exports
│   ├── types.ts            # Generated interfaces + zod schemas
│   └── parser.ts           # Generated parse/serialize helpers
└── tests/
    └── roundtrip.test.ts   # Roundtrip: JSON → Document → JSON
```

**Key design decisions:**
- TypeScript interfaces for structural types
- Zod schemas for runtime validation (`z.discriminatedUnion("type", [...])`)
- Tagged unions use `type` discriminator matching serde
- `BTreeMap<String, String>` → `Record<string, string>`
- `Option<T>` → `T | null`
- `usize` → `number`

## Type Mapping Table

| Rust Type | JSON Schema | Python | TypeScript |
|-----------|------------|--------|------------|
| `Document` | object | `class Document(BaseModel)` | `interface Document` + `documentSchema` |
| `Block` | object | `class Block(BaseModel)` | `interface Block` + `blockSchema` |
| `BlockKind` | oneOf (tagged) | `Annotated[Union[Section, Paragraph, ...], Field(discriminator="type")]` | `z.discriminatedUnion("type", [...])` |
| `Inline` | oneOf (tagged) | `Annotated[Union[Text, Emphasis, ...], Field(discriminator="type")]` | `z.discriminatedUnion("type", [...])` |
| `Attrs` | object | `class Attrs(BaseModel)` | `interface Attrs` + `attrsSchema` |
| `Span` | object | `class Span(BaseModel)` | `interface Span` + `spanSchema` |
| `ListItem` | object | `class ListItem(BaseModel)` | `interface ListItem` |
| `SemanticBlockType` | enum (string) | `class SemanticBlockType(StrEnum)` | `enum SemanticBlockType` + `z.nativeEnum(...)` |
| `CalloutType` | enum (string) | `class CalloutType(StrEnum)` | `enum CalloutType` + `z.nativeEnum(...)` |
| `SkillBlockType` | enum (string) | `class SkillBlockType(StrEnum)` | `enum SkillBlockType` + `z.nativeEnum(...)` |
| `String` | string | `str` | `string` |
| `usize` | integer | `int` | `number` |
| `bool` | boolean | `bool` | `boolean` |
| `Vec<T>` | array of T | `list[T]` | `T[]` |
| `Option<T>` | T or null | `T \| None = None` | `T \| null` |
| `BTreeMap<String, String>` | object (additionalProperties: string) | `dict[str, str]` | `Record<string, string>` |

## Tagged Union Handling

Rust uses `#[serde(tag = "type")]` which produces JSON like:

```json
{"type": "Paragraph", "content": [...]}
{"type": "Text", "text": "hello"}
```

### Python (pydantic v2)

```python
class Paragraph(BaseModel):
    type: Literal["Paragraph"] = "Paragraph"
    content: list[Inline]

class Section(BaseModel):
    type: Literal["Section"] = "Section"
    attrs: Attrs
    title: list[Inline]
    children: list[Block]

BlockKind = Annotated[
    Union[Section, Paragraph, SemanticBlock, Callout, Table, Figure,
          CodeBlock, BlockQuote, List, SkillBlock, ThematicBreak],
    Field(discriminator="type")
]
```

### TypeScript (zod)

```typescript
const paragraphSchema = z.object({
  type: z.literal("Paragraph"),
  content: z.array(inlineSchema),
});

const blockKindSchema = z.discriminatedUnion("type", [
  sectionSchema, paragraphSchema, semanticBlockSchema,
  calloutSchema, tableSchema, figureSchema, codeBlockSchema,
  blockQuoteSchema, listSchema, skillBlockSchema, thematicBreakSchema,
]);
```

## Build & Publish Workflow

### CI Integration

```yaml
# .github/workflows/sdk.yml
sdk-generate:
  steps:
    - cargo run -p aif-cli -- schema > schema.json
    - python scripts/generate_sdks.py schema.json
    - diff --brief sdks/ sdks-prev/  # Detect drift
    - cd sdks/python && pytest
    - cd sdks/ts && npm test

sdk-publish:  # On release tag
  steps:
    - cd sdks/python && python -m build && twine upload dist/*
    - cd sdks/ts && npm publish
```

### Keeping SDKs in Sync

1. **CI check**: On every PR that touches `crates/aif-core/src/ast.rs`, regenerate SDKs and fail if output differs from committed code
2. **Pre-commit hook** (optional): `cargo run -p aif-cli -- schema | python scripts/generate_sdks.py --check` — exits non-zero if SDKs are stale
3. **Version pinning**: SDK package versions track `workspace.package.version` from `Cargo.toml`

### Versioning Strategy

- SDK versions mirror the Rust workspace version (`0.1.0`)
- Breaking AST changes bump minor version (pre-1.0) in all packages simultaneously
- Schema is versioned implicitly via the `$schema` field

## What Each SDK Exposes

### Core API (both languages)

| Function | Purpose |
|----------|---------|
| `parse_document(json_string)` | Parse JSON string → validated `Document` |
| `serialize_document(document)` | Serialize `Document` → JSON string |
| `validate_document(json_string)` | Validate JSON against schema, return errors |

### Type Exports

- All struct types (`Document`, `Block`, `Attrs`, `Span`, `ListItem`)
- All enum types (`BlockKind`, `Inline`, `SemanticBlockType`, `CalloutType`, `SkillBlockType`)
- Individual variant types for tagged unions (`Paragraph`, `Section`, `Text`, `Emphasis`, etc.)

### Not Included (v1)

- AIF text format parsing (that requires the Rust parser)
- Binary format decode (Rust-only for now)
- Skill validation/hashing (Rust-only for now)
- HTML/Markdown/LML compilation (Rust-only)

These can be added later via WASM bindings if demand exists.

## Implementation Tasks

1. **Write `scripts/generate_sdks.py`** — Reads JSON Schema, emits Python + TypeScript
2. **Create `sdks/python/` scaffold** — pyproject.toml, __init__.py, generate types
3. **Create `sdks/ts/` scaffold** — package.json, tsconfig.json, generate types
4. **Add roundtrip tests** — JSON from Rust CLI → SDK parse → serialize → compare
5. **Add CI workflow** — Regenerate + diff + test on PR
6. **Document in CLAUDE.md** — SDK generation commands and structure

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Schema doesn't capture all serde behaviors | Test roundtrip with real Rust-serialized JSON fixtures |
| Tagged union handling breaks | Dedicated test cases for every BlockKind/Inline variant |
| SDK drift from Rust types | CI enforces regeneration check |
| Codegen script becomes complex | Keep it single-file, well-tested, ~500 lines |
| Python/TS ecosystem churn (pydantic v3, zod v4) | Pin major versions in SDK dependencies |
