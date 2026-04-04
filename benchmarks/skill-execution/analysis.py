"""
Analysis module for skill execution benchmark results.

Computes statistical summaries, per-dimension breakdowns, format comparisons,
difficulty segmentation, and generates structured reports.
"""

import json
import math
from pathlib import Path


def load_results(path: str = None) -> dict:
    """Load benchmark results from JSON."""
    if path is None:
        path = str(Path(__file__).parent / "results.json")
    with open(path) as f:
        return json.load(f)


def compute_format_summary(results: dict) -> list[dict]:
    """Compute per-format averages across all scenarios."""
    scenarios = results["scenarios"]
    formats = {}
    for s in scenarios:
        key = s["format_key"]
        if key not in formats:
            formats[key] = {"label": s["format"], "results": []}
        formats[key]["results"].append(s)

    summaries = []
    for key, data in formats.items():
        rs = data["results"]
        n = len(rs)
        summaries.append({
            "format": data["label"],
            "format_key": key,
            "count": n,
            "avg_tokens": sum(r["skill_tokens"] for r in rs) / n,
            "avg_step_coverage": sum(r["step_coverage"] for r in rs) / n,
            "avg_constraint_respect": sum(r["constraint_respect"] for r in rs) / n,
            "avg_output_contract": sum(r["output_contract_met"] for r in rs) / n,
            "avg_overall": sum(r["overall"] for r in rs) / n,
            "min_overall": min(r["overall"] for r in rs),
            "max_overall": max(r["overall"] for r in rs),
            "stddev_overall": stddev([r["overall"] for r in rs]),
        })
    summaries.sort(key=lambda x: -x["avg_overall"])
    return summaries


def compute_difficulty_breakdown(results: dict) -> dict:
    """Break down results by scenario difficulty level."""
    scenarios = results["scenarios"]
    by_difficulty = {}
    for s in scenarios:
        diff = s.get("difficulty", "unknown")
        if diff not in by_difficulty:
            by_difficulty[diff] = {}
        key = s["format_key"]
        if key not in by_difficulty[diff]:
            by_difficulty[diff][key] = {"label": s["format"], "results": []}
        by_difficulty[diff][key]["results"].append(s)

    breakdown = {}
    for diff, formats in by_difficulty.items():
        breakdown[diff] = []
        for key, data in formats.items():
            rs = data["results"]
            n = len(rs)
            breakdown[diff].append({
                "format": data["label"],
                "avg_overall": sum(r["overall"] for r in rs) / n,
                "avg_constraint_respect": sum(r["constraint_respect"] for r in rs) / n,
                "count": n,
            })
        breakdown[diff].sort(key=lambda x: -x["avg_overall"])
    return breakdown


def compute_category_breakdown(results: dict) -> dict:
    """Break down results by skill category."""
    scenarios = results["scenarios"]
    by_cat = {}
    for s in scenarios:
        cat = s.get("category", "unknown")
        if cat not in by_cat:
            by_cat[cat] = {}
        key = s["format_key"]
        if key not in by_cat[cat]:
            by_cat[cat][key] = {"label": s["format"], "results": []}
        by_cat[cat][key]["results"].append(s)

    breakdown = {}
    for cat, formats in by_cat.items():
        breakdown[cat] = []
        for key, data in formats.items():
            rs = data["results"]
            n = len(rs)
            breakdown[cat].append({
                "format": data["label"],
                "avg_overall": sum(r["overall"] for r in rs) / n,
                "count": n,
            })
        breakdown[cat].sort(key=lambda x: -x["avg_overall"])
    return breakdown


def compute_pairwise_wins(results: dict) -> dict:
    """For each scenario, which format scored highest? Count wins."""
    scenarios = results["scenarios"]
    # Group by scenario name
    by_scenario = {}
    for s in scenarios:
        name = s["scenario"]
        if name not in by_scenario:
            by_scenario[name] = []
        by_scenario[name].append(s)

    wins = {}
    ties = 0
    for name, entries in by_scenario.items():
        best = max(entries, key=lambda x: x["overall"])
        best_score = best["overall"]
        winners = [e for e in entries if abs(e["overall"] - best_score) < 0.01]
        if len(winners) > 1:
            ties += 1
        else:
            fmt = best["format"]
            wins[fmt] = wins.get(fmt, 0) + 1

    return {"wins": wins, "ties": ties, "total_scenarios": len(by_scenario)}


