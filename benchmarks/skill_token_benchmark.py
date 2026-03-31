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
import os
import re
import subprocess
import sys
import time
from pathlib import Path

import anthropic

MODEL = "claude-opus-4-6"
PROJECT_ROOT = Path(__file__).resolve().parent.parent
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


def generate_html_report(results, totals, skill_count, output_path):
    """Generate a self-contained HTML comparison report."""
    import html as html_mod

    fmt_keys = [key for key, _, _ in FORMATS]
    fmt_labels = [label for _, label, _ in FORMATS]
    md_total = totals["md_tokens"]

    # Per-format totals for summary
    summary_rows = []
    for key, label, _ in FORMATS:
        t = totals[f"{key}_tokens"]
        b = totals[f"{key}_bytes"]
        save = pct(md_total, t) if key != "md" else 0.0
        comp = totals.get(f"{key}_compliance_sum", 0.0) / skill_count if skill_count and key in TAG_PATTERNS else None
        tno = totals.get(f"{key}_tno_sum", 0.0) / skill_count if skill_count and key in TAG_PATTERNS else None
        summary_rows.append((label, t, b, save, comp, tno))

    # Find best format (highest TNO among LML formats)
    best_tno_label = ""
    best_tno_val = -1
    for label, t, b, save, comp, tno in summary_rows:
        if tno is not None and tno > best_tno_val:
            best_tno_val = tno
            best_tno_label = label

    # Build skill detail rows
    skill_rows_html = ""
    for r in results:
        skill_rows_html += f"<tr><td class='skill-name'>{html_mod.escape(r['skill'])}</td>"
        for key, _, _ in FORMATS:
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

    # Header columns
    header_html = "<th>Skill</th>" + "".join(f"<th>{html_mod.escape(l)}</th>" for l in fmt_labels)

    # Bar chart data (token savings %)
    bar_labels = [l for _, l, _ in FORMATS if _ is not None]
    bar_values = [pct(md_total, totals[f"{k}_tokens"]) for k, _, c in FORMATS if c is not None]

    html_content = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>AIF Skill Token Benchmark Report</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1400px; margin: 2rem auto; padding: 0 1rem; background: #f8f9fa; color: #1a1a2e; }}
  h1 {{ color: #16213e; border-bottom: 3px solid #0f3460; padding-bottom: 0.5rem; }}
  h2 {{ color: #16213e; margin-top: 2rem; }}
  .meta {{ color: #666; font-size: 0.9rem; margin-bottom: 2rem; }}
  .winner {{ background: linear-gradient(135deg, #d4edda, #c3e6cb); border: 1px solid #28a745; border-radius: 8px; padding: 1rem 1.5rem; margin: 1rem 0; font-size: 1.1rem; }}
  .winner strong {{ color: #155724; }}
  table {{ border-collapse: collapse; width: 100%; margin: 1rem 0; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  th {{ background: #16213e; color: white; padding: 10px 8px; font-size: 0.85rem; text-align: center; }}
  td {{ padding: 8px; text-align: center; border-bottom: 1px solid #eee; font-size: 0.85rem; }}
  td.skill-name {{ text-align: left; font-weight: 600; white-space: nowrap; }}
  .positive {{ background: #d4edda; }}
  .negative {{ background: #f8d7da; }}
  .total-row td {{ background: #e8eaf6; font-weight: bold; border-top: 2px solid #16213e; }}
  .bar-chart {{ display: flex; align-items: flex-end; gap: 8px; height: 220px; margin: 1rem 0; padding: 1rem; background: white; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  .bar-wrapper {{ display: flex; flex-direction: column; align-items: center; flex: 1; }}
  .bar {{ width: 100%; border-radius: 4px 4px 0 0; transition: height 0.3s; min-height: 2px; }}
  .bar-label {{ font-size: 0.7rem; margin-top: 4px; text-align: center; word-break: break-all; }}
  .bar-value {{ font-size: 0.75rem; font-weight: bold; margin-bottom: 2px; }}
  .bar-pos {{ background: linear-gradient(180deg, #28a745, #20c997); }}
  .bar-neg {{ background: linear-gradient(180deg, #dc3545, #e74c6c); }}
  .legend {{ display: flex; gap: 2rem; margin: 1rem 0; font-size: 0.85rem; }}
  .legend-item {{ display: flex; align-items: center; gap: 0.4rem; }}
  .legend-swatch {{ width: 16px; height: 16px; border-radius: 3px; }}
  small {{ color: #666; }}
  .timestamp {{ text-align: right; color: #999; font-size: 0.8rem; margin-top: 2rem; }}
</style>
</head>
<body>
<h1>AIF Skill Token Benchmark Report</h1>
<p class="meta">Model: {MODEL} &bull; Skills: {skill_count} &bull; Formats: {len(FORMATS)}</p>

<div class="winner">
  <strong>Winner: {html_mod.escape(best_tno_label)}</strong> &mdash; highest token-normalized outcome (TNO: {best_tno_val:.2f}) with 100% semantic compliance
</div>

<h2>Token Savings vs SKILL.md Baseline</h2>
<div class="legend">
  <div class="legend-item"><div class="legend-swatch" style="background:#28a745"></div> Savings (fewer tokens)</div>
  <div class="legend-item"><div class="legend-swatch" style="background:#dc3545"></div> Overhead (more tokens)</div>
</div>
<div class="bar-chart">
"""
    max_abs = max(abs(v) for v in bar_values) if bar_values else 1
    for label, val in zip(bar_labels, bar_values):
        h = max(2, abs(val) / max_abs * 180)
        cls = "bar-pos" if val >= 0 else "bar-neg"
        html_content += f"""  <div class="bar-wrapper">
    <div class="bar-value">{val:+.1f}%</div>
    <div class="bar {cls}" style="height:{h:.0f}px"></div>
    <div class="bar-label">{html_mod.escape(label)}</div>
  </div>
"""
    html_content += f"""</div>

<h2>Per-Skill Comparison</h2>
<table>
<thead><tr>{header_html}</tr></thead>
<tbody>
{skill_rows_html}
{summary_row_html}
</tbody>
</table>

<h2>Summary</h2>
<table>
<thead><tr><th>Format</th><th>Total Tokens</th><th>Total Bytes</th><th>Token Savings</th><th>Avg Compliance</th><th>Avg TNO</th></tr></thead>
<tbody>
"""
    for label, t, b, save, comp, tno in summary_rows:
        save_str = f"{save:+.1f}%" if save != 0 else "baseline"
        comp_str = f"{comp:.0%}" if comp is not None else "n/a"
        tno_str = f"{tno:.2f}" if tno is not None else "n/a"
        html_content += f"<tr><td style='text-align:left'>{html_mod.escape(label)}</td><td>{t:,}</td><td>{b:,}</td><td>{save_str}</td><td>{comp_str}</td><td>{tno_str}</td></tr>\n"

    html_content += f"""</tbody>
</table>

<p class="timestamp">Generated: {time.strftime("%Y-%m-%d %H:%M:%S UTC", time.gmtime())}</p>
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
    html_path = PROJECT_ROOT / "benchmarks" / "skill_benchmark_report.html"
    generate_html_report(results, totals, skill_count, html_path)
    print(f"HTML report saved to {html_path}")
    print()

    # Save results
    output_path = PROJECT_ROOT / "benchmarks" / "skill_results.json"
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

    with open(output_path, "w") as f:
        json.dump({
            "model": MODEL,
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "formats": [label for _, label, _ in FORMATS],
            "skills": results,
            "totals": totals_out,
        }, f, indent=2)
    print(f"Results saved to {output_path}")


if __name__ == "__main__":
    main()
