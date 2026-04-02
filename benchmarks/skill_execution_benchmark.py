#!/usr/bin/env python3
"""
AIF Skill Execution Quality Benchmark

Measures whether LLMs follow skills better when presented in AIF LML format
versus raw Markdown. For each skill + scenario, the LLM is given the skill
in multiple formats and asked to execute it. A judge LLM then scores how
well the response follows the skill's steps, respects constraints, and
meets the output contract.

Metrics:
1. Step coverage     — fraction of @step blocks reflected in the response
2. Constraint respect — fraction of @red_flag / @verify items honored
3. Output contract   — does the response match @output_contract criteria
4. Routing precision — did the model correctly identify when to apply the skill
5. Overall compliance — weighted average

Requires ANTHROPIC_API_KEY environment variable.

Usage:
    ANTHROPIC_API_KEY=sk-... python benchmarks/skill_execution_benchmark.py
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import anthropic

# ── Configuration ─────────────────────────────────────────────────────────

MODEL_EXECUTOR = "claude-sonnet-4-6"  # Model that executes the skill
MODEL_JUDGE = "claude-sonnet-4-6"     # Model that judges compliance
PROJECT_ROOT = Path(__file__).resolve().parent.parent
AIF_CLI = PROJECT_ROOT / "target" / "release" / "aif-cli"

# Formats to test: (key, label, how_to_get)
FORMATS = [
    ("raw_md", "Raw Markdown", "export"),         # aif skill export → SKILL.md
    ("lml_aggressive", "LML Aggressive", "lml-aggressive"),
    ("html", "HTML", "html"),
    ("json_ir", "JSON IR", "json"),
]

# ── Test Scenarios ────────────────────────────────────────────────────────

SCENARIOS = [
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Code Review: SQL Injection Bug",
        "prompt": (
            "Please review this code change:\n\n"
            "```python\n"
            "# auth.py - login endpoint\n"
            "def login(request):\n"
            "    email = request.POST['email']\n"
            "    password = request.POST['password']\n"
            "    query = f\"SELECT * FROM users WHERE email='{email}' AND password='{password}'\"\n"
            "    user = db.execute(query).fetchone()\n"
            "    if user:\n"
            "        return create_session(user)\n"
            "    return HttpResponse('Invalid credentials', status=401)\n"
            "```\n"
        ),
        "expected_steps": [
            "understand context or intent",
            "identify the SQL injection vulnerability",
            "suggest parameterized query as fix",
            "categorize as blocking issue",
        ],
        "expected_constraints": [
            "should NOT approve without flagging the injection",
            "should provide a concrete fix, not just say 'fix it'",
            "should distinguish blocking from suggestion",
        ],
        "output_contract": "structured review with blocking/suggestion/praise categories",
    },
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Code Review: Clean Code (Should Approve)",
        "prompt": (
            "Please review this code change:\n\n"
            "```rust\n"
            "/// Parse a comma-separated list of IDs, skipping invalid entries.\n"
            "pub fn parse_ids(input: &str) -> Vec<u64> {\n"
            "    input\n"
            "        .split(',')\n"
            "        .filter_map(|s| s.trim().parse::<u64>().ok())\n"
            "        .collect()\n"
            "}\n\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    use super::*;\n\n"
            "    #[test]\n"
            "    fn parses_valid_ids() {\n"
            "        assert_eq!(parse_ids(\"1, 2, 3\"), vec![1, 2, 3]);\n"
            "    }\n\n"
            "    #[test]\n"
            "    fn skips_invalid() {\n"
            "        assert_eq!(parse_ids(\"1, abc, 3\"), vec![1, 3]);\n"
            "    }\n"
            "}\n"
            "```\n"
        ),
        "expected_steps": [
            "understand the code's purpose",
            "check correctness",
            "note good patterns (tests, documentation)",
        ],
        "expected_constraints": [
            "should NOT flag non-issues or bikeshed",
            "should approve or approve with minor comments",
            "should mention the good test coverage",
        ],
        "output_contract": "structured review that approves the clean code",
    },
    {
        "skill_file": "examples/skills/security-guidance.aif",
        "name": "Security: Detect eval() in User Input",
        "prompt": (
            "I'm writing a calculator feature. Here's my code:\n\n"
            "```javascript\n"
            "app.post('/calculate', (req, res) => {\n"
            "    const expression = req.body.expression;\n"
            "    const result = eval(expression);\n"
            "    res.json({ result });\n"
            "});\n"
            "```\n"
        ),
        "expected_steps": [
            "identify eval() with user input as a security risk",
            "explain the vulnerability (arbitrary code execution)",
            "suggest a safe alternative (math parser library)",
        ],
        "expected_constraints": [
            "must flag eval() as critical severity",
            "must provide a concrete safe replacement",
        ],
        "output_contract": "security finding with category, severity, and safe replacement",
    },
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


def execute_skill(client: anthropic.Anthropic, skill_text: str, user_prompt: str) -> str:
    """Have the executor model follow the skill on a given scenario."""
    system = (
        "You are an AI assistant. Follow the skill/instructions below precisely. "
        "Apply them to the user's request.\n\n"
        "=== SKILL ===\n" + skill_text + "\n=== END SKILL ==="
    )
    response = client.messages.create(
        model=MODEL_EXECUTOR,
        max_tokens=2048,
        system=system,
        messages=[{"role": "user", "content": user_prompt}],
    )
    return response.content[0].text


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

SKILL (reference):
{skill_text[:3000]}

ASSISTANT'S RESPONSE:
{response_text[:3000]}

EXPECTED STEPS (did the response cover these?):
{json.dumps(expected_steps)}

EXPECTED CONSTRAINTS (were these respected?):
{json.dumps(expected_constraints)}

OUTPUT CONTRACT:
{output_contract}

Score each dimension 0.0-1.0 and respond with ONLY this JSON (no other text):
{{
  "step_coverage": <float 0-1>,
  "step_details": "<which steps were covered/missed>",
  "constraint_respect": <float 0-1>,
  "constraint_details": "<which constraints were respected/violated>",
  "output_contract_met": <float 0-1>,
  "output_contract_details": "<how well the output matches the contract>",
  "overall": <float 0-1>
}}"""

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
            "step_coverage": 0.0,
            "constraint_respect": 0.0,
            "output_contract_met": 0.0,
            "overall": 0.0,
            "parse_error": text[:200],
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
    print("=" * 80)
    print()

    for scenario in SCENARIOS:
        print(f"Scenario: {scenario['name']}")
        print(f"  Skill: {scenario['skill_file']}")
        print()

        scenario_results = []

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
            t0 = time.time()
            response = execute_skill(client, skill_text, scenario["prompt"])
            exec_time = time.time() - t0

            # Judge
            print(f" judging...", end="", flush=True)
            scores = judge_compliance(
                client,
                skill_text,
                response,
                scenario["expected_steps"],
                scenario["expected_constraints"],
                scenario["output_contract"],
            )
            judge_time = time.time() - t0 - exec_time

            result = {
                "scenario": scenario["name"],
                "format": fmt_label,
                "format_key": fmt_key,
                "skill_tokens": skill_tokens,
                "step_coverage": scores.get("step_coverage", 0),
                "constraint_respect": scores.get("constraint_respect", 0),
                "output_contract_met": scores.get("output_contract_met", 0),
                "overall": scores.get("overall", 0),
                "exec_time_s": round(exec_time, 1),
                "details": {
                    "steps": scores.get("step_details", ""),
                    "constraints": scores.get("constraint_details", ""),
                    "output": scores.get("output_contract_details", ""),
                },
            }
            scenario_results.append(result)
            all_results.append(result)

            print(
                f"  steps={result['step_coverage']:.2f}  "
                f"constraints={result['constraint_respect']:.2f}  "
                f"contract={result['output_contract_met']:.2f}  "
                f"overall={result['overall']:.2f}  "
                f"tokens={skill_tokens}  "
                f"time={exec_time:.1f}s"
            )

        # Per-scenario summary
        if scenario_results:
            print()
            print(f"  {'Format':20s} {'Tokens':>7s} {'Steps':>7s} {'Constr':>7s} {'Contract':>9s} {'Overall':>8s}")
            print(f"  {'-'*62}")
            for r in scenario_results:
                print(
                    f"  {r['format']:20s} {r['skill_tokens']:>7d} "
                    f"{r['step_coverage']:>7.2f} {r['constraint_respect']:>7.2f} "
                    f"{r['output_contract_met']:>9.2f} {r['overall']:>8.2f}"
                )
        print()

    # Global summary by format
    print("=" * 80)
    print("Summary by Format (averaged across all scenarios)")
    print("=" * 80)
    print()
    print(f"{'Format':20s} {'Avg Tokens':>11s} {'Avg Steps':>10s} {'Avg Constr':>11s} {'Avg Contract':>13s} {'Avg Overall':>12s}")
    print("-" * 80)

    for fmt_key, fmt_label, _ in FORMATS:
        fmt_results = [r for r in all_results if r["format_key"] == fmt_key]
        if not fmt_results:
            continue
        n = len(fmt_results)
        avg_tok = sum(r["skill_tokens"] for r in fmt_results) / n
        avg_steps = sum(r["step_coverage"] for r in fmt_results) / n
        avg_constr = sum(r["constraint_respect"] for r in fmt_results) / n
        avg_contract = sum(r["output_contract_met"] for r in fmt_results) / n
        avg_overall = sum(r["overall"] for r in fmt_results) / n
        print(
            f"{fmt_label:20s} {avg_tok:>11.0f} {avg_steps:>10.2f} "
            f"{avg_constr:>11.2f} {avg_contract:>13.2f} {avg_overall:>12.2f}"
        )

    # Key finding
    print()
    fmt_overalls = {}
    for fmt_key, fmt_label, _ in FORMATS:
        fmt_results = [r for r in all_results if r["format_key"] == fmt_key]
        if fmt_results:
            fmt_overalls[fmt_label] = sum(r["overall"] for r in fmt_results) / len(fmt_results)

    if fmt_overalls:
        best = max(fmt_overalls, key=fmt_overalls.get)
        worst = min(fmt_overalls, key=fmt_overalls.get)
        delta = fmt_overalls[best] - fmt_overalls[worst]
        print(f"Best format: {best} (avg overall: {fmt_overalls[best]:.2f})")
        print(f"Worst format: {worst} (avg overall: {fmt_overalls[worst]:.2f})")
        print(f"Delta: {delta:.2f} ({delta*100:.1f} percentage points)")

    # Save results
    output_path = Path(__file__).parent / "skill_execution_results.json"
    with open(output_path, "w") as f:
        json.dump({
            "model_executor": MODEL_EXECUTOR,
            "model_judge": MODEL_JUDGE,
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "scenarios": all_results,
            "summary_by_format": {
                label: {
                    "avg_overall": fmt_overalls.get(label, 0),
                    "count": len([r for r in all_results if r["format"] == label]),
                }
                for _, label, _ in FORMATS
            },
        }, f, indent=2)
    print(f"\nResults saved to {output_path}")


if __name__ == "__main__":
    main()
