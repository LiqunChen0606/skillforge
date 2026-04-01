# PDF Support & Chunk Graph Design

## 1. Overview

Add PDF import (PDF → AST) and PDF export (AST → PDF) to the AIF toolchain, plus a chunk graph model for cross-document evidence linking and sub-document referencing. This enables AIF to serve as the canonical interchange between visual document formats (PDF) and semantic formats (HTML, LML, Markdown).

**Goals:**
- Import PDFs into the AIF AST with best-effort structural detection (headings, paragraphs, tables, lists, figures)
- Export AIF documents to tagged, accessible PDFs with semantic structure
- Chunk documents into addressable sub-document units for LLM context windows, RAG pipelines, and cross-document evidence linking

## 2. Rust PDF Library Selection

### Import: pdf_oxide (recommended)

| Criterion | pdf_oxide | pdf-extract | pdfium-render |
|-----------|-----------|-------------|---------------|
| Pure Rust | Yes | Yes | No (C++ PDFium) |
| License | MIT/Apache-2.0 | MIT | MIT/Apache-2.0 |
| Structured extraction | Yes (font metadata, bboxes, content classification) | No (plain text only) | Moderate (character-level positions) |
| Layout analysis | XY-Cut, Structure Tree, Geometric, Simple | None | None built-in |
| Heading detection | Font-size heuristics + structure tree | None | Manual |
| Table detection | Element partitioning | None | None |
| Maintenance | Active (v0.3.17, March 2026) | Moderate (v0.10.0, Oct 2025) | Active (v0.9.0, March 2026) |

**Decision:** Use `pdf_oxide` as primary. It provides structured extraction with font metadata, bounding boxes, and multiple reading-order strategies — all pure Rust, license-compatible.

**Fallback:** `pdfium-render` behind a feature flag (`pdf-pdfium`) for documents where pdf_oxide's heuristics are insufficient. Requires distributing PDFium shared library.

**Rejected:** `poppler-rs` (GPL viral license), `mupdf-rs` (AGPL viral license) — incompatible with MIT/Apache-2.0 dual licensing.

### Export: krilla (recommended)

| Criterion | krilla | printpdf | typst |
|-----------|--------|----------|-------|
| Pure Rust | Yes | Yes | Yes |
| License | MIT/Apache-2.0 | MIT | Apache-2.0 |
| Tagged PDF (accessibility) | Yes (PDF/UA-1) | No | Yes |
| PDF/A compliance | Yes (1/2/3/4) | No | Yes |
| Font subsetting | CFF + TTF + OpenType | Basic Unicode | Full |
| Layout engine | None (by design) | Basic page-breaking | Full typesetting |
| IR integration | Designed for it | General-purpose | Requires `World` trait |

**Decision:** Use `krilla` for PDF generation. It is explicitly designed for libraries with an intermediate representation — exactly the AIF use case. AIF handles layout; krilla encodes the PDF.

**Alternative path:** `typst` behind a feature flag (`pdf-typst`) for users who need LaTeX-quality typesetting. AIF would emit Typst markup, then compile to PDF via `typst` as a library.

## 3. New Crate: `aif-pdf`

### 3.1 Crate Structure

```
crates/aif-pdf/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API: import_pdf(), export_pdf()
│   ├── import/
│   │   ├── mod.rs           # Import pipeline orchestration
│   │   ├── extractor.rs     # pdf_oxide wrapper: raw extraction
│   │   ├── classifier.rs    # Layout analysis: headings, paragraphs, tables, lists, figures
│   │   ├── mapper.rs        # Classified elements → AIF Block/Inline types
│   │   └── confidence.rs    # Confidence scores for classification decisions
│   ├── export/
│   │   ├── mod.rs           # Export pipeline orchestration
│   │   ├── layout.rs        # Page layout engine: margins, text flow, page breaks
│   │   ├── renderer.rs      # AST → krilla drawing commands
│   │   ├── styles.rs        # Default styling: fonts, sizes, colors per block type
│   │   └── tagged.rs        # PDF structure tags from AIF semantic types
│   └── chunk/
│       ├── mod.rs           # Chunk graph public API
│       ├── graph.rs         # ChunkGraph data structure
│       ├── splitter.rs      # Document → chunks (strategies)
│       ├── linker.rs        # Cross-document evidence linking
│       └── ids.rs           # Deterministic chunk ID generation
└── tests/
    ├── import_tests.rs
    ├── export_tests.rs
    ├── chunk_tests.rs
    └── fixtures/
        ├── sample.pdf
        ├── table-heavy.pdf
        └── multi-column.pdf
```

