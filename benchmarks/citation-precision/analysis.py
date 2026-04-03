"""
Analysis and HTML report generation for citation precision benchmark.
"""

import json
from pathlib import Path

BENCH_DIR = Path(__file__).resolve().parent


def compute_statistics(results: list[dict]) -> dict:
    """Compute aggregate statistics from results."""
    if not results:
        return {}

    by_strategy = {}
    by_budget = {}
    by_difficulty = {}

    for r in results:
        key = r["strategy"]
        by_strategy.setdefault(key, []).append(r)

        bkey = str(r.get("token_budget", "default"))
        by_budget.setdefault(bkey, []).append(r)

        dkey = r["difficulty"]
        by_difficulty.setdefault(dkey, []).append(r)

    def avg(items, field):
        vals = [i[field] for i in items if field in i]
        return sum(vals) / len(vals) if vals else 0.0

    stats = {
        "total_questions": len(results),
        "overall": {
            "answer_accuracy": avg(results, "answer_accuracy"),
            "citation_precision": avg(results, "citation_precision"),
            "citation_recall": avg(results, "citation_recall"),
            "citation_f1": avg(results, "citation_f1"),
        },
        "by_strategy": {},
        "by_budget": {},
        "by_difficulty": {},
    }

    for key, items in by_strategy.items():
        stats["by_strategy"][key] = {
            "count": len(items),
            "answer_accuracy": avg(items, "answer_accuracy"),
            "citation_precision": avg(items, "citation_precision"),
            "citation_recall": avg(items, "citation_recall"),
            "citation_f1": avg(items, "citation_f1"),
        }

    for key, items in by_budget.items():
        stats["by_budget"][key] = {
            "count": len(items),
            "answer_accuracy": avg(items, "answer_accuracy"),
            "citation_precision": avg(items, "citation_precision"),
            "citation_recall": avg(items, "citation_recall"),
            "citation_f1": avg(items, "citation_f1"),
        }

    for key, items in by_difficulty.items():
        stats["by_difficulty"][key] = {
            "count": len(items),
            "answer_accuracy": avg(items, "answer_accuracy"),
            "citation_precision": avg(items, "citation_precision"),
            "citation_recall": avg(items, "citation_recall"),
            "citation_f1": avg(items, "citation_f1"),
        }

    return stats


