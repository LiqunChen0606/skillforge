"""Autofix for SKILL.md files — mechanical fixes for deterministic lint rules.

Fixable rules (safe, deterministic):
- **NameFormat**: normalize the skill name to kebab-case (lowercase, hyphens)
- **DescriptionLength**: truncate descriptions > 1024 chars with ellipsis
- **Frontmatter** (partial): scaffold placeholder `name:` / `description:` when missing
- **NoEmptyBlocks** (partial): insert `TODO: <rule>` placeholder in empty steps/verify
- **RequiredSections** (partial): add stub `## Steps` / `## Verification` if missing

Unsafe / not attempted (needs semantic judgment):
- **BlockTypes**: would require restructuring
- **VersionHash**: requires running `aif skill rehash` (wraps the Rust CLI)
- Security findings: never auto-fixed

`fix_skill_md(text)` returns (fixed_text, list_of_applied_fixes).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Optional


MAX_DESCRIPTION_LEN = 1024
KEBAB_CASE_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
FRONTMATTER_RE = re.compile(r"^---\s*\n(.*?)\n---\s*\n", re.DOTALL)
HEADING_RE = re.compile(r"^##\s+(\w.*?)\s*$", re.MULTILINE)


@dataclass
class FixApplied:
    rule: str
    description: str


def _kebab_case(name: str) -> str:
    """Convert an arbitrary string to kebab-case."""
    # Replace underscores, slashes, colons, camelCase boundaries with hyphens
    s = re.sub(r"[_/:.\s]+", "-", name.strip())
    s = re.sub(r"([a-z])([A-Z])", r"\1-\2", s)
    s = s.lower()
    s = re.sub(r"[^a-z0-9-]", "", s)  # drop anything non-alphanumeric
    s = re.sub(r"-+", "-", s).strip("-")
    return s or "unnamed-skill"


def _parse_frontmatter(text: str) -> tuple[dict[str, str], str, str]:
    """Return (fields, frontmatter_block, body). If no frontmatter, fields={}."""
    m = FRONTMATTER_RE.match(text)
    if not m:
        return {}, "", text
    block = m.group(0)
    body = text[m.end():]
    fields: dict[str, str] = {}
    for line in m.group(1).splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            fields[k.strip()] = v.strip()
    return fields, block, body


def _format_frontmatter(fields: dict[str, str]) -> str:
    lines = ["---"]
    # Preferred order: name, description, version, then everything else alphabetic
    preferred = ["name", "description", "version"]
    seen = set()
    for k in preferred:
        if k in fields:
            lines.append(f"{k}: {fields[k]}")
            seen.add(k)
    for k in sorted(fields):
        if k not in seen:
            lines.append(f"{k}: {fields[k]}")
    lines.append("---")
    lines.append("")
    return "\n".join(lines)


def fix_skill_md(text: str) -> tuple[str, list[FixApplied]]:
    """Apply all available fixes to a SKILL.md string.

    Returns the fixed text and a list of what was changed. Never modifies
    content that would require semantic judgment.
    """
    fixes: list[FixApplied] = []
    fields, frontmatter, body = _parse_frontmatter(text)

    # --- Fix: Frontmatter scaffolding ------------------------------
    if "name" not in fields or not fields["name"].strip():
        # Infer from first H1 in body, or use placeholder
        m = re.search(r"^#\s+(.+?)$", body, re.MULTILINE)
        candidate = m.group(1).strip() if m else "unnamed-skill"
        fields["name"] = _kebab_case(candidate)
        fixes.append(FixApplied(
            rule="Frontmatter",
            description=f"Added missing 'name' field: {fields['name']}",
        ))

    if "description" not in fields or not fields["description"].strip():
        fields["description"] = "TODO: write a one-line description"
        fixes.append(FixApplied(
            rule="Frontmatter",
            description="Added placeholder 'description' field",
        ))

    # --- Fix: NameFormat (kebab-case) ------------------------------
    original_name = fields.get("name", "")
    if original_name and not KEBAB_CASE_RE.match(original_name):
        new_name = _kebab_case(original_name)
        if new_name and new_name != original_name:
            fields["name"] = new_name
            fixes.append(FixApplied(
                rule="NameFormat",
                description=f"Renamed '{original_name}' → '{new_name}' (kebab-case)",
            ))

    # --- Fix: DescriptionLength (truncate) -------------------------
    desc = fields.get("description", "")
    if len(desc) > MAX_DESCRIPTION_LEN:
        truncated = desc[: MAX_DESCRIPTION_LEN - 3].rstrip() + "..."
        fields["description"] = truncated
        fixes.append(FixApplied(
            rule="DescriptionLength",
            description=f"Truncated description from {len(desc)} to {len(truncated)} chars",
        ))

    # --- Fix: NoEmptyBlocks (placeholder TODOs) --------------------
    # Find `## Steps` and `## Verification` sections; if the next non-heading
    # line is empty or missing, insert a TODO placeholder.
    body = _fill_empty_sections(body, fixes)

    # --- Fix: RequiredSections (add stubs) -------------------------
    body, added_sections = _add_missing_sections(body)
    for section in added_sections:
        fixes.append(FixApplied(
            rule="RequiredSections",
            description=f"Added stub '## {section}' section",
        ))

    result = _format_frontmatter(fields) + body.lstrip("\n")
    return result, fixes


def _fill_empty_sections(body: str, fixes: list[FixApplied]) -> str:
    """Insert a TODO placeholder when `## Steps` / `## Verification` are empty."""
    lines = body.splitlines()
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        out.append(line)
        m = re.match(r"^##\s+(Steps|Verification|Precondition|Output Contract)\s*$", line)
        if m:
            # Look ahead for content before next heading or EOF
            section_name = m.group(1)
            j = i + 1
            has_content = False
            while j < len(lines):
                next_line = lines[j].strip()
                if next_line.startswith("## ") or next_line.startswith("# "):
                    break
                if next_line and next_line not in ("TODO", "TODO:"):
                    has_content = True
                    break
                j += 1
            if not has_content:
                placeholder = {
                    "Steps": "1. TODO: describe the first step",
                    "Verification": "TODO: describe how to verify the skill worked",
                    "Precondition": "TODO: describe when this skill applies",
                    "Output Contract": "TODO: describe the expected output",
                }.get(section_name, "TODO: fill in")
                out.append("")
                out.append(placeholder)
                fixes.append(FixApplied(
                    rule="NoEmptyBlocks",
                    description=f"Filled empty '## {section_name}' with TODO",
                ))
        i += 1
    return "\n".join(out)


def _add_missing_sections(body: str) -> tuple[str, list[str]]:
    """If `## Steps` or `## Verification` are missing entirely, append stubs."""
    existing = set(m.group(1) for m in HEADING_RE.finditer(body))
    added: list[str] = []
    suffix_parts: list[str] = []

    if "Steps" not in existing:
        suffix_parts.append("\n## Steps\n\n1. TODO: describe the first step\n")
        added.append("Steps")
    if "Verification" not in existing:
        suffix_parts.append("\n## Verification\n\nTODO: describe how to verify the skill worked\n")
        added.append("Verification")

    if suffix_parts:
        body = body.rstrip() + "\n" + "".join(suffix_parts)
    return body, added


def diff_fixes(original: str, fixed: str) -> str:
    """Produce a unified diff between original and fixed text."""
    import difflib
    lines = difflib.unified_diff(
        original.splitlines(keepends=True),
        fixed.splitlines(keepends=True),
        fromfile="before",
        tofile="after",
        lineterm="",
    )
    return "".join(lines)
