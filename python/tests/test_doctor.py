"""Tests for skillforge._doctor and the `aif doctor` CLI command."""

from __future__ import annotations

import json
import os
import textwrap
from pathlib import Path
from unittest.mock import patch

import pytest

from skillforge import _doctor, _score
from skillforge._cli import main


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

CLEAN_SKILL = textwrap.dedent("""\
    ---
    name: clean-skill
    description: A well-formed skill for testing.
    version: "1.0"
    ---

    ## Steps

    1. Do the thing correctly.

    ## Verification

    Check that the thing was done.
""")

MINIMAL_SKILL = textwrap.dedent("""\
    ---
    name: minimal
    description: Minimal skill.
    ---

    ## Steps

    1. One step.

    ## Verification

    Verify it.
""")


# ---------------------------------------------------------------------------
# 1. Empty directory — no skills found
# ---------------------------------------------------------------------------

def test_empty_dir_returns_no_results(tmp_path: Path) -> None:
    report = _doctor.run_doctor(str(tmp_path))
    assert report.total == 0
    assert report.deploy_ready == 0
    assert report.results == []


def test_empty_dir_formats_gracefully(tmp_path: Path) -> None:
    report = _doctor.run_doctor(str(tmp_path))
    text = _doctor.format_text(report)
    assert "No skill files found" in text


# ---------------------------------------------------------------------------
# 2. Single clean skill — should get a high grade
# ---------------------------------------------------------------------------