### 3.2 Cargo.toml

```toml
[package]
name = "aif-pdf"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
aif-core = { workspace = true }
pdf_oxide = "0.3"
krilla = "0.5"
serde = { workspace = true }
serde_json = { workspace = true }
sha2 = "0.10"

[dev-dependencies]
aif-parser = { workspace = true }
pretty_assertions = "1"

[features]
default = ["import", "export"]
import = []          # PDF → AST
export = []          # AST → PDF
pdf-pdfium = []      # Alternative import backend via pdfium-render
pdf-typst = []       # Alternative export via typst
```

## 4. PDF Import Pipeline (PDF → AST)

### 4.1 Pipeline Stages

```
PDF file
  │
  ▼
┌─────────────────────┐
│  1. Extraction       │  pdf_oxide: pages → spans with font, size, bbox, reading order
│     (extractor.rs)   │
└─────────┬───────────┘
          │  Vec<PageContent>
          ▼
┌─────────────────────┐
│  2. Classification   │  Group spans into logical elements: heading, paragraph, table,
│     (classifier.rs)  │  list, figure, code block. Use font-size ratios, whitespace
└─────────┬───────────┘  gaps, alignment patterns, indentation.
          │  Vec<ClassifiedElement>
          ▼
┌─────────────────────┐
│  3. Mapping          │  ClassifiedElement → AIF Block/Inline types
│     (mapper.rs)      │  Heading → Section, Body text → Paragraph, etc.
└─────────┬───────────┘
          │  Document
          ▼
┌─────────────────────┐
│  4. Confidence       │  Annotate blocks with confidence metadata:
│     (confidence.rs)  │  attrs.pairs["import_confidence"] = "0.85"
└─────────┴───────────┘
```

### 4.2 Classification Heuristics

| PDF Feature | Detection Method | AIF Block Type |
|-------------|-----------------|----------------|
| Large font, bold | Font size > 1.3× body size | `Section` (heading) |
| Regular body text | Default font, normal size | `Paragraph` |
| Aligned columns + grid | Bbox alignment, repeated row patterns | `Table` |
| Indented lines with bullets/numbers | Leading indent + bullet/number prefix | `List` (ordered/unordered) |
| Embedded image | Image XObject in content stream | `Figure` |
| Monospaced font | Font name contains "Mono"/"Courier"/"Consolas" | `CodeBlock` |
| Indented block, italic | Offset text with style change | `BlockQuote` |
| Horizontal rule | Thin rect across page width | `ThematicBreak` |

### 4.3 Confidence Scoring

Every imported block gets a confidence score in `attrs.pairs`:

```rust
pub struct ImportConfidence {
    pub overall: f32,         // 0.0–1.0
    pub classification: f32,  // How confident is the block type?
    pub boundary: f32,        // How confident is the block boundary?
    pub content: f32,         // How confident is the text extraction?
}
```

Encoded as: `attrs.pairs["import_confidence"] = "0.85"` (overall score for simplicity; full breakdown available via `attrs.pairs["import_confidence_detail"] = "{...}"` JSON).

### 4.4 Public API

```rust
/// Import a PDF file into an AIF Document.
pub fn import_pdf(pdf_bytes: &[u8]) -> Result<ImportResult, PdfImportError> { ... }

/// Import result with document and diagnostics.
pub struct ImportResult {
    pub document: Document,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub page_count: usize,
    pub avg_confidence: f32,
}

pub struct ImportDiagnostic {
    pub page: usize,
    pub kind: DiagnosticKind,    // LowConfidence, UnrecognizedElement, SkippedContent
    pub message: String,
    pub confidence: f32,
}
```

