#!/usr/bin/env python3
"""
Skill Token Efficiency Benchmark

Compares token counts for skills in different formats:
- Original SKILL.md (Markdown)
- AIF JSON IR (imported)

Uses Claude's token counting API for accurate measurements.
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import anthropic

MODEL = "claude-opus-4-6"
PROJECT_ROOT = Path(__file__).resolve().parent.parent
AIF_CLI = PROJECT_ROOT / "target" / "release" / "aif-cli"
SKILLS_DIR = PROJECT_ROOT / "tests" / "fixtures" / "skills"


def count_tokens(client: anthropic.Anthropic, text: str) -> int:
    result = client.messages.count_tokens(
        model=MODEL,
        messages=[{"role": "user", "content": text}],
    )
    return result.input_tokens


def import_skill(md_path: str) -> str:
    """Import a SKILL.md via CLI, returns JSON IR."""
    result = subprocess.run(
        [str(AIF_CLI), "skill", "import", md_path],
        capture_output=True, text=True, timeout=30,
    )
    if result.returncode != 0:
        print(f"  Warning: import failed: {result.stderr}", file=sys.stderr)
        return ""
    return result.stdout


def format_size(n: int) -> str:
    if n >= 1_000:
        return f"{n/1_000:.1f}K"
    return str(n)


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

    print("=" * 70)
    print("Skill Token Efficiency Benchmark")
    print(f"Model: {MODEL}")
    print("=" * 70)
    print()

    skill_files = sorted(SKILLS_DIR.glob("*.md"))
    if not skill_files:
        print("No skill fixtures found in", SKILLS_DIR)
        sys.exit(1)

    results = []

    for skill_path in skill_files:
        name = skill_path.stem
        print(f"── {name} ", "─" * (50 - len(name)))

        md_text = skill_path.read_text()
        json_ir = import_skill(str(skill_path))
        if not json_ir:
            print("  SKIP: import failed")
            continue

        md_tokens = count_tokens(client, md_text)
        json_tokens = count_tokens(client, json_ir)

        md_bytes = len(md_text.encode("utf-8"))
        json_bytes = len(json_ir.encode("utf-8"))

        savings_json = (1 - json_tokens / md_tokens) * 100 if md_tokens > 0 else 0

        results.append({
            "skill": name,
            "md_tokens": md_tokens,
            "json_tokens": json_tokens,
            "md_bytes": md_bytes,
            "json_bytes": json_bytes,
            "savings_json_pct": savings_json,
        })

        print(f"  SKILL.md:     {format_size(md_tokens):>8} tokens ({format_size(md_bytes):>8} bytes)")
        print(f"  AIF JSON IR:  {format_size(json_tokens):>8} tokens ({format_size(json_bytes):>8} bytes) → {savings_json:+.1f}%")
        print()

        time.sleep(0.5)

    if not results:
        print("No results.")
        sys.exit(1)

    # Summary
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print()
    print(f"{'Skill':<30} {'SKILL.md':>10} {'AIF JSON':>10} {'Savings':>10}")
    print("─" * 62)

    total_md = total_json = 0
    for r in results:
        total_md += r["md_tokens"]
        total_json += r["json_tokens"]
        print(f"{r['skill']:<30} {format_size(r['md_tokens']):>10} {format_size(r['json_tokens']):>10} {r['savings_json_pct']:>+9.1f}%")

    print("─" * 62)
    avg_savings = (1 - total_json / total_md) * 100 if total_md > 0 else 0
    print(f"{'TOTAL':<30} {format_size(total_md):>10} {format_size(total_json):>10} {avg_savings:>+9.1f}%")
    print()

    # Save results
    output_path = PROJECT_ROOT / "benchmarks" / "skill_results.json"
    with open(output_path, "w") as f:
        json.dump({
            "model": MODEL,
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "skills": results,
        }, f, indent=2)
    print(f"Results saved to {output_path}")


if __name__ == "__main__":
    main()
