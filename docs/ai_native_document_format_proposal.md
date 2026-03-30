# AI-Native Document Format Proposal
## Human-writable, LLM-efficient, and convertible to/from Markdown, HTML, and PDF

**Working names:** `AIDoc`, `SDM` (Semantic Document Model), or `AIF` (AI-native Interchange Format)

---

## 1. Executive summary

This proposal explores a **new document format and toolchain** designed for both **humans** and **LLMs**.

The key thesis is:

> Existing formats split the world awkwardly:
> - **Markdown** is easy to author and relatively token-efficient, but weak in semantics.
> - **PDF** is excellent for distribution and print fidelity, but hard for machines and often semantically lossy in practice.
> - **HTML/XML** can encode strong semantics, but are often too verbose or unpleasant as primary authoring formats.

The proposed answer is **not** “one more markup language.”

It is a **two-layer system**:

1. A **human-writable authoring syntax**, like Markdown or a lighter LaTeX  
2. A **canonical semantic document model (IR/AST)**, optimized for:
   - LLM ingestion
   - chunking
   - citations
   - provenance
   - structured tables/figures
   - transformations
   - export to PDF/HTML/Markdown

The strongest positioning is:

> **A semantic document language for humans and LLMs: concise like Markdown, typed like XML/JATS, renderable like HTML, and publishable to PDF.**

---

## 2. Why this idea is valid

The idea is valid because the pain is real:

- Markdown is widely used in AI workflows because it is light and easy for models to parse, but it throws away or underspecifies many semantics.
- PDF remains dominant for distribution, but extraction for AI pipelines is frequently messy because layout, reading order, tagging quality, tables, figures, and footnotes are inconsistent.
- Many current “AI document” workflows already involve converting PDFs or rich documents into Markdown/JSON/AST-like intermediates before retrieval or summarization.
- Technical docs, research papers, policies, legal texts, internal wikis, and specs increasingly need to be consumed by both **humans** and **agents**.

So the opportunity is not just “a better text file.”

It is a **canonical semantic layer** that can:
- be written directly by people,
- compiled for humans,
- and fed to LLMs with less waste and ambiguity.

---

## 3. What this should *not* be

To succeed, this should **not** be framed as:

- “a direct PDF killer”
- “a magical perfect round-trip format”
- “JSON with a prettier name”
- “Markdown plus 100 extra tags”
- “XML 2.0”

It should also **not** assume that every conversion is symmetric or lossless.

The realistic framing is:

> **A native authoring and semantic source format with compilers to legacy output formats, plus best-effort importers from legacy formats.**

That is much more credible.

---

## 4. Core product vision

### 4.1 The right mental model

Think of this as:

- **LaTeX** for authorability and publishing
- **Markdown** for simplicity
- **Pandoc** for conversion
- **JATS / semantic XML** for typed structure
- **RAG-friendly chunk graphs** for LLMs
- **Agent skill packaging** for executable workflow knowledge

But combined into one coherent system.

### 4.2 What “success” looks like

A user should be able to:

1. **Write** a document in a concise plain-text format
2. **Render** it to:
   - HTML
   - PDF
   - Markdown
   - API/JSON views
   - LLM-optimized views
3. **Import** existing documents from:
   - Markdown
   - HTML
   - PDF
4. **Use the same canonical source** for:
   - documentation
   - research notes
   - reports
   - legal/policy texts
   - educational material
   - structured “skills” for coding agents

---

## 5. The two-layer architecture

This is the most important design decision.

### Layer A: Human-writable surface syntax

A plain-text language that people author directly.

Requirements:
- concise
- readable in raw text
- Git-friendly
- easier than LaTeX
- more semantic than Markdown
- minimal syntax noise
- deterministic parsing

### Layer B: Semantic canonical IR

A parsed abstract syntax tree / intermediate representation.

Requirements:
- typed blocks
- stable IDs
- explicit parent/child relationships
- provenance metadata
- asset references
- chunk boundaries
- summary fields
- evidence links
- transformable into multiple output formats

This separation is essential.

A format that is ideal for machines is often too verbose for humans.  
A syntax that is ideal for humans is often too implicit for machines.

So the system should preserve both.

---

## 6. Proposed file/package structure

The best implementation is probably a **document package**, not just a single text file.

Example:

```text
paper.aidoc/
  manifest.json
  content.aidoc
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

### Why a package is better
Because a serious document needs:
- text
- images
- tables
- attachment data
- metadata
- rendering themes
- optional machine indexes

A package avoids overloading the authoring syntax with binary and structural noise.

### Optional single-file mode
There should still be a lightweight single-file mode for simple use cases:

```text
note.aidoc
```

This is important for adoption.

---

## 7. Authoring syntax design goals

The authoring language should feel like **typed Markdown**.

### Principles
- plain text first
- explicit block roles
- low punctuation noise
- optional metadata, not mandatory metadata everywhere
- human-readable even before rendering
- easy to generate by LLMs
- easy to lint and autoformat

### Bad direction
Too much nested structure inline:

```json
{"type":"section","blocks":[{"type":"paragraph","text":"..."}]}
```

### Better direction
Minimal semantic directives:

```text
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

This is only an example, but the key is:
- readable
- strongly typed
- compact
- machine-friendly

---

## 8. Example semantic model

The canonical IR could look like this conceptually:

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

This is not necessarily the literal storage syntax, but it shows the semantic intent.

---

## 9. What the format must model explicitly

This is where the format really differentiates itself.

### 9.1 Document-level metadata
- title
- subtitle
- summary
- authors
- affiliations
- version
- timestamps
- tags
- status (draft / final / archived)
- language
- license

### 9.2 Structural blocks
- section
- subsection
- paragraph
- list
- code block
- quote
- footnote
- callout / warning / note
- procedure / checklist
- appendix

### 9.3 Rich semantic blocks
- claim
- evidence
- definition
- theorem
- assumption
- result
- conclusion
- requirement
- policy clause
- risk
- recommendation

### 9.4 Media/data blocks
- table
- figure
- equation
- diagram
- dataset
- attachment

### 9.5 Machine-useful metadata
- stable block IDs
- canonical references
- provenance
- revision hash
- source imports
- confidence flags for machine-generated content
- chunk boundaries
- chunk summaries
- parent dependency links
- “requires context from” references

---

## 10. Why this can be better for LLMs

The main win is not merely “a new syntax.”

The real win is **semantic explicitness with compactness**.

### Current problem
LLMs often need to infer:
- what is a heading
- whether bold text is semantic or stylistic
- whether a table is evidence or decoration
- what belongs in the same chunk
- whether a paragraph is a claim, warning, or example
- which image corresponds to which caption
- which dataset underlies which chart

### Proposed improvement
The format tells the model directly:
- block type
- references
- evidence links
- chunk relationships
- summaries
- provenance

That reduces:
- parsing ambiguity
- chunking loss
- hallucinated references
- broken table understanding
- missing context between sections

---

## 11. Token efficiency: how to make this *actually* cheaper than Markdown in practice

This is subtle.

A new format is **not automatically more token-efficient**.

It will only save tokens if it follows strict design discipline.

### Rules for token-efficient design
1. **Keep syntax compact**
2. **Do not repeat verbose keys on every block**
3. **Allow inheritance of metadata**
4. **Store heavy data out-of-line**
5. **Separate machine metadata from authoring text where possible**
6. **Support precomputed summaries rather than repeating context**
7. **Export an LLM-view with presentation-only material stripped**
8. **Use canonical block IDs rather than repeated headings or labels**

### Good strategy
Have multiple views of the same document:

- **Author view** – human-editable syntax
- **Reader view** – polished HTML/PDF
- **LLM view** – compact text + semantic tags + chunk graph
- **API view** – JSON/structured fragments

This is stronger than insisting one textual representation does everything.

---

## 12. Rendering model

The system should compile to:

### 12.1 HTML
For:
- browser reading
- responsive layouts
- search indexing
- annotation
- embedding

### 12.2 PDF
For:
- printing
- static sharing
- archival
- legal/compliance workflows
- publication

### 12.3 Markdown
For:
- lightweight interop
- GitHub-style rendering
- compatibility with existing tools

### 12.4 JSON/API fragments
For:
- application embedding
- agent APIs
- retrieval systems

### 12.5 LLM-optimized export
For:
- chunked ingestion
- RAG pipelines
- low-token prompts
- citations and tool calling

---

## 13. Import model: the reverse direction

This is essential, and it makes the proposal much more practical.

The system should support import from:
- Markdown
- HTML
- PDF
- later DOCX

But conversion guarantees must be stated honestly.

### 13.1 Native `.aidoc` ↔ IR
**Lossless**