## 5. PDF Export Pipeline (AST → PDF)

### 5.1 Pipeline Stages

```
Document (AST)
  │
  ▼
┌─────────────────────┐
│  1. Layout           │  Walk AST, compute bounding boxes, handle page breaks.
│     (layout.rs)      │  Produces LayoutTree with positioned elements.
└─────────┬───────────┘
          │  LayoutTree
          ▼
┌─────────────────────┐
│  2. Styling          │  Apply default styles per block type:
│     (styles.rs)      │  Section → 18pt bold, Paragraph → 12pt, Code → 10pt mono
└─────────┬───────────┘
          │  StyledLayoutTree
          ▼
┌─────────────────────┐
│  3. Rendering        │  Walk StyledLayoutTree, emit krilla drawing commands:
│     (renderer.rs)    │  text(), image(), rect(), etc.
└─────────┬───────────┘
          │  krilla::Document
          ▼
┌─────────────────────┐
│  4. Tagging          │  Map AIF semantic types to PDF structure tags:
│     (tagged.rs)      │  Section → /Sect, Paragraph → /P, Table → /Table, etc.
└─────────┴───────────┘  Produces tagged, accessible PDF (PDF/UA-1).
```

### 5.2 AIF → PDF Structure Tag Mapping

| AIF Block | PDF Structure Tag | Notes |
|-----------|-------------------|-------|
| Section | `/Sect` + `/H1`–`/H6` | Nesting depth → heading level |
| Paragraph | `/P` | |
| SemanticBlock | `/Div` with `/BlockQuote` or role | Custom role attribute |
| Callout | `/Note` | |
| Table | `/Table` + `/TR` + `/TH` + `/TD` | Full table structure |
| Figure | `/Figure` with alt text | |
| CodeBlock | `/Code` | |
| BlockQuote | `/BlockQuote` | |
| List | `/L` + `/LI` + `/Lbl` + `/LBody` | Ordered/unordered |
| SkillBlock | `/Div` with custom role | |

### 5.3 Public API

```rust
/// Export an AIF Document to PDF bytes.
pub fn export_pdf(doc: &Document) -> Result<Vec<u8>, PdfExportError> { ... }

/// Export with custom styling options.
pub fn export_pdf_with_options(doc: &Document, opts: &PdfOptions) -> Result<Vec<u8>, PdfExportError> { ... }

pub struct PdfOptions {
    pub page_size: PageSize,      // A4, Letter, etc.
    pub margins: Margins,
    pub font_family: String,
    pub base_font_size: f32,
    pub tagged: bool,             // PDF/UA-1 accessibility tags
    pub pdf_a: Option<PdfALevel>, // PDF/A compliance level
}
```

## 6. Chunk Graph Model

### 6.1 Design Rationale

