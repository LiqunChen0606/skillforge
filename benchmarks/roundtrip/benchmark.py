#!/usr/bin/env python3
"""
AIF Roundtrip Quality Benchmark

Measures how faithfully AIF documents survive a roundtrip through each format:
  AIF → format (html, markdown, json) → re-import → compare IR

Metrics per (document, format):
  - block_count_ratio     — roundtripped blocks / original blocks
  - block_type_preservation — fraction of blocks keeping the same BlockKind type
  - semantic_type_preservation — fraction of SemanticBlock types surviving
  - metadata_preservation — fraction of metadata keys preserved (excluding _aif_*)
  - inline_fidelity       — fraction of inline element types preserved
  - overall_fidelity      — weighted average

Usage:
    python benchmarks/roundtrip_benchmark.py [--examples-dir examples/]
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
EXAMPLES_DIR = PROJECT_ROOT / "examples"
FORMATS = ["html", "markdown", "json"]


# ── CLI helpers ──────────────────────────────────────────────────────────────

def run_cli(args: list[str], stdin_data: str | None = None) -> str:
    """Run aif-cli and return stdout."""
    cli_bin = PROJECT_ROOT / "target" / "release" / "aif-cli"
    result = subprocess.run(
        [str(cli_bin)] + args,
        capture_output=True, text=True, cwd=PROJECT_ROOT,
        input=stdin_data,
    )
    if result.returncode != 0:
        print(f"  CLI error ({' '.join(args[:3])}): {result.stderr.strip()[:200]}",
              file=sys.stderr)
        return ""
    return result.stdout


def dump_ir(aif_file: str) -> dict | None:
    """Parse an .aif file and return its JSON IR."""
    out = run_cli(["dump-ir", aif_file])
    if not out:
        return None
    return json.loads(out)


def compile_to(aif_file: str, fmt: str, output_path: str) -> bool:
    """Compile .aif to a given format, writing to output_path."""
    out = run_cli(["compile", aif_file, "--format", fmt, "-o", output_path])
    # compile writes to file, stdout may be empty
    return os.path.exists(output_path)


def import_file(file_path: str) -> dict | None:
    """Import a file (html/md) back to JSON IR via stdout."""
    out = run_cli(["import", file_path])
    if not out:
        return None
    return json.loads(out)


def json_roundtrip(json_path: str) -> dict | None:
    """Compile JSON IR back through the JSON format path."""
    out = run_cli(["compile", json_path, "--input-format", "json", "--format", "json"])
    if not out:
        return None
    return json.loads(out)


# ── Block / Inline extraction ────────────────────────────────────────────────

def collect_blocks(blocks: list[dict]) -> list[dict]:
    """Recursively collect all blocks from nested structures."""
    result = []
    for block in blocks:
        kind = block.get("kind", {})
        result.append(kind)
        block_type = kind.get("type", "")
        # Recurse into children depending on block type
        if block_type == "Section":
            result.extend(collect_blocks(kind.get("children", [])))
        elif block_type == "BlockQuote":
            result.extend(collect_blocks(kind.get("content", [])))
        elif block_type == "SkillBlock":
            result.extend(collect_blocks(kind.get("children", [])))
        elif block_type == "List":
            for item in kind.get("items", []):
                result.extend(collect_blocks(item.get("children", [])))
    return result


def collect_inlines(obj) -> list[str]:
    """Recursively collect all inline type names from any JSON structure."""
    types = []
    if isinstance(obj, dict):
        if "type" in obj:
            t = obj["type"]
            # Inline types (not block types)
            if t in ("Text", "Emphasis", "Strong", "InlineCode", "Link",
                      "Image", "Reference", "Footnote", "SoftBreak", "HardBreak"):
                types.append(t)
        for v in obj.values():
            types.extend(collect_inlines(v))
    elif isinstance(obj, list):
        for item in obj:
            types.extend(collect_inlines(item))
    return types


def get_block_types(blocks: list[dict]) -> list[str]:
    """Return list of block type strings."""
    return [b.get("type", "Unknown") for b in blocks]


def get_semantic_types(blocks: list[dict]) -> list[str]:
    """Return list of SemanticBlock block_type values."""
    result = []
    for b in blocks:
        if b.get("type") == "SemanticBlock":
            result.append(b.get("block_type", "Unknown"))
    return result


def get_metadata_keys(doc: dict) -> set[str]:
    """Return metadata keys, excluding _aif_* provenance keys."""
    meta = doc.get("metadata", {})
    return {k for k in meta.keys() if not k.startswith("_aif_")}


# ── Comparison ───────────────────────────────────────────────────────────────

def compare_ir(original: dict, roundtripped: dict) -> dict:
    """Compare original and roundtripped IR, returning fidelity metrics."""
    orig_blocks = collect_blocks(original.get("blocks", []))
    rt_blocks = collect_blocks(roundtripped.get("blocks", []))

    orig_count = len(orig_blocks)
    rt_count = len(rt_blocks)

    # Block count ratio
    block_count_ratio = rt_count / orig_count if orig_count > 0 else 0.0

    # Block type preservation: for each original block, check if there's a
    # matching type in the roundtripped output (order-aligned comparison)
    orig_types = get_block_types(orig_blocks)
    rt_types = get_block_types(rt_blocks)
    matched_types = 0
    for i, t in enumerate(orig_types):
        if i < len(rt_types) and rt_types[i] == t:
            matched_types += 1
    block_type_pres = matched_types / len(orig_types) if orig_types else 1.0

    # Semantic type preservation
    orig_sem = get_semantic_types(orig_blocks)
    rt_sem = get_semantic_types(rt_blocks)
    if orig_sem:
        # Count how many original semantic types appear in roundtripped
        # Use multiset matching (order-independent)
        from collections import Counter
        orig_counter = Counter(orig_sem)
        rt_counter = Counter(rt_sem)
        preserved = sum(min(orig_counter[k], rt_counter.get(k, 0))
                        for k in orig_counter)
        semantic_pres = preserved / len(orig_sem)
    else:
        semantic_pres = 1.0  # No semantic blocks to lose

    # Metadata preservation
    orig_meta = get_metadata_keys(original)
    rt_meta = get_metadata_keys(roundtripped)
    if orig_meta:
        meta_pres = len(orig_meta & rt_meta) / len(orig_meta)
    else:
        meta_pres = 1.0

    # Inline fidelity: compare type distributions
    orig_inlines = collect_inlines(original.get("blocks", []))
    rt_inlines = collect_inlines(roundtripped.get("blocks", []))
    if orig_inlines:
        from collections import Counter
        orig_icnt = Counter(orig_inlines)
        rt_icnt = Counter(rt_inlines)
        total_orig = sum(orig_icnt.values())
        preserved_inlines = sum(min(orig_icnt[k], rt_icnt.get(k, 0))
                                for k in orig_icnt)
        inline_fidelity = preserved_inlines / total_orig
    else:
        inline_fidelity = 1.0

    # Overall fidelity: weighted average
    # block_type=1, semantic=2, metadata=1, inline=1 → /5
    overall = (block_type_pres * 1
               + semantic_pres * 2
               + meta_pres * 1
               + inline_fidelity * 1) / 5.0

    return {
        "block_count_original": orig_count,
        "block_count_roundtripped": rt_count,
        "block_count_ratio": round(block_count_ratio, 4),
        "block_type_preservation": round(block_type_pres, 4),
        "semantic_type_preservation": round(semantic_pres, 4),
        "metadata_preservation": round(meta_pres, 4),
        "inline_fidelity": round(inline_fidelity, 4),
        "overall_fidelity": round(overall, 4),
    }


# ── Main ─────────────────────────────────────────────────────────────────────

def run_benchmark(examples_dir: Path) -> list[dict]:
    aif_files = sorted(examples_dir.glob("**/*.aif"))
    if not aif_files:
        print("No .aif files found in", examples_dir, file=sys.stderr)
        sys.exit(1)

    results = []

    for aif_file in aif_files:
        filename = aif_file.name
        print(f"\n{'='*60}")
        print(f"  {filename}")
        print(f"{'='*60}")

        # Step 1: dump original IR
        original_ir = dump_ir(str(aif_file))
        if original_ir is None:
            print(f"  SKIP: could not parse {filename}", file=sys.stderr)
            continue

        for fmt in FORMATS:
            print(f"  [{fmt:>10}] ", end="", flush=True)

            with tempfile.TemporaryDirectory() as tmpdir:
                # Step 2: compile to format
                ext = {"html": "html", "markdown": "md", "json": "json"}[fmt]
                compiled_path = os.path.join(tmpdir, f"compiled.{ext}")

                ok = compile_to(str(aif_file), fmt, compiled_path)
                if not ok:
                    print("FAIL (compile)")
                    results.append({
                        "file": filename, "format": fmt,
                        "error": "compile_failed",
                    })
                    continue

                # Step 3: re-import back to IR
                if fmt == "json":
                    rt_ir = json_roundtrip(compiled_path)
                else:
                    rt_ir = import_file(compiled_path)

                if rt_ir is None:
                    print("FAIL (import)")
                    results.append({
                        "file": filename, "format": fmt,
                        "error": "import_failed",
                    })
                    continue

                # Step 4: compare
                metrics = compare_ir(original_ir, rt_ir)
                metrics["file"] = filename
                metrics["format"] = fmt
                results.append(metrics)

                print(f"blocks={metrics['block_count_ratio']:.2f}  "
                      f"types={metrics['block_type_preservation']:.2f}  "
                      f"semantic={metrics['semantic_type_preservation']:.2f}  "
                      f"meta={metrics['metadata_preservation']:.2f}  "
                      f"inline={metrics['inline_fidelity']:.2f}  "
                      f"overall={metrics['overall_fidelity']:.2f}")

    return results


def print_table(results: list[dict]):
    """Print a formatted results table."""
    # Filter out errors
    good = [r for r in results if "error" not in r]
    errors = [r for r in results if "error" in r]

    print(f"\n{'='*90}")
    print(f"  ROUNDTRIP FIDELITY RESULTS")
    print(f"{'='*90}")
    header = f"{'File':<38} {'Format':<10} {'Blocks':>7} {'Types':>7} {'Semntic':>7} {'Meta':>7} {'Inline':>7} {'Overall':>7}"
    print(header)
    print("-" * len(header))

    for r in good:
        print(f"{r['file']:<38} {r['format']:<10} "
              f"{r['block_count_ratio']:>7.2f} "
              f"{r['block_type_preservation']:>7.2f} "
              f"{r['semantic_type_preservation']:>7.2f} "
              f"{r['metadata_preservation']:>7.2f} "
              f"{r['inline_fidelity']:>7.2f} "
              f"{r['overall_fidelity']:>7.2f}")

    if errors:
        print(f"\nFailed roundtrips ({len(errors)}):")
        for e in errors:
            print(f"  {e['file']} [{e['format']}]: {e['error']}")

    # Summary by format
    print(f"\n{'='*60}")
    print(f"  SUMMARY BY FORMAT")
    print(f"{'='*60}")
    header2 = f"{'Format':<12} {'Avg Overall':>12} {'Avg Semantic':>13} {'Avg Types':>10} {'Avg Meta':>9} {'Avg Inline':>11}"
    print(header2)
    print("-" * len(header2))

    for fmt in FORMATS:
        fmt_results = [r for r in good if r["format"] == fmt]
        if not fmt_results:
            print(f"{fmt:<12} {'(no data)':>12}")
            continue
        n = len(fmt_results)
        avg_overall = sum(r["overall_fidelity"] for r in fmt_results) / n
        avg_semantic = sum(r["semantic_type_preservation"] for r in fmt_results) / n
        avg_types = sum(r["block_type_preservation"] for r in fmt_results) / n
        avg_meta = sum(r["metadata_preservation"] for r in fmt_results) / n
        avg_inline = sum(r["inline_fidelity"] for r in fmt_results) / n
        print(f"{fmt:<12} {avg_overall:>12.4f} {avg_semantic:>13.4f} {avg_types:>10.4f} {avg_meta:>9.4f} {avg_inline:>11.4f}")


def main():
    examples_dir = EXAMPLES_DIR
    # Allow override via --examples-dir
    args = sys.argv[1:]
    if "--examples-dir" in args:
        idx = args.index("--examples-dir")
        if idx + 1 < len(args):
            examples_dir = Path(args[idx + 1])

    print("AIF Roundtrip Quality Benchmark")
    print(f"Examples directory: {examples_dir}")
    print(f"Formats: {', '.join(FORMATS)}")

    results = run_benchmark(examples_dir)

    print_table(results)

    # Save JSON results
    output_path = PROJECT_ROOT / "benchmarks" / "roundtrip" / "results.json"
    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {output_path}")


if __name__ == "__main__":
    main()