def generate_html_report(data: dict, output_path: Path) -> None:
    """Generate an HTML report from benchmark results."""
    stats = compute_statistics(data.get("results", []))

    strategy_rows = ""
    for strat, s in sorted(stats.get("by_strategy", {}).items()):
        strategy_rows += f"""
        <tr>
            <td>{strat}</td>
            <td>{s['count']}</td>
            <td>{s['answer_accuracy']:.2f}</td>
            <td>{s['citation_precision']:.2f}</td>
            <td>{s['citation_recall']:.2f}</td>
            <td>{s['citation_f1']:.2f}</td>
        </tr>"""

    budget_rows = ""
    for budget, s in sorted(stats.get("by_budget", {}).items()):
        budget_rows += f"""
        <tr>
            <td>{budget}</td>
            <td>{s['count']}</td>
            <td>{s['answer_accuracy']:.2f}</td>
            <td>{s['citation_precision']:.2f}</td>
            <td>{s['citation_recall']:.2f}</td>
            <td>{s['citation_f1']:.2f}</td>
        </tr>"""

    difficulty_rows = ""
    for diff, s in sorted(stats.get("by_difficulty", {}).items()):
        difficulty_rows += f"""
        <tr>
            <td>{diff}</td>
            <td>{s['count']}</td>
            <td>{s['answer_accuracy']:.2f}</td>
            <td>{s['citation_precision']:.2f}</td>
            <td>{s['citation_recall']:.2f}</td>
            <td>{s['citation_f1']:.2f}</td>
        </tr>"""

    overall = stats.get("overall", {})

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>AIF Citation Precision Benchmark</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; max-width: 1100px; margin: 2em auto; padding: 0 1em; color: #1a1a2e; }}
  h1 {{ color: #16213e; border-bottom: 3px solid #0f3460; padding-bottom: 0.3em; }}
  h2 {{ color: #0f3460; margin-top: 1.5em; }}
  table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
  th, td {{ border: 1px solid #ddd; padding: 8px 12px; text-align: center; }}
  th {{ background: #0f3460; color: white; }}
  tr:nth-child(even) {{ background: #f8f9fa; }}
  .summary-cards {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 1em; margin: 1.5em 0; }}
  .card {{ background: linear-gradient(135deg, #667eea, #764ba2); color: white; padding: 1.2em; border-radius: 8px; text-align: center; }}
  .card .value {{ font-size: 2em; font-weight: bold; }}
  .card .label {{ font-size: 0.85em; opacity: 0.9; }}
  .meta {{ color: #666; font-size: 0.9em; margin-bottom: 2em; }}
</style>
</head>
<body>
<h1>AIF Citation Precision Benchmark</h1>
<div class="meta">
  <p>Model: {data.get('model', 'N/A')} | Documents: {data.get('document_count', 0)} |
     Questions: {stats.get('total_questions', 0)} | Timestamp: {data.get('timestamp', 'N/A')}</p>
</div>

<div class="summary-cards">
  <div class="card">
    <div class="value">{overall.get('answer_accuracy', 0):.0%}</div>
    <div class="label">Answer Accuracy</div>
  </div>
  <div class="card">
    <div class="value">{overall.get('citation_precision', 0):.0%}</div>
    <div class="label">Citation Precision</div>
  </div>
  <div class="card">
    <div class="value">{overall.get('citation_recall', 0):.0%}</div>
    <div class="label">Citation Recall</div>
  </div>
  <div class="card">
    <div class="value">{overall.get('citation_f1', 0):.0%}</div>
    <div class="label">Citation F1</div>
  </div>
</div>

<h2>Results by Chunking Strategy</h2>
<table>
  <tr><th>Strategy</th><th>Questions</th><th>Answer Accuracy</th><th>Citation Precision</th><th>Citation Recall</th><th>F1</th></tr>
  {strategy_rows}
</table>

<h2>Results by Token Budget</h2>
<table>
  <tr><th>Token Budget</th><th>Questions</th><th>Answer Accuracy</th><th>Citation Precision</th><th>Citation Recall</th><th>F1</th></tr>
  {budget_rows}
</table>

<h2>Results by Question Difficulty</h2>
<table>
  <tr><th>Difficulty</th><th>Questions</th><th>Answer Accuracy</th><th>Citation Precision</th><th>Citation Recall</th><th>F1</th></tr>
  {difficulty_rows}
</table>

<h2>Methodology</h2>
<p>For each document, chunks are created using 4 strategies (section, token-budget, semantic, fixed-blocks) at multiple token budgets.
The LLM receives the chunks and a question, and must answer with citations to specific chunk IDs.
A judge LLM scores answer accuracy (keyword matching) and citation precision/recall (whether cited chunks contain the source material).</p>

<footer style="margin-top: 3em; padding-top: 1em; border-top: 1px solid #ddd; color: #888; font-size: 0.85em;">
  Generated by AIF Citation Precision Benchmark | <a href="https://github.com/anthropics/aif">SkillForge</a>
</footer>
</body>
</html>"""

    output_path.write_text(html)
    print(f"HTML report written to {output_path}")


def print_text_report(data: dict) -> None:
    """Print a text summary of benchmark results."""
    stats = compute_statistics(data.get("results", []))

    print("=" * 70)
    print("AIF Citation Precision Benchmark — Summary")
    print("=" * 70)

    overall = stats.get("overall", {})
    print(f"\nOverall (n={stats.get('total_questions', 0)}):")
    print(f"  Answer Accuracy:     {overall.get('answer_accuracy', 0):.2f}")
    print(f"  Citation Precision:  {overall.get('citation_precision', 0):.2f}")
    print(f"  Citation Recall:     {overall.get('citation_recall', 0):.2f}")
    print(f"  Citation F1:         {overall.get('citation_f1', 0):.2f}")

    print("\nBy Strategy:")
    for strat, s in sorted(stats.get("by_strategy", {}).items()):
        print(f"  {strat:20s}  acc={s['answer_accuracy']:.2f}  prec={s['citation_precision']:.2f}  "
              f"rec={s['citation_recall']:.2f}  f1={s['citation_f1']:.2f}  (n={s['count']})")

    print("\nBy Token Budget:")
    for budget, s in sorted(stats.get("by_budget", {}).items()):
        print(f"  {budget:10s}  acc={s['answer_accuracy']:.2f}  prec={s['citation_precision']:.2f}  "
              f"rec={s['citation_recall']:.2f}  f1={s['citation_f1']:.2f}  (n={s['count']})")

    print("\nBy Difficulty:")
    for diff, s in sorted(stats.get("by_difficulty", {}).items()):
        print(f"  {diff:10s}  acc={s['answer_accuracy']:.2f}  prec={s['citation_precision']:.2f}  "
              f"rec={s['citation_recall']:.2f}  f1={s['citation_f1']:.2f}  (n={s['count']})")


if __name__ == "__main__":
    results_path = BENCH_DIR / "results.json"
    if not results_path.exists():
        print(f"No results found at {results_path}. Run benchmark.py first.")
    else:
        data = json.loads(results_path.read_text())
        print_text_report(data)
        generate_html_report(data, BENCH_DIR / "report.html")
