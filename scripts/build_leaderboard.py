#!/usr/bin/env python3
"""Build the SkillForge public-skill leaderboard.

Scrapes GitHub for files named SKILL.md, grades each with aif score, and
publishes a weekly leaderboard. Run via a scheduled GitHub Action.

Outputs:
- leaderboard/LEADERBOARD.md   — human-readable top-N table
- leaderboard/data.json        — raw results for programmatic use
"""

from __future__ import annotations

import datetime as dt
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

TOP_N = 25
PER_PAGE = 50            # GitHub Search API max per page
MAX_PAGES = 2            # stay well under rate limits
OUTPUT_DIR = Path("leaderboard")
GITHUB_TOKEN = os.environ.get("GITHUB_TOKEN", "")
USER_AGENT = "SkillForge-Leaderboard/1.0"

# Exclude our own repo so we don't dominate the ranking we just built
EXCLUDE_REPOS = {"LiqunChen0606/skillforge"}


def search_skill_md() -> list[dict]:
    """Search GitHub code for files named SKILL.md."""
    results: list[dict] = []
    headers = {"User-Agent": USER_AGENT, "Accept": "application/vnd.github+json"}
    if GITHUB_TOKEN:
        headers["Authorization"] = f"Bearer {GITHUB_TOKEN}"

    for page in range(1, MAX_PAGES + 1):
        url = (
            "https://api.github.com/search/code"
            f"?q=filename:SKILL.md&per_page={PER_PAGE}&page={page}"
        )
        req = Request(url, headers=headers)
        try:
            with urlopen(req, timeout=30) as resp:
                data = json.loads(resp.read().decode())
        except HTTPError as e:
            if e.code == 403:
                print(f"  ! rate limited (page {page}) — stopping", file=sys.stderr)
                break
            raise
        items = data.get("items", [])
        if not items:
            break
        results.extend(items)
        print(f"  fetched page {page}: {len(items)} results", file=sys.stderr)
    return results


def download_raw(item: dict, dest: Path) -> bool:
    """Download the raw content of a SKILL.md file."""
    owner = item["repository"]["owner"]["login"]
    repo = item["repository"]["name"]
    path = item["path"]
    # Use repo default branch
    default_branch = item["repository"].get("default_branch", "main")
    url = f"https://raw.githubusercontent.com/{owner}/{repo}/{default_branch}/{path}"
    try:
        req = Request(url, headers={"User-Agent": USER_AGENT})
        with urlopen(req, timeout=15) as resp:
            dest.write_bytes(resp.read())
        return True
    except Exception:
        return False


def grade_skill(path: Path) -> dict | None:
    """Run `aif score --format json` on a skill file."""
    try:
        result = subprocess.run(
            ["aif", "score", str(path), "--format", "json"],
            capture_output=True,
            text=True,
            timeout=15,
        )
        # aif score exits 1 for low grades but still emits JSON
        if not result.stdout.strip():
            return None
        return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError):
        return None


def build_leaderboard(items: list[dict]) -> list[dict]:
    """Download, grade, and rank all discovered skills."""
    ranked: list[dict] = []
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        for i, item in enumerate(items):
            repo_full = f"{item['repository']['owner']['login']}/{item['repository']['name']}"
            if repo_full in EXCLUDE_REPOS:
                continue
            local = tmp / f"{i}_SKILL.md"
            if not download_raw(item, local):
                continue
            grade = grade_skill(local)
            if grade is None or grade.get("parse_failed"):
                continue
            ranked.append({
                "repo": repo_full,
                "path": item["path"],
                "url": item["html_url"],
                "score": grade["score"],
                "grade": grade["grade"],
                "color": grade["color"],
                "lint_errors": grade["lint"]["errors"],
                "lint_warnings": grade["lint"]["warnings"],
                "security_findings": sum(grade["security"].values()),
                "description": item["repository"].get("description", ""),
                "stars": _repo_stars(repo_full),
            })
            print(
                f"  graded {repo_full}: {grade['grade']} ({grade['score']}/100)",
                file=sys.stderr,
            )

    # Sort: score desc, then stars desc as tiebreaker
    ranked.sort(key=lambda r: (-r["score"], -r["stars"]))
    return ranked


