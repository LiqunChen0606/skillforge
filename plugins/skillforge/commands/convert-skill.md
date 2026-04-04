---
description: Convert between AIF and Markdown skill formats
argument-hint: <input-file> [--to aif|md]
allowed-tools:
  - Bash
  - Read
  - Write
---

Convert a skill file between AIF (.aif) and Markdown (.md) formats.

## Steps

### Markdown to AIF (input is .md, or --to aif)

1. Import the Markdown skill to AIF format:
   ```
   aif skill import $ARGUMENTS -f json
   ```
2. To get the optimized LML Aggressive format (recommended for LLM consumption):
   ```
   aif skill import $ARGUMENTS -f lml-aggressive
   ```
3. Report the imported skill metadata: name, version, number of blocks mapped.

### AIF to Markdown (input is .aif, or --to md)

1. Export the AIF skill to SKILL.md format:
   ```
   aif skill export $ARGUMENTS
   ```
2. Report the exported structure: section count, whether steps/verification were preserved.

### Verification

After conversion, run a quality check on the output to confirm nothing was lost:
```
aif check <output-file> --format text
```
