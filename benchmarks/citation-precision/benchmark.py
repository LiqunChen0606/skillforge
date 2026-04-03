#!/usr/bin/env python3
"""
AIF Citation Precision Benchmark

Measures how accurately an LLM can answer questions and cite relevant chunks
when documents are split using different chunking strategies and token budgets.

For each document × strategy × budget:
1. Split the document into chunks using `aif chunk split`
2. Present all chunks + a question to the LLM
3. Ask the LLM to answer with citations to chunk IDs
4. Score answer accuracy (keyword matching) and citation precision/recall

Metrics:
- Answer accuracy: fraction of expected keywords found in the answer
- Citation precision: fraction of cited chunks that are relevant
- Citation recall: fraction of relevant chunks that were cited
- Citation F1: harmonic mean of precision and recall

Requires ANTHROPIC_API_KEY environment variable.

Usage:
    ANTHROPIC_API_KEY=sk-... python benchmarks/citation-precision/benchmark.py
    python benchmarks/citation-precision/analysis.py   # analyze saved results
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import anthropic

# Allow running from project root or benchmark dir
BENCH_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = BENCH_DIR.parent.parent
sys.path.insert(0, str(BENCH_DIR))
from questions import GROUND_TRUTH

# ── Configuration ─────────────────────────────────────────────────────────

MODEL = "claude-sonnet-4-6"
AIF_CLI = PROJECT_ROOT / "target" / "release" / "aif-cli"

STRATEGIES = ["section", "token-budget", "semantic", "fixed-blocks"]
TOKEN_BUDGETS = [512, 1024, 2048, 4096]


# ── Helpers ───────────────────────────────────────────────────────────────


def chunk_document(doc_path: str, strategy: str, token_budget: int) -> list[dict]:
    """Split a document into chunks using `aif chunk split` and return chunk data."""
    with tempfile.TemporaryDirectory() as tmpdir:
        cmd = [
            str(AIF_CLI), "chunk", "split", doc_path,
            "--strategy", strategy,
            "--max-tokens", str(token_budget),
            "--output", tmpdir,
        ]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if result.returncode != 0:
            return []

        chunks = []
        chunk_dir = Path(tmpdir)
        for chunk_file in sorted(chunk_dir.glob("*.json")):
            try:
                chunk_data = json.loads(chunk_file.read_text())
                chunks.append({
                    "id": chunk_file.stem,
                    "content": json.dumps(chunk_data, indent=2)[:2000],
                    "blocks": chunk_data.get("blocks", []),
                })
            except (json.JSONDecodeError, KeyError):
                continue

        return chunks


def dump_ir(doc_path: str) -> dict | None:
    """Dump the IR of a document to get section IDs and block structure."""
    cmd = [str(AIF_CLI), "dump-ir", doc_path]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
    if result.returncode != 0:
        return None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return None


def find_relevant_chunk_ids(chunks: list[dict], section_ids: list[str]) -> set[str]:
    """Determine which chunk IDs contain content from the target section IDs."""
    relevant = set()
    for chunk in chunks:
        chunk_text = json.dumps(chunk)
        for section_id in section_ids:
            if section_id in chunk_text:
                relevant.add(chunk["id"])
    return relevant


def ask_with_citations(
    client: anthropic.Anthropic,
    chunks: list[dict],
    question: str,
) -> tuple[str, list[str], float]:
    """Ask the LLM a question with chunk context and get answer + cited chunk IDs."""
    chunk_text = ""
    for chunk in chunks:
        chunk_text += f"\n--- CHUNK [{chunk['id']}] ---\n{chunk['content']}\n"

    prompt = f"""You are answering a question based on document chunks. Each chunk has an ID in brackets.

CHUNKS:
{chunk_text}

QUESTION: {question}

Answer the question based ONLY on the provided chunks. After your answer, list the chunk IDs you used as sources.

Format your response as:
ANSWER: <your answer>
SOURCES: [chunk_id_1], [chunk_id_2], ...

