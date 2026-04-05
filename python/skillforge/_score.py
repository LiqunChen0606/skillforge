"""Skill quality scoring — letter grades + shareable badges.

Maps the output of `skillforge.lint` + `skillforge.scan` to a single 0-100
score and a letter grade (A+ → F). Emits text, JSON, Shields.io endpoint
JSON, or inline SVG.

The scoring weights are deliberately simple so authors can reason about
what moves their grade:

- Start at 100 points.
- Lint Error failures: -7 each (the 10 structural rules are roughly equal).
- Lint Warning failures: -3 each.
- Security findings: -15 (Critical), -8 (High), -3 (Medium), -1 (Low).
- Parse failure: instant F (0 points) — broken skills can't be graded.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any


LETTER_GRADES = [
    (97, "A+"),
    (93, "A"),
    (90, "A-"),
    (87, "B+"),
    (83, "B"),
    (80, "B-"),
    (77, "C+"),
    (73, "C"),
    (70, "C-"),
    (60, "D"),
    (0, "F"),
]

GRADE_COLORS = {
    "A+": "brightgreen",
    "A": "brightgreen",
    "A-": "green",
    "B+": "green",
    "B": "yellowgreen",
    "B-": "yellowgreen",
    "C+": "yellow",
    "C": "yellow",
    "C-": "orange",
    "D": "orange",
    "F": "red",
}

# Hex equivalents for inline SVG (no shields.io dependency).
HEX_COLORS = {
    "brightgreen": "#4c1",
    "green": "#97ca00",
    "yellowgreen": "#a4a61d",
    "yellow": "#dfb317",
    "orange": "#fe7d37",
    "red": "#e05d44",
    "grey": "#9f9f9f",
}


@dataclass
class Score:
    numeric: int                    # 0-100
    grade: str                      # "A+" .. "F"
    lint_errors: int
    lint_warnings: int
    security_critical: int
    security_high: int
    security_medium: int
    security_low: int
    deductions: list[tuple[str, int]]  # [(reason, points_lost), ...]
    parse_failed: bool = False

    @property
    def color(self) -> str:
        return GRADE_COLORS.get(self.grade, "grey")

    def to_dict(self) -> dict[str, Any]:
        return {
            "score": self.numeric,
            "grade": self.grade,
            "color": self.color,
            "lint": {"errors": self.lint_errors, "warnings": self.lint_warnings},
            "security": {
                "critical": self.security_critical,
                "high": self.security_high,
                "medium": self.security_medium,
                "low": self.security_low,
            },
            "deductions": [
                {"reason": r, "points": p} for r, p in self.deductions
            ],
            "parse_failed": self.parse_failed,
        }


def grade_for(numeric: int) -> str:
    for threshold, letter in LETTER_GRADES:
        if numeric >= threshold:
            return letter
    return "F"


def compute_score(
    lint_results: list[dict[str, Any]],
    scan_findings: list[dict[str, Any]],
    parse_failed: bool = False,
) -> Score:
    """Compute a Score from lint results + scan findings.

    `lint_results`: list of {"passed": bool, "severity": "Error"|"Warning", "check": str, ...}
    `scan_findings`: list of {"severity": "Critical"|"High"|"Medium"|"Low", "rule": str, ...}
    """
    if parse_failed:
        return Score(
            numeric=0, grade="F",
            lint_errors=0, lint_warnings=0,
            security_critical=0, security_high=0, security_medium=0, security_low=0,
            deductions=[("Parse failure: skill file is syntactically broken", 100)],
            parse_failed=True,
        )

    score = 100
    deductions: list[tuple[str, int]] = []
    lint_errors = lint_warnings = 0
    crit = high = med = low = 0

    for r in lint_results:
        if r.get("passed", True):
            continue
        sev = r.get("severity", "")
        check = r.get("check", "?")
        if sev == "Error":
            score -= 7
            lint_errors += 1
            deductions.append((f"Lint error: {check}", 7))
        elif sev == "Warning":
            score -= 3
            lint_warnings += 1
            deductions.append((f"Lint warning: {check}", 3))

    for f in scan_findings:
        sev = f.get("severity", "")
        rule = f.get("rule", "?")
        cost = {"Critical": 15, "High": 8, "Medium": 3, "Low": 1}.get(sev, 0)
        if cost == 0:
            continue
        score -= cost
        if sev == "Critical":
            crit += 1
        elif sev == "High":
            high += 1
        elif sev == "Medium":
            med += 1
        elif sev == "Low":
            low += 1
        deductions.append((f"Security [{sev}]: {rule}", cost))

    numeric = max(0, score)
    return Score(
        numeric=numeric, grade=grade_for(numeric),
        lint_errors=lint_errors, lint_warnings=lint_warnings,
        security_critical=crit, security_high=high, security_medium=med, security_low=low,
        deductions=deductions, parse_failed=False,
    )


def format_text(s: Score, path: str) -> str:
    lines = [
        f"SkillForge Score: {path}",
        "=" * 60,
        f"  Score:  {s.numeric}/100  ({s.grade})",
        f"  Lint:   {s.lint_errors} errors, {s.lint_warnings} warnings",
        f"  Security: {s.security_critical} critical, {s.security_high} high, "
        f"{s.security_medium} medium, {s.security_low} low",
    ]
    if s.deductions:
        lines.append("")
        lines.append("Deductions:")
        for reason, points in s.deductions[:10]:
            lines.append(f"  -{points:>2} pts  {reason}")
        if len(s.deductions) > 10:
            lines.append(f"  ... and {len(s.deductions) - 10} more")
    lines.append("-" * 60)
    lines.append(f"Grade: {s.grade}")
    return "\n".join(lines)


def format_shields(s: Score) -> str:
    """Emit Shields.io endpoint JSON. Commit this file to your repo, then
    reference via https://img.shields.io/endpoint?url=... in README."""
    payload = {
        "schemaVersion": 1,
        "label": "SkillForge",
        "message": s.grade,
        "color": s.color,
        "cacheSeconds": 3600,
    }
    return json.dumps(payload, indent=2)


