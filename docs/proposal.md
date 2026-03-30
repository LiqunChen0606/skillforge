# AIF: AI-native Interchange Format

> A semantic document language for humans and LLMs: concise like Markdown, typed like XML/JATS, renderable like HTML, and publishable to PDF.

---

## 1. Executive Summary

This proposal defines a **new document format and toolchain** designed for both humans and LLMs.

**The problem:** existing formats split the world awkwardly.

| Format | Strength | Weakness |
|--------|----------|----------|
| Markdown | Easy to author, relatively token-efficient | Weak semantics |
| PDF | Excellent distribution and print fidelity | Hard for machines, often semantically lossy |
| HTML/XML | Strong semantic encoding | Too verbose or unpleasant for primary authoring |

**The solution** is a two-layer system:

1. **Surface syntax** — a human-writable authoring format, like Markdown or a lighter LaTeX
2. **Semantic IR** — a canonical document model optimized for LLM ingestion, chunking, citations, provenance, structured data, transformations, and multi-format export

This is **not** "one more markup language." It is a semantic document platform.

---

## 2. Motivation

The pain is real:

- Markdown is widely used in AI workflows because it is light and parseable, but it underspecifies many semantics.
- PDF remains dominant for distribution, but extraction for AI pipelines is messy — layout, reading order, tagging quality, tables, figures, and footnotes are inconsistent.
- Many current AI document workflows already convert PDFs or rich documents into Markdown/JSON/AST intermediates before retrieval or summarization.
- Technical docs, research papers, policies, legal texts, and specs increasingly need to be consumed by both humans and agents.

The opportunity is a **canonical semantic layer** that can be written by people, compiled for humans, and fed to LLMs with less waste and ambiguity.

---

## 3. Non-Goals

To maintain credibility, AIF should **not** be framed as:

- A direct PDF killer
- A magical perfect round-trip format
- JSON with a prettier name
- Markdown plus 100 extra tags
- XML 2.0

It should not assume that every conversion is symmetric or lossless.

**Realistic framing:** a native authoring and semantic source format with compilers to legacy output formats, plus best-effort importers from legacy formats.

---

## 4. Product Vision

### 4.1 Mental Model

Think of AIF as the intersection of:

- **LaTeX** — authorability and publishing
- **Markdown** — simplicity
- **Pandoc** — conversion
- **JATS/semantic XML** — typed structure
- **RAG chunk graphs** — LLM-native retrieval
- **Agent skill packaging** — executable workflow knowledge

### 4.2 Success Criteria

A user should be able to:

1. **Write** a document in a concise plain-text format
2. **Render** to HTML, PDF, Markdown, JSON/API views, and LLM-optimized views
3. **Import** from Markdown, HTML, and PDF
4. **Use the same source** for documentation, research, reports, legal texts, educational material, and agent skills

---

## 5. Two-Layer Architecture

This is the most important design decision.

### Layer A: Surface Syntax

A plain-text language that people author directly.

**Requirements:** concise, readable in raw text, Git-friendly, easier than LaTeX, more semantic than Markdown, minimal syntax noise, deterministic parsing.

### Layer B: Semantic IR

A parsed AST / intermediate representation.

**Requirements:** typed blocks, stable IDs, explicit parent/child relationships, provenance metadata, asset references, chunk boundaries, summary fields, evidence links, multi-format transformability.

**Why two layers?** A format ideal for machines is too verbose for humans. A syntax ideal for humans is too implicit for machines. The system must preserve both.

---

## 6. File and Package Structure

### Document Package (full)

```
paper.aif/
  manifest.json
  content.aif
  assets/
    fig1.png
    fig1.csv
    table2.csv
    appendix.pdf
  views/
    default.theme
    print.theme
  index/
    chunks.json
    summaries.json
    references.json
```

A serious document needs text, images, tables, attachments, metadata, themes, and machine indexes. A package avoids overloading the authoring syntax with binary and structural noise.

### Single-File Mode (lightweight)

```
note.aif
```

Essential for adoption in simple use cases.

---

## 7. Authoring Syntax

The authoring language should feel like **typed Markdown**.

### Design Principles

