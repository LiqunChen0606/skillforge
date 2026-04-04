#!/usr/bin/env python3
"""
Multi-run variance analysis for skill execution benchmark.

Runs each scenario×format N times to compute confidence intervals.
Determines which format differences are statistically significant.

Usage:
    ANTHROPIC_API_KEY=sk-... python benchmarks/skill-execution/multi_run.py --runs 5
    python benchmarks/skill-execution/multi_run.py --analyze  # analyze saved results (no API)
"""

import argparse
import json
import math
import os
import sys
import time
from pathlib import Path

# Import from sibling modules
BENCH_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(BENCH_DIR))
from scenarios import SCENARIOS
from benchmark import compile_skill, execute_skill, judge_compliance, FORMATS, MODEL_EXECUTOR, MODEL_JUDGE

PROJECT_ROOT = BENCH_DIR.parent.parent


def compute_stats(values):
    """Compute mean, stddev, min, max, 95% CI."""
    n = len(values)
    if n == 0:
        return {"mean": 0, "stddev": 0, "min": 0, "max": 0, "ci95_low": 0, "ci95_high": 0, "n": 0}
    mean = sum(values) / n
    if n < 2:
        return {"mean": mean, "stddev": 0, "min": mean, "max": mean, "ci95_low": mean, "ci95_high": mean, "n": n}
    variance = sum((v - mean) ** 2 for v in values) / (n - 1)
    stddev = math.sqrt(variance)
    # 95% CI: mean +/- t * (stddev / sqrt(n)), using t-distribution critical values
    t_val = {3: 4.303, 4: 3.182, 5: 2.776, 6: 2.571, 7: 2.447, 8: 2.365, 9: 2.306, 10: 2.262}.get(n, 1.96)
    margin = t_val * (stddev / math.sqrt(n))
    return {
        "mean": round(mean, 4),
        "stddev": round(stddev, 4),
        "min": round(min(values), 4),
        "max": round(max(values), 4),
        "ci95_low": round(mean - margin, 4),
        "ci95_high": round(mean + margin, 4),
        "n": n,
    }


def is_significant(stats_a, stats_b):
    """Check if two distributions have non-overlapping 95% CIs."""
    return stats_a["ci95_low"] > stats_b["ci95_high"] or stats_b["ci95_low"] > stats_a["ci95_high"]


def run_multi(num_runs, api_key):
    """Run the full benchmark num_runs times."""
    import anthropic
    client = anthropic.Anthropic(api_key=api_key)

    all_runs = []  # List of {scenario, format, run_id, overall, step_coverage, ...}

    total = len(SCENARIOS) * len(FORMATS) * num_runs
    completed = 0

    print(f"Multi-run benchmark: {len(SCENARIOS)} scenarios x {len(FORMATS)} formats x {num_runs} runs = {total} LLM calls")
    print(f"Executor: {MODEL_EXECUTOR} | Judge: {MODEL_JUDGE}")
    print("=" * 80)

    for scenario in SCENARIOS:
        for fmt_key, fmt_label, fmt_arg in FORMATS:
            skill_text = compile_skill(str(PROJECT_ROOT / scenario["skill_file"]), fmt_arg)
            if "[compilation failed" in skill_text or "[read failed" in skill_text:
                completed += num_runs
                print(f"  SKIP {scenario['name'][:30]:30s} | {fmt_label:15s} (compilation failed)")
                continue

            for run_id in range(num_runs):
                completed += 1
                print(f"  [{completed}/{total}] {scenario['name'][:30]:30s} | {fmt_label:15s} | run {run_id+1}/{num_runs}", end="", flush=True)

                response_text, exec_time = execute_skill(client, skill_text, scenario["prompt"])
                scores = judge_compliance(
                    client, skill_text, response_text,
                    scenario["expected_steps"],
                    scenario["expected_constraints"],
                    scenario["output_contract"],
                )

                result = {
                    "scenario": scenario["name"],
                    "format": fmt_label,
                    "format_key": fmt_key,
                    "run_id": run_id,
                    "overall": scores.get("overall", 0),
                    "step_coverage": scores.get("step_coverage", 0),
                    "constraint_respect": scores.get("constraint_respect", 0),
                    "output_contract_met": scores.get("output_contract_met", 0),
                    "exec_time_s": round(exec_time, 1),
                }
                all_runs.append(result)
                print(f"  overall={result['overall']:.2f}")

                time.sleep(0.5)  # Rate limit courtesy

    return all_runs


