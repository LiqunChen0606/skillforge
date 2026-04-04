"""
Analysis module for the adversarial constraint resistance benchmark.

Computes:
- Per-format resistance scores
- Per-adversarial-category breakdown (skip_pressure / constraint_override / social_engineering)
- Per-skill-category resistance (which skill's constraints are hardest to maintain)
- Delta vs non-adversarial baseline (loads standard benchmark results if available)
- Pairwise format wins on resistance_score
"""

import json
import math
from pathlib import Path


def load_results(path: str = None) -> dict:
    """Load adversarial benchmark results from JSON."""
    if path is None:
        path = str(Path(__file__).parent / "results.json")
    with open(path) as f:
        return json.load(f)


def load_baseline_results() -> dict | None:
    """Load standard benchmark results for delta comparison, if available."""
    baseline_path = Path(__file__).parent.parent / "skill-execution" / "results.json"
    if baseline_path.exists():
        with open(baseline_path) as f:
            return json.load(f)
    return None


def stddev(values: list[float]) -> float:
    if len(values) < 2:
        return 0.0
    mean = sum(values) / len(values)
    variance = sum((v - mean) ** 2 for v in values) / (len(values) - 1)
    return math.sqrt(variance)


def avg(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


# ── Per-Format Summary ────────────────────────────────────────────────────


def compute_format_summary(results: dict) -> list[dict]:
    """Compute per-format averages across all adversarial scenarios."""
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
        resistance_scores = [r["resistance_score"] for r in rs]
        summaries.append({
            "format": data["label"],
            "format_key": key,
            "count": n,
            "avg_tokens": avg([r["skill_tokens"] for r in rs]),
            "avg_resistance": avg(resistance_scores),
            "min_resistance": min(resistance_scores),
            "max_resistance": max(resistance_scores),
            "stddev_resistance": stddev(resistance_scores),
            "avg_step_coverage": avg([r["step_coverage"] for r in rs]),
            "avg_constraint_respect": avg([r["constraint_respect"] for r in rs]),
            "avg_output_contract": avg([r["output_contract_met"] for r in rs]),
            "avg_overall": avg([r["overall"] for r in rs]),
        })
    summaries.sort(key=lambda x: -x["avg_resistance"])
    return summaries


# ── Per-Adversarial-Category Breakdown ────────────────────────────────────


def compute_adversarial_category_breakdown(results: dict) -> dict:
    """Break down resistance scores by adversarial category."""
    scenarios = results["scenarios"]
    by_cat = {}
    for s in scenarios:
        cat = s["adversarial_category"]
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
            breakdown[cat].append({
                "format": data["label"],
                "avg_resistance": avg([r["resistance_score"] for r in rs]),
                "avg_constraint_respect": avg([r["constraint_respect"] for r in rs]),
                "avg_overall": avg([r["overall"] for r in rs]),
                "count": len(rs),
            })
        breakdown[cat].sort(key=lambda x: -x["avg_resistance"])
    return breakdown


# ── Per-Skill-Category Breakdown ──────────────────────────────────────────


def compute_skill_category_breakdown(results: dict) -> dict:
    """Break down resistance scores by skill category (code-review, security, etc.)."""
    scenarios = results["scenarios"]
    by_skill = {}
    for s in scenarios:
        cat = s["category"]
        if cat not in by_skill:
            by_skill[cat] = {}
        key = s["format_key"]
        if key not in by_skill[cat]:
            by_skill[cat][key] = {"label": s["format"], "results": []}
        by_skill[cat][key]["results"].append(s)

    breakdown = {}
    for cat, formats in by_skill.items():
        breakdown[cat] = []
        for key, data in formats.items():
            rs = data["results"]
            breakdown[cat].append({
                "format": data["label"],
                "avg_resistance": avg([r["resistance_score"] for r in rs]),
                "avg_overall": avg([r["overall"] for r in rs]),
                "count": len(rs),
            })
        breakdown[cat].sort(key=lambda x: -x["avg_resistance"])
    return breakdown


# ── Pairwise Wins ─────────────────────────────────────────────────────────


def compute_resistance_wins(results: dict) -> dict:
    """For each scenario, which format had the highest resistance_score?"""
    scenarios = results["scenarios"]
    by_scenario = {}
    for s in scenarios:
        name = s["scenario"]
        if name not in by_scenario:
            by_scenario[name] = []
        by_scenario[name].append(s)

    wins = {}
    ties = 0
    for name, entries in by_scenario.items():
        best = max(entries, key=lambda x: x["resistance_score"])
        best_score = best["resistance_score"]
        winners = [e for e in entries if abs(e["resistance_score"] - best_score) < 0.01]
        if len(winners) > 1:
            ties += 1
        else:
            fmt = best["format"]
            wins[fmt] = wins.get(fmt, 0) + 1

    return {"wins": wins, "ties": ties, "total_scenarios": len(by_scenario)}


# ── Delta vs Non-Adversarial Baseline ─────────────────────────────────────


def compute_baseline_delta(adversarial_results: dict, baseline_results: dict) -> list[dict]:
    """
    Compare adversarial constraint_respect to baseline constraint_respect.

    Only compares the constraint_resistance scenario type from the baseline,
    since that is the closest apples-to-apples comparison.
    """
    # Get baseline constraint_resistance results by format
    baseline_by_format = {}
    for s in baseline_results["scenarios"]:
        if s.get("scenario_type") == "constraint_resistance":
            key = s["format_key"]
            if key not in baseline_by_format:
                baseline_by_format[key] = []
            baseline_by_format[key].append(s)

    # Get adversarial results by format
    adv_by_format = {}
    for s in adversarial_results["scenarios"]:
        key = s["format_key"]
        if key not in adv_by_format:
            adv_by_format[key] = []
        adv_by_format[key].append(s)

    deltas = []
    for fmt_key in adv_by_format:
        adv_rs = adv_by_format[fmt_key]
        adv_avg = avg([r["constraint_respect"] for r in adv_rs])
        label = adv_rs[0]["format"]

        baseline_avg = None
        if fmt_key in baseline_by_format:
            baseline_rs = baseline_by_format[fmt_key]
            baseline_avg = avg([r["constraint_respect"] for r in baseline_rs])

        deltas.append({
            "format": label,
            "adversarial_constraint": round(adv_avg, 3),
            "baseline_constraint": round(baseline_avg, 3) if baseline_avg is not None else None,
            "delta": round(adv_avg - baseline_avg, 3) if baseline_avg is not None else None,
        })
    deltas.sort(key=lambda x: -(x["delta"] or -999))
    return deltas


# ── Hardest Scenarios ─────────────────────────────────────────────────────


def compute_hardest_scenarios(results: dict, bottom_n: int = 5) -> list[dict]:
    """Find scenarios where resistance was lowest (averaged across formats)."""
    scenarios = results["scenarios"]
    by_scenario = {}
    for s in scenarios:
        name = s["scenario"]
        if name not in by_scenario:
            by_scenario[name] = {"adversarial_category": s["adversarial_category"], "results": []}
        by_scenario[name]["results"].append(s)

    ranked = []
    for name, data in by_scenario.items():
        rs = data["results"]
        ranked.append({
            "scenario": name,
            "adversarial_category": data["adversarial_category"],
            "avg_resistance": avg([r["resistance_score"] for r in rs]),
            "min_resistance": min(r["resistance_score"] for r in rs),
            "worst_format": min(rs, key=lambda r: r["resistance_score"])["format"],
        })
    ranked.sort(key=lambda x: x["avg_resistance"])
    return ranked[:bottom_n]


# ── Report ────────────────────────────────────────────────────────────────


def print_full_report(results: dict):
    """Print a comprehensive adversarial analysis report to stdout."""
    print("=" * 80)
    print("ADVERSARIAL CONSTRAINT RESISTANCE — FULL ANALYSIS")
    print(f"Executor: {results['model_executor']} | Judge: {results['model_judge']}")
    print(f"Timestamp: {results['timestamp']}")
    n_unique = len(set(s["scenario"] for s in results["scenarios"]))
    print(f"Scenarios: {results['total_runs']} runs across {n_unique} unique adversarial scenarios")
    print("=" * 80)

    # 1. Format Summary — Resistance Focus
    print("\n1. FORMAT SUMMARY — RESISTANCE SCORES")
    print("-" * 80)
    summaries = compute_format_summary(results)
    print(f"{'Format':20s} {'Resist':>7s} {'StdDev':>7s} {'Min':>5s} {'Max':>5s} "
          f"{'Steps':>7s} {'Constr':>8s} {'Overall':>8s} {'Tokens':>7s}")
    for s in summaries:
        print(f"{s['format']:20s} {s['avg_resistance']:>7.2f} {s['stddev_resistance']:>7.3f} "
              f"{s['min_resistance']:>5.2f} {s['max_resistance']:>5.2f} "
              f"{s['avg_step_coverage']:>7.2f} {s['avg_constraint_respect']:>8.2f} "
              f"{s['avg_overall']:>8.2f} {s['avg_tokens']:>7.0f}")

    # 2. Adversarial Category Breakdown
    print("\n2. ADVERSARIAL CATEGORY BREAKDOWN")
    print("-" * 80)
    cat_labels = {
        "skip_pressure": "Skip Pressure (user rushes/skips steps)",
        "constraint_override": "Constraint Override (user contradicts @red_flag)",
        "social_engineering": "Social Engineering (authority/urgency manipulation)",
    }
    by_adv_cat = compute_adversarial_category_breakdown(results)
    for cat in ["skip_pressure", "constraint_override", "social_engineering"]:
        if cat not in by_adv_cat:
            continue
        label = cat_labels.get(cat, cat)
        print(f"\n  [{label}]")
        for f in by_adv_cat[cat]:
            print(f"    {f['format']:20s}  resist={f['avg_resistance']:.2f}  "
                  f"constr={f['avg_constraint_respect']:.2f}  "
                  f"overall={f['avg_overall']:.2f}  (n={f['count']})")

    # 3. Skill Category Breakdown
    print("\n3. SKILL CATEGORY BREAKDOWN (which skill constraints are hardest to maintain?)")
    print("-" * 80)
    by_skill = compute_skill_category_breakdown(results)
    for cat, formats in sorted(by_skill.items()):
        print(f"\n  [{cat}]")
        for f in formats:
            print(f"    {f['format']:20s}  resist={f['avg_resistance']:.2f}  "
                  f"overall={f['avg_overall']:.2f}  (n={f['count']})")

    # 4. Resistance Wins
    print("\n4. RESISTANCE WINS (which format scored highest resistance per scenario)")
    print("-" * 80)
    pw = compute_resistance_wins(results)
    for fmt, count in sorted(pw["wins"].items(), key=lambda x: -x[1]):
        print(f"  {fmt:20s}  {count} wins")
    print(f"  {'(ties)':20s}  {pw['ties']}")
    print(f"  Total scenarios: {pw['total_scenarios']}")

    # 5. Hardest Scenarios
    print("\n5. HARDEST SCENARIOS (lowest avg resistance across formats)")
    print("-" * 80)
    hardest = compute_hardest_scenarios(results)
    for h in hardest:
        print(f"  resist={h['avg_resistance']:.2f}  [{h['adversarial_category']}]  "
              f"{h['scenario']}  (worst: {h['worst_format']} at {h['min_resistance']:.2f})")

    # 6. Baseline Delta
    print("\n6. DELTA vs NON-ADVERSARIAL BASELINE")
    print("-" * 80)
    baseline = load_baseline_results()
    if baseline is not None:
        deltas = compute_baseline_delta(results, baseline)
        print(f"{'Format':20s} {'Adv Constr':>12s} {'Base Constr':>12s} {'Delta':>8s}")
        for d in deltas:
            base_str = f"{d['baseline_constraint']:.3f}" if d["baseline_constraint"] is not None else "N/A"
            delta_str = f"{d['delta']:+.3f}" if d["delta"] is not None else "N/A"
            print(f"  {d['format']:20s} {d['adversarial_constraint']:>12.3f} {base_str:>12s} {delta_str:>8s}")
    else:
        print("  (No baseline results found at benchmarks/skill-execution/results.json)")
        print("  Run the standard benchmark first to enable delta comparison.")

    # 7. Key Findings
    print("\n7. KEY FINDINGS")
    print("-" * 80)
    if summaries:
        best = summaries[0]
        worst = summaries[-1]
        delta = best["avg_resistance"] - worst["avg_resistance"]
        print(f"  Best resistance:  {best['format']} (avg: {best['avg_resistance']:.2f})")
        print(f"  Worst resistance: {worst['format']} (avg: {worst['avg_resistance']:.2f})")
        print(f"  Gap: {delta:.2f} ({delta*100:.1f} percentage points)")

        # Category difficulty ranking
        cat_avgs = {}
        for cat, formats in by_adv_cat.items():
            cat_avgs[cat] = avg([f["avg_resistance"] for f in formats])
        if cat_avgs:
            hardest_cat = min(cat_avgs, key=cat_avgs.get)
            easiest_cat = max(cat_avgs, key=cat_avgs.get)
            print(f"\n  Hardest adversarial category: {hardest_cat} (avg resist: {cat_avgs[hardest_cat]:.2f})")
            print(f"  Easiest adversarial category: {easiest_cat} (avg resist: {cat_avgs[easiest_cat]:.2f})")

        # LML vs Markdown comparison
        lml = next((s for s in summaries if "lml" in s["format_key"].lower()), None)
        md = next((s for s in summaries if "md" in s["format_key"].lower()), None)
        if lml and md:
            r_delta = lml["avg_resistance"] - md["avg_resistance"]
            print(f"\n  LML vs Markdown resistance delta: {r_delta:+.2f} ({r_delta*100:+.1f}pp)")
            print(f"    LML Aggressive: resist={lml['avg_resistance']:.2f}, constr={lml['avg_constraint_respect']:.2f}")
            print(f"    Raw Markdown:   resist={md['avg_resistance']:.2f}, constr={md['avg_constraint_respect']:.2f}")


if __name__ == "__main__":
    results = load_results()
    print_full_report(results)
