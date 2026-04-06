"""Tests for skillforge._generate.

All tests mock the Anthropic API — no real HTTP calls are made.
"""

from __future__ import annotations

import sys
from types import SimpleNamespace
from unittest.mock import MagicMock, patch

import pytest

from skillforge import _generate


# ---------------------------------------------------------------------------
# Sample LLM responses
# ---------------------------------------------------------------------------

VALID_SKILL_MD = """\
---
name: react-pr-code-review
description: Review React pull requests for correctness, performance, and best practices.
version: 1.0.0
---

# React PR Code Review

## Precondition

Use this skill when reviewing a pull request that modifies React components,
hooks, or related TypeScript/JavaScript code.

## Steps

1. Check component prop types and ensure they are explicitly typed with TypeScript interfaces.
2. Review hooks usage: verify that useEffect dependencies are complete and correct.
3. Look for unnecessary re-renders by checking whether memoization (useMemo, useCallback) is applied appropriately.
4. Verify that error boundaries are present for components that fetch data or render dynamic content.
5. Confirm that accessibility attributes (aria-*, role, alt) are present on interactive and image elements.

## Verification

- All prop types are explicitly defined with TypeScript interfaces or PropTypes.
- No ESLint warnings remain in the changed files.
- useEffect dependency arrays are complete (no missing deps warning).
- Interactive elements have appropriate aria labels or roles.

## Red Flags

- Inline function definitions inside render that create new references on every render.
- Direct DOM mutations bypassing React state.
- Using index as the key prop in dynamic lists.
"""

MISSING_STEPS_SKILL_MD = """\
---
name: bad-skill
description: A skill missing steps.
version: 1.0.0
---

# Bad Skill

## Precondition

Some precondition here.

## Verification

- Check one.
- Check two.
- Check three.
"""

TOO_FEW_VERIFICATION_BULLETS = """\
---
name: react-pr-code-review
description: Review React pull requests for correctness and best practices.
version: 1.0.0
---

# React PR Code Review

## Precondition

Use when reviewing a React PR.

## Steps

1. Check prop types.
2. Review hooks.
3. Verify accessibility.

## Verification

- Props are typed.
"""


# ---------------------------------------------------------------------------
# Helper: build a minimal mock Anthropic client
# ---------------------------------------------------------------------------


def _make_mock_client(response_text: str) -> MagicMock:
    block = SimpleNamespace(text=response_text)
    message = SimpleNamespace(content=[block])
    client = MagicMock()
    client.messages.create.return_value = message
    return client


# ---------------------------------------------------------------------------
# Test: system prompt structure
# ---------------------------------------------------------------------------


def test_system_prompt_contains_required_sections():
    """The system prompt must instruct the LLM to produce all required sections."""
    sp = _generate._SYSTEM_PROMPT
    assert "## Steps" in sp
    assert "## Verification" in sp
    assert "## Precondition" in sp
    assert "kebab-case" in sp
    assert "1.0.0" in sp
    # Must tell LLM to output raw text only
    assert "ONLY" in sp or "only" in sp


# ---------------------------------------------------------------------------
# Test: _validate_skill_md
# ---------------------------------------------------------------------------


def test_validate_valid_skill_passes():
    errors = _generate._validate_skill_md(VALID_SKILL_MD)
    assert errors == [], f"Expected no errors, got: {errors}"


def test_validate_missing_steps_section():
    errors = _generate._validate_skill_md(MISSING_STEPS_SKILL_MD)
    assert any("Steps" in e for e in errors), errors


def test_validate_too_few_verification_bullets():
    errors = _generate._validate_skill_md(TOO_FEW_VERIFICATION_BULLETS)
    assert any("Verification" in e and "bullet" in e.lower() for e in errors), errors


def test_validate_non_kebab_name():
    bad = VALID_SKILL_MD.replace("name: react-pr-code-review", "name: React PR Code Review")
    errors = _generate._validate_skill_md(bad)
    assert any("kebab" in e.lower() for e in errors), errors


def test_validate_missing_frontmatter():
    errors = _generate._validate_skill_md("# No frontmatter here\n\n## Steps\n1. A\n")
    assert any("frontmatter" in e.lower() for e in errors), errors


# ---------------------------------------------------------------------------
# Test: generate_skill with a mock client — happy path
# ---------------------------------------------------------------------------


def test_generate_skill_happy_path():
    """generate_skill should return fixed skill text when the LLM produces a valid response."""
    mock_client = _make_mock_client(VALID_SKILL_MD)

    with patch.object(_generate, "_get_client", return_value=mock_client):
        result = _generate.generate_skill("code review skill for React PRs")

    # Must start with frontmatter
    assert result.startswith("---"), repr(result[:80])
    # Must contain key sections
    assert "## Steps" in result
    assert "## Verification" in result
    # mock_client.messages.create called once (no retry needed)
    assert mock_client.messages.create.call_count == 1