This is the only layer where full round-tripping should be promised.

### 13.2 Markdown → native format
**High fidelity, partial semantic reconstruction**

Easy to recover:
- headings
- lists
- links
- code
- tables
- images

Harder to recover:
- claims
- evidence relationships
- procedure semantics
- callout types
- formal references

### 13.3 HTML → native format
**High fidelity if semantics are clean**

Easy to recover:
- sections
- lists
- figures
- tables
- captions
- headings
- footnotes (sometimes)

Harder when HTML is:
- div-heavy
- ad-heavy
- script-heavy
- layout-first rather than semantic-first

### 13.4 Tagged PDF → native format
**Moderate to good structural recovery**

Possible to recover:
- headings
- reading order
- tables
- figures
- lists
- paragraphs

Depends heavily on source quality.

### 13.5 Untagged or badly tagged PDF → native format
**Best effort only**

Never promise perfect reconstruction here.

This is a key credibility point.

---

## 14. Why this is differentiated from Pandoc

Pandoc is the most obvious comparison, and a good one.

Pandoc is already extremely strong at:
- format conversion
- reader/writer architecture
- AST-based transformation
- filters
- broad compatibility

So “Pandoc but new” is not enough.

To be truly differentiated, this project should do **at least six things Pandoc does not center as first-class goals**.

### 14.1 AI-native semantics
Pandoc’s AST is designed as a broad interoperability representation. It is powerful, but even Pandoc’s manual notes that its intermediate representation is less expressive than some source formats, so perfect conversions should not be expected. Your system should instead center an **AI-native semantic model** from day one:
- claims
- evidence
- definitions
- requirements
- procedures
- chunk metadata
- provenance
- machine confidence

This is a materially different design target.

### 14.2 Stable chunk graph as a primitive
Pandoc is not primarily a chunking/retrieval system.

Your format should make these first-class:
- chunk IDs
- semantic chunk boundaries
- summary nodes
- context dependency graph
- “must include parent context” relationships
- references across chunks

This is a major differentiator for LLM/RAG use.

### 14.3 Native support for evidence/data linkage
A table, chart, and claim should be linked natively:
- claim → evidence refs
- figure → source dataset
- chart → underlying CSV/JSON
- recommendation → supporting sections

Pandoc can be extended, but this should be **intrinsic** in your model.

### 14.4 Multi-view compilation
Instead of merely converting formats, the system should produce purpose-specific views:
- author view
- publish view
- print view
- API view
- LLM view
- skill view

Pandoc converts documents.  
This system should compile **knowledge products**.

### 14.5 Round-trip-oriented import normalization
Pandoc reads and writes many formats, but your differentiator should be a more opinionated “normalize into one canonical semantic form” workflow:
- import messy docs
- normalize structure
- attach IDs and provenance
- infer semantics where possible
- preserve import trace
- expose quality/confidence diagnostics

### 14.6 Agent-executable documents
This is the biggest differentiator.

The document format should be able to represent not just content, but **operational instructions for coding agents**:
- preconditions
- tools required
- steps
- examples
- failure modes
- output contracts
- reusable templates

This takes the format beyond static documents.

### 14.7 Semantic linting and quality checks
Add first-class validation:
- missing references
- orphaned figures
- claims without evidence
- unresolved citations
- broken chunk dependencies
- undefined terms
- non-deterministic procedure steps

Pandoc is a superb converter.  
Your product should be a **semantic document platform**.

---

## 15. Why this could be especially good for “skills” used by Claude Code and Codex

This is one of the most interesting parts of the idea.

### 15.1 Current “skills” model
Modern coding agents increasingly support reusable task guidance:
- Anthropic Claude Code uses `SKILL.md`-based packaged instructions and docs for extending Claude with reusable capabilities.
- OpenAI Codex supports reusable agent guidance via `AGENTS.md` and also supports “agent skills” for task-specific capabilities.

These are very powerful patterns, but today they are still largely text-first and convention-heavy.

### 15.2 The opportunity
A new document format could become a much better substrate for skills.

Instead of an untyped Markdown skill, you could have a **structured skill document**:

- title
- purpose
- when to use
- when not to use
- required tools
- required files
- preconditions
- variable inputs
- step graph
- validation checks
- error recovery
- examples
- expected outputs
- references

That makes discovery and execution much more reliable.

