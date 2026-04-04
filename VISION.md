# SkillForge Vision: Artifact Skills

> Use typed skills to generate and verify structured artifacts — spreadsheets, slide decks, diagrams, reports — from clean inputs and known templates.

## The Platform

SkillForge is evolving from a document compiler into a three-layer platform:

```
Layer 1: Semantic Format (AIF)      — canonical source of truth
Layer 2: Skills                     — executable workflows (code review, debugging, migration)
Layer 3: Artifact Skills            — structured output generation (spreadsheets, decks, diagrams)
```

Each layer builds on the one below. Skills operate on typed documents. Artifact skills extend skills to produce verified business outputs.

## Why Artifact Skills?

Many high-value workflows don't end with "code changed." They end with artifacts:

- Migration dashboards and readiness workbooks
- Executive summary decks
- Benchmark result spreadsheets
- Architecture delta diagrams
- Risk matrices and audit reports
- Release packets and runbooks

These outputs are repetitive, template-based, error-prone, and important to get right. Traditional automation is too rigid. Generic AI generation is too unreliable. **Typed artifact skills** sit in the middle: structured input → template binding → generation → verification → export.

## The Reusable Pattern

Every artifact skill follows the same structure:

```aif
@artifact_skill[name="migration-dashboard", artifact_type="spreadsheet"]

  @input_schema
    migration_summary, evaluator_results, candidate_branches, unresolved_issues
  @end

  @template
    Use workbook template `migration_dashboard_v1.xlsx`
  @end

  @binding
    Map evaluator_results → sheet "Checks"
    Map candidate_branches → sheet "Branches"
    Map unresolved_issues → sheet "Issues"
  @end

  @generate
    Populate formulas, summary cells, and conditional formatting.
  @end

  @verify
    - Required sheets exist
    - All evaluator categories represented
    - No unresolved placeholder cells remain
    - Formula cells resolve without errors
  @end

  @export
    Save workbook (.xlsx) and export summary as PDF.
  @end

  @output_contract
    Return: workbook path, PDF path, verification summary (pass/fail per check).
  @end

@end
```

This is far more useful than loose prompting. The verification layer catches errors that generic AI silently produces.

## Artifact Categories

### 1. Spreadsheet Skills (highest priority)

Best combination of structure, business value, and verifiability.

| Skill | Inputs | Outputs |
|-------|--------|---------|
| KPI Dashboard | metrics, dates, teams, targets | summary sheet, raw data, trend charts |
| Benchmark Workbook | experiment runs, baselines, metrics | results sheet, delta comparison, charts |
| Migration Evaluator | repo metadata, evaluator results, branches | pass/fail matrix, risk tab, rollout checklist |
| Financial Model | assumptions, monthly numbers, scenarios | model tab, scenario tab, charts |

**Target platforms:** Excel, Numbers, Google Sheets

### 2. Deck / Presentation Skills

High business value, template-driven.

| Skill | Inputs | Outputs |
|-------|--------|---------|
| Executive Migration Summary | status, risks, rollout plan | title, overview, risk, plan, appendix slides |
| Quarterly Review | KPIs, wins, misses, next plan | company template deck with charts |
| Benchmark Results | experiments, comparisons, narrative | methodology, comparison, recommendation slides |
| Customer Proposal | profile, scope, timeline, pricing | branded proposal with all commercial sections |

**Target platforms:** PowerPoint, Keynote, Google Slides

### 3. Diagram Skills

Very differentiated — generic AI produces pretty but inaccurate diagrams.

| Skill | Inputs | Outputs |
|-------|--------|---------|
| Architecture Diagram | services, dependencies, data stores | service graph with domain grouping |
| Migration Flow | phases, decisions, rollback points | flow diagram with verification nodes |
| Incident Timeline | timestamped events, actors, mitigations | chronological visualization |
| Dependency Map | package/service graph, ownership | clustered diagram with risk highlighting |

**Target formats:** Mermaid, Graphviz, SVG, draw.io JSON

### 4. Report & Document Skills

Cleanest bridge to the semantic document format.

| Skill | Inputs | Outputs |
|-------|--------|---------|
| Migration Report | summary, changed components, issues | executive summary, findings, appendix, PDF |
| Audit Summary | findings, severity, remediations | risk table, remediation appendix |
| Experiment Report | hypothesis, setup, results, charts | report with figures/tables, HTML/PDF |
| SOP / Runbook | procedure steps, prerequisites, escalations | structured runbook, printable + machine-readable |

### 5. Notebook / Analysis Skills

Great for technical users.

| Skill | Inputs | Outputs |
|-------|--------|---------|
| Benchmark Notebook | experiment data, baselines, metrics | Jupyter notebook with plots, tables, findings |
| Regression Analysis | pre/post metrics, build stats | regression notebook with deltas |
| Model Eval | predictions, labels, comparison configs | metric tables, slice analysis, recommendations |

## How This Connects to SkillForge Today

SkillForge already has the foundational pieces:

| Capability | Status | How Artifact Skills Uses It |
|-----------|--------|----------------------------|
| AIF semantic format | Done | Source layer for all artifact inputs |
| Typed skill blocks | Done | `@step`, `@verify`, `@output_contract` already exist |
| Skill validation | Done | `aif skill eval` validates artifact skill structure |
| Skill signing | Done | Cryptographic integrity for shared artifact skill registries |
| Migration engine | Done | First consumer — migration outputs feed artifact skills |
| Python bindings | Done | `import skillforge` for programmatic artifact generation |

The new blocks needed: `@artifact_skill`, `@input_schema`, `@template`, `@binding`, `@generate`, `@export`.

## Sequencing

### Phase 1: Migration Artifact Pack (next)

Generate migration outputs using the existing migration engine:
- Migration evaluator workbook (Excel/Numbers)
- Executive summary deck (PowerPoint/Keynote)
- Migration report (AIF → PDF)
- Migration flow diagram (Mermaid/SVG)

### Phase 2: Benchmark Artifact Pack

SkillForge's own benchmarks as the dogfood case:
- Benchmark workbook from `results.json`
- Results deck for presentations
- Analysis notebook (Jupyter)

### Phase 3: General Artifact Skills Platform

Generalize the machinery for any artifact type:
- Pluggable template engines
- Cross-artifact verification (deck references workbook data)
- Artifact skill marketplace

## Risks

1. **Too many artifact types too early** — focus on spreadsheets first
2. **Weak verification** — without strong validators, this is just fancy templating
3. **Poor template handling** — binding must be deterministic and inspectable
4. **Weak integration** — artifact skills must connect to real workflows, not standalone
5. **UI automation trap** — prefer direct file/API manipulation over fragile UI automation

## Strategic Moat

- **Skill corpus** — high-value artifact skills accumulate and compound
- **Verification logic** — stronger validators = more trustworthy system
- **Template ecosystem** — reusable, quality-controlled templates
- **Workflow integration** — artifact skills become more defensible when tightly connected to SkillForge's migration, eval, and skill authoring workflows
- **Semantic source format** — all artifacts originate from AIF, making the platform coherent and sticky