def format_svg(s: Score) -> str:
    """Inline SVG badge, self-contained, no external dependencies.

    Uses the classic flat-style Shields.io layout so it visually matches
    other badges on a README. Dimensions computed from text length.
    """
    label = "skillforge"
    message = s.grade
    bg_color = HEX_COLORS.get(s.color, HEX_COLORS["grey"])

    # Approximate character widths (Verdana 11px at 10x scale).
    label_width = _text_width(label) + 10
    message_width = _text_width(message) + 10
    total_width = label_width + message_width

    # Text-anchor coords (x * 10 for subpixel precision via textLength).
    label_text_x = (label_width / 2) * 10
    message_text_x = (label_width + message_width / 2) * 10

    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{total_width}" height="20" role="img" aria-label="skillforge: {message}">
  <title>skillforge: {message}</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="r">
    <rect width="{total_width}" height="20" rx="3" fill="#fff"/>
  </clipPath>
  <g clip-path="url(#r)">
    <rect width="{label_width}" height="20" fill="#555"/>
    <rect x="{label_width}" width="{message_width}" height="20" fill="{bg_color}"/>
    <rect width="{total_width}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="110">
    <text aria-hidden="true" x="{label_text_x}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="{_text_width(label) * 10}">{label}</text>
    <text x="{label_text_x}" y="140" transform="scale(.1)" fill="#fff" textLength="{_text_width(label) * 10}">{label}</text>
    <text aria-hidden="true" x="{message_text_x}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="{_text_width(message) * 10}">{message}</text>
    <text x="{message_text_x}" y="140" transform="scale(.1)" fill="#fff" textLength="{_text_width(message) * 10}">{message}</text>
  </g>
</svg>
'''


def _text_width(text: str) -> int:
    """Approximate pixel width of text in 11px Verdana."""
    # Rough avg: 7px per char for Verdana at 11px, but wider chars exist.
    wide = sum(1 for c in text if c in "WMm")
    narrow = sum(1 for c in text if c in "iIl1.,")
    normal = len(text) - wide - narrow
    return wide * 10 + narrow * 4 + normal * 7