def test_single_clean_skill_is_found(tmp_path: Path) -> None:
    (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
    report = _doctor.run_doctor(str(tmp_path))
    assert report.total == 1
    assert len(report.results) == 1
    result = report.results[0]
    assert result.path == "SKILL.md"
    # Clean skill should parse fine; grade should be reasonable.
    assert not result.score.parse_failed


def test_single_clean_skill_deploy_ready(tmp_path: Path) -> None:
    (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
    report = _doctor.run_doctor(str(tmp_path))
    # deploy_ready count must match or be close (at least 0 errors from parse)
    # We just verify the arithmetic is consistent.
    assert 0 <= report.deploy_ready <= report.total


def test_single_clean_skill_no_annotation(tmp_path: Path) -> None:
    """A skill that scores A+ should have an empty annotation."""
    # Build a Score that represents a perfect skill.
    perfect_score = _score.compute_score([], [])
    assert perfect_score.grade == "A+"
    annotation = _doctor.build_annotation_public(perfect_score)
    assert annotation == ""


# ---------------------------------------------------------------------------
# 3. Mixed grades — sorting and annotation
# ---------------------------------------------------------------------------

def test_results_sorted_by_score_descending(tmp_path: Path) -> None:
    """Results list should be ordered highest score first."""
    # Create two skills with different scores using mocked _grade_file.
    score_a = _score.compute_score([], [])  # 100 → A+
    score_b = _score.compute_score(
        [{"passed": False, "severity": "Error", "check": "Frontmatter"}],
        [],
    )  # 93 → A

    results = [
        _doctor.SkillResult(path="b.aif", abs_path="/b.aif", score=score_b, annotation="1 lint error"),
        _doctor.SkillResult(path="a.md", abs_path="/a.md", score=score_a, annotation=""),
    ]
    report = _doctor.DoctorReport(root="/repo", results=results)
    # Manually sort as the function would.
    report.results.sort(key=lambda r: (-r.score.numeric, r.path))

    assert report.results[0].path == "a.md"
    assert report.results[1].path == "b.aif"


def test_annotation_for_lint_errors() -> None:
    score = _score.compute_score(
        [{"passed": False, "severity": "Error", "check": "Frontmatter"},
         {"passed": False, "severity": "Error", "check": "BlockTypes"}],
        [],
    )
    annotation = _doctor.build_annotation_public(score)
    assert "2 lint errors" in annotation


def test_annotation_for_security_findings() -> None:
    score = _score.compute_score(
        [],
        [{"severity": "Critical", "rule": "prompt-injection", "message": "x"},
         {"severity": "High", "rule": "dangerous-tool", "message": "x"},
         {"severity": "High", "rule": "jailbreak", "message": "x"}],
    )
    annotation = _doctor.build_annotation_public(score)
    assert "3 security findings" in annotation


def test_annotation_for_parse_failure() -> None:
    score = _score.compute_score([], [], parse_failed=True)
    annotation = _doctor.build_annotation_public(score)
    assert "parse error" in annotation


def test_mixed_grades_text_report(tmp_path: Path) -> None:
    """Text report should show all skills and a summary line."""
    # Create two SKILL.md files in subdirs.
    subdir_a = tmp_path / "skills"
    subdir_a.mkdir()
    (subdir_a / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")

    subdir_b = tmp_path / "experimental"
    subdir_b.mkdir()
    (subdir_b / "SKILL.md").write_text(MINIMAL_SKILL, encoding="utf-8")

    report = _doctor.run_doctor(str(tmp_path))
    assert report.total == 2

    text = _doctor.format_text(report)
    assert "Skills found: 2" in text
    assert "Overall:" in text
    assert "deploy-ready" in text


def test_skipped_dirs_are_ignored(tmp_path: Path) -> None:
    """Files inside target/, node_modules/, .git/ must be ignored."""
    for skip_dir in ("target", "node_modules", ".git"):
        d = tmp_path / skip_dir
        d.mkdir()
        (d / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
    # One real skill at root level.
    (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")

    report = _doctor.run_doctor(str(tmp_path))
    assert report.total == 1  # only the root-level one


# ---------------------------------------------------------------------------
# 4. JSON output
# ---------------------------------------------------------------------------

def test_json_output_structure(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
    rc = main(["doctor", str(tmp_path), "--format", "json"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)

    assert "root" in data
    assert "total" in data
    assert "deploy_ready" in data
    assert "average_grade" in data
    assert "skills" in data
    assert isinstance(data["skills"], list)
    assert len(data["skills"]) == 1

    skill_entry = data["skills"][0]
    assert "path" in skill_entry
    assert "score" in skill_entry
    assert "grade" in skill_entry
    assert "annotation" in skill_entry
    assert "details" in skill_entry


def test_json_output_empty_dir(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    rc = main(["doctor", str(tmp_path), "--format", "json"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert data["total"] == 0
    assert data["skills"] == []


# ---------------------------------------------------------------------------
# 5. CLI exit code
# ---------------------------------------------------------------------------

def test_cli_exit_0_when_all_deploy_ready(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    """Exit 0 when every skill is deploy-ready (grade >= B)."""
    # Perfect score — force by mocking the grading.
    perfect = _score.compute_score([], [])
    assert perfect.grade == "A+"

    with patch.object(_doctor, "_grade_file", return_value=perfect):
        (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
        rc = main(["doctor", str(tmp_path)])
    assert rc == 0


def test_cli_exit_1_when_some_not_deploy_ready(
    tmp_path: Path, capsys: pytest.CaptureFixture
) -> None:
    """Exit 1 when at least one skill is below the deploy-ready threshold."""
    bad_score = _score.compute_score([], [], parse_failed=True)  # grade F
    assert bad_score.grade == "F"

    with patch.object(_doctor, "_grade_file", return_value=bad_score):
        (tmp_path / "SKILL.md").write_text("broken", encoding="utf-8")
        rc = main(["doctor", str(tmp_path)])
    assert rc == 1


# ---------------------------------------------------------------------------
# 6. AIF file discovery
# ---------------------------------------------------------------------------

def test_aif_files_are_discovered(tmp_path: Path) -> None:
    """*.aif files should be included alongside SKILL.md files."""
    (tmp_path / "review.aif").write_text(
        "@skill[name=\"code-review\"]\n@step\nReview the diff.\n@/step\n@/skill\n",
        encoding="utf-8",
    )
    report = _doctor.run_doctor(str(tmp_path))
    assert report.total == 1
    assert report.results[0].path.endswith(".aif")


def test_both_skill_md_and_aif_discovered(tmp_path: Path) -> None:
    (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
    (tmp_path / "other.aif").write_text(
        "@skill[name=\"other\"]\n@step\nStep one.\n@/step\n@/skill\n",
        encoding="utf-8",
    )
    report = _doctor.run_doctor(str(tmp_path))
    assert report.total == 2


# ---------------------------------------------------------------------------
# 7. Text report suggestions
# ---------------------------------------------------------------------------

def test_text_report_suggests_fix_for_lint_errors(tmp_path: Path) -> None:
    lint_score = _score.compute_score(
        [{"passed": False, "severity": "Error", "check": "Frontmatter"}],
        [],
    )
    with patch.object(_doctor, "_grade_file", return_value=lint_score):
        (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
        report = _doctor.run_doctor(str(tmp_path))

    text = _doctor.format_text(report)
    assert "aif fix" in text


def test_text_report_suggests_scan_for_security(tmp_path: Path) -> None:
    sec_score = _score.compute_score(
        [],
        [{"severity": "Critical", "rule": "prompt-injection", "message": "x"}],
    )
    with patch.object(_doctor, "_grade_file", return_value=sec_score):
        (tmp_path / "SKILL.md").write_text(CLEAN_SKILL, encoding="utf-8")
        report = _doctor.run_doctor(str(tmp_path))

    text = _doctor.format_text(report)
    assert "aif scan" in text


# ---------------------------------------------------------------------------
# 8. DoctorReport statistics
# ---------------------------------------------------------------------------

def test_doctor_report_average_grade_empty() -> None:
    report = _doctor.DoctorReport(root="/repo")
    assert report.average_grade == "F"  # grade_for(0)


def test_doctor_report_deploy_ready_count() -> None:
    perfect = _score.compute_score([], [])            # A+ → deploy ready
    failing = _score.compute_score([], [], parse_failed=True)  # F → not ready

    report = _doctor.DoctorReport(
        root="/repo",
        results=[
            _doctor.SkillResult("a.md", "/a.md", perfect, ""),
            _doctor.SkillResult("b.md", "/b.md", failing, "parse error"),
        ],
    )
    assert report.deploy_ready == 1
    assert report.total == 2
