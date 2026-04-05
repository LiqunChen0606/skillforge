---
description: Security scan (OWASP AST10 aligned) for a SKILL.md or .aif file
argument-hint: <path-to-skill-file>
allowed-tools:
  - Bash
  - Read
---

Run SkillForge's security scan on the specified skill file. Catches prompt-injection, hidden Unicode, dangerous tools, external fetches, privilege escalation, and data-exfiltration patterns.

## Steps

1. Run the scan:
   ```bash
   aif scan $ARGUMENTS
   ```

2. If findings exist, explain each one and suggest remediation:
   - **Critical / High** — must fix before deploying the skill
   - **Medium** — review; may be intentional (e.g., a documented `curl` instruction for a dev tool)
   - **Low** — informational

3. If clean, confirm no security findings and the skill is safe to publish.