Documents are too large for LLM context windows. Chunks are addressable sub-document units that:
- Fit within token budgets
- Preserve semantic boundaries (don't split mid-paragraph)
- Enable cross-document references (claim in doc A cites evidence in doc B)
- Support RAG retrieval at the right granularity

### 6.2 Core Types

```rust
/// Unique identifier for a chunk, derived from content + position.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub String);  // Format: "{doc_hash}:{block_path}"

/// A chunk is a contiguous slice of blocks from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub source_doc: String,           // Document identifier (path or hash)
    pub block_path: Vec<usize>,       // Path from root: [section_idx, child_idx, ...]
    pub blocks: Vec<Block>,           // The actual content
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub title: Option<String>,        // Section title if chunk starts at a section
    pub block_types: Vec<String>,     // Summary of block types contained
    pub estimated_tokens: usize,      // Approximate token count
    pub depth: usize,                 // Nesting depth in document
    pub sequence: usize,              // Position in linear chunk order
    pub total_chunks: usize,          // Total chunks in source document
}

/// A directed link between chunks (evidence, dependency, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLink {
    pub source: ChunkId,
    pub target: ChunkId,
    pub link_type: LinkType,
    pub label: Option<String>,        // Human-readable description
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType {
    Evidence,       // Source cites target as evidence
    Dependency,     // Source depends on target (skill chaining)
    Continuation,   // Target continues source (same document, next chunk)
    CrossReference, // General reference between documents
    Refutation,     // Source refutes target
}

/// The chunk graph: nodes are chunks, edges are links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkGraph {
    pub chunks: BTreeMap<ChunkId, Chunk>,
    pub links: Vec<ChunkLink>,
    pub documents: BTreeMap<String, DocumentEntry>,  // Source documents
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEntry {
    pub path: String,
    pub content_hash: String,         // SHA-256 of document content
    pub chunk_count: usize,
    pub title: Option<String>,
}
```

### 6.3 Chunk ID Generation

Chunk IDs are deterministic and content-addressable:

```
{document_content_hash_prefix_8chars}:{block_path_dot_separated}

Examples:
  a1b2c3d4:0           → First top-level block
  a1b2c3d4:2.0         → First child of third top-level block
  a1b2c3d4:2.3.1       → Second child of fourth child of third top-level block
```

This scheme ensures:
- Same content always gets the same ID
- IDs are stable across re-chunking if content doesn't change
- Block path encodes structural position for navigation

### 6.4 Chunking Strategies

```rust
pub enum ChunkStrategy {
    /// Split at section boundaries. Each top-level section = one chunk.
    Section,

    /// Split at a target token count, respecting block boundaries.
    TokenBudget { max_tokens: usize },

    /// Split at semantic block boundaries (claim, evidence, etc.).
    Semantic,

    /// Fixed number of top-level blocks per chunk.
    FixedBlocks { blocks_per_chunk: usize },
}
```

**Default strategy:** `TokenBudget { max_tokens: 2048 }` — splits at the nearest block boundary without exceeding the token budget. Never splits within a block.

### 6.5 Integration with Existing AST

The chunk graph lives alongside the AST, not inside it. The `Document` type is unchanged. Chunking is a post-processing step:

```rust
// Chunking a single document
let doc: Document = parse("input.aif")?;
let chunks: Vec<Chunk> = chunk_document(&doc, "input.aif", ChunkStrategy::TokenBudget { max_tokens: 2048 })?;

// Building a graph across documents
let mut graph = ChunkGraph::new();
graph.add_document("doc_a.aif", &doc_a, ChunkStrategy::Section)?;
graph.add_document("doc_b.aif", &doc_b, ChunkStrategy::Section)?;
graph.add_link(ChunkLink {
    source: ChunkId("a1b2c3d4:2".into()),
    target: ChunkId("e5f6g7h8:0.1".into()),
    link_type: LinkType::Evidence,
    label: Some("Clinical trial data supporting claim".into()),
})?;
```

### 6.6 Cross-Document Evidence Linking

Links are explicit and typed. They are stored in the `ChunkGraph`, not embedded in the AST. This keeps the AST clean and allows links to be maintained independently.

**Auto-linking from AIF references:** When a document contains `@ref[target=doc_b#section-2]`, the chunking pipeline can automatically create a `CrossReference` link to the chunk containing `section-2` in `doc_b`.

**Manual linking via CLI:**
```bash
aif chunk link --source doc_a.aif:2 --target doc_b.aif:0.1 --type evidence --label "Supporting data"
```

### 6.7 Serialization

The chunk graph serializes to JSON for storage and interchange:

```json
{
  "documents": {
    "doc_a.aif": { "path": "doc_a.aif", "content_hash": "a1b2c3d4...", "chunk_count": 5 }
  },
  "chunks": {
    "a1b2c3d4:0": { "source_doc": "doc_a.aif", "blocks": [...], "metadata": {...} }
  },
  "links": [
    { "source": "a1b2c3d4:2", "target": "e5f6g7h8:0.1", "link_type": "Evidence" }
  ]
}
```

## 7. CLI Integration

New subcommands for `aif-cli`:

```bash
# PDF import
aif import input.pdf [-o output.aif]           # PDF → AIF (extend existing import)
aif import input.pdf --diagnostics              # Show confidence scores

# PDF export
aif compile input.aif -f pdf [-o output.pdf]    # AIF → PDF
aif compile input.aif -f pdf --tagged           # Tagged/accessible PDF
aif compile input.aif -f pdf --pdf-a 2          # PDF/A-2 compliance

# Chunking
aif chunk input.aif                              # Chunk with default strategy
aif chunk input.aif --strategy section           # Section-based chunking
aif chunk input.aif --max-tokens 4096            # Token-budget chunking
aif chunk input.aif -o chunks/                   # Output individual chunk files

# Chunk graph
aif chunk graph --add doc_a.aif doc_b.aif        # Build graph from documents
aif chunk graph --show                            # Display graph summary
aif chunk link --source ... --target ... --type   # Add a link
aif chunk export-graph -o graph.json              # Export graph as JSON
```

## 8. Implementation Considerations

### 8.1 PDF Import Quality

PDF is fundamentally a visual format. Import quality depends on:
- **Well-structured PDFs** (tagged, bookmarked): Near-perfect mapping via structure tree
- **Simple text PDFs** (single column, standard fonts): Good results from font-size heuristics
- **Complex layouts** (multi-column, sidebars, footnotes): Degraded accuracy; confidence scores reflect this
- **Scanned PDFs / images**: Out of scope for v1 (would require OCR integration)

The confidence scoring system allows downstream consumers to decide whether to trust the import or request human review.

### 8.2 Export Layout Engine

The layout engine (`layout.rs`) is the most complex part of the export pipeline. It must handle:
- Text wrapping and line breaking
- Page breaking (avoid orphans/widows)
- Table layout (column width calculation)
- Figure placement
- Nested sections with proper indentation

**Strategy:** Start with a simple single-column layout engine. Iterate on complexity based on real-world needs. The layout engine is internal and can be improved without API changes.

### 8.3 Chunk Token Estimation

Token estimation uses a simple heuristic: `word_count * 1.3` (approximation for BPE tokenizers). For more accurate estimates, consumers can integrate a real tokenizer. The estimate is metadata, not a hard constraint.

### 8.4 Feature Flags

The crate uses feature flags to control dependencies:
- `import` (default): Enables pdf_oxide dependency for PDF → AST
- `export` (default): Enables krilla dependency for AST → PDF
- `pdf-pdfium`: Alternative import backend via pdfium-render
- `pdf-typst`: Alternative export via typst as library

Users who only need import can disable `export` to avoid pulling in krilla's dependencies, and vice versa.

## 9. Dependencies Summary

| Dependency | Version | Purpose | Feature Flag |
|------------|---------|---------|--------------|
| aif-core | workspace | AST types | always |
| pdf_oxide | 0.3 | PDF text/structure extraction | `import` |
| krilla | 0.5 | PDF generation with tags/accessibility | `export` |
| sha2 | 0.10 | Content hashing for chunk IDs | always |
| serde + serde_json | workspace | Chunk graph serialization | always |
| pdfium-render | 0.9 | Alternative PDF extraction | `pdf-pdfium` |

## 10. Testing Strategy

| Test Category | What | How |
|---------------|------|-----|
| Import roundtrip | PDF → AST → PDF → AST should produce similar AST | Structural comparison (block types match) |
| Import classification | Known PDFs → expected block types | Fixture PDFs with expected output |
| Export visual | AST → PDF renders correctly | Snapshot testing (compare PDF page images) |
| Export tags | Tagged PDF has correct structure | Parse output PDF structure tree |
| Chunk boundaries | Chunks respect block boundaries | Unit tests on splitter strategies |
| Chunk IDs | Deterministic, stable across re-chunking | Hash consistency tests |
| Chunk graph | Links resolve correctly | Graph traversal tests |
| Confidence | Scores reflect actual accuracy | Compare import vs ground truth |

## 11. Non-Goals (v1)

- OCR for scanned/image PDFs
- PDF form fields
- PDF annotations/comments import
- Multi-column layout detection (deferred to v2)
- PDF encryption/DRM handling
- Custom font embedding in export (use system fonts)
- Real-time streaming import for large PDFs