_star_cache: dict[str, int] = {}


def _repo_stars(full_name: str) -> int:
    if full_name in _star_cache:
        return _star_cache[full_name]
    headers = {"User-Agent": USER_AGENT}
    if GITHUB_TOKEN:
        headers["Authorization"] = f"Bearer {GITHUB_TOKEN}"
    try:
        req = Request(f"https://api.github.com/repos/{full_name}", headers=headers)
        with urlopen(req, timeout=10) as resp:
            n = json.loads(resp.read().decode()).get("stargazers_count", 0)
    except Exception:
        n = 0
    _star_cache[full_name] = n
    return n


def render_markdown(ranked: list[dict], generated_at: str) -> str:
    top = ranked[:TOP_N]
    lines = [
        "# SkillForge Public Skills Leaderboard",
        "",
        f"Auto-generated weekly. Last update: **{generated_at}**.",
        "",
        f"Ranking {len(top)} of {len(ranked)} SKILL.md files discovered on GitHub, "
        "graded by [SkillForge](https://github.com/LiqunChen0606/skillforge).",
        "",
        "| # | Grade | Score | Repository | Stars | Security |",
        "| - | ----- | ----- | ---------- | ----- | -------- |",
    ]
    for i, r in enumerate(top, 1):
        grade_badge = f"**{r['grade']}**"
        sec = "clean" if r["security_findings"] == 0 else f"{r['security_findings']} finding(s)"
        lines.append(
            f"| {i} | {grade_badge} | {r['score']}/100 | "
            f"[{r['repo']}]({r['url']}) | {r['stars']:,} | {sec} |"
        )
    lines.extend([
        "",
        "## Methodology",
        "",
        "- GitHub Code Search for filename `SKILL.md`",
        f"- Top {PER_PAGE * MAX_PAGES} results downloaded and graded with `aif score`",
        "- Scoring: 10 structural lint checks (-7 per error, -3 per warning) "
        "+ 6 OWASP AST10-aligned security checks (-15 Critical, -8 High, "
        "-3 Medium, -1 Low)",
        "- Tiebreaker: repository stars (higher = ranks first)",
        "",
        "## Add your skill to the list",
        "",
        "Any public GitHub repo with a `SKILL.md` file at the default branch "
        "will be included in the next weekly scan. To improve your grade:",
        "",
        "```bash",
        "pip install aif-skillforge",
        "aif fix SKILL.md --write    # autofix deterministic issues",
        "aif score SKILL.md          # check your grade",
        "```",
        "",
        "## Earn a badge",
        "",
        "Add a SkillForge grade badge to your skill's README:",
        "",
        "```bash",
        "aif score SKILL.md --format shields -o badge.json",
        "```",
        "",
        "Then in your README:",
        "",
        "```markdown",
        "![SkillForge](https://img.shields.io/endpoint?url="
        "https://raw.githubusercontent.com/USER/REPO/main/badge.json)",
        "```",
    ])
    return "\n".join(lines) + "\n"


def main() -> int:
    print("Fetching SKILL.md files from GitHub...", file=sys.stderr)
    items = search_skill_md()
    print(f"Found {len(items)} SKILL.md files", file=sys.stderr)

    print("Downloading and grading...", file=sys.stderr)
    ranked = build_leaderboard(items)
    print(f"Graded {len(ranked)} skills successfully", file=sys.stderr)

    if not ranked:
        print("ERROR: no skills graded successfully", file=sys.stderr)
        return 1

    OUTPUT_DIR.mkdir(exist_ok=True)
    generated_at = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    data_file = OUTPUT_DIR / "data.json"
    data_file.write_text(json.dumps({
        "generated_at": generated_at,
        "total_graded": len(ranked),
        "skills": ranked,
    }, indent=2))

    md_file = OUTPUT_DIR / "LEADERBOARD.md"
    md_file.write_text(render_markdown(ranked, generated_at))

    print(f"Wrote {md_file} and {data_file}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