def compute_type_breakdown(results: dict) -> dict:
    """Break down results by scenario type (standard, constraint_resistance, etc.)."""
    scenarios = results["scenarios"]
    by_type = {}
    for s in scenarios:
        stype = s.get("scenario_type", "standard")
        if stype not in by_type:
            by_type[stype] = {}
        key = s["format_key"]
        if key not in by_type[stype]:
            by_type[stype][key] = {"label": s["format"], "results": []}
        by_type[stype][key]["results"].append(s)

    breakdown = {}
    for stype, formats in by_type.items():
        breakdown[stype] = []
        for key, data in formats.items():
            rs = data["results"]
            n = len(rs)
            breakdown[stype].append({
                "format": data["label"],
                "avg_overall": sum(r["overall"] for r in rs) / n,
                "avg_step_coverage": sum(r["step_coverage"] for r in rs) / n,
                "avg_constraint_respect": sum(r["constraint_respect"] for r in rs) / n,
                "avg_output_contract": sum(r["output_contract_met"] for r in rs) / n,
                "count": n,
            })
        breakdown[stype].sort(key=lambda x: -x["avg_overall"])
    return breakdown


def compute_token_efficiency(results: dict) -> list[dict]:
    """Compute compliance-per-token ratio for each format."""
    summaries = compute_format_summary(results)
    for s in summaries:
        if s["avg_tokens"] > 0:
            s["compliance_per_1k_tokens"] = round(s["avg_overall"] / (s["avg_tokens"] / 1000), 4)
        else:
            s["compliance_per_1k_tokens"] = 0
    summaries.sort(key=lambda x: -x["compliance_per_1k_tokens"])
    return summaries


def stddev(values: list[float]) -> float:
    if len(values) < 2:
        return 0.0
    mean = sum(values) / len(values)
    variance = sum((v - mean) ** 2 for v in values) / (len(values) - 1)
    return math.sqrt(variance)


