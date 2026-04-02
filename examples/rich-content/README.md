# AIF Rich Content: Tables, Figures, Media Metadata

This example demonstrates how AIF handles content **beyond raw text** — structured tables, SVG figures with metadata, audio/video with duration and dimensions, cross-references between data and claims, and document-level metadata.

## What AIF Preserves That Markdown Cannot

| Content Type | Markdown | AIF | What's Different |
|-------------|----------|-----|-----------------|
| Tables | GFM pipes, no captions | `@table` with caption, ID, `refs` | Captioned, addressable, cross-linked |
| Figures | `![alt](src)` | `@figure[width, height, mime, alt, refs]` | Dimensions, MIME type, evidence links |
| Audio | No native support | `@audio[duration, mime, alt]` | Duration, MIME, accessibility |
| Video | No native support | `@video[width, height, duration, mime, poster]` | Full media metadata, poster frame |
| Metadata | YAML frontmatter (informal) | `#key: value` (typed, preserved in IR) | Survives all format roundtrips |
| Cross-refs | None | `refs=id1,id2` on any block | Machine-verifiable evidence chains |

## The Example Document

[climate_data.aif](climate_data.aif) demonstrates all of these:

```
Document metadata:
  #title, #author, #summary, #date, #source, #license

Semantic blocks:
  @definition  — What a temperature anomaly is
  @table       — 5 years of data with caption and refs
  @figure x2   — SVG charts with width, height, mime, alt, refs
  @audio       — Podcast with duration (1847.5s)
  @video       — Timelapse with dimensions, duration, poster frame
  @claim x2    — Scientific claims linked to evidence via refs
  @evidence x2 — Supporting data linked to tables/figures
  @conclusion  — Summary linked to evidence chain
```

## Try It

### Compile to different formats

```bash
# HTML — tables get <thead>/<tbody>, figures get <figure> with metadata attributes
aif compile examples/rich-content/climate_data.aif --format html

# LML Aggressive — tables as pipe-delimited, figures as @figure: with abbreviated metadata
aif compile examples/rich-content/climate_data.aif --format lml-aggressive

# JSON IR — full typed AST with all metadata preserved
aif compile examples/rich-content/climate_data.aif --format json

# Markdown — tables as GFM, figures as ![alt](src), captions lost
aif compile examples/rich-content/climate_data.aif --format markdown
```

### Lint for structural issues

```bash
aif lint examples/rich-content/climate_data.aif
```

Expected output (all checks pass):
```
Document Lint: examples/rich-content/climate_data.aif
============================================================
  [+] ClaimsWithoutEvidence
  [+] BrokenReferences
  [+] BrokenEvidenceLinks      ← verifies refs=temp-table,e1 targets exist
  [+] OrphanedMedia
  [+] DuplicateIds
  [+] EmptySections
  [+] MissingMetadata
  [+] EmptyFootnotes
  [+] MalformedTables
------------------------------------------------------------
9 checks: 9 passed, 0 failed
```

### Dump the IR to see metadata preservation

```bash
aif dump-ir examples/rich-content/climate_data.aif | python3 -m json.tool | head -20
```

Output shows typed metadata:
```json
{
  "metadata": {
    "title": "Global Temperature Anomalies 2020-2024",
    "author": "AIF Documentation Team",
    "summary": "Demonstrating AIF's rich content handling...",
    "date": "2026-04-02",
    "source": "NASA GISS Surface Temperature Analysis",
    "license": "CC-BY-4.0"
  },
  "blocks": [...]
}
```

## How Each Content Type Works

### Tables

AIF tables have: an ID (for cross-referencing), a caption (preserved in HTML/LML, lost in Markdown), headers, and rows. The `refs` attribute links the table to related figures or claims.

```aif
@table[id=temp-table, refs=fig-chart]: Global Mean Temperature Anomalies (°C)
| Year | Annual Anomaly | 5-Year Mean | Rank |
| 2020 | +1.02 | +0.98 | 2nd |
| 2021 | +0.85 | +0.99 | 6th |
```

**HTML output:**
```html
<table id="temp-table">
  <caption>Global Mean Temperature Anomalies (°C)</caption>
  <thead><tr><th>Year</th><th>Annual Anomaly</th>...</tr></thead>
  <tbody><tr><td>2020</td><td>+1.02</td>...</tr></tbody>
</table>
```