- Plain text first
- Explicit block roles
- Low punctuation noise
- Optional metadata (not mandatory everywhere)
- Human-readable before rendering
- Easy for LLMs to generate
- Easy to lint and autoformat

### Example

```
#title: AI-Native Docs
#summary: A document format designed for humans and LLMs.

@section[intro]: Why this exists
Markdown is easy to write but weak in semantics.
PDF is strong for distribution but weak for extraction.

@claim[id=main]
A semantic source format can reduce parsing loss and improve chunking.

@table[id=latency, src=latency.csv]
caption: Latency comparison by model

@figure[id=arch, src=arch.png]
caption: System architecture
```

**Key properties:** readable, strongly typed, compact, machine-friendly.

---

## 8. Semantic IR Example

The canonical IR (conceptual, not necessarily literal storage format):

```yaml
doc:
  id: doc_001
  title: "Example Research Note"
  summary: "This document explains X and concludes Y."
  sections:
    - id: sec_1
      type: section
      title: "Introduction"
      blocks:
        - id: p_1
          type: paragraph
          text: "..."
        - id: c_1
          type: claim
          text: "Method A reduces latency by 22%."
          evidence_refs: [tbl_2]
    - id: tbl_2
      type: table
      caption: "Latency comparison"
      data_ref: "assets/table2.csv"
      schema:
        columns:
          - name: model
            type: string
          - name: latency_ms
            type: number
```

---

## 9. Semantic Model

What AIF must model explicitly, and where it differentiates itself.

### 9.1 Document Metadata

title, subtitle, summary, authors, affiliations, version, timestamps, tags, status (draft/final/archived), language, license

### 9.2 Structural Blocks

section, subsection, paragraph, list, code block, quote, footnote, callout/warning/note, procedure/checklist, appendix

### 9.3 Rich Semantic Blocks

claim, evidence, definition, theorem, assumption, result, conclusion, requirement, policy clause, risk, recommendation

### 9.4 Media and Data Blocks

table, figure, equation, diagram, dataset, attachment

### 9.5 Machine Metadata

stable block IDs, canonical references, provenance, revision hash, source imports, confidence flags (for machine-generated content), chunk boundaries, chunk summaries, parent dependency links, "requires context from" references

---

## 10. LLM Advantages

The real win is **semantic explicitness with compactness**.

### Current Problem

LLMs must infer: what is a heading, whether bold is semantic or stylistic, whether a table is evidence or decoration, what belongs in the same chunk, whether a paragraph is a claim or example, which image matches which caption, which dataset underlies which chart.

### AIF Solution

The format tells the model directly: block type, references, evidence links, chunk relationships, summaries, provenance.

**Reduces:** parsing ambiguity, chunking loss, hallucinated references, broken table understanding, missing cross-section context.

---

## 11. Token Efficiency

A new format is **not automatically more token-efficient**. It saves tokens only with strict design discipline.

### Design Rules

1. Keep syntax compact
2. Do not repeat verbose keys on every block
3. Allow metadata inheritance
4. Store heavy data out-of-line
5. Separate machine metadata from authoring text
6. Support precomputed summaries over repeated context
7. Export an LLM-view with presentation material stripped
8. Use canonical block IDs over repeated headings

### Multi-View Strategy

| View | Purpose |
|------|---------|
| Author view | Human-editable syntax |
| Reader view | Polished HTML/PDF |
| LLM view | Compact text + semantic tags + chunk graph |
| API view | JSON/structured fragments |

This is stronger than insisting one representation does everything.

---

## 12. Output Targets

### HTML
Browser reading, responsive layouts, search indexing, annotation, embedding.

### PDF
Printing, static sharing, archival, legal/compliance, publication.

### Markdown
Lightweight interop, GitHub rendering, compatibility with existing tools.

### JSON/API
Application embedding, agent APIs, retrieval systems.

### LLM-Optimized
Chunked ingestion, RAG pipelines, low-token prompts, citations, tool calling.

---

## 13. Import Model

Essential for practicality.

### Conversion Fidelity

