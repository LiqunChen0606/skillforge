# Binary IR + Versioning — Design Document

**Date:** 2026-03-31
**Status:** Approved

## Overview

Phase 1 of the AIF future work roadmap. Two independent features that can be built in parallel:

1. **Binary IR** (`aif-binary` crate) — Two encoding modes for different consumers
2. **Skill Versioning** (`aif-skill` extensions) — Semver + content-addressable hashing with semantic diff

## Binary IR

### Two Modes

| Mode | Encoding | Purpose | Consumers |
|------|----------|---------|-----------|
| **Wire** | postcard (serde) | Tool-to-tool transfer, bulk pipeline ingestion | Rust-native (v1) |
| **Token-optimized** | Custom compact binary | Minimize LLM token count below Markdown baseline | LLMs via base64 or raw bytes |

### Wire Format (postcard)

- Leverages existing `#[derive(Serialize, Deserialize)]` on AST types
- postcard is a no_std-friendly, compact binary format (~2x smaller than JSON)
- Near-zero implementation cost: `postcard::to_allocvec(&doc)` / `postcard::from_bytes(&bytes)`
- Decode function for roundtrip verification

### Token-Optimized Format

- Header dictionary: maps repeated tag/field names to single-byte IDs
- Varint-encoded block lengths (no closing tags needed)
- Field names omitted (positional encoding within block types)
- Text content stored as raw UTF-8 (no escaping overhead)
- Goal: beat Markdown on token count while preserving full semantic structure

### Crate Structure

```
crates/aif-binary/
  Cargo.toml
  src/
    lib.rs           # render_wire(), render_token_optimized(), decode_wire()
    wire.rs          # postcard serialization/deserialization
    token_opt.rs     # custom compact format encoder
    dictionary.rs    # tag name -> byte ID mapping, header generation
```

### CLI Integration

```bash
aif compile input.aif -f binary-wire [-o output]
aif compile input.aif -f binary-token [-o output]
aif skill import input.md --format binary-wire
aif skill import input.md --format binary-token
```

## Skill Versioning

### Dual Identity

| Component | What | Where |
|-----------|------|-------|
| `version` attr | Semver string (e.g. `1.2.0`) | Skill block attrs |
| `hash` attr | SHA-256 content hash (existing) | Skill block attrs |

Semver is for humans ("use v2.1"). Hash is for machines ("is this the same skill?"). Same model as npm/cargo.

### Semantic Diff

`aif skill diff old.aif new.aif` — Block-level AST comparison with change classification.

**Change Classifications:**

| Classification | Examples | Semver Impact |
|---------------|----------|---------------|
| **Breaking** | Removed step, changed precondition, removed verify | Major bump |
| **Additive** | New step, new example, new fallback | Minor bump |
| **Cosmetic** | Rewording text, reordering within block | Patch bump |

**Diff output:** Structured report listing which blocks were added/removed/modified, with classification per change.

### Auto Bump

`aif skill bump input.aif` — Computes diff against previous version (by hash), determines highest-severity change, bumps version accordingly.

- `--dry-run` flag shows what would change without modifying the file
- Updates both `version` and `hash` attrs

### Implementation Location

Extensions to existing `aif-skill` crate:

```
crates/aif-skill/src/
  version.rs     # semver parsing, bump logic
  diff.rs        # block-level AST diff
  classify.rs    # change classification (breaking/additive/cosmetic)
```

### CLI Integration

```bash
aif skill diff old.aif new.aif [--format json|text]
aif skill bump input.aif [--dry-run]
```

## Benchmark Integration

Add `binary-wire` and `binary-token` to the existing benchmark suite. The token-optimized format is the key metric — we expect it to beat Markdown on raw token count.

## Execution Order

Binary IR and Versioning are independent. Can be built in parallel or sequentially. Recommended: Binary IR first (higher novelty, benchmark-measurable), then Versioning.
