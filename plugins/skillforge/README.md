# SkillForge — Claude Code Plugin

**Quality layer for SKILL.md.** Lint, security-scan, and sign your Agent Skill files from inside Claude Code.

## Install

1. Install the `aif` CLI (Python wheel, no Rust required):
   ```bash
   pip install aif-skillforge
   ```
2. Add to your `~/.claude/settings.json`:
   ```json
   {
     "extraKnownMarketplaces": {
       "skillforge-marketplace": {
         "source": {
           "source": "url",
           "url": "https://github.com/LiqunChen0606/skillforge.git"
         }
       }
     },
     "enabledPlugins": {
       "skillforge@skillforge-marketplace": true
     }
   }
   ```
   Or via slash commands (Claude Code 1.x+):
   ```
   /plugin marketplace add LiqunChen0606/skillforge
   /plugin install skillforge@skillforge-marketplace
   ```

## Commands

| Command | What it does |
|---|---|
| `/lint-skill <path>` | One-command quality check: 10 structural lint + 6 security checks |
| `/scan-skill <path>` | Security-only scan (OWASP Agentic Skills Top 10 aligned) |
| `/sign-skill <path>` | Sign a skill with Ed25519 so consumers can detect tampering |
| `/verify-skill <path>` | Verify integrity hash + Ed25519 signature |
| `/convert-skill <path> [--to aif\|md]` | Convert SKILL.md ↔ .aif |

## Example

```
/lint-skill skills/code-review.md
```

Output:

```
SkillForge Quality Check: skills/code-review.md
============================================================
  [+] Parsed
  [+] Lint: 10 checks passed
  [+] Security: clean
------------------------------------------------------------
PASS — skills/code-review.md is clean
```

When a skill has issues, Claude Code will explain each one and suggest a concrete fix.

## What it catches

**Structural lint (10 checks):** Missing frontmatter, missing `## Steps` / `## Verification`, invalid name format, overlong descriptions, empty placeholder blocks, mixed block types, hash mismatches, claims without evidence, broken references, undefined terms.

**Security scan (6 rules, OWASP AST10 aligned):** Prompt injection ("ignore previous instructions"), hidden Unicode (zero-width chars), dangerous tools (`eval`, `rm -rf`), external fetches (`curl url | bash`), privilege escalation (sudo / admin requests), data exfiltration (credential harvesting).

## Why

The [Agent Skills standard](https://agentskills.io) is adopted by 30+ platforms. SKILL.md has no native quality tooling — broken skills fail silently at runtime, quietly misbehaving. SkillForge adds `eslint`-level rigor.

## Links

- Repo: https://github.com/LiqunChen0606/skillforge
- PyPI: https://pypi.org/project/aif-skillforge/
- Pre-commit hook + GitHub Action also available