| Source | Fidelity | Notes |
|--------|----------|-------|
| Native `.aif` <-> IR | **Lossless** | Only layer where full round-trip is promised |
| Markdown -> AIF | **High** | Headings, lists, links, code, tables, images recovered easily. Claims, evidence, procedures harder. |
| Clean HTML -> AIF | **High** | Sections, lists, figures, tables, captions, headings. Degrades with div-heavy/script-heavy sources. |
| Tagged PDF -> AIF | **Moderate-Good** | Headings, reading order, tables, figures, lists. Quality depends on source tagging. |
| Untagged PDF -> AIF | **Best effort** | Never promise perfect reconstruction. Key credibility point. |

---

## 14. Differentiation from Pandoc

Pandoc is the most obvious comparison and already extremely strong at format conversion, AST transformation, and broad compatibility. "Pandoc but new" is not enough.

AIF must do **at least six things Pandoc does not center as first-class goals:**

### 14.1 AI-Native Semantics
Pandoc's AST targets broad interoperability. AIF centers claims, evidence, definitions, requirements, procedures, chunk metadata, provenance, and machine confidence from day one.

### 14.2 Stable Chunk Graph
First-class chunk IDs, semantic boundaries, summary nodes, context dependency graphs, "must include parent context" relationships, cross-chunk references. Major differentiator for LLM/RAG.

### 14.3 Evidence/Data Linkage
claim -> evidence refs, figure -> source dataset, chart -> underlying CSV, recommendation -> supporting sections. Intrinsic to the model, not bolt-on.

### 14.4 Multi-View Compilation
Purpose-specific views (author, publish, print, API, LLM, skill). Pandoc converts documents; AIF compiles **knowledge products**.

### 14.5 Import Normalization
Opinionated "normalize into one canonical semantic form" workflow: import, normalize, attach IDs/provenance, infer semantics, preserve import trace, expose quality diagnostics.

### 14.6 Agent-Executable Documents
Represent operational instructions for coding agents: preconditions, tools, steps, examples, failure modes, output contracts, reusable templates.

### 14.7 Semantic Linting
First-class validation: missing references, orphaned figures, claims without evidence, unresolved citations, broken chunk dependencies, undefined terms, non-deterministic procedure steps.

---

## 15. Agent Skills

One of the most strategically important aspects of AIF.

### Current State

Modern coding agents support reusable task guidance:
- Claude Code uses `SKILL.md` for packaged instructions
- Codex supports `AGENTS.md` and agent skills

These are powerful patterns but still largely text-first and convention-heavy.

### The Opportunity

A structured skill document replaces untyped Markdown with explicit: title, purpose, triggers, anti-triggers, required tools, preconditions, variable inputs, step graph, validation checks, error recovery, examples, expected outputs, references.

### Skill Block Types

```
@skill[id=pdf_summary]
title: Summarize a PDF with citations

@precondition
The input file must be present and readable.

@tool[name=screenshot]
Use screenshots for pages containing tables, charts, or figures.

@step[id=1]
Extract document structure and reading order.

@decision[id=2]
If the PDF is image-based, switch to OCR or screenshot-assisted parsing.

@verify
Every summary paragraph must cite the source page or block ID.

@output_contract
Return a concise summary with citations and a risk note if extraction quality is low.
```

### Skill Metadata Extensions

| Category | Fields |
|----------|--------|
| Core | skill_id, name, summary, tags, owner, version, supported tools, target agents, confidence, risk level |
| Routing | trigger conditions, anti-triggers, input schema, expected environment, expected artifacts |
| Execution | ordered steps, dependencies, conditional branches, retries, fallbacks, guardrails, validation |
| Output | required shape, citation requirements, formatting, completion criteria |

### Strategic Value

Skills are not just documents — they are **executable operational knowledge**. A semantic format makes discovery, routing, parameterization, verification, and reuse explicit. This gives AIF a wedge beyond "better docs" into knowledge packaging, workflow specification, and agent skill substrate.

---

## 16. Example Workflows

### A: Authoring a Document

1. Write `content.aif`
2. Parse to semantic IR
3. Validate references and unresolved claims
4. Export to HTML (site), PDF (sharing), LLM view (retrieval)
5. Agent consumes chunk graph directly

### B: Importing and Normalizing a PDF

1. Import PDF
2. Recover structure (best-effort)
3. Produce normalized IR
4. Add confidence diagnostics (heading quality, table extraction, figure-caption linkage)
5. Export normalized `.aif` for editing or publishing

