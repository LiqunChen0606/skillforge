"""LLM-powered skill generation from a plain-English description.

Usage::

    from skillforge._generate import generate_skill
    skill_md = generate_skill("code review skill for React PRs")

Requires the ``anthropic`` package (optional dependency):
    pip install anthropic
"""

from __future__ import annotations

import os
import re

from skillforge import _fix

# ---------------------------------------------------------------------------
# System prompt
# ---------------------------------------------------------------------------

_SYSTEM_PROMPT = """\
You are SkillForge, an expert at writing SKILL.md files for AI coding agents.

A SKILL.md file is a structured Markdown document that teaches an AI agent how
to perform a specific task. You must produce a single, complete SKILL.md that
strictly follows the format below.

## SKILL.md Format

```
---
name: <kebab-case-name>
description: <one concise sentence, max 120 chars>
version: 1.0.0
---

# <Human-Readable Title>

## Precondition

<1-3 sentences: when this skill applies, what context is required>

## Steps

1. <First concrete, domain-specific step — not a placeholder>
2. <Second step>
3. <...continue for all steps — minimum 3, maximum 10>

## Verification

<Concrete criteria that confirm the skill worked. At least 3 bullet points
starting with "-". Each bullet should be specific and checkable.>

## Red Flags

<Optional section. Include only if there are important failure modes or
anti-patterns the agent should avoid. Use "-" bullets.>
```

## Rules you MUST follow

1. `name` MUST be kebab-case (lowercase letters, digits, hyphens only).
2. `description` MUST be one sentence, ≤ 120 characters.
3. `version` MUST be exactly `1.0.0`.
4. The `## Steps` section MUST use an ordered list (1. 2. 3. ...).
5. Each step MUST be concrete and domain-specific — NO generic placeholders
   like "TODO" or "describe the first step".
6. The `## Verification` section MUST contain at least 3 "-" bullet points.
7. Include `## Precondition` with when/where this skill applies.
8. Include `## Red Flags` only if there are real failure modes to call out.
9. Output ONLY the raw SKILL.md text — no code fences, no explanation,
   no preamble, no trailing commentary.
10. The very first character of your response MUST be `---` (the YAML
    frontmatter opening delimiter).
"""

_USER_TEMPLATE = "Generate a SKILL.md for: {prompt}"

_RETRY_USER_TEMPLATE = """\
The previous output failed validation with these errors:

{errors}

Please fix ALL of the above issues and regenerate the complete SKILL.md.
Remember: output ONLY the raw SKILL.md text starting with `---`.
"""

# ---------------------------------------------------------------------------
# API helpers
# ---------------------------------------------------------------------------


def _get_client(api_key: str | None, provider: str):
    """Import the Anthropic client lazily and return an initialized instance."""
    if provider != "anthropic":
        raise ValueError(
            f"Unsupported provider: {provider!r}. Only 'anthropic' is supported."
        )
    try:
        import anthropic
    except ImportError:
        raise ImportError(
            "The 'anthropic' package is required for skill generation.\n"
            "Install it with:  pip install anthropic"
        ) from None

    resolved_key = (
        api_key
        or os.environ.get("ANTHROPIC_API_KEY")
        or os.environ.get("AIF_LLM_API_KEY")
    )
    return anthropic.Anthropic(api_key=resolved_key)


def _call_llm(
    client,
    model: str,
    messages: list[dict],
) -> str:
    """Call the Anthropic Messages API and return the text response."""
    response = client.messages.create(
        model=model,
        max_tokens=2048,
        system=_SYSTEM_PROMPT,
        messages=messages,
    )
    # Extract text from the first content block.
    for block in response.content:
        if hasattr(block, "text"):
            return block.text
    return ""


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------


