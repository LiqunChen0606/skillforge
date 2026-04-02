#!/usr/bin/env python3
"""
AIF Document Token Efficiency Benchmark

Downloads real Wikipedia articles and compares token costs across raw formats
(HTML, PDF, Markdown) versus AIF output formats (JSON, HTML, Markdown roundtrip,
LML modes). Uses Claude's token counting API for accurate measurements.
"""

import base64
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import anthropic
import fitz  # PyMuPDF
import html2text

# ── Configuration ─────────────────────────────────────────────────────────

MODEL = "claude-opus-4-6"

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

# Raw input formats (baseline) + AIF output formats
# (key, label, type)  type: "raw" = direct content, "aif" = compiled from JSON IR
FORMATS = [
    ("raw_html",         "Raw HTML",         "raw"),
    ("clean_html_text",  "Cleaned HTML",     "raw"),
    ("raw_pdf",          "Raw PDF (file)",    "raw"),
    ("raw_pdf_text",     "Raw PDF (text)",    "raw"),
    ("raw_md",           "Raw Markdown",      "raw"),
    ("aif_json",         "AIF JSON IR",       "aif_import"),
    ("aif_html",         "AIF HTML",          "aif_compile", "html"),
    ("aif_md",           "AIF Markdown (RT)", "aif_compile", "markdown"),
    ("aif_lml",          "AIF LML",           "aif_compile", "lml"),
    ("aif_lml_compact",  "AIF LML Compact",   "aif_compile", "lml-compact"),
    ("aif_lml_conserv",  "AIF LML Conserv.",   "aif_compile", "lml-conservative"),
    ("aif_lml_moderate", "AIF LML Moderate",  "aif_compile", "lml-moderate"),
    ("aif_lml_aggress",  "AIF LML Aggress.",  "aif_compile", "lml-aggressive"),
]

# ── Helpers ───────────────────────────────────────────────────────────────


