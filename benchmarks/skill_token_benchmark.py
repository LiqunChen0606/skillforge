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
]


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
        texts["md"] = md_text
        for key, _, cli_fmt in FORMATS:
            if cli_fmt is None:
                continue
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
            nbytes = len(text.encode("utf-8"))
            r[f"{key}_tokens"] = tokens
            r[f"{key}_bytes"] = nbytes

            if md_tokens is None:
                md_tokens = tokens
                r[f"{key}_save_pct"] = 0.0
            else:
                r[f"{key}_save_pct"] = pct(md_tokens, tokens)

            # Compliance scoring
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