def _validate_skill_md(text: str) -> list[str]:
    """Run quick structural validation and return a list of error strings.

    This is intentionally lightweight — full AIF lint requires PyO3 bindings
    which may not be available in all test environments. The check covers the
    rules stated in the system prompt.
    """
    errors: list[str] = []

    # Must start with frontmatter
    if not text.lstrip().startswith("---"):
        errors.append("Missing YAML frontmatter (must start with '---')")
        return errors  # everything else will be garbled

    # Parse frontmatter
    fm_match = re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.DOTALL)
    if not fm_match:
        errors.append("Frontmatter block is not closed (missing closing '---')")
        return errors

    fm_body = fm_match.group(1)
    fields: dict[str, str] = {}
    for line in fm_body.splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            fields[k.strip()] = v.strip()

    # name
    name = fields.get("name", "")
    if not name:
        errors.append("Frontmatter missing 'name' field")
    elif not re.match(r"^[a-z0-9]+(?:-[a-z0-9]+)*$", name):
        errors.append(f"'name' must be kebab-case, got: {name!r}")

    # description
    desc = fields.get("description", "")
    if not desc:
        errors.append("Frontmatter missing 'description' field")
    elif len(desc) > 120:
        errors.append(f"'description' exceeds 120 chars ({len(desc)} chars)")

    # Required sections
    body = text[fm_match.end():]
    headings = set(re.findall(r"^##\s+(.+?)\s*$", body, re.MULTILINE))

    if "Steps" not in headings:
        errors.append("Missing '## Steps' section")
    else:
        # Must have at least 3 ordered list items
        steps_match = re.search(
            r"## Steps\s*\n(.*?)(?=\n## |\Z)", body, re.DOTALL
        )
        if steps_match:
            step_items = re.findall(r"^\d+\.", steps_match.group(1), re.MULTILINE)
            if len(step_items) < 3:
                errors.append(
                    f"'## Steps' must have at least 3 numbered items, found {len(step_items)}"
                )
            # Check for unfilled placeholders
            if "TODO" in steps_match.group(1):
                errors.append("'## Steps' contains unfilled TODO placeholders")

    if "Verification" not in headings:
        errors.append("Missing '## Verification' section")
    else:
        verify_match = re.search(
            r"## Verification\s*\n(.*?)(?=\n## |\Z)", body, re.DOTALL
        )
        if verify_match:
            bullets = re.findall(r"^-\s+\S", verify_match.group(1), re.MULTILINE)
            if len(bullets) < 3:
                errors.append(
                    f"'## Verification' must have at least 3 '-' bullets, found {len(bullets)}"
                )

    return errors


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def generate_skill(
    prompt: str,
    provider: str = "anthropic",
    model: str | None = None,
    api_key: str | None = None,
) -> str:
    """Generate a SKILL.md from a plain-English description.

    Args:
        prompt: Natural-language description of the skill to generate.
        provider: LLM provider (only ``"anthropic"`` is supported).
        model: Model identifier. Defaults to ``claude-sonnet-4-20250514``.
        api_key: API key. Falls back to ``ANTHROPIC_API_KEY`` /
            ``AIF_LLM_API_KEY`` environment variables.

    Returns:
        A valid SKILL.md string (post-processed through _fix.fix_skill_md).

    Raises:
        ImportError: If the ``anthropic`` package is not installed.
        ValueError: If the provider is unsupported or the LLM fails after
            one retry.
    """
    resolved_model = model or "claude-sonnet-4-20250514"
    client = _get_client(api_key, provider)

    # First attempt
    messages: list[dict] = [
        {"role": "user", "content": _USER_TEMPLATE.format(prompt=prompt)}
    ]
    raw = _call_llm(client, resolved_model, messages)
    skill_md, _ = _fix.fix_skill_md(raw)
    errors = _validate_skill_md(skill_md)

    if not errors:
        return skill_md

    # One retry with error feedback
    messages.append({"role": "assistant", "content": raw})
    messages.append({
        "role": "user",
        "content": _RETRY_USER_TEMPLATE.format(errors="\n".join(f"- {e}" for e in errors)),
    })
    raw2 = _call_llm(client, resolved_model, messages)
    skill_md2, _ = _fix.fix_skill_md(raw2)
    errors2 = _validate_skill_md(skill_md2)

    if errors2:
        # Return best effort, caller can check
        raise ValueError(
            f"Skill generation failed after retry. Remaining errors:\n"
            + "\n".join(f"  - {e}" for e in errors2)
        )

    return skill_md2
