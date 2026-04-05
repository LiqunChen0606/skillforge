---
description: Grade a SKILL.md or .aif file A+..F and optionally generate a shareable badge
argument-hint: <path-to-skill-file> [--min-grade B] [--format svg|shields]
allowed-tools:
  - Bash
  - Read
---

Compute a quality score for a skill file. Scoring starts at 100, deducts points for each lint warning (-3), lint error (-7), and security finding (-1 to -15 by severity). Maps to a letter grade A+..F.

## Steps

1. Run the score:
   ```bash
   aif score $ARGUMENTS
   ```

2. Report the grade and any deductions. If the user wants to improve their grade, explain what the biggest deductions are and suggest fixes.

3. If the user asks for a shareable badge, run:
   ```bash
   aif score <file> --format shields -o badge.json
   ```
   And tell them to commit `badge.json` then reference it in their README:
   ```markdown
   ![SkillForge](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/USER/REPO/main/badge.json)
   ```

4. If they want a standalone SVG instead (no Shields.io dependency), run:
   ```bash
   aif score <file> --format svg -o badge.svg
   ```

## Grade thresholds

- **A+**: 97-100 — all checks clean
- **A**: 93-96
- **A-**: 90-92
- **B+/B/B-**: 80s — minor issues
- **C+/C/C-**: 70s — multiple issues, review before deploy
- **D**: 60-69 — significant problems
- **F**: <60 — critical security finding or multiple blocker-level lint errors
