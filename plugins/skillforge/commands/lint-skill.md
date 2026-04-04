---
description: Lint an AIF skill file for structural quality
argument-hint: <path-to-skill.aif>
allowed-tools:
  - Bash
  - Read
---

Run SkillForge's 7-point structural lint on the specified skill file.

1. Run: `aif skill eval $ARGUMENTS --stage 1 --report text`
2. Present the results clearly — list passing and failing checks
3. If any checks fail, suggest specific fixes
4. If the file is a SKILL.md (Markdown), first import it: `aif skill import $ARGUMENTS -f json` then lint the JSON IR
