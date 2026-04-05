# SkillForge

### The quality layer for SKILL.md — lint, sign, test your [Agent Skills](https://agentskills.io)

> **SkillForge** is `eslint` / `rubocop` for AI skill files. Catch missing sections, empty steps, malformed metadata, prompt-injection patterns, and broken references **before** you deploy the skill. Sign skills with Ed25519 so consumers can detect tampering. Test skill compliance in CI with a JUnit report.

[![PyPI](https://img.shields.io/pypi/v/aif-skillforge?label=PyPI&color=blue)](https://pypi.org/project/aif-skillforge/)
![License](https://img.shields.io/badge/License-Apache--2.0%20%7C%20MIT-lightgrey)
![Agent Skills](https://img.shields.io/badge/Compatible-agentskills.io-blueviolet)

---

## 60-second quick start

```bash
pip install aif-skillforge
aif check SKILL.md       # lint + security scan
aif score SKILL.md       # letter grade (A+..F) with shareable badge
```

That's it. Output:

```
SkillForge Quality Check: SKILL.md
============================================================
  [+] Parsed SKILL.md (1 skill block)
  [+] Skill: code-review v1.0
  [+] Lint: 7/7 checks passed
  [+] Document lint: 10/10 checks passed
------------------------------------------------------------
PASS — SKILL.md is clean
```

## What it checks

**7 structural lint checks** — the things a reviewer would reject:

| Check | Catches |
|---|---|
| Frontmatter | Missing `name` or `description` |
| RequiredSections | No `@step` or `@verify` block — skill is aspirational |
| DescriptionLength | Description > 1024 chars (won't fit in context routing) |
| NameFormat | Invalid chars in skill name |
| NoEmptyBlocks | Placeholder steps / verify blocks that were never filled in |
| BlockTypes | Non-skill content mixed into the skill block |
| VersionHash | Hash doesn't match content (tampered, or stale) |

**6 security checks** (OWASP Agentic Skills Top 10 aligned):

| Rule | Catches |
|---|---|
| prompt-injection | "Ignore previous instructions" patterns |
| hidden-unicode | Zero-width characters, direction overrides |
| dangerous-tool | `eval`, `exec`, `rm -rf`, unrestricted shell |
| external-fetch | `curl url \| bash` style remote execution |
| privilege-escalation | `sudo`, admin requests, role manipulation |
| data-exfiltration | Credential-harvesting patterns |

Run `aif scan SKILL.md` for security-only output.

## Grade your skill with a shareable badge

```bash
aif score my-skill.md
```

Output:

```
SkillForge Score: my-skill.md
============================================================
  Score:  100/100  (A+)
  Lint:   0 errors, 0 warnings
  Security: 0 critical, 0 high, 0 medium, 0 low
------------------------------------------------------------
Grade: A+
```

Add a badge to your skill's README:

```bash
# Generate a Shields.io endpoint file, commit it to your repo
aif score my-skill.md --format shields -o badge.json
git add badge.json && git commit -m "add skillforge badge"
```

Then in your README:

```markdown
![SkillForge](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/USER/REPO/main/badge.json)
```

The badge auto-updates when you re-run `aif score` and re-commit the JSON. Grade mapping: A+ (97-100), A (93-96), A- (90-92), B+ (87-89), B (83-86), B- (80-82), C+ (77-79), C (73-76), C- (70-72), D (60-69), F (<60). An A+ means all 10 lint checks pass and zero security findings; an F means a critical security finding or multiple lint errors.

Also available: `--format svg` for a standalone SVG badge, `--format json` for CI integration, and `--min-grade C` to fail the command if the grade drops below a threshold.

## Install it once, run it everywhere

### Pre-commit hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/LiqunChen0606/skillforge
    rev: v0.6.4
    hooks:
      - id: aif-check
```

Now every commit that touches `SKILL.md` or `*.aif` gets linted automatically.

### GitHub Actions

`.github/workflows/skill-lint.yml`:

```yaml
on: [push, pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: LiqunChen0606/skillforge@v0.6.4
        with:
          path: ./skills
```

PRs that break skill quality fail CI before merge.

### Claude Code plugin

Add to your `~/.claude/settings.json`:

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

Or via slash commands (if your Claude Code version supports them):

```
/plugin marketplace add LiqunChen0606/skillforge
/plugin install skillforge@skillforge-marketplace
```

Then from any Claude Code session:

```
/lint-skill my-skill.md
/scan-skill my-skill.md
/sign-skill my-skill.md
/verify-skill my-skill.md
```

## Autofix

```bash
aif fix SKILL.md --write     # fix and overwrite the file
aif fix SKILL.md --check     # dry-run: show what would change, exit 1 if fixes needed
aif fix SKILL.md --diff      # print unified diff of proposed fixes
```

Fixes: NameFormat (kebab-case), missing frontmatter (scaffold name/description), DescriptionLength (truncate), RequiredSections (add stub `## Steps` / `## Verification`), NoEmptyBlocks (insert TODO placeholders).

## MCP server (Claude Desktop / Cursor)

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "skillforge": {
      "command": "aif",
      "args": ["mcp-server"]
    }
  }
}
```

Exposes 4 tools: `check_skill`, `score_skill`, `scan_skill`, `fix_skill`. Any MCP-compatible client can invoke them natively during a conversation.

## [Leaderboard](leaderboard/LEADERBOARD.md)

Weekly scan of public SKILL.md files on GitHub, graded by `aif score`, ranked top 25. Any public repo with a SKILL.md is automatically included.

## Sign skills for tamper detection

When you publish a skill, sign it. Consumers verify.

```bash
aif skill keygen                                        # Generate Ed25519 keypair (one time)
aif skill sign my-skill.aif --key ~/.aif/private.key    # Sign
aif skill verify-signature my-skill.aif \
    --signature <sig> --pubkey ~/.aif/public.key
```

Any byte change to the skill file invalidates the signature. Useful when skills come from untrusted marketplaces.

## Test skill compliance in CI

```bash
aif skill test my-skill.aif --format junit -o test-results.xml
```

Emits standard JUnit XML that GitHub Actions, Jenkins, CircleCI, etc. all display natively. Add `--baseline baseline.json` to catch regressions vs. a saved baseline.

## Python API

```python
import skillforge

# Lint
results = skillforge.lint(open("my-skill.aif").read())
# Security scan
findings = skillforge.scan(open("my-skill.aif").read())
# Sign / verify
priv, pub = skillforge.generate_keypair()
sig = skillforge.sign_skill(open("my-skill.aif").read(), priv)
```

Full walkthrough: [tutorial/skillforge_tutorial.ipynb](tutorial/skillforge_tutorial.ipynb).

---

## What SkillForge is NOT trying to be

- **Not a skill runtime** — it lints and signs, it doesn't execute skills
- **Not an LLM framework** — pairs with Claude Code, Cursor, any SKILL.md consumer
- **Not a replacement for SKILL.md** — works directly on your existing Markdown files

## Why this exists

The [Agent Skills standard](https://agentskills.io) is adopted by 30+ platforms, and skills are proliferating fast. Nobody has linting, signing, or CI for them. A broken skill doesn't throw errors — it just quietly makes your agent behave wrong. SkillForge fixes that.

---

## Research & advanced features

SkillForge is built on **AIF** (AI-native Interchange Format), a typed semantic IR for documents. The quality tools above are the production-ready surface. The underlying format has additional capabilities documented separately:

- **[AIF document format](docs/aif-format.md)** — typed blocks (`@claim`, `@evidence`, `@step`, `@red_flag`), HTML/PDF/Markdown import, LML output modes for LLM consumption
- **[Skill execution benchmark](benchmarks/skill-execution/)** — measures whether typed formats improve LLM compliance (spoiler: +4pp overall on 126 runs, claude-opus-4-6)
- **[Token comparison benchmark](benchmarks/document-tokens/)** — honest size comparison across raw HTML, cleaned text, Markdown, AIF LML (10 Wikipedia articles)
- **[Roundtrip tutorial](tutorial/skillforge_roundtrips.ipynb)** — HTML/Markdown/PDF → AIF → format with fidelity checks on live Wikipedia data

These are research / power-user capabilities. The 60-second quick start above is the supported path.

---

## License

Dual-licensed under Apache 2.0 OR MIT.

## Contributing

Issues and PRs welcome. If you're using SkillForge in production, a star on GitHub helps. If something's broken, file an issue — single-author project, fast turnaround.