### 15.3 Skill-specific block types
The format could support blocks like:
- `@skill`
- `@precondition`
- `@tool`
- `@input`
- `@step`
- `@decision`
- `@fallback`
- `@verify`
- `@example`
- `@output_contract`

Example:

```text
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

This is much more executable than loose Markdown prose.

### 15.4 Why this benefits Claude Code and Codex-style workflows
Because skills are not just documents. They are **executable operational knowledge**.

A good skill needs:
- discovery
- routing
- parameterization
- verification
- adaptation
- reuse

A semantic document format can encode those explicitly.

### 15.5 Why this matters strategically
This gives the project a strong wedge beyond “better docs.”

It becomes:
- a document format
- a knowledge packaging system
- a workflow specification format
- an agent skill substrate

That is far more differentiated and potentially more valuable.

---

## 16. Suggested skill extensions to the format

A “skill profile” could be layered on top of the base document model.

### Core skill metadata
- skill_id
- name
- summary
- tags
- owner
- version
- supported tools
- target agent(s)
- confidence
- risk level

### Task routing fields
- trigger conditions
- anti-trigger conditions
- input schema
- expected environment
- expected artifacts

### Execution fields
- ordered steps
- step dependencies
- conditional branches
- retries
- fallbacks
- guardrails
- validation logic

### Output fields
- required output shape
- citation requirements
- formatting requirements
- completion criteria

This could make the format uniquely valuable for AI engineering workflows.

---

## 17. Example end-to-end workflow

### Case A: Authoring a research/spec document
1. Author writes `content.aidoc`
2. Compiler parses to semantic IR
3. Validator checks references and unresolved claims
4. Export to:
   - HTML for site
   - PDF for sharing
   - LLM view for retrieval
5. Agent can consume chunk graph directly

### Case B: Importing a PDF and normalizing it
1. Import PDF
2. Recover structure best-effort
3. Produce normalized IR
4. Add confidence diagnostics:
   - heading quality
   - table extraction quality
   - figure-caption linkage confidence
5. Export normalized `.aidoc` for editing or further publishing

### Case C: Writing an agent skill
1. Author writes a skill document in native syntax
2. Compiler validates:
   - required tools specified
   - steps ordered
   - output contract defined
3. Export to:
   - human-readable docs
   - skill package for Claude/Codex-style agent systems
   - compact LLM view for routing and execution

---

## 18. Where to launch first

This absolutely should not launch as “universal replacement for all documents.”

Best wedges:

### 18.1 AI-native technical docs
Why:
- agents read docs increasingly often
- documentation already gets transformed into Markdown or custom views for LLMs
- semantics, examples, code, and procedures matter

### 18.2 Research / academic papers
Why:
- claims, evidence, figures, equations, appendices
- very strong need for machine readability

### 18.3 Internal knowledge bases / specs / RFCs
Why:
- lots of procedural and reference-heavy content
- strong need for chunking, provenance, and agent consumption

### 18.4 Agent skills / workflow playbooks
Why:
- extremely strong differentiation
- execution reliability matters more than page fidelity
- fast path to real AI-native adoption

If I had to choose one wedge:
**technical docs + skills** is probably the sharpest commercial entry point.

---

## 19. Why “replace PDF” is still the wrong claim

PDF has huge incumbent advantages:
- static fidelity
- print workflows
- signatures
- archival
- legal acceptance
- universal openability

So the winning message is not:
> “PDF is dead.”

It is:
> **Author natively in an AI-native semantic format, then publish to PDF when you need a final artifact.**

That is believable.

---

## 20. Why “replace Markdown” is only partially true

Markdown is simple, beloved, and good enough for many lightweight use cases.

So the stronger claim is:
> **Replace Markdown in domains where semantics, chunking, provenance, and structured reasoning matter.**

Examples:
- docs
- specs
- papers
- policies
- skills
- structured knowledge

For simple notes and README files, Markdown may remain fine.

---

## 21. Risks and failure modes

### 21.1 Too much complexity
If the syntax is not significantly easier than XML/LaTeX, adoption dies.

### 21.2 Weak author experience
If authoring is not pleasant, the format becomes just an internal pipeline format.

### 21.3 Overpromising PDF round-trip
Perfect PDF reconstruction is unrealistic.

### 21.4 Token overhead explosion
If semantics are encoded too verbosely, you lose the LLM-efficiency claim.

### 21.5 No killer wedge
If it launches as “better docs for everyone,” it risks becoming too diffuse.

### 21.6 Reinventing Pandoc badly
If the project is mostly “another AST converter,” it loses.

---

## 22. MVP recommendation

### Phase 1: narrow the scope
Build:
- native syntax
- parser
- semantic IR
- compiler to HTML + Markdown
- LLM-view export
- Markdown importer

Support only:
- title
- sections
- paragraphs
- lists
- code
- tables
- figures
- citations
- callouts
- claim/evidence blocks

### Phase 2: add PDF export and PDF importer
- PDF export via HTML/LaTeX pipeline or direct renderer
- Tagged PDF import first
- confidence diagnostics on import

### Phase 3: skill profile
Add:
- skill block types
- validation
- package export
- examples and tool declarations

This gives a strong differentiated wedge.

---

## 23. Benchmark plan

To prove the idea is more than elegant theory, benchmark it against Markdown + PDF workflows.

### Metrics
1. **Token count**
   - source view
   - LLM view
   - import-normalized view

2. **Retrieval quality**
   - chunking accuracy
   - citation precision
   - answer grounding quality

3. **Table/figure fidelity**
   - data recovery
   - caption linkage
   - chart-source linkage

4. **Author productivity**
   - time to write
   - time to edit
   - syntax error rate
   - learnability

5. **Round-trip quality**
   - native → HTML → native
   - native → Markdown → native
   - native → PDF → import diagnostics

6. **Skill execution quality**
   - routing precision
   - missing-step rate
   - tool-usage correctness
   - output-contract adherence

If these benchmarks show real gains, the format has a credible chance.

---

## 24. Strategic moat

What creates defensibility here?

### 24.1 Native semantic corpus
If teams author directly in this format, they accumulate a rich corpus of:
- chunk graphs
- references
- claims/evidence structure
- skills
- validation rules
- agent-ready knowledge

That is much more valuable than flat Markdown.

### 24.2 Better import normalization
A strong importer stack becomes a moat over time.

### 24.3 Linting + validation
If the system helps teams write better documents and better skills, not just different files, it becomes sticky.

### 24.4 Agent ecosystem compatibility
If skills and structured docs can feed Claude Code, Codex, and similar tools cleanly, that is strategically powerful.

---

## 25. Final verdict

### Is the idea valid?
**Yes.**

### Is it strongest as “replace Markdown and PDF outright”?
**No.**

### Is it strongest as “a human-writable semantic source format + canonical IR + compilers/importers + skill substrate”?
**Yes.**

### Is that differentiated from Pandoc?
**Potentially yes**, but only if the project emphasizes:
- AI-native semantics
- chunk graphs
- provenance
- evidence linkage
- multi-view compilation
- validation
- executable skill packaging

### Is the skills angle important?
**Very.**

It gives the project a wedge that is newer, more AI-native, and less directly comparable to existing document converters.

---

## 26. Recommended product thesis

If I were writing the one-sentence pitch, I would use:

> **AIDoc is an AI-native document language and compiler: write once in a concise semantic syntax, publish to HTML/PDF/Markdown, import legacy documents, and package structured knowledge and agent skills in a format both humans and LLMs can use.**

That is the strongest current framing.

---

## 27. Recommended next steps

1. Define the minimal native syntax
2. Define the semantic IR schema
3. Implement parser + formatter
4. Implement HTML + LLM-view compiler
5. Implement Markdown importer
6. Add claim/evidence/table/figure primitives
7. Add skill profile extension
8. Benchmark on:
   - docs
   - research papers
   - workflow skills
9. Publish examples and a reference CLI
10. Delay ambitious PDF import promises until quality is measurable

---

## 28. References and grounding notes

This proposal is grounded in the current ecosystem:

- Pandoc already demonstrates the value of a reader → AST → writer architecture, while also explicitly noting that its intermediate representation is less expressive than some source formats and that perfect conversions should not be expected.
- Claude Code supports reusable skills through `SKILL.md` packaging and associated skill authoring guidance.
- Codex supports reusable agent guidance via `AGENTS.md` and also supports agent skills as a task-specific capability layer.
- HTML standards explicitly define semantic elements and attributes, which is why HTML remains important as a target render format even if it is not ideal as the primary authoring syntax.

These realities suggest the opportunity is not “another converter,” but a **semantic authoring + compilation + agent-skill platform**.