def print_full_report(results: dict):
    """Print a comprehensive analysis report to stdout."""
    print("=" * 80)
    print("SKILL EXECUTION QUALITY — FULL ANALYSIS")
    print(f"Executor: {results['model_executor']} | Judge: {results['model_judge']}")
    print(f"Timestamp: {results['timestamp']}")
    print(f"Scenarios: {len(results['scenarios'])} runs across {len(set(s['scenario'] for s in results['scenarios']))} unique scenarios")
    print("=" * 80)

    # 1. Format Summary
    print("\n1. FORMAT SUMMARY (averaged across all scenarios)")
    print("-" * 80)
    summaries = compute_format_summary(results)
    print(f"{'Format':20s} {'Tokens':>7s} {'Steps':>7s} {'Constr':>8s} {'Contract':>9s} {'Overall':>8s} {'StdDev':>7s} {'Min':>5s} {'Max':>5s}")
    for s in summaries:
        print(f"{s['format']:20s} {s['avg_tokens']:>7.0f} {s['avg_step_coverage']:>7.2f} "
              f"{s['avg_constraint_respect']:>8.2f} {s['avg_output_contract']:>9.2f} "
              f"{s['avg_overall']:>8.2f} {s['stddev_overall']:>7.3f} {s['min_overall']:>5.2f} {s['max_overall']:>5.2f}")

    # 2. Token Efficiency
    print("\n2. TOKEN EFFICIENCY (compliance per 1K tokens)")
    print("-" * 80)
    efficiency = compute_token_efficiency(results)
    print(f"{'Format':20s} {'Tokens':>7s} {'Overall':>8s} {'Compliance/1K tok':>18s}")
    for e in efficiency:
        print(f"{e['format']:20s} {e['avg_tokens']:>7.0f} {e['avg_overall']:>8.2f} {e['compliance_per_1k_tokens']:>18.4f}")

    # 3. Difficulty Breakdown
    print("\n3. DIFFICULTY BREAKDOWN")
    print("-" * 80)
    by_diff = compute_difficulty_breakdown(results)
    for diff, formats in sorted(by_diff.items()):
        print(f"\n  [{diff.upper()}]")
        for f in formats:
            print(f"    {f['format']:20s}  overall={f['avg_overall']:.2f}  constraints={f['avg_constraint_respect']:.2f}  (n={f['count']})")

    # 4. Category Breakdown
    print("\n4. CATEGORY BREAKDOWN")
    print("-" * 80)
    by_cat = compute_category_breakdown(results)
    for cat, formats in sorted(by_cat.items()):
        print(f"\n  [{cat}]")
        for f in formats:
            print(f"    {f['format']:20s}  overall={f['avg_overall']:.2f}  (n={f['count']})")

    # 5. Scenario Type Breakdown
    print("\n5. SCENARIO TYPE BREAKDOWN")
    print("-" * 80)
    by_type = compute_type_breakdown(results)
    type_labels = {
        "standard": "Standard (find the bug / apply the skill)",
        "constraint_resistance": "Constraint Resistance (user pressures to skip steps)",
        "multi_step": "Multi-Step (ordered workflow compliance)",
        "conflicting_instructions": "Conflicting Instructions (user contradicts skill)",
        "edge_case": "Edge Cases (unusual inputs, safe code, out-of-scope)",
    }
    for stype, formats in sorted(by_type.items()):
        label = type_labels.get(stype, stype)
        print(f"\n  [{label}]")
        for f in formats:
            print(f"    {f['format']:20s}  overall={f['avg_overall']:.2f}  "
                  f"steps={f['avg_step_coverage']:.2f}  "
                  f"constr={f['avg_constraint_respect']:.2f}  "
                  f"contract={f['avg_output_contract']:.2f}  "
                  f"(n={f['count']})")

    # 6. Pairwise Wins
    print("\n6. PAIRWISE WINS (which format scored highest per scenario)")
    print("-" * 80)
    pw = compute_pairwise_wins(results)
    for fmt, count in sorted(pw["wins"].items(), key=lambda x: -x[1]):
        print(f"  {fmt:20s}  {count} wins")
    print(f"  {'(ties)':20s}  {pw['ties']}")
    print(f"  Total scenarios: {pw['total_scenarios']}")

    # 7. Key Finding
    print("\n7. KEY FINDINGS")
    print("-" * 80)
    best = summaries[0]
    worst = summaries[-1]
    delta = best["avg_overall"] - worst["avg_overall"]
    print(f"  Best:  {best['format']} (overall: {best['avg_overall']:.2f}, {best['avg_tokens']:.0f} tokens)")
    print(f"  Worst: {worst['format']} (overall: {worst['avg_overall']:.2f}, {worst['avg_tokens']:.0f} tokens)")
    print(f"  Delta: {delta:.2f} ({delta*100:.1f} percentage points)")

    # Where does the gap come from?
    if len(summaries) >= 2:
        lml = next((s for s in summaries if "lml" in s["format_key"].lower()), None)
        md = next((s for s in summaries if "md" in s["format_key"].lower()), None)
        if lml and md:
            print(f"\n  LML vs Markdown breakdown:")
            print(f"    Step coverage:      {lml['avg_step_coverage']:.2f} vs {md['avg_step_coverage']:.2f} (delta: {lml['avg_step_coverage']-md['avg_step_coverage']:+.2f})")
            print(f"    Constraint respect: {lml['avg_constraint_respect']:.2f} vs {md['avg_constraint_respect']:.2f} (delta: {lml['avg_constraint_respect']-md['avg_constraint_respect']:+.2f})")
            print(f"    Output contract:    {lml['avg_output_contract']:.2f} vs {md['avg_output_contract']:.2f} (delta: {lml['avg_output_contract']-md['avg_output_contract']:+.2f})")
            print(f"    Tokens:             {lml['avg_tokens']:.0f} vs {md['avg_tokens']:.0f} ({(1-lml['avg_tokens']/md['avg_tokens'])*100:+.1f}%)")


if __name__ == "__main__":
    results = load_results()
    print_full_report(results)
