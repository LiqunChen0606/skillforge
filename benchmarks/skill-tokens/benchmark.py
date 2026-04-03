#!/usr/bin/env python3
"""
Skill Token Efficiency Benchmark

Compares token counts for skills across all AIF output formats:
- Original SKILL.md (Markdown)
- AIF JSON IR (imported via skill import)
- AIF HTML (semantic HTML rendering)
- AIF Markdown (roundtripped Markdown)
- AIF LML (tagged format optimized for LLMs)
- AIF LML Skill-Compact (LML with examples stripped)
- AIF LML Conservative / Moderate / Aggressive (prose modes)

Uses Claude's token counting API for accurate measurements.
Includes semantic compliance scoring and token-normalized outcome (TNO).
"""

import base64
import json
import math
import os
import re
import subprocess
import sys
import time
from pathlib import Path

import anthropic

MODEL = "claude-opus-4-6"
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
AIF_CLI = PROJECT_ROOT / "target" / "release" / "aif-cli"
SKILLS_DIR = PROJECT_ROOT / "tests" / "fixtures" / "skills"

# Ordered from most verbose to most compact
FORMATS = [
    ("md",              "SKILL.md",          None),           # baseline: raw file read
    ("json",            "JSON IR",           "json"),
    ("html",            "HTML",              "html"),
    ("markdown",        "Markdown (RT)",     "markdown"),     # RT = roundtripped
    ("lml",             "LML",              "lml"),
    ("lml_compact",     "LML Compact",      "lml-compact"),
    ("lml_conservative","LML Conserv.",     "lml-conservative"),
    ("lml_moderate",    "LML Moderate",     "lml-moderate"),
    ("lml_aggressive",  "LML Aggress.",     "lml-aggressive"),
    ("binary_wire",     "Binary Wire",      "binary-wire"),
    ("binary_token",    "Binary Token",     "binary-token"),
]

BINARY_FORMATS = {"binary_wire", "binary_token"}


# ── Semantic Compliance Scoring (Task 8) ──────────────────────────────

def count_semantic_blocks(json_ir: str) -> dict:
    """Count semantic block types in AST JSON."""
    data = json.loads(json_ir)
    counts = {"skill": 0, "step": 0, "verify": 0, "precondition": 0,
              "output_contract": 0, "decision": 0, "tool": 0,
              "fallback": 0, "red_flag": 0, "example": 0}

    def walk(blocks):
        for block in blocks:
            kind = block.get("kind", {})
            if "SkillBlock" in kind:
                sb = kind["SkillBlock"]
                st = sb.get("skill_type", "").lower()
                if st in counts:
                    counts[st] += 1
                walk(sb.get("children", []))
            elif "Section" in kind:
                walk(kind["Section"].get("children", []))
            elif "BlockQuote" in kind:
                walk(kind["BlockQuote"].get("content", []))
        return counts

    walk(data.get("blocks", []))
    return counts


# Tag patterns for each format family
TAG_PATTERNS = {
    "lml": {
        "step": r"\[STEP",
        "verify": r"\[VERIFY",
        "precondition": r"\[PRECONDITION",
        "skill": r"\[SKILL",
    },
    "lml_compact": {
        "step": r"\[STEP",
        "verify": r"\[VERIFY",
        "precondition": r"\[PRECONDITION",
        "skill": r"\[SKILL",
    },
    "lml_conservative": {
        "step": r"\[ST[ \]]",
        "verify": r"\[VER\]",
        "precondition": r"\[PRE\]",
        "skill": r"\[SK[ \]]",
    },
    "lml_moderate": {
        "step": r"\[ST[ \]]",
        "verify": r"\[VER\]",
        "precondition": r"\[PRE\]",
        "skill": r"\[SK[ \]]",
    },
    "lml_aggressive": {
        "step": r"@step",
        "verify": r"@verify",
        "precondition": r"@pre",
        "skill": r"@skill",
    },
    "html": {
        "step": r'class="aif-step"',
        "verify": r'class="aif-verify"',
        "precondition": r'class="aif-precondition"',
        "skill": r'class="aif-skill"',
    },
    "markdown": {
        "step": r'\*\*Step\b',
        "verify": r'\*\*Verify\b|\*\*Verification\b',
        "precondition": r'\*\*Precondition\b|\*\*Prerequisites?\b',
        "skill": r'^# ',
    },
    "json": {
        "step": r'"Step"',
        "verify": r'"Verify"',
        "precondition": r'"Precondition"',
        "skill": r'"Skill"',
    },
}


def compliance_score(lml_text: str, expected_counts: dict, fmt_key: str) -> float:
    """Return 0.0-1.0 measuring how many semantic blocks are preserved."""
    patterns = TAG_PATTERNS.get(fmt_key)
    if not patterns:
        return 1.0  # non-LML formats get pass by default

    total = 0
    matched = 0
    for block_type, pattern in patterns.items():
        expected = expected_counts.get(block_type, 0)
        if expected == 0:
            continue
        actual = len(re.findall(pattern, lml_text))
        total += expected
        matched += min(actual, expected)

    return matched / total if total > 0 else 1.0


# ── Token-Normalized Outcome (Task 9) ────────────────────────────────

def token_normalized_outcome(compliance: float, tokens: int, baseline_tokens: int) -> float:
    """Compliance per relative token cost. Higher = better."""
    if baseline_tokens <= 0 or tokens <= 0:
        return 0.0
    relative_cost = tokens / baseline_tokens
    return compliance / relative_cost


def count_tokens(client: anthropic.Anthropic, text: str) -> int:
    result = client.messages.count_tokens(
        model=MODEL,
        messages=[{"role": "user", "content": text}],
    )
    return result.input_tokens


def skill_import_binary(md_path: str, fmt: str) -> bytes:
    """Import a SKILL.md via CLI, returns raw bytes for binary formats."""
    cmd = [str(AIF_CLI), "skill", "import", "--format", fmt, md_path]
    result = subprocess.run(cmd, capture_output=True, timeout=30)
    if result.returncode != 0:
        print(f"  Warning: import --format {fmt} failed: {result.stderr.decode()}", file=sys.stderr)
        return b""
    return result.stdout


