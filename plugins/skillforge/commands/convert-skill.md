---
description: Convert between AIF and Markdown skill formats
argument-hint: <input-file> [--to aif|md]
allowed-tools:
  - Bash
  - Read
  - Write
---

Convert a skill between AIF and Markdown formats.

If the input is a .md file:
1. Run: `aif skill import $ARGUMENTS -f json -o <output>.json`
2. Report the imported skill metadata (name, version, block count)

If the input is a .aif file:
1. Run: `aif skill export $ARGUMENTS -o <output>.md`
2. Report the exported skill structure

If `--to aif` is specified, compile to LML Aggressive:
1. Run: `aif compile $ARGUMENTS --format lml-aggressive`
