"""Live grade display that updates on every file save.

Usage:
    from skillforge._watch import watch_file
    watch_file("SKILL.md")

Or via CLI:
    aif watch SKILL.md
"""

from __future__ import annotations

import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Optional

import skillforge
from skillforge import _score


# ---------------------------------------------------------------------------
# ANSI helpers
# ---------------------------------------------------------------------------

RESET = "\033[0m"
BOLD = "\033[1m"
BRIGHT_GREEN = "\033[92m"
YELLOW = "\033[33m"
RED = "\033[31m"
DIM = "\033[2m"

_ANSI_RE = re.compile(r"\033\[[0-9;]*m")


def _ansi_grade_color(grade: str) -> str:
    """Return ANSI color code for a grade string."""
    if grade in ("A+", "A", "A-"):
        return BRIGHT_GREEN
    if grade.startswith("B"):
        return YELLOW
    return RED


def _progress_bar(numeric: int, width: int = 20) -> str:
    """Render a Unicode block-character progress bar for a 0-100 score."""
    filled = round(numeric / 100 * width)
    empty = width - filled
    return "\u2588" * filled + "\u2591" * empty


def _seconds_ago(ts: float) -> str:
    """Format elapsed seconds as a human-readable 'Xs ago' string."""
    elapsed = int(time.time() - ts)
    if elapsed < 2:
        return "just now"
    if elapsed < 60:
        return f"{elapsed}s ago"
    minutes = elapsed // 60
    return f"{minutes}m ago"


def _visible_length(s: str) -> int:
    """Compute the visible (printable) length of a string, ignoring ANSI codes."""
    return len(_ANSI_RE.sub("", s))


# ---------------------------------------------------------------------------
# Pure rendering function (testable without filesystem)
# ---------------------------------------------------------------------------

def render_display(
    score: _score.Score,
    path: str,
    lint_total: int,
    lint_passed: int,
    security_clean: bool,
    security_findings: int,
    last_change_ts: Optional[float],
    terminal_width: int = 60,
) -> str:
    """Render the watch display as a string.

    This is a pure function — it takes only data and returns a string.
    No I/O, no side effects. Makes it fully testable.
    """
    grade_color = _ansi_grade_color(score.grade)
    bar = _progress_bar(score.numeric)
    inner = terminal_width - 2

    title = f" SkillForge Watch: {Path(path).name} "
    # Clamp title to inner width so the box never overflows
    if len(title) > inner:
        title = title[: inner - 3] + "... "
    title_padded = title.center(inner, "\u2500")
    border_top = f"\u250c{title_padded}\u2510"
    border_bot = "\u2514" + "\u2500" * inner + "\u2518"
    sep = "\u2502" + " " * inner + "\u2502"

    def row(content: str) -> str:
        visible_len = _visible_length(content)
        padding = max(0, inner - visible_len - 2)
        return f"\u2502 {content}{' ' * padding} \u2502"

    grade_label = f"{BOLD}{grade_color}Grade: {score.grade}  ({score.numeric}/100){RESET}"
    bar_str = f"{grade_color}{bar}{RESET} {score.numeric}%"
    grade_line = f"{grade_label}    {bar_str}"

    lint_failures = lint_total - lint_passed
    lint_line = f"  Lint:     {lint_passed}/{lint_total} passed"
    if lint_failures > 0:
        lint_line += f"  {RED}({lint_failures} failed){RESET}"

    if security_clean:
        sec_line = "  Security: clean"
    else:
        sec_line = f"  Security: {RED}{security_findings} finding(s){RESET}"

    if last_change_ts is not None:
        change_line = f"  Last change: {_seconds_ago(last_change_ts)}"
    else:
        change_line = "  Last change: \u2014"

    watch_line = f"  {DIM}Watching... (Ctrl+C to stop){RESET}"

    lines = [
        border_top,
        sep,
        row(grade_line),
        sep,
        row(lint_line),
        row(sec_line),
        sep,
        row(change_line),
        row(watch_line),
        border_bot,
    ]
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Score computation (mirrors cmd_check / cmd_score logic)
# ---------------------------------------------------------------------------

def _compute_for_file(source: str, path: str) -> tuple[_score.Score, int, int, bool, int]:
    """Run lint + scan + score for source text.

    Returns (score, lint_total, lint_passed, security_clean, security_count).
    """
    from skillforge._cli import _as_aif

    try:
        aif_src = _as_aif(source, path)
    except Exception:
        aif_src = source

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

    s = _score.compute_score(lint_results, scan_findings, parse_failed=parse_failed)

    lint_total = len(lint_results)
    lint_passed = sum(1 for r in lint_results if r.get("passed", False))
    high_sev = [f for f in scan_findings if f.get("severity") in ("Critical", "High")]
    security_clean = not high_sev
    security_count = len(scan_findings)

    return s, lint_total, lint_passed, security_clean, security_count


# ---------------------------------------------------------------------------
# Watch loop
# ---------------------------------------------------------------------------

def _clear() -> None:
    """Clear the terminal screen (no-op on Windows without ANSI support)."""
    sys.stdout.write("\033[2J\033[H")
    sys.stdout.flush()


def _get_mtime(path: str) -> Optional[float]:
    try:
        return os.stat(path).st_mtime
    except OSError:
        return None


def _terminal_width() -> int:
    try:
        return min(os.get_terminal_size().columns, 80)
    except OSError:
        return 60


def watch_file(path: str) -> None:
    """Watch a file and live-update a grade display on every change.

    Polls mtime every 0.5 s. Prints the display immediately on start,
    then updates on every detected change. Exits cleanly on Ctrl+C.
    """
    file_path = Path(path)
    if not file_path.exists():
        print(f"error: file not found: {path}", file=sys.stderr)
        sys.exit(3)

    last_mtime: Optional[float] = None
    last_change_ts: Optional[float] = None
    last_reprint: float = 0.0

    # Cached results for timer-only refreshes
    cached_score: Optional[_score.Score] = None
    cached_lint_total: int = 0
    cached_lint_passed: int = 0
    cached_sec_clean: bool = True
    cached_sec_count: int = 0

    REPRINT_INTERVAL = 2.0  # seconds between timer-only reprints

    try:
        while True:
            now = time.time()
            mtime = _get_mtime(path)
            changed = mtime != last_mtime

            if changed:
                last_mtime = mtime
                last_change_ts = now

                try:
                    source = file_path.read_text(encoding="utf-8")
                except OSError as e:
                    _clear()
                    print(f"error reading {path}: {e}")
                    time.sleep(0.5)
                    continue

                (
                    cached_score,
                    cached_lint_total,
                    cached_lint_passed,
                    cached_sec_clean,
                    cached_sec_count,
                ) = _compute_for_file(source, path)

            # Reprint on change or every REPRINT_INTERVAL for the running timer
            if changed or (cached_score is not None and now - last_reprint >= REPRINT_INTERVAL):
                if cached_score is not None:
                    _clear()
                    display = render_display(
                        score=cached_score,
                        path=path,
                        lint_total=cached_lint_total,
                        lint_passed=cached_lint_passed,
                        security_clean=cached_sec_clean,
                        security_findings=cached_sec_count,
                        last_change_ts=last_change_ts,
                        terminal_width=_terminal_width(),
                    )
                    print(display)
                    last_reprint = now

            time.sleep(0.5)

    except KeyboardInterrupt:
        print("\nStopped.")
        sys.exit(0)
