---
description: Lint a SKILL.md or .aif file for structural quality
argument-hint: <path-to-file>
allowed-tools:
  - Bash
  - Read
---

Run SkillForge's structural quality check on the specified skill file.

## Steps

1. Run the check command:
   ```
   aif check $ARGUMENTS --format text
   ```

2. If the command exits with a non-zero status, lint failures were found. Present the results clearly and suggest specific fixes for each failing check:
   - **Frontmatter**: Add `name` and `description` to the skill header
   - **RequiredSections**: Add `@step` and `@verify` blocks (or `## Steps` / `## Verification` sections in Markdown)
   - **BlockTypes**: Remove non-skill blocks from inside the `@skill` block
   - **VersionHash**: Run `aif skill rehash <file>` to recompute the hash
   - **DescriptionLength**: Shorten the description to under 1024 characters
   - **NameFormat**: Use only lowercase alphanumeric characters and hyphens in the skill name
   - **NoEmptyBlocks**: Fill in placeholder `@step` or `@verify` blocks with content

3. If all checks pass, confirm the file is clean.
