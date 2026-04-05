"""Tests for skillforge._score."""

from skillforge import _score


def test_perfect_skill_is_a_plus():
    score = _score.compute_score(
        lint_results=[{"passed": True, "severity": "Error", "check": "Frontmatter"}],
        scan_findings=[],
    )
    assert score.numeric == 100
    assert score.grade == "A+"
    assert score.color == "brightgreen"


def test_one_lint_error_drops_to_93():
    score = _score.compute_score(
        lint_results=[
            {"passed": True, "severity": "Error", "check": "X"},
            {"passed": False, "severity": "Error", "check": "Frontmatter"},
        ],
        scan_findings=[],
    )
    assert score.numeric == 93
    assert score.grade == "A"


def test_critical_security_finding_is_15_points():
    score = _score.compute_score(
        lint_results=[],
        scan_findings=[{"severity": "Critical", "rule": "prompt-injection"}],
    )
    assert score.numeric == 85
    assert score.grade == "B"


def test_multiple_critical_findings_drop_to_f():
    score = _score.compute_score(
        lint_results=[],
        scan_findings=[
            {"severity": "Critical", "rule": "prompt-injection"},
            {"severity": "Critical", "rule": "dangerous-tool"},
            {"severity": "High", "rule": "dangerous-tool"},
            {"severity": "Medium", "rule": "external-fetch"},
        ],
    )
    # 100 - 15 - 15 - 8 - 3 = 59 → F
    assert score.numeric == 59
    assert score.grade == "F"


def test_parse_failure_is_instant_f():
    score = _score.compute_score([], [], parse_failed=True)
    assert score.numeric == 0
    assert score.grade == "F"
    assert score.parse_failed is True


def test_grade_thresholds():
    assert _score.grade_for(100) == "A+"
    assert _score.grade_for(97) == "A+"
    assert _score.grade_for(96) == "A"
    assert _score.grade_for(93) == "A"
    assert _score.grade_for(92) == "A-"
    assert _score.grade_for(80) == "B-"
    assert _score.grade_for(79) == "C+"
    assert _score.grade_for(70) == "C-"
    assert _score.grade_for(69) == "D"
    assert _score.grade_for(59) == "F"
    assert _score.grade_for(0) == "F"


def test_shields_format_is_valid_json():
    import json
    score = _score.compute_score([], [])
    payload = json.loads(_score.format_shields(score))
    assert payload["schemaVersion"] == 1
    assert payload["label"] == "SkillForge"
    assert payload["message"] == "A+"
    assert payload["color"] == "brightgreen"


def test_svg_contains_grade_and_color():
    score = _score.compute_score([], [])
    svg = _score.format_svg(score)
    assert "<svg" in svg
    assert "A+" in svg
    assert "#4c1" in svg  # brightgreen hex
    assert "</svg>" in svg


def test_svg_for_f_grade_is_red():
    score = _score.compute_score([], [], parse_failed=True)
    svg = _score.format_svg(score)
    assert "F" in svg
    assert "#e05d44" in svg  # red hex


def test_text_format_shows_deductions():
    score = _score.compute_score(
        lint_results=[{"passed": False, "severity": "Error", "check": "Frontmatter"}],
        scan_findings=[{"severity": "High", "rule": "dangerous-tool"}],
    )
    text = _score.format_text(score, "test.md")
    assert "85/100" in text
    assert "B" in text
    assert "Frontmatter" in text
    assert "dangerous-tool" in text