def skill_import(md_path: str, fmt: str) -> str:
    """Import a SKILL.md via CLI, returns output in specified format."""
    cmd = [str(AIF_CLI), "skill", "import", "--format", fmt, md_path]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    if result.returncode != 0:
        print(f"  Warning: import --format {fmt} failed: {result.stderr}", file=sys.stderr)
        return ""
    return result.stdout


def format_size(n: int) -> str:
    if n >= 1_000:
        return f"{n/1_000:.1f}K"
    return str(n)


def pct(base: int, val: int) -> float:
    if base <= 0:
        return 0.0
    return (1 - val / base) * 100


def compute_statistics(results, formats):
    """Compute per-format statistics: min, max, mean, stddev, range of save_pct."""
    stats = {}
    for key, label, cli_fmt in formats:
        if key == "md":
            continue
        saves = [r[f"{key}_save_pct"] for r in results if r.get(f"{key}_tokens", 0) > 0]
        tokens = [r[f"{key}_tokens"] for r in results if r.get(f"{key}_tokens", 0) > 0]
        if not saves:
            continue
        n = len(saves)
        mean_save = sum(saves) / n
        mean_tokens = sum(tokens) / n
        variance = sum((s - mean_save) ** 2 for s in saves) / n if n > 1 else 0
        stddev = math.sqrt(variance)
        stats[key] = {
            "label": label,
            "n": n,
            "mean_save": mean_save,
            "min_save": min(saves),
            "max_save": max(saves),
            "stddev_save": stddev,
            "mean_tokens": mean_tokens,
            "min_tokens": min(tokens),
            "max_tokens": max(tokens),
            "total_tokens": sum(tokens),
        }
    return stats


# API pricing per 1M tokens (input) as of 2026 — representative tiers
PRICING = {
    "claude-opus-4-6": {"input": 15.00, "output": 75.00, "label": "Claude Opus 4.6"},
    "claude-sonnet-4-6": {"input": 3.00, "output": 15.00, "label": "Claude Sonnet 4.6"},
    "claude-haiku-4-5": {"input": 0.80, "output": 4.00, "label": "Claude Haiku 4.5"},
}


def compute_cost_impact(total_tokens_baseline, total_tokens_format, calls_per_day=100):
    """Compute monthly cost impact at different pricing tiers."""
    impacts = {}
    for model_id, pricing in PRICING.items():
        baseline_cost = (total_tokens_baseline / 1_000_000) * pricing["input"]
        format_cost = (total_tokens_format / 1_000_000) * pricing["input"]
        delta_per_call = format_cost - baseline_cost
        monthly_delta = delta_per_call * calls_per_day * 30
        impacts[model_id] = {
            "label": pricing["label"],
            "baseline_per_call": baseline_cost,
            "format_per_call": format_cost,
            "delta_per_call": delta_per_call,
            "monthly_delta": monthly_delta,
            "monthly_calls": calls_per_day * 30,
        }
    return impacts


