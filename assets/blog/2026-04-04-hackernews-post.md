# Hacker News / short-form posts

Three versions of the launch post, sized for different channels.

---

## HN title (pick one)

- **SkillForge – ESLint for Agent Skills (SKILL.md files)**
- **Show HN: A quality layer for SKILL.md files (lint, security scan, sign)**
- **Your SKILL.md has no linter. I built one.**

## HN body (~200 words, first comment)

Hi HN — I got tired of shipping Agent Skills with empty verify blocks, malformed names, and once, a literal "ignore previous instructions" I'd pasted as a test and forgotten to remove. The [Agent Skills standard](https://agentskills.io) is adopted by 30+ platforms (Claude Code, Cursor, etc.) and there's zero quality tooling — no ESLint, no bandit, no signing.

So I built one. `pip install aif-skillforge` then `aif check SKILL.md` runs 10 structural lints + 6 security checks (OWASP Agentic Skills Top 10 aligned) on your skill file. Exit 0 on pass, 1 on fail. Drops into pre-commit and GitHub Actions:

```yaml
- uses: LiqunChen0606/skillforge@v0.5.0
```

Also includes Ed25519 signing for skill integrity (useful when skills come from marketplaces), JUnit XML output for CI, and a Claude Code plugin so you can `/lint-skill` from inside a session.

**What's next:** if you're writing Agent Skills, I'd love your issues and PRs. If you're at Anthropic / Claude DevRel, I want this in official docs. If you're OWASP AST10, happy to upstream the scanner rules.

Repo: https://github.com/LiqunChen0606/skillforge
PyPI: https://pypi.org/project/aif-skillforge/

---

## X / Twitter thread (7 posts)

**1/** Writing Agent Skills for Claude Code, Cursor, etc.? They're just Markdown. No linter, no security scanner, no signing. Nothing stops you from shipping `## Steps: (empty)` to production.

I built SkillForge to fix that. 🧵

**2/** `pip install aif-skillforge`
`aif check SKILL.md`

10 structural lints + 6 security checks. Exit code 0/1. Done.

**3/** What it catches:

Structural: missing sections, invalid names, empty placeholder blocks, overlong descriptions, stale hashes.

Security (OWASP AST10 aligned): prompt injection, hidden Unicode, eval(), curl|bash, sudo patterns, credential harvesting.

**4/** Install once, run everywhere:

• Pre-commit hook (fires on every `SKILL.md` edit)
• GitHub Action (`LiqunChen0606/skillforge@v0.5.0`)
• Claude Code plugin (`/lint-skill` in any session)

**5/** Also: Ed25519 signing. When you publish a skill to a marketplace, consumers verify. Any byte change invalidates the signature.

```
aif skill sign my-skill.aif --key private.key
aif skill verify-signature ...
```

**6/** What it's NOT: a runtime, a skill DSL, or a replacement for human review. It's a pre-flight check. Catches the 80% of obvious issues that shouldn't reach production.

**7/** Try it:

repo → github.com/LiqunChen0606/skillforge
pypi → pypi.org/project/aif-skillforge
issues welcome, PRs welcome, single-author → fast turnaround

---

## LinkedIn (~300 words)

**Shipping AI skills with zero quality tooling is wild.**

If you're writing Agent Skills for Claude Code, Cursor, or any other tool that follows the [agentskills.io](https://agentskills.io) standard, you're publishing Markdown files to production with no linter, no security scanner, no signing. Nothing catches the missing sections, the malformed names, the prompt-injection strings you pasted as a test and forgot to remove.

Every other ecosystem has this solved. JavaScript has ESLint, Python has Ruff, Rust has Clippy. Agent Skills have nothing.

So I built SkillForge — `eslint` for SKILL.md files.

One command:

```
pip install aif-skillforge
aif check SKILL.md
```

10 structural lint checks (frontmatter, required sections, empty blocks, hash verification), plus 6 security checks aligned with the OWASP Agentic Skills Top 10 (prompt injection, hidden Unicode, dangerous tools, external fetches, privilege escalation, data exfiltration).

Drops into pre-commit hooks, GitHub Actions (`LiqunChen0606/skillforge@v0.5.0`), and Claude Code (`/plugin install LiqunChen0606/skillforge`). Ed25519 signing included for skill integrity verification.

It's early — I'm the only author, shipped last week. If you're writing skills professionally, I'd love your feedback.

**Try it:** https://github.com/LiqunChen0606/skillforge

**Full writeup:** [link to blog post]

#AgentSkills #ClaudeCode #DeveloperTools #AISecurity
