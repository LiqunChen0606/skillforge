#!/usr/bin/env python3
"""
AIF Token Efficiency Benchmark

Downloads real Wikipedia articles as HTML, converts through the AIF pipeline,
and compares token costs across formats using Claude Opus 4.6.
"""

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import base64

import anthropic
import fitz  # PyMuPDF
import html2text

# ── Configuration ──────────────────────────────────────────────────────────

MODEL = "claude-opus-4-6"

# Wikipedia articles to benchmark (title → URL path)
ARTICLES = {
    "Photosynthesis": "Photosynthesis",
    "General_relativity": "General_relativity",
    "Python_(programming_language)": "Python_(programming_language)",
    "World_War_II": "World_War_II",
    "Quantum_computing": "Quantum_computing",
    "DNA": "DNA",
    "Climate_change": "Climate_change",
    "Machine_learning": "Machine_learning",
    "Roman_Empire": "Roman_Empire",
    "Artificial_intelligence": "Artificial_intelligence",
}

PROJECT_ROOT = Path(__file__).resolve().parent.parent
AIF_CLI = PROJECT_ROOT / "target" / "release" / "aif-cli"

# ── Helpers ────────────────────────────────────────────────────────────────


def fetch_wikipedia_html(title: str) -> str:
    """Fetch the raw HTML content of a Wikipedia article via the REST API."""
    import urllib.request

    url = f"https://en.wikipedia.org/api/rest_v1/page/html/{title}"
    req = urllib.request.Request(url, headers={"User-Agent": "AIF-Benchmark/1.0"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        return resp.read().decode("utf-8")


def fetch_wikipedia_pdf(title: str) -> bytes | None:
    """Fetch the PDF rendering of a Wikipedia article."""
    import urllib.request

    url = f"https://en.wikipedia.org/api/rest_v1/page/pdf/{title}"
    req = urllib.request.Request(url, headers={"User-Agent": "AIF-Benchmark/1.0"})
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return resp.read()
    except Exception as e:
        print(f"  Warning: PDF fetch failed: {e}", file=sys.stderr)
        return None


def pdf_to_text(pdf_bytes: bytes) -> str:
    """Extract plain text from a PDF using PyMuPDF."""
    doc = fitz.open(stream=pdf_bytes, filetype="pdf")
    text = ""
    for page in doc:
        text += page.get_text()
    doc.close()
    return text


def count_tokens_pdf(client: anthropic.Anthropic, pdf_bytes: bytes) -> int:
    """Count tokens for a PDF document sent as a base64-encoded file to Claude."""
    b64 = base64.standard_b64encode(pdf_bytes).decode("ascii")
    result = client.messages.count_tokens(
        model=MODEL,
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "document",
                        "source": {
                            "type": "base64",
                            "media_type": "application/pdf",
                            "data": b64,
                        },
                    },
                ],
            }
        ],
    )
    return result.input_tokens


def html_to_markdown(html: str) -> str:
    """Convert HTML to Markdown using html2text."""
    h = html2text.HTML2Text()
    h.body_width = 0  # no line wrapping
    h.ignore_links = False
    h.ignore_images = False
    h.ignore_tables = False
    h.protect_links = True
    h.unicode_snob = True
    return h.handle(html)