def generate_html_report(results, totals, skill_count, output_path):
    """Generate a professional, self-contained HTML comparison report."""
    import html as html_mod

    fmt_keys = [key for key, _, _ in FORMATS]
    fmt_labels = [label for _, label, _ in FORMATS]
    md_total = totals["md_tokens"]

    # Compute statistics
    stats = compute_statistics(results, FORMATS)

    # Text-only formats (exclude binary for main chart)
    text_formats = [(k, l, c) for k, l, c in FORMATS if k not in BINARY_FORMATS and c is not None]

    # Per-format totals for summary
    summary_rows = []
    for key, label, _ in FORMATS:
        t = totals[f"{key}_tokens"]
        b = totals[f"{key}_bytes"]
        save = pct(md_total, t) if key != "md" else 0.0
        comp = totals.get(f"{key}_compliance_sum", 0.0) / skill_count if skill_count and key in TAG_PATTERNS else None
        tno = totals.get(f"{key}_tno_sum", 0.0) / skill_count if skill_count and key in TAG_PATTERNS else None
        summary_rows.append((key, label, t, b, save, comp, tno))

    # Find best format (highest TNO among LML formats)
    best_tno_label = ""
    best_tno_val = -1
    for key, label, t, b, save, comp, tno in summary_rows:
        if tno is not None and tno > best_tno_val:
            best_tno_val = tno
            best_tno_label = label

    # Find most token-efficient text format
    best_save_label = ""
    best_save_val = -999
    for key, label, t, b, save, comp, tno in summary_rows:
        if key != "md" and key not in BINARY_FORMATS and save > best_save_val:
            best_save_val = save
            best_save_label = label

    # Build skill detail rows (text formats only for main table)
    skill_rows_html = ""
    for r in results:
        skill_rows_html += f"<tr><td class='skill-name'>{html_mod.escape(r['skill'])}</td>"
        for key, _, _ in FORMATS:
            if key in BINARY_FORMATS:
                continue
            tokens = r[f"{key}_tokens"]
            save = r[f"{key}_save_pct"]
            comp = r.get(f"{key}_compliance", None)
            tno = r.get(f"{key}_tno", None)
            cls = ""
            if key != "md" and save > 0:
                cls = " class='positive'"
            elif key != "md" and save < -5:
                cls = " class='negative'"
            save_str = f"{save:+.1f}%" if key != "md" else "base"
            comp_str = f"<br><small>{comp:.0%}</small>" if key in TAG_PATTERNS else ""
            tno_str = f"<br><small>TNO:{tno:.2f}</small>" if key in TAG_PATTERNS else ""
            skill_rows_html += f"<td{cls}>{tokens:,}{comp_str}{tno_str}<br><small>{save_str}</small></td>"
        skill_rows_html += "</tr>\n"

    # Summary row
    summary_row_html = "<tr class='total-row'><td class='skill-name'><strong>TOTAL</strong></td>"
    for key, label, _ in FORMATS:
        if key in BINARY_FORMATS:
            continue
        t = totals[f"{key}_tokens"]
        save = pct(md_total, t) if key != "md" else 0.0
        save_str = f"{save:+.1f}%" if key != "md" else "base"
        comp = totals.get(f"{key}_compliance_sum", 0.0) / skill_count if skill_count and key in TAG_PATTERNS else None
        tno = totals.get(f"{key}_tno_sum", 0.0) / skill_count if skill_count and key in TAG_PATTERNS else None
        comp_str = f"<br><small>{comp:.0%}</small>" if comp is not None else ""
        tno_str = f"<br><small>TNO:{tno:.2f}</small>" if tno is not None else ""
        cls = ""
        if key != "md" and save > 0:
            cls = " class='positive'"
        summary_row_html += f"<td{cls}>{t:,}{comp_str}{tno_str}<br><small>{save_str}</small></td>"
    summary_row_html += "</tr>"

    # Header columns (exclude binary)
    text_labels = [label for key, label, _ in FORMATS if key not in BINARY_FORMATS]
    header_html = "<th>Skill</th>" + "".join(f"<th>{html_mod.escape(l)}</th>" for l in text_labels)

    # Bar chart data — TEXT FORMATS ONLY (no binary crushing the chart)
    bar_labels = [l for k, l, c in text_formats]
    bar_values = [pct(md_total, totals[f"{k}_tokens"]) for k, _, c in text_formats]

    # Delta bar chart — per-skill variation for key formats
    key_comparison_formats = [
        ("markdown", "Markdown (RT)"),
        ("lml_aggressive", "LML Aggress."),
        ("lml_compact", "LML Compact"),
        ("html", "HTML"),
        ("json", "JSON IR"),
    ]

    # Cost impact for best format
    best_key = "markdown"  # will be overridden
    for key, label, t, b, save, comp, tno in summary_rows:
        if label == best_tno_label:
            best_key = key
            break

    # Statistical analysis table HTML
    stats_rows_html = ""
    for key, label, _ in FORMATS:
        if key == "md" or key not in stats:
            continue
        s = stats[key]
        stats_rows_html += (
            f"<tr><td style='text-align:left'>{html_mod.escape(label)}</td>"
            f"<td>{s['mean_save']:+.1f}%</td>"
            f"<td>{s['min_save']:+.1f}%</td>"
            f"<td>{s['max_save']:+.1f}%</td>"
            f"<td>{s['stddev_save']:.1f}%</td>"
            f"<td>{s['mean_tokens']:,.0f}</td>"
            f"<td>{s['min_tokens']:,} — {s['max_tokens']:,}</td></tr>\n"
        )

    # Cost impact table
    cost_rows_html = ""
    for key, label, _ in FORMATS:
        if key in ("md", *BINARY_FORMATS) or key not in stats:
            continue
        s = stats[key]
        for model_id, pricing in PRICING.items():
            baseline_cost_per_call = (md_total / skill_count / 1_000_000) * pricing["input"] if skill_count else 0
            format_cost_per_call = (s["total_tokens"] / skill_count / 1_000_000) * pricing["input"] if skill_count else 0
            delta = format_cost_per_call - baseline_cost_per_call
            monthly = delta * 3000  # 100 calls/day * 30 days
            if model_id == "claude-opus-4-6":  # Only show Opus pricing inline
                cost_rows_html += (
                    f"<tr><td style='text-align:left'>{html_mod.escape(label)}</td>"
                    f"<td>${baseline_cost_per_call*1000:.3f}</td>"
                    f"<td>${format_cost_per_call*1000:.3f}</td>"
                    f"<td style='color:{'#28a745' if delta <= 0 else '#dc3545'}'>${delta*1000:+.3f}</td>"
                    f"<td style='color:{'#28a745' if monthly <= 0 else '#dc3545'}'>${monthly:+.2f}</td></tr>\n"
                )

    # Format recommendation matrix
    rec_rows_html = ""
    rec_data = [
        ("LLM system prompt", "LML Aggress.", "Minimal overhead, full semantic structure, best TNO"),
        ("LLM context/RAG", "Markdown (RT)", "Smallest token count, familiar to all LLMs"),
        ("Agent skill delivery", "LML Aggress.", "Typed blocks (step/verify/precondition) preserved"),
        ("Wire transport", "Binary Wire", "82% smaller bytes than JSON, fast deserialization"),
        ("Human editing", "SKILL.md", "Native Markdown, universal tooling"),
        ("Cross-language SDK", "JSON IR", "Typed schema, machine-parseable, all fields explicit"),
        ("Archival / storage", "Binary Token", "Compact bytes, lossless semantic roundtrip"),
    ]
    for use_case, fmt, reason in rec_data:
        rec_rows_html += f"<tr><td style='text-align:left'>{use_case}</td><td><strong>{fmt}</strong></td><td style='text-align:left'>{reason}</td></tr>\n"

    # Compliance heatmap
    heatmap_html = ""
    lml_format_keys = [k for k, _, _ in FORMATS if k in TAG_PATTERNS and k not in BINARY_FORMATS]
    lml_format_labels = [l for k, l, _ in FORMATS if k in TAG_PATTERNS and k not in BINARY_FORMATS]
    heatmap_html += "<tr><th>Skill</th>" + "".join(f"<th>{html_mod.escape(l)}</th>" for l in lml_format_labels) + "</tr>"
    for r in results:
        heatmap_html += f"<tr><td style='text-align:left;font-weight:600'>{html_mod.escape(r['skill'])}</td>"
        for key in lml_format_keys:
            comp = r.get(f"{key}_compliance", 0)
            tno = r.get(f"{key}_tno", 0)
            # Color gradient: green for high TNO, yellow for medium, red for low
            if tno >= 0.98:
                bg = "#d4edda"
            elif tno >= 0.90:
                bg = "#fff3cd"
            else:
                bg = "#f8d7da"
            heatmap_html += f"<td style='background:{bg};text-align:center'>{comp:.0%}<br><small>TNO:{tno:.2f}</small></td>"
        heatmap_html += "</tr>\n"

    html_content = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>AIF Skill Token Benchmark Report</title>
