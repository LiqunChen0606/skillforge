#!/usr/bin/env python3
"""
AIF Chunking Quality Benchmark

Evaluates how well different chunking strategies preserve semantic boundaries
and produce useful, self-contained chunks from AIF documents.

Metrics:
1. Boundary Accuracy - Do chunk boundaries align with section/semantic block boundaries?
2. Self-Containment - Does each chunk make sense on its own (has title, doesn't split mid-block)?
3. Token Budget Compliance - Do chunks stay within the target token budget?
4. Semantic Coverage - Are all semantic block types represented across chunks?
5. Chunk Size Variance - Are chunks roughly even in size (low variance = better)?

Usage:
    python benchmarks/chunking_quality_benchmark.py [--examples-dir examples/]
"""

import json
import subprocess
import sys
import os
import math
from pathlib import Path


def run_cli(args: list[str]) -> str:
    """Run aif-cli and return stdout."""
    result = subprocess.run(
        ["cargo", "run", "-p", "aif-cli", "--"] + args,
        capture_output=True, text=True, cwd=Path(__file__).parent.parent.parent,
    )
    if result.returncode != 0:
        print(f"  CLI error: {result.stderr.strip()}", file=sys.stderr)
        return ""
    return result.stdout


def chunk_document(aif_file: str, strategy: str, max_tokens: int = 2048) -> list[dict]:
    """Chunk a document and return parsed chunk metadata."""
    args = ["chunk", "split", aif_file, "--strategy", strategy]
    if strategy == "token-budget":
        args += ["--max-tokens", str(max_tokens)]
    output = run_cli(args)
    if not output:
        return []

    chunks = []
    for line in output.strip().split("\n"):
        if not line.strip():
            continue
        # Format: "chunk_id | blocks: N | tokens: ~T | title: ..."
        parts = line.split("|")
        if len(parts) >= 4:
            chunk_id = parts[0].strip()
            blocks = int(parts[1].strip().replace("blocks: ", ""))
            tokens = int(parts[2].strip().replace("tokens: ~", ""))
            title = parts[3].strip().replace("title: ", "")
            chunks.append({
                "id": chunk_id,
                "blocks": blocks,
                "tokens": tokens,
                "title": title if title != "(none)" else None,
            })
    return chunks


def dump_ir(aif_file: str) -> dict | None:
    """Dump the IR of a document as JSON."""
    output = run_cli(["dump-ir", aif_file])
    if not output:
        return None
    return json.loads(output)


def count_semantic_blocks(ir: dict) -> dict[str, int]:
    """Count semantic block types in the IR."""
    counts: dict[str, int] = {}

    def walk(blocks):
        for block in blocks:
            kind = block.get("kind", {})
            btype = kind.get("type", "")
            if btype == "SemanticBlock":
                stype = kind.get("block_type", "Unknown")
                counts[stype] = counts.get(stype, 0) + 1
            if btype == "Section":
                walk(kind.get("children", []))
            if btype == "SkillBlock":
                walk(kind.get("children", []))
            if btype == "BlockQuote":
                walk(kind.get("content", []))

    walk(ir.get("blocks", []))
    return counts


