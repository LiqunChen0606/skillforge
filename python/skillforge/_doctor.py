"""Repo-wide skill health report for SkillForge.

`run_doctor(path)` walks a directory recursively, finds all SKILL.md and
.aif files (skipping target/, node_modules/, .git/), grades each one, and
returns a `DoctorReport` ready to print as text or serialize to JSON.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import skillforge
from skillforge import _score

# Directories to skip when walking the tree.
_SKIP_DIRS = {"target", "node_modules", ".git", ".tox", "__pycache__", ".venv", "venv"}

# Grade order for deploy-readiness threshold (>= B).
GRADE_ORDER = ["F", "D", "C-", "C", "C+", "B-", "B", "B+", "A-", "A", "A+"]
DEPLOY_READY_THRESHOLD = "B"  # grade >= B → deploy-ready


@dataclass
class SkillResult:
    path: str           # relative path shown in the report
    abs_path: str       # absolute path for suggestions
    score: _score.Score
    annotation: str     # short one-liner note for non-A+ skills ("2 lint errors", etc.)


@dataclass
class DoctorReport:
    root: str
    results: list[SkillResult] = field(default_factory=list)

    # --- computed properties -------------------------------------------------

    @property
    def total(self) -> int:
        return len(self.results)

    @property
    def deploy_ready(self) -> int:
        threshold_idx = GRADE_ORDER.index(DEPLOY_READY_THRESHOLD)
        return sum(
            1 for r in self.results
            if GRADE_ORDER.index(r.score.grade) >= threshold_idx
        )

    @property
    def average_numeric(self) -> float:
        if not self.results:
            return 0.0
        return sum(r.score.numeric for r in self.results) / len(self.results)

    @property
    def average_grade(self) -> str:
        return _score.grade_for(int(round(self.average_numeric)))

    def to_dict(self) -> dict[str, Any]:
        return {
            "root": self.root,
            "total": self.total,
            "deploy_ready": self.deploy_ready,
            "average_score": round(self.average_numeric, 1),
            "average_grade": self.average_grade,
            "skills": [
                {
                    "path": r.path,
                    "score": r.score.numeric,
                    "grade": r.score.grade,
                    "annotation": r.annotation,
                    "details": r.score.to_dict(),
                }
                for r in self.results
            ],
        }


# ---------------------------------------------------------------------------
# Core scan logic
# ---------------------------------------------------------------------------

def _find_skill_files(root: Path) -> list[Path]:
    """Walk root recursively, skipping excluded dirs, returning skill files."""
    results: list[Path] = []
    for dirpath, dirnames, filenames in _os_walk(root):
        # Prune skipped directories in-place so os.walk won't descend.
        dirnames[:] = [d for d in dirnames if d not in _SKIP_DIRS]
        for fname in filenames:
            if fname == "SKILL.md" or fname.endswith(".aif"):
                results.append(dirpath / fname)
    return sorted(results)


def _os_walk(root: Path):
    """Thin wrapper around os.walk so tests can monkeypatch it."""
    import os
    for dirpath, dirnames, filenames in os.walk(root):
        yield Path(dirpath), dirnames, filenames


def _grade_file(abs_path: Path) -> _score.Score:
    """Read a skill file and return its Score. Never raises."""
    try:
        source = abs_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return _score.compute_score([], [], parse_failed=True)

    # Convert to AIF via the same helper pattern as _cli.py
    aif_src = _to_aif(source, str(abs_path))

    parse_failed = False
    try:
        skillforge.parse(aif_src)
    except (ValueError, Exception):
        parse_failed = True

    lint_results: list = []
    scan_findings: list = []
    if not parse_failed:
        try:
            lint_results = json.loads(skillforge.lint(aif_src))
        except (ValueError, Exception):
            lint_results = []
        try:
            scan_findings = json.loads(skillforge.scan(aif_src))
        except (ValueError, Exception):
            scan_findings = []

    return _score.compute_score(lint_results, scan_findings, parse_failed=parse_failed)


def _to_aif(source: str, path: str) -> str:
    """Mirror of _cli._as_aif — convert SKILL.md / generic MD to AIF."""
    stripped = source.lstrip()
    looks_like_aif = stripped.startswith("@") or stripped.startswith("#")
    if path.endswith(".md") or not looks_like_aif:
        try:
            block_json = skillforge.import_skill_md(source)
        except (ValueError, Exception):
            try:
                ir_json = skillforge.import_markdown(source)
                return skillforge.render(ir_json, "lml-aggressive")
            except (ValueError, Exception):
                return source
        doc = {"metadata": {}, "blocks": [json.loads(block_json)]}
        try:
            return skillforge.render(json.dumps(doc), "lml-aggressive")
        except (ValueError, Exception):
            return source
    return source


def build_annotation_public(result: _score.Score) -> str:
    """Public alias for _build_annotation (used in tests)."""
    return _build_annotation(result)


def _build_annotation(result: _score.Score) -> str:
    """Build a short human-readable note for non-A+ skills."""
    parts: list[str] = []
    if result.parse_failed:
        return "parse error — file is syntactically broken"
    if result.lint_errors:
        noun = "lint error" if result.lint_errors == 1 else "lint errors"
        parts.append(f"{result.lint_errors} {noun}")
    if result.lint_warnings:
        noun = "lint warning" if result.lint_warnings == 1 else "lint warnings"
        parts.append(f"{result.lint_warnings} {noun}")
    total_sec = (
        result.security_critical + result.security_high
        + result.security_medium + result.security_low
    )
    if total_sec:
        noun = "security finding" if total_sec == 1 else "security findings"
        parts.append(f"{total_sec} {noun}")
    return ", ".join(parts) if parts else ""


def run_doctor(path: str) -> DoctorReport:
    """Scan *path* for skill files and return a DoctorReport."""
    root = Path(path).resolve()
    report = DoctorReport(root=str(root))

    if not root.exists():
        return report

    if root.is_file():
        files = [root]
    else:
        files = _find_skill_files(root)

    for f in files:
        score = _grade_file(f)
        annotation = _build_annotation(score)
        # Compute display path relative to root (or absolute if outside root).
        try:
            display = str(f.relative_to(root))
        except ValueError:
            display = str(f)

        report.results.append(SkillResult(
            path=display,
            abs_path=str(f),
            score=score,
            annotation=annotation,
        ))

    # Sort by numeric score descending, then by path for stable ties.
    report.results.sort(key=lambda r: (-r.score.numeric, r.path))
    return report


# ---------------------------------------------------------------------------
# Formatting
# ---------------------------------------------------------------------------

def _grade_width() -> int:
    return 3  # "A+", "B-", "F " — padded to 3 chars for alignment


def format_text(report: DoctorReport) -> str:
    lines: list[str] = []
    header = f"SkillForge Doctor — {report.root}"
    lines.append(header)
    lines.append("═" * len(header))

    if not report.results:
        lines.append("No skill files found.")
        return "\n".join(lines)

    lines.append(f"Skills found: {report.total}")
    lines.append(f"Average grade: {report.average_grade}")
    lines.append("")

    # Find skills that need suggestions (grade < A+).
    suggestions: list[SkillResult] = []
    worst_grade: str | None = None

    for r in report.results:
        grade_padded = r.score.grade.ljust(2)
        annotation_str = f"  \u2190 {r.annotation}" if r.annotation else ""
        lines.append(f"  {grade_padded}  {r.path}{annotation_str}")
        if r.score.grade != "A+":
            suggestions.append(r)
            # Track worst grade for the primary suggestion.
            if worst_grade is None:
                idx_current = GRADE_ORDER.index(r.score.grade)
                worst_grade = r.score.grade
            else:
                if GRADE_ORDER.index(r.score.grade) < GRADE_ORDER.index(worst_grade):
                    worst_grade = r.score.grade

    # Actionable suggestions.
    if suggestions:
        lines.append("")
        # Suggest fix for the lowest-scoring fixable skill.
        worst = max(suggestions, key=lambda r: (-GRADE_ORDER.index(r.score.grade)))
        if not worst.score.parse_failed and (
            worst.score.lint_errors > 0 or worst.score.lint_warnings > 0
        ):
            lines.append(
                f"Run `aif fix {worst.path} --write` to fix the {worst.score.grade}."
            )
        # Suggest scan for skills with security findings.
        sec_skills = [
            r for r in suggestions
            if (r.score.security_critical + r.score.security_high) > 0
        ]
        for r in sec_skills[:2]:
            lines.append(
                f"Run `aif scan {r.path}` to see security details."
            )

    lines.append("")
    threshold_idx = GRADE_ORDER.index(DEPLOY_READY_THRESHOLD)
    lines.append(
        f"Overall: {report.deploy_ready}/{report.total} skills are deploy-ready"
        f" (grade \u2265 {DEPLOY_READY_THRESHOLD})."
    )

    return "\n".join(lines)
