#!/usr/bin/env python3
"""
AIF Skill Execution Quality Benchmark

Measures whether LLMs follow skills better when presented in AIF LML format
versus raw Markdown. For each skill + scenario, the LLM is given the skill
in multiple formats and asked to execute it. A judge LLM then scores how
well the response follows the skill's steps, respects constraints, and
meets the output contract.

Metrics:
1. Step coverage      — fraction of @step blocks reflected in the response
2. Constraint respect — fraction of @red_flag / @verify items honored
3. Output contract    — does the response match @output_contract criteria
4. Overall compliance — weighted average

Results are saved with full executor responses for post-hoc analysis.

Requires ANTHROPIC_API_KEY environment variable.

Usage:
    ANTHROPIC_API_KEY=sk-... python benchmarks/skill-execution/benchmark.py
    python benchmarks/skill-execution/analysis.py   # analyze saved results
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import anthropic

# Allow running from project root or benchmark dir
BENCH_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = BENCH_DIR.parent.parent
sys.path.insert(0, str(BENCH_DIR))
from scenarios import SCENARIOS

# ── Configuration ─────────────────────────────────────────────────────────

MODEL_EXECUTOR = "claude-sonnet-4-6"
MODEL_JUDGE = "claude-sonnet-4-6"
AIF_CLI = PROJECT_ROOT / "target" / "release" / "aif-cli"

FORMATS = [
    ("raw_md", "Raw Markdown", "export"),
    ("lml_aggressive", "LML Aggressive", "lml-aggressive"),
    ("html", "HTML", "html"),
    ("json_ir", "JSON IR", "json"),
]

# ── Helpers ───────────────────────────────────────────────────────────────


def compile_skill(skill_path: str, fmt: str) -> str:
    """Compile an AIF skill to a target format."""
    if fmt == "export":
        result = subprocess.run(
            [str(AIF_CLI), "skill", "export", skill_path],
            capture_output=True, text=True, timeout=15,
        )
    else:
        result = subprocess.run(
            [str(AIF_CLI), "compile", skill_path, "--format", fmt],
            capture_output=True, text=True, timeout=15,
        )
    if result.returncode != 0:
        return f"[compilation failed: {result.stderr.strip()[:200]}]"
    return result.stdout


def execute_skill(client: anthropic.Anthropic, skill_text: str, user_prompt: str) -> tuple[str, float]:
    """Have the executor model follow the skill. Returns (response_text, latency_s)."""
    system = (
        "You are an AI assistant. Follow the skill/instructions below precisely. "
        "Apply them to the user's request.\n\n"
        "=== SKILL ===\n" + skill_text + "\n=== END SKILL ==="
    )
    t0 = time.time()
    response = client.messages.create(
        model=MODEL_EXECUTOR,
        max_tokens=2048,
        system=system,
        messages=[{"role": "user", "content": user_prompt}],
    )
    latency = time.time() - t0
    return response.content[0].text, latency


def judge_compliance(
    client: anthropic.Anthropic,
    skill_text: str,
    response_text: str,
    expected_steps: list[str],
    expected_constraints: list[str],
    output_contract: str,
) -> dict:
    """Have the judge model score the executor's compliance."""
    prompt = f"""You are evaluating whether an AI assistant correctly followed a skill/instruction set.

SKILL (reference — first 3000 chars):
{skill_text[:3000]}

ASSISTANT'S RESPONSE (first 3000 chars):
{response_text[:3000]}

EXPECTED STEPS (did the response cover these?):
{json.dumps(expected_steps)}

EXPECTED CONSTRAINTS (were these respected?):
{json.dumps(expected_constraints)}

OUTPUT CONTRACT:
{output_contract}

Score each dimension from 0.0 to 1.0. Be precise — 0.5 means half the items were covered.

Respond with ONLY this JSON (no other text, no markdown fences):
{{"step_coverage": <float>, "step_details": "<which steps covered/missed>", "constraint_respect": <float>, "constraint_details": "<which constraints respected/violated>", "output_contract_met": <float>, "output_contract_details": "<how well output matches contract>", "overall": <float>}}"""

    response = client.messages.create(
        model=MODEL_JUDGE,
        max_tokens=1024,
        messages=[{"role": "user", "content": prompt}],
    )
    text = response.content[0].text.strip()

    # Extract JSON from response (handle markdown code blocks)
    if "```" in text:
        text = text.split("```")[1]
        if text.startswith("json"):
            text = text[4:]
        text = text.strip()

    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return {
            "step_coverage": 0.0, "constraint_respect": 0.0,
            "output_contract_met": 0.0, "overall": 0.0,
            "parse_error": text[:300],
        }