### C: Writing an Agent Skill

1. Write skill in AIF syntax
2. Validate (tools specified, steps ordered, output contract defined)
3. Export to human-readable docs, skill package, compact LLM view

---

## 17. Launch Strategy

AIF should **not** launch as "universal replacement for all documents."

### Best Wedges (in priority order)

| Wedge | Why |
|-------|-----|
| **Technical docs + agent skills** | Sharpest entry point. Agents read docs constantly; skills need structure. |
| Research/academic papers | Claims, evidence, figures, equations. Strong machine readability need. |
| Internal knowledge bases/specs/RFCs | Procedural, reference-heavy. Strong chunking and agent consumption need. |

### Positioning

- **Not:** "PDF is dead" or "Replace Markdown everywhere"
- **Yes:** "Author in an AI-native semantic format, publish to PDF when you need a final artifact"
- **Yes:** "Replace Markdown where semantics, chunking, provenance, and structured reasoning matter"

For simple notes and READMEs, Markdown remains fine.

---

## 18. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Too much syntax complexity | Keep authoring syntax strictly simpler than XML/LaTeX |
| Weak author experience | Invest in editor tooling, LSP, autoformat from day one |
| Overpromising PDF round-trip | State honest fidelity guarantees per source type |
| Token overhead explosion | Enforce compact design rules; benchmark continuously |
| No killer wedge | Focus launch on technical docs + skills |
| Reinventing Pandoc badly | Differentiate on semantics, chunks, skills, linting — not conversion breadth |

---

## 19. MVP Roadmap

### Phase 1: Core

- Native syntax definition
- Parser
- Semantic IR
- Compilers: HTML, Markdown, LLM-view
- Markdown importer
- Block types: title, sections, paragraphs, lists, code, tables, figures, citations, callouts, claim/evidence

### Phase 2: PDF

- PDF export (via HTML/LaTeX pipeline or direct renderer)
- Tagged PDF import
- Confidence diagnostics on import

### Phase 3: Skills

- Skill block types (`@skill`, `@precondition`, `@step`, `@verify`, `@output_contract`)
- Skill validation
- Skill package export

---

## 20. Benchmark Plan

Prove the idea with data, not theory.

| Metric | What to Measure |
|--------|-----------------|
| **Token count** | Source view, LLM view, import-normalized view |
| **Retrieval quality** | Chunking accuracy, citation precision, answer grounding |
| **Table/figure fidelity** | Data recovery, caption linkage, chart-source linkage |
| **Author productivity** | Time to write/edit, syntax error rate, learnability |
| **Round-trip quality** | native -> HTML -> native, native -> Markdown -> native, native -> PDF -> diagnostics |
| **Skill execution** | Routing precision, missing-step rate, tool-usage correctness, output-contract adherence |

---

## 21. Strategic Moat

| Moat | Mechanism |
|------|-----------|
| Native semantic corpus | Teams authoring in AIF accumulate rich chunk graphs, references, claims/evidence, skills, validation rules |
| Import normalization | Strong importer stack compounds over time |
| Linting and validation | Helps teams write better documents and skills, not just different files |
| Agent ecosystem | Clean skill feed into Claude Code, Codex, and similar tools |

---

## 22. Conclusion

**Is the idea valid?** Yes.

**Strongest framing?** A human-writable semantic source format + canonical IR + compilers/importers + skill substrate. Not "replace PDF/Markdown outright."

**Is the skills angle important?** Very. It provides a wedge that is newer, more AI-native, and less directly comparable to existing document converters.

### Product Thesis

> **AIF is an AI-native document language and compiler: write once in a concise semantic syntax, publish to HTML/PDF/Markdown, import legacy documents, and package structured knowledge and agent skills in a format both humans and LLMs can use.**

---

## References

- Pandoc demonstrates the value of reader -> AST -> writer architecture, while noting its IR is less expressive than some source formats.
- Claude Code supports reusable skills through `SKILL.md` packaging.
- Codex supports reusable agent guidance via `AGENTS.md` and agent skills.
- HTML semantic elements remain important as a target render format even when not ideal for primary authoring.
