---
description: Run SkillForge quality check (lint + security scan) on a SKILL.md or .aif file
argument-hint: <path-to-skill-file>
allowed-tools:
  - Bash
  - Read
---

Run SkillForge's one-command quality check on the specified skill file.

## Setup (first run only)

If `aif` is not installed, install it:

```bash
pip install aif-skillforge
```

## Steps

1. Run the check:
   ```bash
   aif check $ARGUMENTS
   ```

2. If the command exits non-zero, it failed either lint or security. Present the output clearly and suggest concrete fixes based on which checks failed:

   **Lint failures:**
   - **Frontmatter** → Add `name` and `description` YAML frontmatter
   - **RequiredSections** → Add `## Steps` / `## Verification` sections (or `@step` / `@verify` blocks)
   - **NameFormat** → Use only lowercase alphanumeric + hyphens in the skill name
   - **DescriptionLength** → Shorten description to ≤1024 chars
   - **NoEmptyBlocks** → Fill in placeholder step/verify content
   - **BlockTypes** → Remove non-skill blocks from inside the skill body

   **Security findings (OWASP AST10):**
   - **prompt-injection** → Remove phrases like "ignore previous instructions"
   - **hidden-unicode** → Strip zero-width chars / direction overrides
   - **dangerous-tool** → Replace `eval`, unrestricted shell with safe alternatives
   - **external-fetch** → Remove `curl URL | bash` patterns
   - **privilege-escalation** → Remove sudo / admin requests
   - **data-exfiltration** → Remove credential-harvesting patterns

3. If all checks pass, confirm the file is clean and ready to deploy.