# ── Main ──────────────────────────────────────────────────────────────────


def main():
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("Error: ANTHROPIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    client = anthropic.Anthropic(api_key=api_key)
    all_results = []

    print("=" * 80)
    print("AIF Skill Execution Quality Benchmark")
    print(f"Executor: {MODEL_EXECUTOR} | Judge: {MODEL_JUDGE}")
    print(f"Scenarios: {len(SCENARIOS)} | Formats: {len(FORMATS)}")
    print("=" * 80)

    for scenario in SCENARIOS:
        print(f"\nScenario: {scenario['name']}")
        print(f"  Category: {scenario['category']} | Difficulty: {scenario['difficulty']}")
        print(f"  {scenario['description']}")

        for fmt_key, fmt_label, fmt_arg in FORMATS:
            skill_text = compile_skill(
                str(PROJECT_ROOT / scenario["skill_file"]), fmt_arg
            )
            if "[compilation failed" in skill_text:
                print(f"  {fmt_label:20s}  SKIP (compilation failed)")
                continue

            skill_tokens = client.messages.count_tokens(
                model=MODEL_EXECUTOR,
                messages=[{"role": "user", "content": skill_text}],
            ).input_tokens

            # Execute
            print(f"  {fmt_label:20s}  executing...", end="", flush=True)
            response_text, exec_time = execute_skill(client, skill_text, scenario["prompt"])

            # Judge
            print(f" judging...", end="", flush=True)
            t0 = time.time()
            scores = judge_compliance(
                client, skill_text, response_text,
                scenario["expected_steps"],
                scenario["expected_constraints"],
                scenario["output_contract"],
            )
            judge_time = time.time() - t0

            result = {
                "scenario": scenario["name"],
                "category": scenario["category"],
                "difficulty": scenario["difficulty"],
                "format": fmt_label,
                "format_key": fmt_key,
                "skill_tokens": skill_tokens,
                "step_coverage": scores.get("step_coverage", 0),
                "constraint_respect": scores.get("constraint_respect", 0),
                "output_contract_met": scores.get("output_contract_met", 0),
                "overall": scores.get("overall", 0),
                "exec_time_s": round(exec_time, 1),
                "judge_time_s": round(judge_time, 1),
                "details": {
                    "steps": scores.get("step_details", ""),
                    "constraints": scores.get("constraint_details", ""),
                    "output": scores.get("output_contract_details", ""),
                },
                "executor_response_preview": response_text[:500],
            }
            all_results.append(result)

            print(
                f"  steps={result['step_coverage']:.2f}  "
                f"constr={result['constraint_respect']:.2f}  "
                f"contract={result['output_contract_met']:.2f}  "
                f"overall={result['overall']:.2f}  "
                f"tokens={skill_tokens}  "
                f"time={exec_time:.1f}s"
            )

    # Save results
    output = {
        "model_executor": MODEL_EXECUTOR,
        "model_judge": MODEL_JUDGE,
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "scenario_count": len(SCENARIOS),
        "format_count": len(FORMATS),
        "total_runs": len(all_results),
        "scenarios": all_results,
    }

    output_path = BENCH_DIR / "results.json"
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nResults saved to {output_path}")

    # Run analysis
    print()
    from analysis import load_results, print_full_report
    print_full_report(output)


if __name__ == "__main__":
    main()