def analyze_multi(all_runs):
    """Compute statistics and significance tests."""
    # Group by format
    by_format = {}
    for r in all_runs:
        key = r["format_key"]
        if key not in by_format:
            by_format[key] = {"label": r["format"], "overalls": [], "steps": [], "constraints": [], "contracts": []}
        by_format[key]["overalls"].append(r["overall"])
        by_format[key]["steps"].append(r["step_coverage"])
        by_format[key]["constraints"].append(r["constraint_respect"])
        by_format[key]["contracts"].append(r["output_contract_met"])

    print("\n" + "=" * 80)
    print("MULTI-RUN VARIANCE ANALYSIS")
    print("=" * 80)

    # Per-format overall stats
    print(f"\n{'Format':20s} {'Mean':>6s} {'StdDev':>7s} {'Min':>5s} {'Max':>5s} {'95% CI':>15s} {'N':>4s}")
    print("-" * 65)
    format_stats = {}
    for key, data in sorted(by_format.items(), key=lambda x: -sum(x[1]["overalls"]) / len(x[1]["overalls"])):
        stats = compute_stats(data["overalls"])
        format_stats[key] = {"overall": stats, "label": data["label"]}
        print(f"{data['label']:20s} {stats['mean']:>6.3f} {stats['stddev']:>7.3f} {stats['min']:>5.2f} {stats['max']:>5.2f} [{stats['ci95_low']:.3f}, {stats['ci95_high']:.3f}] {stats['n']:>4d}")

    # Per-metric breakdown
    for metric, metric_key in [("Step Coverage", "steps"), ("Constraint Respect", "constraints"), ("Output Contract", "contracts")]:
        print(f"\n{metric}:")
        print(f"  {'Format':20s} {'Mean':>6s} {'StdDev':>7s} {'95% CI':>15s}")
        print("  " + "-" * 50)
        for key, data in sorted(by_format.items(), key=lambda x: -sum(x[1][metric_key]) / len(x[1][metric_key])):
            stats = compute_stats(data[metric_key])
            format_stats[key][metric_key] = stats
            print(f"  {data['label']:20s} {stats['mean']:>6.3f} {stats['stddev']:>7.3f} [{stats['ci95_low']:.3f}, {stats['ci95_high']:.3f}]")

    # Pairwise significance tests on overall score
    print(f"\nPairwise Significance (non-overlapping 95% CIs on overall):")
    keys = sorted(format_stats.keys(), key=lambda k: -format_stats[k]["overall"]["mean"])
    for i in range(len(keys)):
        for j in range(i + 1, len(keys)):
            a_key, b_key = keys[i], keys[j]
            a_label = format_stats[a_key]["label"]
            b_label = format_stats[b_key]["label"]
            sig = is_significant(format_stats[a_key]["overall"], format_stats[b_key]["overall"])
            delta = format_stats[a_key]["overall"]["mean"] - format_stats[b_key]["overall"]["mean"]
            marker = "*** SIGNIFICANT" if sig else "n.s."
            print(f"  {a_label} vs {b_label}: delta={delta:+.3f} {marker}")

    # Per-scenario variance (helps identify high-variance scenarios)
    by_scenario = {}
    for r in all_runs:
        key = r["scenario"]
        if key not in by_scenario:
            by_scenario[key] = {}
        fmt = r["format_key"]
        if fmt not in by_scenario[key]:
            by_scenario[key][fmt] = []
        by_scenario[key][fmt].append(r["overall"])

    print(f"\nPer-Scenario Variance (stddev of overall across runs):")
    print(f"  {'Scenario':40s} {'Format':20s} {'Mean':>6s} {'StdDev':>7s} {'Range':>11s}")
    print("  " + "-" * 85)
    high_variance = []
    for scenario_name, fmt_data in sorted(by_scenario.items()):
        for fmt_key, values in sorted(fmt_data.items()):
            stats = compute_stats(values)
            if stats["stddev"] > 0.1:
                high_variance.append((scenario_name, format_stats.get(fmt_key, {}).get("label", fmt_key), stats))
            print(f"  {scenario_name[:40]:40s} {format_stats.get(fmt_key, {}).get('label', fmt_key):20s} {stats['mean']:>6.3f} {stats['stddev']:>7.3f} [{stats['min']:.2f}-{stats['max']:.2f}]")

    if high_variance:
        print(f"\nHigh-variance combinations (stddev > 0.10):")
        for name, fmt, stats in high_variance:
            print(f"  {name[:40]:40s} {fmt:20s} stddev={stats['stddev']:.3f}")

    return format_stats


def main():
    parser = argparse.ArgumentParser(description="Multi-run variance analysis for skill execution benchmark")
    parser.add_argument("--runs", type=int, default=5, help="Number of runs per scenario x format (default: 5)")
    parser.add_argument("--analyze", action="store_true", help="Analyze saved results (no API calls)")
    args = parser.parse_args()

    output_path = BENCH_DIR / "multi_run_results.json"

    if args.analyze:
        if not output_path.exists():
            print(f"Error: {output_path} not found. Run without --analyze first.", file=sys.stderr)
            sys.exit(1)
        with open(output_path) as f:
            data = json.load(f)
        print(f"Loaded {len(data['runs'])} runs from {output_path}")
        print(f"  Executor: {data.get('model_executor', 'unknown')} | Judge: {data.get('model_judge', 'unknown')}")
        print(f"  Runs per combo: {data.get('runs_per_combo', '?')} | Timestamp: {data.get('timestamp', '?')}")
        analyze_multi(data["runs"])
        return

    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("Error: ANTHROPIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    all_runs = run_multi(args.runs, api_key)

    output = {
        "model_executor": MODEL_EXECUTOR,
        "model_judge": MODEL_JUDGE,
        "runs_per_combo": args.runs,
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "total_runs": len(all_runs),
        "runs": all_runs,
    }
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nResults saved to {output_path}")

    analyze_multi(all_runs)


if __name__ == "__main__":
    main()