If you cannot answer from the chunks, say "ANSWER: Cannot determine from provided chunks" and "SOURCES: none"."""

    t0 = time.time()
    response = client.messages.create(
        model=MODEL,
        max_tokens=1024,
        messages=[{"role": "user", "content": prompt}],
    )
    latency = time.time() - t0
    text = response.content[0].text

    # Extract answer
    answer = ""
    if "ANSWER:" in text:
        answer = text.split("ANSWER:")[1].split("SOURCES:")[0].strip()

    # Extract cited chunk IDs
    cited = []
    if "SOURCES:" in text:
        sources_text = text.split("SOURCES:")[1].strip()
        cited = re.findall(r'\[?(chunk[_-]?\d+|[a-zA-Z0-9_-]+)\]?', sources_text)
        # Normalize: only keep IDs that match actual chunk IDs
        chunk_ids = {c["id"] for c in chunks}
        cited = [c for c in cited if c in chunk_ids]

    return answer, cited, latency


def score_answer(answer: str, keywords: list[str]) -> float:
    """Score answer accuracy by fraction of keywords found."""
    if not keywords:
        return 1.0
    answer_lower = answer.lower()
    found = sum(1 for kw in keywords if kw.lower() in answer_lower)
    return found / len(keywords)


def citation_metrics(cited: list[str], relevant: set[str]) -> dict:
    """Compute citation precision, recall, and F1."""
    cited_set = set(cited)
    if not cited_set and not relevant:
        return {"precision": 1.0, "recall": 1.0, "f1": 1.0}

    true_positives = len(cited_set & relevant)
    precision = true_positives / len(cited_set) if cited_set else 0.0
    recall = true_positives / len(relevant) if relevant else 0.0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0.0

    return {"precision": precision, "recall": recall, "f1": f1}


# ── Main ──────────────────────────────────────────────────────────────────


def main():
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("Error: ANTHROPIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    # Check CLI exists
    if not AIF_CLI.exists():
        print(f"Error: aif-cli not found at {AIF_CLI}", file=sys.stderr)
        print("Run: cargo build --release -p aif-cli", file=sys.stderr)
        sys.exit(1)

    client = anthropic.Anthropic(api_key=api_key)
    all_results = []

    print("=" * 80)
    print("AIF Citation Precision Benchmark")
    print(f"Model: {MODEL}")
    print(f"Documents: {len(GROUND_TRUTH)} | Strategies: {len(STRATEGIES)} | Budgets: {len(TOKEN_BUDGETS)}")
    print("=" * 80)

    for doc_path, doc_info in GROUND_TRUTH.items():
        full_path = str(PROJECT_ROOT / doc_path)
        print(f"\nDocument: {doc_info['name']} ({doc_path})")

        if not Path(full_path).exists():
            print(f"  SKIP — file not found")
            continue

        for strategy in STRATEGIES:
            # For section strategy, budget doesn't matter much; run once
            budgets = [2048] if strategy == "section" else TOKEN_BUDGETS

            for budget in budgets:
                chunks = chunk_document(full_path, strategy, budget)
                if not chunks:
                    print(f"  {strategy:15s} budget={budget:5d}  SKIP (chunking failed)")
                    continue

                print(f"  {strategy:15s} budget={budget:5d}  chunks={len(chunks):2d}", end="")

                for q in doc_info["questions"]:
                    relevant = find_relevant_chunk_ids(chunks, q["source_section_ids"])

                    answer, cited, latency = ask_with_citations(
                        client, chunks, q["question"]
                    )

                    accuracy = score_answer(answer, q["answer_keywords"])
                    metrics = citation_metrics(cited, relevant)

                    result = {
                        "document": doc_path,
                        "doc_name": doc_info["name"],
                        "strategy": strategy,
                        "token_budget": budget,
                        "num_chunks": len(chunks),
                        "question": q["question"],
                        "difficulty": q["difficulty"],
                        "answer_accuracy": round(accuracy, 3),
                        "citation_precision": round(metrics["precision"], 3),
                        "citation_recall": round(metrics["recall"], 3),
                        "citation_f1": round(metrics["f1"], 3),
                        "num_cited": len(cited),
                        "num_relevant": len(relevant),
                        "latency_s": round(latency, 1),
                        "answer_preview": answer[:200],
                    }
                    all_results.append(result)

                print(f"  questions={len(doc_info['questions'])}", end="")
                # Print avg metrics for this strategy+budget
                strat_results = [r for r in all_results
                                 if r["strategy"] == strategy and r["token_budget"] == budget
                                 and r["document"] == doc_path]
                avg_acc = sum(r["answer_accuracy"] for r in strat_results) / len(strat_results)
                avg_f1 = sum(r["citation_f1"] for r in strat_results) / len(strat_results)
                print(f"  avg_acc={avg_acc:.2f}  avg_f1={avg_f1:.2f}")

    # Save results
    output = {
        "model": MODEL,
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "document_count": len(GROUND_TRUTH),
        "strategy_count": len(STRATEGIES),
        "budget_count": len(TOKEN_BUDGETS),
        "total_questions": len(all_results),
        "results": all_results,
    }

    output_path = BENCH_DIR / "results.json"
    output_path.write_text(json.dumps(output, indent=2))
    print(f"\nResults saved to {output_path}")

    # Generate reports
    print()
    from analysis import print_text_report, generate_html_report
    print_text_report(output)
    generate_html_report(output, BENCH_DIR / "report.html")


if __name__ == "__main__":
    main()
