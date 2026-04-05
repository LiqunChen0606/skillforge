"""Tests for skillforge._fix."""

from skillforge import _fix


def test_kebab_case_conversion():
    assert _fix._kebab_case("My Skill") == "my-skill"
    assert _fix._kebab_case("My_Bad_Name!") == "my-bad-name"
    assert _fix._kebab_case("camelCaseName") == "camel-case-name"
    assert _fix._kebab_case("already-kebab") == "already-kebab"
    assert _fix._kebab_case("") == "unnamed-skill"
    assert _fix._kebab_case("!!!") == "unnamed-skill"


def test_adds_missing_name_from_h1():
    text = """---
description: something
---

# Code Review

## Steps

1. Read the PR
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert "name: code-review" in fixed
    assert any(f.rule == "Frontmatter" for f in applied)


def test_adds_missing_description():
    text = """---
name: my-skill
---

# My Skill

## Steps
1. Do thing.

## Verification
Check thing.
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert "description:" in fixed
    assert "TODO" in fixed
    assert any("description" in f.description.lower() for f in applied)


def test_normalizes_name_to_kebab_case():
    text = """---
name: My_Bad_Name!
description: ok
---

# Skill

## Steps
1. Do.

## Verification
Check.
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert "name: my-bad-name" in fixed
    assert any(f.rule == "NameFormat" for f in applied)


def test_truncates_long_description():
    long_desc = "x" * 2000
    text = f"""---
name: my-skill
description: {long_desc}
---

## Steps
1. Do.

## Verification
Check.
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert any(f.rule == "DescriptionLength" for f in applied)
    # Find the description line in the output
    for line in fixed.splitlines():
        if line.startswith("description:"):
            assert len(line) <= 13 + 1024  # "description: " + 1024 chars
            assert line.endswith("...")


def test_adds_missing_steps_section():
    text = """---
name: my-skill
description: ok
---

# Skill

## Verification
Check.
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert "## Steps" in fixed
    assert any(f.rule == "RequiredSections" and "Steps" in f.description for f in applied)


def test_adds_missing_verification_section():
    text = """---
name: my-skill
description: ok
---

# Skill

## Steps
1. Do.
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert "## Verification" in fixed
    assert any(f.rule == "RequiredSections" and "Verification" in f.description for f in applied)


def test_fills_empty_sections():
    text = """---
name: my-skill
description: ok
---

# Skill

## Steps

## Verification
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert "TODO" in fixed
    assert sum(1 for f in applied if f.rule == "NoEmptyBlocks") == 2


def test_clean_skill_no_changes():
    text = """---
name: clean-skill
description: A clean skill that needs no fixes
---

# Clean Skill

## Steps

1. Do the thing carefully
2. Verify intermediate state

## Verification

Confirm the output matches expectations.
"""
    fixed, applied = _fix.fix_skill_md(text)
    assert applied == []


def test_diff_output():
    before = "---\nname: Bad!\n---\n"
    after, _ = _fix.fix_skill_md(before)
    diff = _fix.diff_fixes(before, after)
    assert "---" in diff
    assert "before" in diff
    assert "after" in diff


def test_infers_name_from_first_h1_if_no_frontmatter_name():
    text = """---
description: ok
---

# API Rate Limiter

## Steps
1. Do.

## Verification
Check.
"""
    fixed, _ = _fix.fix_skill_md(text)
    assert "name: api-rate-limiter" in fixed