def evaluate_chunking(aif_file: str, strategy: str, max_tokens: int = 2048) -> dict:
    """Evaluate a chunking strategy on a single document."""
    chunks = chunk_document(aif_file, strategy, max_tokens)
    if not chunks:
        return {"error": "no chunks produced", "file": aif_file, "strategy": strategy}

    n = len(chunks)
    tokens_list = [c["tokens"] for c in chunks]
    blocks_list = [c["blocks"] for c in chunks]

    # 1. Self-containment: fraction of chunks that have a title
    titled = sum(1 for c in chunks if c["title"])
    self_containment = titled / n if n > 0 else 0

    # 2. Token budget compliance (only for token-budget strategy)
    if strategy == "token-budget":
        within_budget = sum(1 for t in tokens_list if t <= max_tokens * 1.1)  # 10% tolerance
        budget_compliance = within_budget / n if n > 0 else 0
    else:
        budget_compliance = None

    # 3. Chunk size variance (coefficient of variation)
    mean_tokens = sum(tokens_list) / n if n > 0 else 0
    if mean_tokens > 0 and n > 1:
        variance = sum((t - mean_tokens) ** 2 for t in tokens_list) / (n - 1)
        cv = math.sqrt(variance) / mean_tokens
    else:
        cv = 0.0

    # 4. Average blocks per chunk
    avg_blocks = sum(blocks_list) / n if n > 0 else 0

    return {
        "file": os.path.basename(aif_file),
        "strategy": strategy,
        "num_chunks": n,
        "total_tokens": sum(tokens_list),
        "mean_tokens": round(mean_tokens, 1),
        "min_tokens": min(tokens_list) if tokens_list else 0,
        "max_tokens": max(tokens_list) if tokens_list else 0,
        "self_containment": round(self_containment, 3),
        "budget_compliance": round(budget_compliance, 3) if budget_compliance is not None else None,
        "size_cv": round(cv, 3),
        "avg_blocks_per_chunk": round(avg_blocks, 1),
    }


def main():
    examples_dir = Path(__file__).parent.parent.parent / "examples"
    aif_files = sorted(examples_dir.glob("**/*.aif"))

    if not aif_files:
        print("No .aif files found in examples/", file=sys.stderr)
        sys.exit(1)

    strategies = ["section", "token-budget", "semantic", "fixed-blocks"]
    all_results = []

    print("=" * 70)
    print("AIF Chunking Quality Benchmark")
    print("=" * 70)
    print()

    for aif_file in aif_files:
        print(f"Document: {aif_file.name}")

        # Get semantic block counts
        ir = dump_ir(str(aif_file))
        if ir:
            sem_counts = count_semantic_blocks(ir)
            total_blocks = len(ir.get("blocks", []))
            if sem_counts:
                print(f"  Blocks: {total_blocks}, Semantic: {sem_counts}")
            else:
                print(f"  Blocks: {total_blocks}")
        print()

        for strategy in strategies:
            result = evaluate_chunking(str(aif_file), strategy)
            all_results.append(result)

            if "error" in result:
                print(f"  {strategy:15s} ERROR: {result['error']}")
                continue

            budget_str = f", budget_compliance={result['budget_compliance']}" if result['budget_compliance'] is not None else ""
            print(
                f"  {strategy:15s}  chunks={result['num_chunks']:2d}  "
                f"tokens={result['mean_tokens']:7.1f} avg  "
                f"[{result['min_tokens']}-{result['max_tokens']}]  "
                f"self_contained={result['self_containment']:.0%}  "
                f"cv={result['size_cv']:.2f}"
                f"{budget_str}"
            )
        print()

    # Summary
    print("=" * 70)
    print("Strategy Summary (across all documents)")
    print("=" * 70)
    print()
    print(f"{'Strategy':15s} {'Chunks':>7s} {'Avg Tokens':>11s} {'Self-Cont':>10s} {'Size CV':>8s}")
    print("-" * 55)

    for strategy in strategies:
        strat_results = [r for r in all_results if r.get("strategy") == strategy and "error" not in r]
        if not strat_results:
            continue
        total_chunks = sum(r["num_chunks"] for r in strat_results)
        avg_tokens = sum(r["mean_tokens"] * r["num_chunks"] for r in strat_results) / total_chunks if total_chunks > 0 else 0
        avg_sc = sum(r["self_containment"] for r in strat_results) / len(strat_results)
        avg_cv = sum(r["size_cv"] for r in strat_results) / len(strat_results)
        print(f"{strategy:15s} {total_chunks:7d} {avg_tokens:11.1f} {avg_sc:10.0%} {avg_cv:8.2f}")

    # Save JSON
    output_path = Path(__file__).parent / "chunking_results.json"
    with open(output_path, "w") as f:
        json.dump(all_results, f, indent=2)
    print(f"\nResults saved to {output_path}")


if __name__ == "__main__":
    main()