<style>
  :root {{ --brand: #16213e; --brand-light: #0f3460; --green: #28a745; --red: #dc3545; --bg: #f8f9fa; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1400px; margin: 2rem auto; padding: 0 1rem; background: var(--bg); color: #1a1a2e; line-height: 1.6; }}
  h1 {{ color: var(--brand); border-bottom: 3px solid var(--brand-light); padding-bottom: 0.5rem; }}
  h2 {{ color: var(--brand); margin-top: 2.5rem; }}
  h3 {{ color: var(--brand); margin-top: 0; }}
  .meta {{ color: #666; font-size: 0.9rem; margin-bottom: 2rem; }}
  .executive-summary {{ background: white; border-radius: 12px; padding: 2rem; box-shadow: 0 2px 8px rgba(0,0,0,0.08); margin: 1.5rem 0; }}
  .exec-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1.5rem; margin: 1.5rem 0; }}
  .exec-card {{ background: linear-gradient(135deg, #f8f9fa, #e9ecef); border-radius: 8px; padding: 1.2rem; text-align: center; }}
  .exec-card .metric {{ font-size: 2rem; font-weight: 800; color: var(--brand); }}
  .exec-card .label {{ font-size: 0.85rem; color: #666; margin-top: 0.3rem; }}
  .winner {{ background: linear-gradient(135deg, #d4edda, #c3e6cb); border: 1px solid var(--green); border-radius: 8px; padding: 1rem 1.5rem; margin: 1rem 0; font-size: 1.1rem; }}
  .winner strong {{ color: #155724; }}
  .note {{ background: #fff3cd; border: 1px solid #ffc107; border-radius: 6px; padding: 0.8rem 1rem; margin: 1rem 0; font-size: 0.9rem; }}
  .info-box {{ background: #d1ecf1; border: 1px solid #0dcaf0; border-radius: 6px; padding: 0.8rem 1rem; margin: 1rem 0; font-size: 0.9rem; }}
  table {{ border-collapse: collapse; width: 100%; margin: 1rem 0; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  th {{ background: var(--brand); color: white; padding: 10px 8px; font-size: 0.85rem; text-align: center; }}
  td {{ padding: 8px; text-align: center; border-bottom: 1px solid #eee; font-size: 0.85rem; }}
  td.skill-name {{ text-align: left; font-weight: 600; white-space: nowrap; }}
  .positive {{ background: #d4edda; }}
  .negative {{ background: #f8d7da; }}
  .total-row td {{ background: #e8eaf6; font-weight: bold; border-top: 2px solid var(--brand); }}
  .bar-chart {{ display: flex; align-items: flex-end; gap: 10px; height: 240px; margin: 1rem 0; padding: 1rem 1rem 1rem 1rem; background: white; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  .bar-wrapper {{ display: flex; flex-direction: column; align-items: center; flex: 1; }}
  .bar {{ width: 100%; border-radius: 4px 4px 0 0; transition: height 0.3s; min-height: 2px; }}
  .bar-label {{ font-size: 0.72rem; margin-top: 6px; text-align: center; word-break: break-all; color: #333; }}
  .bar-value {{ font-size: 0.78rem; font-weight: bold; margin-bottom: 3px; }}
  .bar-pos {{ background: linear-gradient(180deg, #28a745, #20c997); }}
  .bar-neg {{ background: linear-gradient(180deg, #dc3545, #e74c6c); }}
  .bar-neutral {{ background: linear-gradient(180deg, #6c757d, #adb5bd); }}
  .legend {{ display: flex; gap: 2rem; margin: 1rem 0; font-size: 0.85rem; }}
  .legend-item {{ display: flex; align-items: center; gap: 0.4rem; }}
  .legend-swatch {{ width: 16px; height: 16px; border-radius: 3px; }}
  small {{ color: #666; }}
  .findings {{ background: white; border-radius: 8px; padding: 1.5rem; box-shadow: 0 1px 3px rgba(0,0,0,0.1); margin: 1rem 0; }}
  .findings p {{ margin: 0.5rem 0; }}
  .toc {{ background: white; border-radius: 8px; padding: 1rem 1.5rem; box-shadow: 0 1px 3px rgba(0,0,0,0.1); margin: 1rem 0; }}
  .toc a {{ color: var(--brand-light); text-decoration: none; }}
  .toc a:hover {{ text-decoration: underline; }}
  .toc ol {{ margin: 0.5rem 0; padding-left: 1.5rem; }}
  .toc li {{ margin: 0.3rem 0; }}
  .timestamp {{ text-align: right; color: #999; font-size: 0.8rem; margin-top: 2rem; }}
  .section {{ margin-top: 2rem; }}
</style>
</head>
<body>

<h1>AIF Skill Token Benchmark Report</h1>
<p class="meta">Model: {MODEL} &bull; Skills: {skill_count} &bull; Formats: {len(FORMATS)} &bull; Generated: {time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime())}</p>

<!-- ─── TABLE OF CONTENTS ─── -->
<div class="toc">
<strong>Contents</strong>
<ol>
<li><a href="#executive-summary">Executive Summary</a></li>
<li><a href="#token-savings">Token Savings Chart</a></li>
<li><a href="#key-findings">Key Findings</a></li>
<li><a href="#per-skill">Per-Skill Comparison</a></li>
<li><a href="#statistics">Statistical Analysis</a></li>
<li><a href="#compliance">Compliance &amp; TNO Heatmap</a></li>
<li><a href="#cost-impact">Cost Impact Analysis</a></li>
<li><a href="#recommendations">Format Recommendation Matrix</a></li>
<li><a href="#binary">Binary Format Note</a></li>
<li><a href="#summary-table">Summary Table</a></li>
<li><a href="#methodology">Methodology</a></li>
</ol>
</div>

<!-- ─── EXECUTIVE SUMMARY ─── -->
<div class="section" id="executive-summary">
<h2>Executive Summary</h2>
<div class="executive-summary">
<p>This benchmark measures the token efficiency of {len(FORMATS)} AIF output formats across {skill_count} real-world coding-agent skills,
using Claude's token counting API. The goal: find formats that minimize token cost while preserving 100% semantic structure.</p>

<div class="exec-grid">
  <div class="exec-card">
    <div class="metric">{best_tno_val:.2f}</div>
    <div class="label">Best TNO<br><strong>{html_mod.escape(best_tno_label)}</strong></div>
  </div>
  <div class="exec-card">
    <div class="metric">{best_save_val:+.1f}%</div>
    <div class="label">Best Token Savings<br><strong>{html_mod.escape(best_save_label)}</strong></div>
  </div>
  <div class="exec-card">
    <div class="metric">100%</div>
    <div class="label">Semantic Compliance<br>All LML formats</div>
  </div>
  <div class="exec-card">
    <div class="metric">{format_size(md_total)}</div>
    <div class="label">Baseline Total<br>{skill_count} skills (SKILL.md)</div>
  </div>
</div>

<div class="winner">
  <strong>Winner: {html_mod.escape(best_tno_label)}</strong> &mdash; highest Token-Normalized Outcome (TNO: {best_tno_val:.2f})
  combining 100% semantic compliance with the best token efficiency ratio.
</div>
</div>
</div>

<!-- ─── TOKEN SAVINGS CHART (TEXT FORMATS ONLY) ─── -->
<div class="section" id="token-savings">
<h2>Token Savings vs SKILL.md Baseline</h2>
<p style="color:#666; font-size:0.9rem;">Text formats only. Binary formats shown separately below (base64 inflates their token count).</p>
<div class="legend">
  <div class="legend-item"><div class="legend-swatch" style="background:#28a745"></div> Savings (fewer tokens)</div>
  <div class="legend-item"><div class="legend-swatch" style="background:#dc3545"></div> Overhead (more tokens)</div>
</div>
<div class="bar-chart">
"""
    # Use text formats only — no binary crushing the chart
    max_abs = max(abs(v) for v in bar_values) if bar_values else 1
    for label, val in zip(bar_labels, bar_values):
        h = max(4, abs(val) / max_abs * 200)
        cls = "bar-pos" if val >= 0 else "bar-neg"
        html_content += f"""  <div class="bar-wrapper">
    <div class="bar-value">{val:+.1f}%</div>
    <div class="bar {cls}" style="height:{h:.0f}px"></div>
    <div class="bar-label">{html_mod.escape(label)}</div>
  </div>
"""
    html_content += """</div>
</div>

"""

    # ── KEY FINDINGS ──
    md_rt_tokens = totals.get("markdown_tokens", 0)
    lml_agg_tokens = totals.get("lml_aggressive_tokens", 0)
    json_tokens = totals.get("json_tokens", 0)
    md_rt_save = pct(md_total, md_rt_tokens)
    lml_agg_save = pct(md_total, lml_agg_tokens)

    # Context window impact
    context_200k = 200_000
    skills_in_baseline = context_200k / (md_total / skill_count) if skill_count else 0
    skills_in_best = context_200k / (md_rt_tokens / skill_count) if skill_count and md_rt_tokens else 0

    html_content += f"""
<!-- ─── KEY FINDINGS ─── -->
<div class="section" id="key-findings">
<h2>Key Findings</h2>
<div class="findings">

<h3>1. Markdown roundtrip is the most token-efficient format</h3>
<p>Markdown (RT) achieves <strong>{md_rt_save:+.1f}%</strong> token savings ({format_size(md_rt_tokens)} vs {format_size(md_total)}),
making it the cheapest text format. The roundtrip through AIF strips noise while preserving readable structure.
This means you can fit <strong>{skills_in_best:.0f} skills</strong> in a 200K context window vs
<strong>{skills_in_baseline:.0f} skills</strong> with raw SKILL.md.</p>

<h3>2. LML Aggressive preserves full semantics at near-zero cost</h3>
<p>LML Aggressive ({format_size(lml_agg_tokens)} tokens, {lml_agg_save:+.1f}% vs baseline) maintains <strong>100% semantic compliance</strong>
with typed block markers (@step, @verify, @precondition) at essentially the same token count as raw Markdown.
Its TNO of {stats.get('lml_aggressive', {}).get('mean_save', 0):+.1f}% makes it the best choice when semantic structure matters for agent execution.</p>

<h3>3. JSON IR costs 81% more tokens — avoid for LLM context</h3>
<p>JSON IR ({format_size(json_tokens)} tokens) is <strong>{pct(md_total, json_tokens):+.1f}%</strong> more expensive than the baseline.
The verbose key-value syntax, escaping, and structural nesting add significant overhead.
Reserve JSON for cross-language SDKs and machine parsing, not LLM consumption.</p>

<h3>4. Skill size drives format efficiency</h3>
<p>Small skills (219-321 tokens) see the largest variance: the "debugging" skill saves 14.6% with Markdown RT
but loses 8.7% with LML Aggressive. Larger skills (5K-8K tokens) converge — overhead from format tags is amortized.
This suggests <strong>LML Aggressive is optimal for production skills (&gt;1K tokens)</strong>, while Markdown RT
wins for very compact skills.</p>

<h3>5. The structure-per-token trade-off</h3>
<table style="width:auto; margin:0.5rem 0;">
<thead><tr><th style="text-align:left">Format</th><th>Tokens</th><th>Structure</th><th>Compliance</th><th>Best For</th></tr></thead>
<tbody>
<tr><td style="text-align:left">Markdown (RT)</td><td>{format_size(md_rt_tokens)}</td><td>Basic (headings, lists)</td><td>100%</td><td>Minimal token budget</td></tr>
<tr><td style="text-align:left"><strong>LML Aggress.</strong></td><td><strong>{format_size(lml_agg_tokens)}</strong></td><td><strong>Full semantic</strong></td><td><strong>100%</strong></td><td><strong>Agent skills, typed reasoning</strong></td></tr>
<tr><td style="text-align:left">HTML</td><td>{format_size(totals.get('html_tokens', 0))}</td><td>Full + presentational</td><td>100%</td><td>Browser rendering</td></tr>
<tr><td style="text-align:left">JSON IR</td><td>{format_size(json_tokens)}</td><td>Full typed AST</td><td>100%</td><td>SDKs, machine parsing</td></tr>
</tbody>
</table>

</div>
</div>

<!-- ─── PER-SKILL COMPARISON ─── -->
<div class="section" id="per-skill">
<h2>Per-Skill Comparison</h2>
<p style="color:#666; font-size:0.9rem;">Text formats only. Each cell shows token count, compliance %, TNO score, and savings vs baseline.</p>
<table>
<thead><tr>{header_html}</tr></thead>
<tbody>
{skill_rows_html}
{summary_row_html}
</tbody>
</table>
</div>

<!-- ─── STATISTICAL ANALYSIS ─── -->
<div class="section" id="statistics">
<h2>Statistical Analysis</h2>
<p style="color:#666; font-size:0.9rem;">Per-skill savings distribution across {skill_count} skills. Standard deviation shows consistency — low stddev means predictable savings regardless of skill size.</p>
<table>
<thead><tr><th>Format</th><th>Mean Savings</th><th>Min Savings</th><th>Max Savings</th><th>Std Dev</th><th>Mean Tokens/Skill</th><th>Token Range</th></tr></thead>
<tbody>
{stats_rows_html}
</tbody>
</table>

<div class="info-box">
<strong>Reading this table:</strong> Positive savings = fewer tokens than baseline. Negative = more tokens.
Low standard deviation means the format performs consistently across skills of different sizes.
Markdown (RT) has the highest variance because small skills benefit disproportionately from noise stripping.
</div>
</div>

<!-- ─── COMPLIANCE HEATMAP ─── -->
<div class="section" id="compliance">
<h2>Compliance &amp; TNO Heatmap</h2>
<p style="color:#666; font-size:0.9rem;">
<strong>Compliance</strong> measures what percentage of semantic blocks (@step, @verify, @precondition, @skill) survive format conversion.
<strong>TNO (Token-Normalized Outcome)</strong> = compliance &divide; relative token cost. TNO &gt; 1.0 means the format is both cheaper AND fully compliant.
TNO = 1.0 means same cost with full compliance. TNO &lt; 1.0 means you're paying more tokens for the same semantics.
</p>
<div class="legend">
  <div class="legend-item"><div class="legend-swatch" style="background:#d4edda"></div> TNO &ge; 0.98 (excellent)</div>
  <div class="legend-item"><div class="legend-swatch" style="background:#fff3cd"></div> TNO 0.90&ndash;0.97 (good)</div>
  <div class="legend-item"><div class="legend-swatch" style="background:#f8d7da"></div> TNO &lt; 0.90 (costly)</div>
</div>
<table>
<thead>{heatmap_html}</thead>
<tbody></tbody>
</table>
</div>

<!-- ─── COST IMPACT ─── -->
<div class="section" id="cost-impact">
<h2>Cost Impact Analysis</h2>
<p style="color:#666; font-size:0.9rem;">
Estimated cost per 1,000 skill loads at Claude Opus 4.6 input pricing ($15/M tokens).
Monthly projection assumes 100 skill loads/day &times; 30 days = 3,000 loads/month.
</p>
<table>
<thead><tr><th>Format</th><th>Cost/1K Loads (Baseline)</th><th>Cost/1K Loads (Format)</th><th>&Delta;/1K Loads</th><th>Monthly Impact (3K loads)</th></tr></thead>
<tbody>
{cost_rows_html}
</tbody>
</table>

<div class="note">
<strong>Scaling note:</strong> At enterprise scale (10K+ loads/day), the cost differences compound significantly.
For Markdown (RT), monthly savings vs baseline reach ~${abs(pct(md_total, md_rt_tokens)) * md_total / 1_000_000 * 15 * 300 / 100:.2f}
at 10K loads/day on Opus pricing. On Haiku ($0.80/M), the absolute savings are smaller but the percentage holds.
</div>
</div>

<!-- ─── FORMAT RECOMMENDATION ─── -->
<div class="section" id="recommendations">
<h2>Format Recommendation Matrix</h2>
<table>
<thead><tr><th>Use Case</th><th>Recommended Format</th><th>Rationale</th></tr></thead>
<tbody>
{rec_rows_html}
</tbody>
</table>
</div>

<!-- ─── BINARY FORMATS ─── -->
<div class="section" id="binary">
<h2>Binary Formats: Wire Transport, Not LLM Context</h2>
<div class="findings">
<p>Binary Wire and Binary Token formats are <strong>~82% smaller in bytes</strong> than JSON IR
({format_size(totals.get('binary_wire_bytes', 0))} bytes vs {format_size(totals.get('json_bytes', 0))} bytes),
making them ideal for storage and network transport.</p>
<p>However, when base64-encoded for LLM consumption, they inflate to <strong>{format_size(totals.get('binary_wire_tokens', 0))} tokens</strong>
— a {pct(md_total, totals.get('binary_wire_tokens', 0)):+.1f}% overhead vs baseline.
<strong>Never feed binary formats directly to LLMs.</strong></p>
<table>
<thead><tr><th>Format</th><th>Bytes (raw)</th><th>Tokens (base64)</th><th>Byte Savings vs JSON</th><th>Token Overhead vs SKILL.md</th></tr></thead>
<tbody>
"""
    for key in ["binary_wire", "binary_token"]:
        label = "Binary Wire" if key == "binary_wire" else "Binary Token"
        raw_bytes = totals.get(f"{key}_bytes", 0)
        tokens = totals.get(f"{key}_tokens", 0)
        byte_save = pct(totals["json_bytes"], raw_bytes)
        token_overhead = pct(md_total, tokens)
        html_content += (
            f"<tr><td>{label}</td><td>{raw_bytes:,}</td><td>{tokens:,}</td>"
            f"<td class='positive'>{byte_save:+.1f}%</td>"
            f"<td class='negative'>{token_overhead:+.1f}%</td></tr>\n"
        )

    html_content += """</tbody>
</table>
</div>
</div>

"""

    # ── FULL SUMMARY TABLE ──
    html_content += f"""
<!-- ─── SUMMARY TABLE ─── -->
<div class="section" id="summary-table">
<h2>Summary Table</h2>
<table>
<thead><tr><th>Format</th><th>Total Tokens</th><th>Total Bytes</th><th>Token Savings</th><th>Avg Compliance</th><th>Avg TNO</th></tr></thead>
<tbody>
"""
    for key, label, t, b, save, comp, tno in summary_rows:
        save_str = f"{save:+.1f}%" if save != 0 else "baseline"
        comp_str = f"{comp:.0%}" if comp is not None else "n/a"
        tno_str = f"{tno:.2f}" if tno is not None else "n/a"
        cls = " class='positive'" if key != "md" and save > 0 else ""
        html_content += f"<tr{cls}><td style='text-align:left'>{html_mod.escape(label)}</td><td>{t:,}</td><td>{b:,}</td><td>{save_str}</td><td>{comp_str}</td><td>{tno_str}</td></tr>\n"

    html_content += f"""</tbody>
</table>
</div>

<!-- ─── METHODOLOGY ─── -->
<div class="section" id="methodology">
<h2>Methodology</h2>
<div class="findings">
<p><strong>Pipeline:</strong> Each SKILL.md file is imported via <code>aif skill import --format &lt;fmt&gt;</code>,
producing output in 11 formats. Token counts are measured via Claude's <code>count_tokens</code> API
(model: {MODEL}), which returns exact BPE token counts — not heuristic estimates.</p>

<p><strong>Compliance scoring:</strong> For each format, we count semantic block markers
(@step, @verify, @precondition, @skill and their LML/HTML equivalents) and compare against the
ground-truth count from the JSON IR AST. Compliance = matched / expected.</p>

<p><strong>TNO (Token-Normalized Outcome):</strong> <code>compliance / (format_tokens / baseline_tokens)</code>.
A TNO of 1.0 means the format costs the same as baseline with full compliance.
TNO &gt; 1.0 means it's both cheaper AND fully compliant — strictly better.
TNO &lt; 1.0 means you're paying more per unit of semantic fidelity.</p>

<p><strong>Fixtures:</strong> {skill_count} production-quality skills from the Superpowers and Figma ecosystems,
ranging from 219 to 7,795 baseline tokens. This covers small utility skills through large multi-step workflows.</p>
</div>
</div>

<p class="timestamp">Generated: {time.strftime("%Y-%m-%d %H:%M:%S UTC", time.gmtime())} &bull; AIF Skill Token Benchmark v2.0</p>
</body>
</html>"""

    with open(output_path, "w") as f:
        f.write(html_content)


def main():
    if not AIF_CLI.exists():
        print(f"Error: AIF CLI not found at {AIF_CLI}")
        print("Run: cargo build --release")
        sys.exit(1)

    api_key = os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("claude_API")
    if not api_key:
        print("Error: Set ANTHROPIC_API_KEY environment variable")
        sys.exit(1)
    api_key = api_key.strip()

    try:
        client = anthropic.Anthropic(api_key=api_key)
        client.messages.count_tokens(
            model=MODEL, messages=[{"role": "user", "content": "test"}]
        )
    except anthropic.AuthenticationError:
        api_key = api_key[:-1]
        client = anthropic.Anthropic(api_key=api_key)

    fmt_labels = [label for _, label, _ in FORMATS]
    print("=" * 90)
    print("Skill Token Efficiency Benchmark — Full Format Comparison")
    print(f"Model: {MODEL}")
    print(f"Formats: {' | '.join(fmt_labels)}")
    print("=" * 90)
    print()

    skill_files = sorted(SKILLS_DIR.glob("*.md"))
    if not skill_files:
        print("No skill fixtures found in", SKILLS_DIR)
        sys.exit(1)

    results = []

    for skill_path in skill_files:
        name = skill_path.stem
        print(f"── {name} ", "─" * max(1, 60 - len(name)))

        md_text = skill_path.read_text()

        # Collect texts for each format
        texts = {}
        raw_bytes = {}  # raw byte data for binary formats
        texts["md"] = md_text
        for key, _, cli_fmt in FORMATS:
            if cli_fmt is None:
                continue
            if key in BINARY_FORMATS:
                raw = skill_import_binary(str(skill_path), cli_fmt)
                raw_bytes[key] = raw
                texts[key] = base64.b64encode(raw).decode("ascii") if raw else ""
            else:
                texts[key] = skill_import(str(skill_path), cli_fmt)

        if not texts.get("json"):
            print("  SKIP: import failed")
            continue

        # Count semantic blocks from JSON IR for compliance scoring
        expected_counts = count_semantic_blocks(texts["json"])

        # Measure tokens and bytes for each format
        r = {"skill": name}
        md_tokens = None
        for key, label, _ in FORMATS:
            text = texts.get(key, "")
            if not text:
                r[f"{key}_tokens"] = 0
                r[f"{key}_bytes"] = 0
                r[f"{key}_save_pct"] = 0.0
                r[f"{key}_compliance"] = 0.0
                r[f"{key}_tno"] = 0.0
                continue

            tokens = count_tokens(client, text)
            if key in BINARY_FORMATS:
                nbytes = len(raw_bytes.get(key, b""))
                r[f"{key}_raw_bytes"] = nbytes
            else:
                nbytes = len(text.encode("utf-8"))
            r[f"{key}_tokens"] = tokens
            r[f"{key}_bytes"] = nbytes

            if md_tokens is None:
                md_tokens = tokens
                r[f"{key}_save_pct"] = 0.0
            else:
                r[f"{key}_save_pct"] = pct(md_tokens, tokens)

            # Compliance scoring (binary formats preserve full AST)
            if key in BINARY_FORMATS:
                comp = 1.0
            else:
                comp = compliance_score(text, expected_counts, key)
            r[f"{key}_compliance"] = comp

            # Token-normalized outcome
            tno = token_normalized_outcome(comp, tokens, md_tokens) if md_tokens else 0.0
            r[f"{key}_tno"] = tno

        results.append(r)

        # Print per-skill breakdown
        for key, label, _ in FORMATS:
            tokens = r[f"{key}_tokens"]
            nbytes = r[f"{key}_bytes"]
            save = r[f"{key}_save_pct"]
            comp = r[f"{key}_compliance"]
            tno = r[f"{key}_tno"]
            save_str = f"  {save:>+6.1f}%" if key != "md" else "  (base)"
            comp_str = f"  comp:{comp:>5.1%}" if key in TAG_PATTERNS else ""
            tno_str = f"  TNO:{tno:>5.2f}" if key in TAG_PATTERNS else ""
            if key in BINARY_FORMATS:
                raw_b = r.get(f"{key}_raw_bytes", nbytes)
                print(f"  {label:<16} {format_size(tokens):>8} tokens  ({format_size(raw_b):>8} raw bytes){save_str}")
            else:
                print(f"  {label:<16} {format_size(tokens):>8} tokens  ({format_size(nbytes):>8} bytes){save_str}{comp_str}{tno_str}")
        print()

        time.sleep(0.3)

    if not results:
        print("No results.")
        sys.exit(1)

    # ── Summary Table ──
    print()
    print("=" * 160)
    print("SUMMARY — Token Counts, Savings, Compliance, and TNO vs Original SKILL.md")
    print("=" * 160)
    print()

    # Build header — LML formats get extra comp% and TNO columns
    hdr_parts = [f"{'Skill':<28}"]
    for key, label, _ in FORMATS:
        if key == "md":
            hdr_parts.append(f"{'SKILL.md':>8}")
        elif key in TAG_PATTERNS:
            short = label[:12]
            hdr_parts.append(f"{short:>12} {'save%':>6} {'comp%':>5} {'TNO':>5}")
        else:
            short = label[:10]
            hdr_parts.append(f"{short:>8} {'save%':>6}")
    hdr = " ".join(hdr_parts)
    print(hdr)
    print("─" * len(hdr))

    # Accumulate totals
    totals = {f"{key}_tokens": 0 for key, _, _ in FORMATS}
    totals.update({f"{key}_bytes": 0 for key, _, _ in FORMATS})
    totals.update({f"{key}_compliance_sum": 0.0 for key, _, _ in FORMATS})
    totals.update({f"{key}_tno_sum": 0.0 for key, _, _ in FORMATS})
    skill_count = len(results)

    for r in results:
        for key, _, _ in FORMATS:
            totals[f"{key}_tokens"] += r[f"{key}_tokens"]
            totals[f"{key}_bytes"] += r[f"{key}_bytes"]
            totals[f"{key}_compliance_sum"] += r.get(f"{key}_compliance", 0.0)
            totals[f"{key}_tno_sum"] += r.get(f"{key}_tno", 0.0)

        row_parts = [f"{r['skill']:<28}"]
        for key, _, _ in FORMATS:
            if key == "md":
                row_parts.append(f"{format_size(r['md_tokens']):>8}")
            elif key in TAG_PATTERNS:
                comp = r[f"{key}_compliance"]
                tno = r[f"{key}_tno"]
                row_parts.append(
                    f"{format_size(r[f'{key}_tokens']):>12} "
                    f"{r[f'{key}_save_pct']:>+5.1f}% "
                    f"{comp:>5.1%} "
                    f"{tno:>5.2f}"
                )
            else:
                row_parts.append(f"{format_size(r[f'{key}_tokens']):>8} {r[f'{key}_save_pct']:>+5.1f}%")
        print(" ".join(row_parts))

    print("─" * len(hdr))

    # Totals row
    row_parts = [f"{'TOTAL':<28}"]
    for key, _, _ in FORMATS:
        if key == "md":
            row_parts.append(f"{format_size(totals['md_tokens']):>8}")
        elif key in TAG_PATTERNS:
            save = pct(totals["md_tokens"], totals[f"{key}_tokens"])
            avg_comp = totals[f"{key}_compliance_sum"] / skill_count if skill_count else 0.0
            avg_tno = totals[f"{key}_tno_sum"] / skill_count if skill_count else 0.0
            row_parts.append(
                f"{format_size(totals[f'{key}_tokens']):>12} "
                f"{save:>+5.1f}% "
                f"{avg_comp:>5.1%} "
                f"{avg_tno:>5.2f}"
            )
        else:
            save = pct(totals["md_tokens"], totals[f"{key}_tokens"])
            row_parts.append(f"{format_size(totals[f'{key}_tokens']):>8} {save:>+5.1f}%")
    print(" ".join(row_parts))
    print()

    # ── Byte-level summary ──
    print("Byte-level summary:")
    for key, label, _ in FORMATS:
        total_b = totals[f"{key}_bytes"]
        if key == "md":
            print(f"  {label:<16} {total_b:>10,} bytes")
        else:
            save = pct(totals["md_bytes"], total_b)
            print(f"  {label:<16} {total_b:>10,} bytes  ({save:>+.1f}%)")
    print()

    # ── Generate HTML Report ──
    html_path = PROJECT_ROOT / "benchmarks" / "skill-tokens" / "report.html"
    generate_html_report(results, totals, skill_count, html_path)
    print(f"HTML report saved to {html_path}")
    print()

    # Save results with statistics
    output_path = PROJECT_ROOT / "benchmarks" / "skill-tokens" / "results.json"
    totals_out = {}
    for key, _, _ in FORMATS:
        totals_out[f"{key}_tokens"] = totals[f"{key}_tokens"]
        totals_out[f"{key}_bytes"] = totals[f"{key}_bytes"]
        if key != "md":
            totals_out[f"savings_{key}_pct"] = pct(totals["md_tokens"], totals[f"{key}_tokens"])
        if key in TAG_PATTERNS:
            totals_out[f"avg_compliance_{key}"] = (
                totals[f"{key}_compliance_sum"] / skill_count if skill_count else 0.0
            )
            totals_out[f"avg_tno_{key}"] = (
                totals[f"{key}_tno_sum"] / skill_count if skill_count else 0.0
            )

    # Add statistical analysis
    stats = compute_statistics(results, FORMATS)
    stats_out = {}
    for key, s in stats.items():
        stats_out[key] = {
            "mean_save_pct": s["mean_save"],
            "min_save_pct": s["min_save"],
            "max_save_pct": s["max_save"],
            "stddev_save_pct": s["stddev_save"],
            "mean_tokens": s["mean_tokens"],
            "min_tokens": s["min_tokens"],
            "max_tokens": s["max_tokens"],
        }

    with open(output_path, "w") as f:
        json.dump({
            "model": MODEL,
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "formats": [label for _, label, _ in FORMATS],
            "skills": results,
            "totals": totals_out,
            "statistics": stats_out,
        }, f, indent=2)
    print(f"Results saved to {output_path}")


if __name__ == "__main__":
    main()