def fetch_wikipedia_html(title: str) -> str:
    import urllib.request
    url = f"https://en.wikipedia.org/api/rest_v1/page/html/{title}"
    req = urllib.request.Request(url, headers={"User-Agent": "AIF-Benchmark/1.0"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        return resp.read().decode("utf-8")


def fetch_wikipedia_pdf(title: str) -> bytes | None:
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
    doc = fitz.open(stream=pdf_bytes, filetype="pdf")
    text = ""
    for page in doc:
        text += page.get_text()
    doc.close()
    return text


def html_to_markdown(html: str) -> str:
    h = html2text.HTML2Text()
    h.body_width = 0
    h.ignore_links = False
    h.ignore_images = False
    h.ignore_tables = False
    h.protect_links = True
    h.unicode_snob = True
    return h.handle(html)


def clean_html_to_text(html: str) -> str:
    """Strip HTML to clean text content — analogous to PDF text extraction.

    Removes: scripts, styles, nav, header, footer, sidebars, infoboxes,
    reference lists, edit links, and all HTML tags. Preserves paragraph
    structure as double-newlines. This gives a fair comparison baseline:
    "what if you just extracted the text from HTML like you do from PDF?"
    """
    import re
    # Remove script and style blocks entirely
    text = re.sub(r'<script[^>]*>.*?</script>', '', html, flags=re.DOTALL | re.IGNORECASE)
    text = re.sub(r'<style[^>]*>.*?</style>', '', html, flags=re.DOTALL | re.IGNORECASE)
    # Remove nav, header, footer, sidebar elements
    for tag in ['nav', 'header', 'footer', 'aside']:
        text = re.sub(rf'<{tag}[^>]*>.*?</{tag}>', '', text, flags=re.DOTALL | re.IGNORECASE)
    # Remove Wikipedia-specific chrome: infoboxes, reference lists, edit links
    text = re.sub(r'<table[^>]*class="[^"]*infobox[^"]*"[^>]*>.*?</table>', '', text, flags=re.DOTALL | re.IGNORECASE)
    text = re.sub(r'<div[^>]*class="[^"]*reflist[^"]*"[^>]*>.*?</div>', '', text, flags=re.DOTALL | re.IGNORECASE)
    text = re.sub(r'<span[^>]*class="[^"]*mw-editsection[^"]*"[^>]*>.*?</span>', '', text, flags=re.DOTALL | re.IGNORECASE)
    # Replace block elements with newlines
    text = re.sub(r'<(?:p|div|br|h[1-6]|li|tr|td|th)[^>]*/?>', '\n', text, flags=re.IGNORECASE)
    # Remove all remaining tags
    text = re.sub(r'<[^>]+>', '', text)
    # Decode HTML entities
    import html as html_mod
    text = html_mod.unescape(text)
    # Collapse whitespace, preserve paragraph breaks
    text = re.sub(r'[ \t]+', ' ', text)
    text = re.sub(r'\n[ \t]*\n', '\n\n', text)
    text = re.sub(r'\n{3,}', '\n\n', text)
    return text.strip()


def count_tokens(client: anthropic.Anthropic, text: str) -> int:
    result = client.messages.count_tokens(
        model=MODEL,
        messages=[{"role": "user", "content": text}],
    )
    return result.input_tokens


def count_tokens_pdf(client: anthropic.Anthropic, pdf_bytes: bytes) -> int:
    b64 = base64.standard_b64encode(pdf_bytes).decode("ascii")
    result = client.messages.count_tokens(
        model=MODEL,
        messages=[{
            "role": "user",
            "content": [{
                "type": "document",
                "source": {
                    "type": "base64",
                    "media_type": "application/pdf",
                    "data": b64,
                },
            }],
        }],
    )
    return result.input_tokens


def aif_import_md(md_text: str) -> str:
    """Import Markdown → AIF JSON IR via CLI."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False) as f:
        f.write(md_text)
        md_path = f.name
    try:
        result = subprocess.run(
            [str(AIF_CLI), "import", md_path],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            print(f"  Warning: AIF import failed: {result.stderr}", file=sys.stderr)
            return ""
        return result.stdout
    finally:
        os.unlink(md_path)


def aif_compile_json(json_ir: str, fmt: str) -> str:
    """Compile AIF JSON IR → output format via CLI."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        f.write(json_ir)
        json_path = f.name
    try:
        result = subprocess.run(
            [str(AIF_CLI), "compile", "--input-format", "json", "-f", fmt, json_path],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            print(f"  Warning: compile to {fmt} failed: {result.stderr}", file=sys.stderr)
            return ""
        return result.stdout
    finally:
        os.unlink(json_path)


def format_size(n: int) -> str:
    if n >= 1_000_000:
        return f"{n/1_000_000:.1f}M"
    if n >= 1_000:
        return f"{n/1_000:.1f}K"
    return str(n)


def pct(base: int, val: int) -> float:
    if base <= 0:
        return 0.0
    return (1 - val / base) * 100


# ── HTML Report ───────────────────────────────────────────────────────────


def generate_html_report(results, totals, article_count, output_path):
    import html as html_mod

    fmt_labels = [f[1] for f in FORMATS]
    fmt_keys = [f[0] for f in FORMATS]
    html_total = totals["raw_html_tokens"]

    # Find best AIF format
    best_key, best_tokens = "", float("inf")
    for f in FORMATS:
        k = f[0]
        if k.startswith("aif_") and totals[f"{k}_tokens"] > 0:
            if totals[f"{k}_tokens"] < best_tokens:
                best_tokens = totals[f"{k}_tokens"]
                best_key = k
    best_label = next((f[1] for f in FORMATS if f[0] == best_key), "")
    best_save = pct(html_total, best_tokens)

    # Build article detail rows
    article_rows = ""
    for r in results:
        article_rows += f"<tr><td class='article-name'>{html_mod.escape(r['article'])}</td>"
        for f in FORMATS:
            k = f[0]
            tokens = r.get(f"{k}_tokens", 0)
            save = r.get(f"{k}_save_pct", 0.0)
            cls = ""
            if k != "raw_html" and save > 5:
                cls = " class='positive'"
            elif k != "raw_html" and save < -5:
                cls = " class='negative'"
            save_str = f"{save:+.1f}%" if k != "raw_html" else "base"
            tok_str = format_size(tokens) if tokens > 0 else "N/A"
            article_rows += f"<td{cls}>{tok_str}<br><small>{save_str}</small></td>"
        article_rows += "</tr>\n"

    # Total row
    total_row = "<tr class='total-row'><td class='article-name'><strong>TOTAL</strong></td>"
    for f in FORMATS:
        k = f[0]
        t = totals[f"{k}_tokens"]
        save = pct(html_total, t) if k != "raw_html" else 0.0
        save_str = f"{save:+.1f}%" if k != "raw_html" else "base"
        tok_str = format_size(t) if t > 0 else "N/A"
        cls = ""
        if k != "raw_html" and save > 5:
            cls = " class='positive'"
        total_row += f"<td{cls}>{tok_str}<br><small>{save_str}</small></td>"
    total_row += "</tr>"

    header = "<th>Article</th>" + "".join(f"<th>{html_mod.escape(l)}</th>" for l in fmt_labels)

    # Bar chart data — savings vs Raw HTML
    bar_data = []
    for f in FORMATS:
        k = f[0]
        if k == "raw_html":
            continue
        t = totals[f"{k}_tokens"]
        if t > 0:
            bar_data.append((f[1], pct(html_total, t)))

    max_abs = max((abs(v) for _, v in bar_data), default=1)
    bars_html = ""
    for label, val in bar_data:
        h = max(2, abs(val) / max_abs * 180)
        cls = "bar-pos" if val >= 0 else "bar-neg"
        bars_html += f"""  <div class="bar-wrapper">
    <div class="bar-value">{val:+.1f}%</div>
    <div class="bar {cls}" style="height:{h:.0f}px"></div>
    <div class="bar-label">{html_mod.escape(label)}</div>
  </div>\n"""

    # Summary table
    summary_rows = ""
    for f in FORMATS:
        k, label = f[0], f[1]
        t = totals[f"{k}_tokens"]
        b = totals[f"{k}_bytes"]
        save = pct(html_total, t) if k != "raw_html" else 0.0
        save_str = f"{save:+.1f}%" if k != "raw_html" else "baseline"
        tok_str = f"{t:,}" if t > 0 else "N/A"
        byte_str = f"{b:,}" if b > 0 else "N/A"
        summary_rows += f"<tr><td style='text-align:left'>{html_mod.escape(label)}</td><td>{tok_str}</td><td>{byte_str}</td><td>{save_str}</td></tr>\n"

    html_content = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>AIF Document Token Benchmark Report</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1600px; margin: 2rem auto; padding: 0 1rem; background: #f8f9fa; color: #1a1a2e; }}
  h1 {{ color: #16213e; border-bottom: 3px solid #0f3460; padding-bottom: 0.5rem; }}
  h2 {{ color: #16213e; margin-top: 2rem; }}
  .meta {{ color: #666; font-size: 0.9rem; margin-bottom: 2rem; }}
  .winner {{ background: linear-gradient(135deg, #d4edda, #c3e6cb); border: 1px solid #28a745; border-radius: 8px; padding: 1rem 1.5rem; margin: 1rem 0; font-size: 1.1rem; }}
  .winner strong {{ color: #155724; }}
  table {{ border-collapse: collapse; width: 100%; margin: 1rem 0; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  th {{ background: #16213e; color: white; padding: 10px 8px; font-size: 0.82rem; text-align: center; }}
  td {{ padding: 8px; text-align: center; border-bottom: 1px solid #eee; font-size: 0.82rem; }}
  td.article-name {{ text-align: left; font-weight: 600; white-space: nowrap; }}
  .positive {{ background: #d4edda; }}
  .negative {{ background: #f8d7da; }}
  .total-row td {{ background: #e8eaf6; font-weight: bold; border-top: 2px solid #16213e; }}
  .bar-chart {{ display: flex; align-items: flex-end; gap: 6px; height: 240px; margin: 1rem 0; padding: 1rem; background: white; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
  .bar-wrapper {{ display: flex; flex-direction: column; align-items: center; flex: 1; }}
  .bar {{ width: 100%; border-radius: 4px 4px 0 0; min-height: 2px; }}
  .bar-label {{ font-size: 0.65rem; margin-top: 4px; text-align: center; word-break: break-all; }}
  .bar-value {{ font-size: 0.7rem; font-weight: bold; margin-bottom: 2px; }}
  .bar-pos {{ background: linear-gradient(180deg, #28a745, #20c997); }}
  .bar-neg {{ background: linear-gradient(180deg, #dc3545, #e74c6c); }}
  .legend {{ display: flex; gap: 2rem; margin: 1rem 0; font-size: 0.85rem; }}
  .legend-item {{ display: flex; align-items: center; gap: 0.4rem; }}
  .legend-swatch {{ width: 16px; height: 16px; border-radius: 3px; }}
  small {{ color: #666; }}
  .note {{ background: #fff3cd; border: 1px solid #ffc107; border-radius: 6px; padding: 0.8rem 1rem; margin: 1rem 0; font-size: 0.9rem; }}
  .timestamp {{ text-align: right; color: #999; font-size: 0.8rem; margin-top: 2rem; }}
</style>
</head>
<body>
<h1>AIF Document Token Benchmark Report</h1>
<p class="meta">Model: {MODEL} &bull; Articles: {article_count} Wikipedia articles &bull; Formats: {len(FORMATS)}</p>

<div class="winner">
  <strong>Most Efficient AIF Format: {html_mod.escape(best_label)}</strong> &mdash; {best_save:+.1f}% tokens vs Raw HTML ({format_size(best_tokens)} total)
</div>

<div class="note">
  <strong>What this measures:</strong> Real Wikipedia articles fetched as raw HTML and PDF, converted to Markdown,
  then imported into AIF and compiled to all output formats. Token counts via Claude {MODEL} token counting API.
  Baseline is Raw HTML.
</div>

<h2>Token Savings vs Raw HTML Baseline</h2>
<div class="legend">
  <div class="legend-item"><div class="legend-swatch" style="background:#28a745"></div> Savings (fewer tokens)</div>
  <div class="legend-item"><div class="legend-swatch" style="background:#dc3545"></div> Overhead (more tokens)</div>
</div>
<div class="bar-chart">
{bars_html}</div>

<h2>Per-Article Comparison</h2>
<table>
<thead><tr>{header}</tr></thead>
<tbody>
{article_rows}
{total_row}
</tbody>
</table>

<h2>Summary</h2>
<table>
<thead><tr><th>Format</th><th>Total Tokens</th><th>Total Bytes</th><th>Token Savings vs HTML</th></tr></thead>
<tbody>
{summary_rows}</tbody>
</table>

<p class="timestamp">Generated: {time.strftime("%Y-%m-%d %H:%M:%S UTC", time.gmtime())}</p>
</body>
</html>"""

    with open(output_path, "w") as f:
        f.write(html_content)


# ── Main ──────────────────────────────────────────────────────────────────


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

    try:
        client = anthropic.Anthropic(api_key=api_key)
        client.messages.count_tokens(
            model=MODEL, messages=[{"role": "user", "content": "test"}]
        )
    except anthropic.AuthenticationError:
        api_key = api_key[:-1]
        client = anthropic.Anthropic(api_key=api_key)

    print("=" * 100)
    print("AIF Document Token Efficiency Benchmark")
    print(f"Model: {MODEL}")
    print(f"Source: {len(ARTICLES)} Wikipedia articles")
    print(f"Formats: Raw HTML, Raw PDF, Raw Markdown → AIF JSON, HTML, Markdown(RT), LML (5 modes)")
    print("=" * 100)
    print()

    results = []

    for title, url_path in ARTICLES.items():
        print(f"── {title} ", "─" * max(1, 70 - len(title)))

        # 1. Fetch raw HTML
        try:
            raw_html = fetch_wikipedia_html(url_path)
        except Exception as e:
            print(f"  SKIP: Failed to fetch: {e}")
            continue

        # 2. Fetch PDF
        print("  Fetching PDF...")
        pdf_bytes = fetch_wikipedia_pdf(url_path)

        # 3. Convert HTML → Markdown
        md_text = html_to_markdown(raw_html)

        # 4. Import Markdown → AIF JSON IR
        print("  Importing to AIF...")
        aif_json = aif_import_md(md_text)
        if not aif_json:
            print("  SKIP: AIF import failed")
            continue

        # 5. Compile JSON IR to all AIF output formats
        aif_outputs = {}
        for f in FORMATS:
            if len(f) == 4 and f[2] == "aif_compile":
                fmt = f[3]
                aif_outputs[f[0]] = aif_compile_json(aif_json, fmt)

        # 6. Extract PDF text
        pdf_text = pdf_to_text(pdf_bytes) if pdf_bytes else ""

        # 7. Count tokens for everything
        print("  Counting tokens...")

        r = {"article": title}

        # Raw HTML
        html_tokens = count_tokens(client, raw_html)
        r["raw_html_tokens"] = html_tokens
        r["raw_html_bytes"] = len(raw_html.encode("utf-8"))
        r["raw_html_save_pct"] = 0.0

        # Cleaned HTML (text extracted, chrome stripped — fair baseline)
        clean_text = clean_html_to_text(raw_html)
        clean_tok = count_tokens(client, clean_text)
        r["clean_html_text_tokens"] = clean_tok
        r["clean_html_text_bytes"] = len(clean_text.encode("utf-8"))
        r["clean_html_text_save_pct"] = pct(html_tokens, clean_tok)

        # Raw PDF (native file)
        if pdf_bytes:
            try:
                pdf_tok = count_tokens_pdf(client, pdf_bytes)
                r["raw_pdf_tokens"] = pdf_tok
                r["raw_pdf_bytes"] = len(pdf_bytes)
                r["raw_pdf_save_pct"] = pct(html_tokens, pdf_tok)
            except Exception as e:
                print(f"  Warning: PDF native token count failed: {e}")
                r["raw_pdf_tokens"] = 0
                r["raw_pdf_bytes"] = 0
                r["raw_pdf_save_pct"] = 0.0
        else:
            r["raw_pdf_tokens"] = 0
            r["raw_pdf_bytes"] = 0
            r["raw_pdf_save_pct"] = 0.0

        # Raw PDF (text extraction)
        if pdf_text:
            pdf_text_tok = count_tokens(client, pdf_text)
            r["raw_pdf_text_tokens"] = pdf_text_tok
            r["raw_pdf_text_bytes"] = len(pdf_text.encode("utf-8"))
            r["raw_pdf_text_save_pct"] = pct(html_tokens, pdf_text_tok)
        else:
            r["raw_pdf_text_tokens"] = 0
            r["raw_pdf_text_bytes"] = 0
            r["raw_pdf_text_save_pct"] = 0.0

        # Raw Markdown
        md_tokens = count_tokens(client, md_text)
        r["raw_md_tokens"] = md_tokens
        r["raw_md_bytes"] = len(md_text.encode("utf-8"))
        r["raw_md_save_pct"] = pct(html_tokens, md_tokens)

        # AIF JSON IR
        json_tokens = count_tokens(client, aif_json)
        r["aif_json_tokens"] = json_tokens
        r["aif_json_bytes"] = len(aif_json.encode("utf-8"))
        r["aif_json_save_pct"] = pct(html_tokens, json_tokens)

        # AIF compiled formats
        for f in FORMATS:
            if len(f) == 4 and f[2] == "aif_compile":
                k = f[0]
                text = aif_outputs.get(k, "")
                if text:
                    tok = count_tokens(client, text)
                    r[f"{k}_tokens"] = tok
                    r[f"{k}_bytes"] = len(text.encode("utf-8"))
                    r[f"{k}_save_pct"] = pct(html_tokens, tok)
                else:
                    r[f"{k}_tokens"] = 0
                    r[f"{k}_bytes"] = 0
                    r[f"{k}_save_pct"] = 0.0

        results.append(r)

        # Print per-article summary
        for f in FORMATS:
            k, label = f[0], f[1]
            tokens = r.get(f"{k}_tokens", 0)
            nbytes = r.get(f"{k}_bytes", 0)
            save = r.get(f"{k}_save_pct", 0.0)
            if tokens == 0:
                print(f"  {label:<20} {'N/A':>8}")
                continue
            save_str = f"  {save:>+6.1f}%" if k != "raw_html" else "  (base)"
            print(f"  {label:<20} {format_size(tokens):>8} tokens  ({format_size(nbytes):>8} bytes){save_str}")
        print()

        time.sleep(0.3)

    if not results:
        print("No results.")
        sys.exit(1)

    # ── Summary Table ──────────────────────────────────────────────────────

    print()
    print("=" * 140)
    print("SUMMARY — Token Counts and Savings vs Raw HTML")
    print("=" * 140)
    print()

    hdr_parts = [f"{'Article':<30}"]
    for f in FORMATS:
        short = f[1][:16]
        hdr_parts.append(f"{short:>12} {'save%':>7}")
    hdr = " ".join(hdr_parts)
    print(hdr)
    print("─" * len(hdr))

    # Accumulate totals
    totals = {}
    for f in FORMATS:
        k = f[0]
        totals[f"{k}_tokens"] = 0
        totals[f"{k}_bytes"] = 0

    for r in results:
        for f in FORMATS:
            k = f[0]
            totals[f"{k}_tokens"] += r.get(f"{k}_tokens", 0)
            totals[f"{k}_bytes"] += r.get(f"{k}_bytes", 0)

        row_parts = [f"{r['article']:<30}"]
        for f in FORMATS:
            k = f[0]
            tokens = r.get(f"{k}_tokens", 0)
            save = r.get(f"{k}_save_pct", 0.0)
            if tokens == 0:
                row_parts.append(f"{'N/A':>12} {'':>7}")
            else:
                row_parts.append(f"{format_size(tokens):>12} {save:>+6.1f}%")
        print(" ".join(row_parts))

    print("─" * len(hdr))

    html_total = totals["raw_html_tokens"]
    row_parts = [f"{'TOTAL':<30}"]
    for f in FORMATS:
        k = f[0]
        t = totals[f"{k}_tokens"]
        save = pct(html_total, t) if k != "raw_html" else 0.0
        if t == 0:
            row_parts.append(f"{'N/A':>12} {'':>7}")
        else:
            row_parts.append(f"{format_size(t):>12} {save:>+6.1f}%")
    print(" ".join(row_parts))
    print()

    # ── Byte-level summary ──
    print("Byte-level summary:")
    for f in FORMATS:
        k, label = f[0], f[1]
        total_b = totals[f"{k}_bytes"]
        if total_b == 0:
            print(f"  {label:<20} {'N/A':>12}")
        elif k == "raw_html":
            print(f"  {label:<20} {total_b:>12,} bytes")
        else:
            save = pct(totals["raw_html_bytes"], total_b)
            print(f"  {label:<20} {total_b:>12,} bytes  ({save:>+.1f}%)")
    print()

    # ── Generate HTML Report ──
    html_path = PROJECT_ROOT / "benchmarks" / "results.html"
    generate_html_report(results, totals, len(results), html_path)
    print(f"HTML report saved to {html_path}")

    # ── Save JSON results ──
    output_path = PROJECT_ROOT / "benchmarks" / "results.json"
    totals_out = {}
    for f in FORMATS:
        k = f[0]
        totals_out[f"{k}_tokens"] = totals[f"{k}_tokens"]
        totals_out[f"{k}_bytes"] = totals[f"{k}_bytes"]
        if k != "raw_html":
            totals_out[f"savings_{k}_pct"] = pct(html_total, totals[f"{k}_tokens"])

    with open(output_path, "w") as f:
        json.dump({
            "model": MODEL,
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "formats": [f[1] for f in FORMATS],
            "articles": results,
            "totals": totals_out,
        }, f, indent=2)
    print(f"Results saved to {output_path}")


if __name__ == "__main__":
    main()