**LML Aggressive output:**
```
@table: Global Mean Temperature Anomalies (°C)
| Year | Annual Anomaly | 5-Year Mean | Rank |
| --- | --- | --- | --- |
| 2020 | +1.02 | +0.98 | 2nd |
```

**Markdown output:** (caption lost — no GFM standard for table captions)
```markdown
| Year | Annual Anomaly | 5-Year Mean | Rank |
| --- | --- | --- | --- |
| 2020 | +1.02 | +0.98 | 2nd |
```

### Figures with SVG

AIF figures carry 6 optional metadata fields via `MediaMeta`:

```aif
@figure[id=fig-chart, src=temperature_trend.svg, alt=Temperature trend, width=800, height=400, mime=image/svg+xml, refs=temp-table]
Caption text here.
```

| Field | Purpose | Example |
|-------|---------|---------|
| `alt` | Accessibility text | "Temperature trend 2020-2024" |
| `width` | Display width in pixels | 800 |
| `height` | Display height in pixels | 400 |
| `mime` | MIME type | image/svg+xml |
| `src` | File path or URL | temperature_trend.svg |
| `refs` | Cross-reference to related blocks | temp-table |

All 6 fields survive roundtrip through HTML, LML, JSON, and binary formats.

### Audio and Video

Same `MediaMeta` structure with additional fields:

```aif
@video[id=video-viz, src=arctic_ice_timelapse.mp4, alt=Arctic ice timelapse, width=1920, height=1080, duration=120.0, mime=video/mp4, poster=arctic_ice_poster.jpg]
Caption for the video.
```

| Field | Audio | Video |
|-------|-------|-------|
| `alt` | Yes | Yes |
| `duration` | Yes (seconds) | Yes (seconds) |
| `mime` | Yes | Yes |
| `width` | No | Yes |
| `height` | No | Yes |
| `poster` | No | Yes (poster frame image) |

### Cross-References (Evidence Linkage)

The `refs` attribute creates machine-verifiable links between blocks:

```aif
@claim[id=c1, refs=temp-table,e1]
The 2020-2024 period is the warmest five-year period.

@evidence[id=e1, refs=temp-table]
All five years rank in the top 6 warmest years globally.
```

`aif lint` validates that all `refs` targets exist in the document. If you reference a nonexistent ID, the `BrokenEvidenceLinks` check fails:

```
[x] BrokenEvidenceLinks [ERROR]: refs attribute points to 'nonexistent' but no block with that ID exists
```

### Document Metadata

All `#key: value` lines become entries in `Document.metadata`, preserved across all format roundtrips:

```aif
#title: Global Temperature Anomalies 2020-2024
#author: AIF Documentation Team
#date: 2026-04-02
#source: NASA GISS Surface Temperature Analysis
#license: CC-BY-4.0
```

In JSON IR, these appear as a flat `metadata` object. In HTML, `title` becomes `<title>` and others become `<meta>` tags. The `MissingMetadata` lint check warns if `title` is absent.

## Format Comparison: What Survives Each Roundtrip

| Content | AIF → HTML → AIF | AIF → MD → AIF | AIF → JSON → AIF | AIF → Binary → AIF |
|---------|-------------------|-----------------|-------------------|---------------------|
| Table headers | Lossless | Lossless | Lossless | Lossless |
| Table caption | Lossless | **Lost** | Lossless | Lossless |
| Table `refs` | Lossless (AIF mode) | Lost | Lossless | Lossless |
| Figure `alt` | Lossless | Lossless | Lossless | Lossless |
| Figure `width/height` | Lossless | Lost | Lossless | Lossless |
| Figure `mime` | Lossless | Lost | Lossless | Lossless |
| Audio `duration` | Lossless | Lost | Lossless | Lossless |
| Video `poster` | Lossless | Lost | Lossless | Lossless |
| `@claim` type | Lossless | **Flattened** | Lossless | Lossless |
| `refs` links | Lossless (AIF mode) | Lost | Lossless | Lossless |
| Document metadata | Lossless | Partial | Lossless | Lossless |

**Key takeaway:** JSON and binary roundtrips are fully lossless. HTML roundtrip is lossless when AIF CSS classes are present. Markdown roundtrip loses captions, media metadata, semantic types, and cross-references — which is exactly why AIF exists.