def markdown_to_aif(md_text: str) -> str:
    """Import Markdown via AIF CLI → JSON IR → compile to AIF text.

    Since we don't have a JSON→AIF text compiler, we return the AIF JSON IR
    and also compile to each output format.
    """
    with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False) as f:
        f.write(md_text)
        md_path = f.name

    try:
        result = subprocess.run(
            [str(AIF_CLI), "import", md_path],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            print(f"  Warning: AIF import failed: {result.stderr}", file=sys.stderr)
            return ""
        return result.stdout
    finally:
        os.unlink(md_path)


def aif_json_to_format(json_ir: str, fmt: str) -> str:
    """Given AIF JSON IR, we need to go through a .aif file.

    Since the CLI compiles from .aif source (not JSON), we use the markdown
    import path: MD → JSON IR. For output formats, we re-import and compile.
    """
    # Write JSON IR to temp, but CLI doesn't accept JSON as input for compile.
    # Instead, we'll do: write markdown to temp → import → compile
    # Actually, we can't easily do this without a JSON→compile path.
    # Let's use a different approach: compile from markdown directly.
    return ""


def compile_from_md(md_text: str, fmt: str) -> str:
    """Import Markdown → AIF IR, then we need to compile to output format.

    The CLI doesn't have a direct md→html path, so we'll do it in two steps
    by writing a temp .aif file. But we don't have JSON→AIF source either.

    Workaround: Use the library functions directly via a small Rust helper,
    or just measure the formats we can produce.
    """
    # For now: the JSON IR from import IS one of our measured formats.
    # For LML/HTML/MD output, we'd need the .aif source.
    # Let's use the wiki_article.aif example for those.
    return ""


def count_tokens(client: anthropic.Anthropic, text: str) -> int:
    """Count tokens using Claude's token counting API."""
    result = client.messages.count_tokens(
        model=MODEL,
        messages=[{"role": "user", "content": text}],
    )
    return result.input_tokens


def format_size(n: int) -> str:
    if n >= 1_000_000:
        return f"{n/1_000_000:.1f}M"
    if n >= 1_000:
        return f"{n/1_000:.1f}K"
    return str(n)


# ── Main ───────────────────────────────────────────────────────────────────


def main():
    if not AIF_CLI.exists():
        print(f"Error: AIF CLI not found at {AIF_CLI}")
        print("Run: cargo build --release")
        sys.exit(1)

    api_key = os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("claude_API")
    if not api_key:
        print("Error: Set ANTHROPIC_API_KEY or claude_API environment variable")
        sys.exit(1)
    api_key = api_key.strip()
    # Validate key format: should end with alphanumeric matching typical key pattern
    # Try the key, and if auth fails, try trimming the last character (known .bash_profile typo)
    try:
        test_client = anthropic.Anthropic(api_key=api_key)
        test_client.messages.count_tokens(
            model=MODEL, messages=[{"role": "user", "content": "test"}]
        )
        client = test_client
    except anthropic.AuthenticationError:
        api_key = api_key[:-1]
        client = anthropic.Anthropic(api_key=api_key)

    print("=" * 80)
    print("AIF Token Efficiency Benchmark")
    print(f"Model: {MODEL}")
    print("=" * 80)
    print()

    results = []

    for title, url_path in ARTICLES.items():
        print(f"── {title} ", "─" * (60 - len(title)))

        # 1. Fetch raw HTML
        try:
            raw_html = fetch_wikipedia_html(url_path)
        except Exception as e:
            print(f"  SKIP: Failed to fetch: {e}")
            continue

        # 2. Fetch PDF
        print("  Fetching PDF...")
        pdf_bytes = fetch_wikipedia_pdf(url_path)

        # 3. Convert to Markdown
        md_text = html_to_markdown(raw_html)

        # 4. Import to AIF JSON IR
        aif_json = markdown_to_aif(md_text)
        if not aif_json:
            print("  SKIP: AIF import failed")
            continue

        try:
            doc = json.loads(aif_json)
        except json.JSONDecodeError:
            print("  SKIP: Invalid JSON from import")
            continue

        # 5. Extract PDF text for text-based token counting
        pdf_text = pdf_to_text(pdf_bytes) if pdf_bytes else ""

        # 6. Count tokens for each format
        print("  Counting tokens...")

        html_tokens = count_tokens(client, raw_html)
        md_tokens = count_tokens(client, md_text)
        json_tokens = count_tokens(client, aif_json)

        # PDF tokens: try native PDF first, fall back to extracted text
        pdf_native_tokens = 0
        pdf_text_tokens = 0
        if pdf_bytes:
            try:
                pdf_native_tokens = count_tokens_pdf(client, pdf_bytes)
            except Exception as e:
                print(f"  Warning: PDF native token count failed: {e}")
            if pdf_text:
                pdf_text_tokens = count_tokens(client, pdf_text)

        # Calculate sizes in bytes
        html_bytes = len(raw_html.encode("utf-8"))
        md_bytes = len(md_text.encode("utf-8"))
        json_bytes = len(aif_json.encode("utf-8"))
        pdf_bytes_size = len(pdf_bytes) if pdf_bytes else 0
        pdf_text_bytes = len(pdf_text.encode("utf-8")) if pdf_text else 0

        savings_md = (1 - md_tokens / html_tokens) * 100
        savings_json = (1 - json_tokens / html_tokens) * 100
        savings_pdf_native = (1 - pdf_native_tokens / html_tokens) * 100 if pdf_native_tokens else 0
        savings_pdf_text = (1 - pdf_text_tokens / html_tokens) * 100 if pdf_text_tokens else 0

        results.append(
            {
                "article": title,
                "html_tokens": html_tokens,
                "md_tokens": md_tokens,
                "aif_json_tokens": json_tokens,
                "pdf_native_tokens": pdf_native_tokens,
                "pdf_text_tokens": pdf_text_tokens,
                "html_bytes": html_bytes,
                "md_bytes": md_bytes,
                "aif_json_bytes": json_bytes,
                "pdf_bytes": pdf_bytes_size,
                "pdf_text_bytes": pdf_text_bytes,
                "savings_md_pct": savings_md,
                "savings_aif_pct": savings_json,
                "savings_pdf_native_pct": savings_pdf_native,
                "savings_pdf_text_pct": savings_pdf_text,
            }
        )

        print(
            f"  HTML:       {format_size(html_tokens):>8} tokens ({format_size(html_bytes):>8} bytes)"
        )
        if pdf_native_tokens:
            print(
                f"  PDF (file): {format_size(pdf_native_tokens):>8} tokens ({format_size(pdf_bytes_size):>8} bytes) → {savings_pdf_native:.1f}% vs HTML"
            )
        if pdf_text_tokens:
            print(
                f"  PDF (text): {format_size(pdf_text_tokens):>8} tokens ({format_size(pdf_text_bytes):>8} bytes) → {savings_pdf_text:.1f}% vs HTML"
            )
        print(
            f"  Markdown:   {format_size(md_tokens):>8} tokens ({format_size(md_bytes):>8} bytes) → {savings_md:.1f}% vs HTML"
        )
        print(
            f"  AIF JSON:   {format_size(json_tokens):>8} tokens ({format_size(json_bytes):>8} bytes) → {savings_json:.1f}% vs HTML"
        )
        print()

        # Rate limiting courtesy
        time.sleep(0.5)

    if not results:
        print("No results collected.")
        sys.exit(1)

    # ── Summary Table ──────────────────────────────────────────────────────

    print()
    print("=" * 100)
    print("SUMMARY")
    print("=" * 100)
    print()
    header = (
        f"{'Article':<30} {'Raw HTML':>10} {'PDF(file)':>10} {'PDF(text)':>10}"
        f" {'Markdown':>10} {'AIF JSON':>10} {'PDF Δ':>8} {'MD Δ':>8} {'AIF Δ':>8}"
    )
    print(header)
    print("─" * len(header))

    total_html = total_md = total_aif = total_pdf_native = total_pdf_text = 0

    for r in results:
        total_html += r["html_tokens"]
        total_md += r["md_tokens"]
        total_aif += r["aif_json_tokens"]
        total_pdf_native += r["pdf_native_tokens"]
        total_pdf_text += r["pdf_text_tokens"]

        pdf_n = format_size(r["pdf_native_tokens"]) if r["pdf_native_tokens"] else "N/A"
        pdf_t = format_size(r["pdf_text_tokens"]) if r["pdf_text_tokens"] else "N/A"
        pdf_delta = f"{r['savings_pdf_native_pct']:>+7.1f}%" if r["pdf_native_tokens"] else "    N/A"

        print(
            f"{r['article']:<30} "
            f"{format_size(r['html_tokens']):>10} "
            f"{pdf_n:>10} "
            f"{pdf_t:>10} "
            f"{format_size(r['md_tokens']):>10} "
            f"{format_size(r['aif_json_tokens']):>10} "
            f"{pdf_delta} "
            f"{r['savings_md_pct']:>+7.1f}% "
            f"{r['savings_aif_pct']:>+7.1f}%"
        )

    print("─" * len(header))

    avg_md_savings = (1 - total_md / total_html) * 100
    avg_aif_savings = (1 - total_aif / total_html) * 100
    avg_pdf_native_savings = (1 - total_pdf_native / total_html) * 100 if total_pdf_native else 0
    avg_pdf_text_savings = (1 - total_pdf_text / total_html) * 100 if total_pdf_text else 0

    pdf_total_n = format_size(total_pdf_native) if total_pdf_native else "N/A"
    pdf_total_t = format_size(total_pdf_text) if total_pdf_text else "N/A"
    pdf_total_delta = f"{avg_pdf_native_savings:>+7.1f}%" if total_pdf_native else "    N/A"

    print(
        f"{'TOTAL':<30} "
        f"{format_size(total_html):>10} "
        f"{pdf_total_n:>10} "
        f"{pdf_total_t:>10} "
        f"{format_size(total_md):>10} "
        f"{format_size(total_aif):>10} "
        f"{pdf_total_delta} "
        f"{avg_md_savings:>+7.1f}% "
        f"{avg_aif_savings:>+7.1f}%"
    )
    print()
    print(f"Total HTML tokens:       {total_html:>10,}")
    if total_pdf_native:
        print(f"Total PDF (file) tokens: {total_pdf_native:>10,} ({avg_pdf_native_savings:+.1f}%)")
    if total_pdf_text:
        print(f"Total PDF (text) tokens: {total_pdf_text:>10,} ({avg_pdf_text_savings:+.1f}%)")
    print(f"Total Markdown tokens:   {total_md:>10,} ({avg_md_savings:+.1f}%)")
    print(f"Total AIF JSON tokens:   {total_aif:>10,} ({avg_aif_savings:+.1f}%)")
    print()

    # ── Also test .aif source format with the built-in example ─────────

    print("=" * 80)
    print("AIF SOURCE FORMAT vs HTML (using built-in example)")
    print("=" * 80)
    print()

    example_aif = PROJECT_ROOT / "examples" / "wiki_article.aif"
    example_html = PROJECT_ROOT / "examples" / "wiki_article.html"
    example_md = PROJECT_ROOT / "examples" / "wiki_article_output.md"
    example_lml = PROJECT_ROOT / "examples" / "wiki_article.lml"

    formats = {}
    for label, path in [
        ("AIF source", example_aif),
        ("HTML output", example_html),
        ("Markdown output", example_md),
        ("LML output", example_lml),
    ]:
        if path.exists():
            text = path.read_text()
            tokens = count_tokens(client, text)
            size = len(text.encode("utf-8"))
            formats[label] = {"tokens": tokens, "bytes": size}
            print(f"  {label:<20} {tokens:>6} tokens  {format_size(size):>8} bytes")
            time.sleep(0.3)

    if "HTML output" in formats and "AIF source" in formats:
        html_t = formats["HTML output"]["tokens"]
        aif_t = formats["AIF source"]["tokens"]
        print()
        print(
            f"  AIF source is {(1 - aif_t/html_t)*100:.1f}% fewer tokens than HTML output"
        )
        if "LML output" in formats:
            lml_t = formats["LML output"]["tokens"]
            print(
                f"  LML output is {(1 - lml_t/html_t)*100:.1f}% fewer tokens than HTML output"
            )
    print()

    # ── Save results as JSON ───────────────────────────────────────────────

    output_path = PROJECT_ROOT / "benchmarks" / "results.json"
    with open(output_path, "w") as f:
        json.dump(
            {
                "model": MODEL,
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "articles": results,
                "example_formats": {
                    k: v for k, v in formats.items()
                },
            },
            f,
            indent=2,
        )
    print(f"Results saved to {output_path}")


if __name__ == "__main__":
    main()