# ---------------------------------------------------------------------------
# Test: generate_skill retries on validation failure
# ---------------------------------------------------------------------------


def test_generate_skill_retries_on_bad_output():
    """If the first response is invalid, generate_skill retries once."""
    # First call returns invalid output; second call returns valid output.
    block_bad = SimpleNamespace(text=MISSING_STEPS_SKILL_MD)
    block_good = SimpleNamespace(text=VALID_SKILL_MD)
    msg_bad = SimpleNamespace(content=[block_bad])
    msg_good = SimpleNamespace(content=[block_good])

    mock_client = MagicMock()
    mock_client.messages.create.side_effect = [msg_bad, msg_good]

    with patch.object(_generate, "_get_client", return_value=mock_client):
        result = _generate.generate_skill("code review skill for React PRs")

    # Should have called the API twice
    assert mock_client.messages.create.call_count == 2
    assert "## Steps" in result


# ---------------------------------------------------------------------------
# Test: generate_skill raises ValueError after two bad responses
# ---------------------------------------------------------------------------


def test_generate_skill_raises_after_two_failures():
    """If both attempts produce invalid output, generate_skill raises ValueError."""
    mock_client = _make_mock_client(MISSING_STEPS_SKILL_MD)
    # Both calls return the bad response
    mock_client.messages.create.return_value = SimpleNamespace(
        content=[SimpleNamespace(text=MISSING_STEPS_SKILL_MD)]
    )

    with patch.object(_generate, "_get_client", return_value=mock_client):
        with pytest.raises(ValueError, match="failed after retry"):
            _generate.generate_skill("code review skill for React PRs")

    assert mock_client.messages.create.call_count == 2


# ---------------------------------------------------------------------------
# Test: post-processing passes through _fix.fix_skill_md
# ---------------------------------------------------------------------------


def test_generate_skill_applies_fix():
    """Even a valid response is post-processed by fix_skill_md (e.g. bad name fixed)."""
    # Name is not kebab-case — fix_skill_md should normalize it
    bad_name_skill = VALID_SKILL_MD.replace(
        "name: react-pr-code-review", "name: React_PR_Code_Review!"
    )
    mock_client = _make_mock_client(bad_name_skill)

    with patch.object(_generate, "_get_client", return_value=mock_client):
        result = _generate.generate_skill("code review skill for React PRs")

    # After fix_skill_md the name should be kebab-case
    import re
    fm_match = re.search(r"name:\s+(.+)", result)
    assert fm_match is not None
    name_val = fm_match.group(1).strip()
    assert re.match(r"^[a-z0-9]+(?:-[a-z0-9]+)*$", name_val), f"Not kebab-case: {name_val!r}"


# ---------------------------------------------------------------------------
# Test: unsupported provider raises ValueError
# ---------------------------------------------------------------------------


def test_unsupported_provider_raises():
    with pytest.raises(ValueError, match="Unsupported provider"):
        _generate.generate_skill("anything", provider="openai")


# ---------------------------------------------------------------------------
# Test: missing anthropic package shows helpful error
# ---------------------------------------------------------------------------


def test_missing_anthropic_package_raises_import_error(monkeypatch):
    """When anthropic is not installed, a clear ImportError is raised."""
    import builtins
    real_import = builtins.__import__

    def mock_import(name, *args, **kwargs):
        if name == "anthropic":
            raise ImportError("No module named 'anthropic'")
        return real_import(name, *args, **kwargs)

    with patch("builtins.__import__", side_effect=mock_import):
        with pytest.raises(ImportError, match="pip install anthropic"):
            _generate._get_client(api_key=None, provider="anthropic")


# ---------------------------------------------------------------------------
# Test: user_template includes the prompt
# ---------------------------------------------------------------------------


def test_user_template_includes_prompt():
    prompt = "code review skill for React PRs"
    rendered = _generate._USER_TEMPLATE.format(prompt=prompt)
    assert prompt in rendered


# ---------------------------------------------------------------------------
# Test: retry user template includes error text
# ---------------------------------------------------------------------------


def test_retry_template_includes_errors():
    errors = ["Missing '## Steps' section", "Frontmatter missing 'name' field"]
    rendered = _generate._RETRY_USER_TEMPLATE.format(
        errors="\n".join(f"- {e}" for e in errors)
    )
    assert "## Steps" in rendered
    assert "name" in rendered
