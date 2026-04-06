"""Tests for skillforge._watch rendering logic."""

from __future__ import annotations

import re
import time

from skillforge import _score
from skillforge._watch import (
    _ansi_grade_color,
    _progress_bar,
    _seconds_ago,
    _visible_length,
    BRIGHT_GREEN,
    YELLOW,
    RED,
    render_display,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_score(numeric: int, **kwargs) -> _score.Score:
    grade = _score.grade_for(numeric)
    defaults = dict(
        lint_errors=0,
        lint_warnings=0,
        security_critical=0,
        security_high=0,
        security_medium=0,
        security_low=0,
        deductions=[],
        parse_failed=False,
    )
    defaults.update(kwargs)
    return _score.Score(numeric=numeric, grade=grade, **defaults)


ANSI_ESCAPE = re.compile(r"\033\[[0-9;]*m")


def strip_ansi(s: str) -> str:
    return ANSI_ESCAPE.sub("", s)


# ---------------------------------------------------------------------------
# Test: color selection
# ---------------------------------------------------------------------------

class TestAnsiGradeColor:
    def test_a_plus_is_bright_green(self):
        assert _ansi_grade_color("A+") == BRIGHT_GREEN

    def test_a_is_bright_green(self):
        assert _ansi_grade_color("A") == BRIGHT_GREEN

    def test_a_minus_is_bright_green(self):
        assert _ansi_grade_color("A-") == BRIGHT_GREEN

    def test_b_plus_is_yellow(self):
        assert _ansi_grade_color("B+") == YELLOW

    def test_b_is_yellow(self):
        assert _ansi_grade_color("B") == YELLOW

    def test_b_minus_is_yellow(self):
        assert _ansi_grade_color("B-") == YELLOW

    def test_c_plus_is_red(self):
        assert _ansi_grade_color("C+") == RED

    def test_c_is_red(self):
        assert _ansi_grade_color("C") == RED

    def test_f_is_red(self):
        assert _ansi_grade_color("F") == RED

    def test_d_is_red(self):
        assert _ansi_grade_color("D") == RED


# ---------------------------------------------------------------------------
# Test: progress bar
# ---------------------------------------------------------------------------

class TestProgressBar:
    def test_full_bar_at_100(self):
        bar = _progress_bar(100, width=20)
        assert bar == "\u2588" * 20
        assert "\u2591" not in bar

    def test_empty_bar_at_0(self):
        bar = _progress_bar(0, width=20)
        assert bar == "\u2591" * 20
        assert "\u2588" not in bar

    def test_half_bar_at_50(self):
        bar = _progress_bar(50, width=20)
        # 50% of 20 = 10 filled, 10 empty
        assert bar == "\u2588" * 10 + "\u2591" * 10

    def test_bar_width_is_respected(self):
        bar = _progress_bar(75, width=8)
        assert len(bar) == 8

    def test_bar_contains_only_block_chars(self):
        for score in (0, 25, 50, 75, 100):
            bar = _progress_bar(score, width=20)
            for ch in bar:
                assert ch in ("\u2588", "\u2591"), f"unexpected char {ch!r} at score={score}"


# ---------------------------------------------------------------------------
# Test: seconds_ago formatting
# ---------------------------------------------------------------------------

class TestSecondsAgo:
    def test_just_now(self):
        assert _seconds_ago(time.time()) == "just now"
        assert _seconds_ago(time.time() - 1) == "just now"

    def test_seconds(self):
        assert _seconds_ago(time.time() - 30) == "30s ago"
        assert _seconds_ago(time.time() - 59) == "59s ago"

    def test_minutes(self):
        result = _seconds_ago(time.time() - 90)
        assert result == "1m ago"

    def test_several_minutes(self):
        result = _seconds_ago(time.time() - 300)
        assert result == "5m ago"


# ---------------------------------------------------------------------------
# Test: visible_length strips ANSI codes
# ---------------------------------------------------------------------------

class TestVisibleLength:
    def test_plain_string(self):
        assert _visible_length("hello") == 5

    def test_string_with_ansi_codes(self):
        colored = f"\033[92mGrade: A+\033[0m"
        assert _visible_length(colored) == len("Grade: A+")

    def test_empty_string(self):
        assert _visible_length("") == 0

    def test_multiple_ansi_codes(self):
        s = f"\033[1m\033[92mBold Green\033[0m"
        assert _visible_length(s) == len("Bold Green")


# ---------------------------------------------------------------------------
# Test: render_display (main integration)
# ---------------------------------------------------------------------------

class TestRenderDisplay:
    def _render(self, numeric=100, lint_total=10, lint_passed=10,
                security_clean=True, security_findings=0,
                last_change_ts=None, path="SKILL.md", width=60):
        score = _make_score(numeric)
        return render_display(
            score=score,
            path=path,
            lint_total=lint_total,
            lint_passed=lint_passed,
            security_clean=security_clean,
            security_findings=security_findings,
            last_change_ts=last_change_ts,
            terminal_width=width,
        )

    def test_contains_grade(self):
        display = strip_ansi(self._render(numeric=100))
        assert "A+" in display

    def test_contains_score_number(self):
        display = strip_ansi(self._render(numeric=85))
        assert "85" in display

    def test_contains_lint_counts(self):
        display = strip_ansi(self._render(lint_total=10, lint_passed=8))
        assert "8/10" in display

    def test_security_clean_label(self):
        display = strip_ansi(self._render(security_clean=True))
        assert "clean" in display

    def test_security_findings_shown(self):
        display = strip_ansi(self._render(security_clean=False, security_findings=3))
        assert "3 finding(s)" in display

    def test_watching_label_present(self):
        display = strip_ansi(self._render())
        assert "Watching" in display
        assert "Ctrl+C" in display

    def test_last_change_shown_when_provided(self):
        ts = time.time() - 10
        display = strip_ansi(self._render(last_change_ts=ts))
        assert "Last change:" in display
        # Allow "10s ago" or "11s ago" due to execution timing
        assert "s ago" in display

    def test_last_change_dash_when_none(self):
        display = strip_ansi(self._render(last_change_ts=None))
        assert "Last change:" in display
        # Should show em-dash or similar placeholder
        assert "\u2014" in display or "\u2014" in display

    def test_box_has_top_and_bottom_borders(self):
        display = self._render()
        lines = display.splitlines()
        assert lines[0].startswith("\u250c")  # top-left corner
        assert lines[-1].startswith("\u2514")  # bottom-left corner

    def test_filename_in_title(self):
        display = strip_ansi(self._render(path="skills/my-skill.md"))
        assert "my-skill.md" in display

    def test_perfect_score_shows_full_bar(self):
        display = self._render(numeric=100)
        # Full bar: 20 filled blocks
        assert "\u2588" * 20 in display

    def test_zero_score_shows_empty_bar(self):
        display = self._render(numeric=0)
        assert "\u2591" * 20 in display

    def test_a_plus_grade_color_is_bright_green(self):
        score = _make_score(100)
        display = render_display(
            score=score, path="SKILL.md",
            lint_total=10, lint_passed=10,
            security_clean=True, security_findings=0,
            last_change_ts=None, terminal_width=60,
        )
        # BRIGHT_GREEN ANSI code should appear in the raw output
        assert BRIGHT_GREEN in display

    def test_f_grade_color_is_red(self):
        score = _make_score(0, parse_failed=True)
        display = render_display(
            score=score, path="SKILL.md",
            lint_total=0, lint_passed=0,
            security_clean=True, security_findings=0,
            last_change_ts=None, terminal_width=60,
        )
        assert RED in display

    def test_b_grade_color_is_yellow(self):
        score = _make_score(83)  # B
        display = render_display(
            score=score, path="SKILL.md",
            lint_total=5, lint_passed=4,
            security_clean=True, security_findings=0,
            last_change_ts=None, terminal_width=60,
        )
        assert YELLOW in display

    def test_lint_failure_count_shown(self):
        display = strip_ansi(self._render(lint_total=10, lint_passed=7))
        assert "3 failed" in display

    def test_no_lint_failure_label_on_clean(self):
        display = strip_ansi(self._render(lint_total=10, lint_passed=10))
        assert "failed" not in display

    def test_render_is_deterministic(self):
        """Same inputs within the same second always produce same output."""
        # Use None for last_change_ts to avoid time.time() drift between two calls.
        a = self._render(numeric=90, last_change_ts=None)
        b = self._render(numeric=90, last_change_ts=None)
        assert a == b
