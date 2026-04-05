# Awesome-list submissions — ready to paste

Submit to the lists in this order (highest stars first). Each one has the exact URL, form fields, and entry text pre-filled.

---

## 1. `hesreallyhim/awesome-claude-code` (36.5k ⭐)

**How to submit:** GitHub Issue form (they don't accept PRs for additions).

**Direct link:**
https://github.com/hesreallyhim/awesome-claude-code/issues/new?template=recommend-resource.yml

**Form fields:**

- **Display Name**: SkillForge
- **Category**: Tooling
- **Sub-Category**: General
- **Primary Link**: https://github.com/LiqunChen0606/skillforge
- **Author Name**: Liqun Chen
- **Author Link**: https://github.com/LiqunChen0606
- **License**: Apache-2.0 OR MIT
- **Description**: Quality layer for SKILL.md — lints Agent Skill files for structural issues and OWASP-aligned security patterns (prompt injection, hidden Unicode, dangerous tools). Ships as a pip-installable CLI, pre-commit hook, GitHub Action, and Claude Code plugin.

---

## 2. `travisvn/awesome-claude-skills` (10.5k ⭐)

**How to submit:** Check their CONTRIBUTING.md — typically PR against README.md.

**Direct link to fork + edit:**
https://github.com/travisvn/awesome-claude-skills/edit/main/README.md

**Entry (paste into appropriate section — likely "Tools" or "Quality"):**

```markdown
- [SkillForge](https://github.com/LiqunChen0606/skillforge) — Lint, security-scan, and sign SKILL.md files. Pip-installable (`pip install aif-skillforge`), works as pre-commit hook + GitHub Action + Claude Code plugin. 10 structural lint checks, 6 OWASP AST10-aligned security checks, Ed25519 signing.
```

---

## 3. `ccplugins/awesome-claude-code-plugins` (666 ⭐)

**How to submit:** PR against README.md, add under "Code Quality Testing" section.

**Direct link to fork + edit:**
https://github.com/ccplugins/awesome-claude-code-plugins/edit/main/README.md

**Entry (paste under `### Code Quality Testing`):**

```markdown
- [skillforge](https://github.com/LiqunChen0606/skillforge) — Lint and security-scan SKILL.md / .aif Agent Skill files. Catches missing sections, empty steps, prompt-injection patterns, broken references. Ships as pip-installable CLI, pre-commit hook, and GitHub Action alongside the Claude Code plugin.
```

**PR title:** `Add skillforge plugin (SKILL.md quality checker)`

---

## 4. `Prat011/awesome-llm-skills` (1.0k ⭐)

**How to submit:** PR against README.md.

**Direct link:**
https://github.com/Prat011/awesome-llm-skills/edit/main/README.md

**Entry:**

```markdown
- [SkillForge](https://github.com/LiqunChen0606/skillforge) — Quality layer for SKILL.md files. Lint, OWASP-aligned security scan, Ed25519 signing. Works with Claude Code, Cursor, and any agentskills.io-compliant tool.
```

---

## 5. (Lower-priority) Other lists worth a one-line PR

These are smaller but active. Batch them if you want:

| Repo | Stars | URL |
|---|---|---|
| `rohitg00/awesome-claude-code-toolkit` | 1.0k | https://github.com/rohitg00/awesome-claude-code-toolkit |
| `jqueryscript/awesome-claude-code` | 245 | https://github.com/jqueryscript/awesome-claude-code |
| `LangGPT/awesome-claude-code` | (new) | https://github.com/LangGPT/awesome-claude-code |

Same entry as #2 above works for these.

---

## Submission tips

1. **Do `hesreallyhim/awesome-claude-code` first** — 36.5k stars, biggest reach. It's an issue form, takes 2 minutes.
2. **Space PRs ~2-3 per week** across the lists. Don't submit 5 at once from the same day — looks spammy.
3. **Watch the PRs for maintainer feedback**. Some maintainers want you to add entries alphabetically, some want you to update table-of-contents counts, etc.
4. **Once any of these merge**, cross-link back from your README: "Listed in [awesome-claude-code](https://github.com/hesreallyhim/awesome-claude-code)".

---

## Template PR description (for any of the above)

> ## What
> Adds SkillForge to the list under **[Tooling / Quality / Plugins]**.
>
> ## What SkillForge does
> Quality layer for SKILL.md files — 10 structural lint checks + 6 OWASP AST10-aligned security checks. Pip-installable (`pip install aif-skillforge`), also available as pre-commit hook, GitHub Action, and Claude Code plugin.
>
> ## Links
> - Repo: https://github.com/LiqunChen0606/skillforge
> - PyPI: https://pypi.org/project/aif-skillforge/
> - GitHub Action: https://github.com/marketplace/actions/skillforge-skill-md-quality-check
> - License: Apache-2.0 OR MIT
