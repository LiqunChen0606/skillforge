# Skill Execution Quality Benchmark

Does the format you present a skill in actually affect how well the LLM follows it? This benchmark answers that question empirically.

## TL;DR

**LML Aggressive scores +4pp overall** (0.84 vs 0.80) — but the advantage concentrates on hard scenarios (+11pp) and constraint resistance (+18pp). On easy/standard scenarios, all formats perform equally. 73 runs across 5 skills × 19 scenarios × 4 formats (claude-sonnet-4-6).

| Format | Tokens | Steps | Constraints | Contract | **Overall** | **Hard** |
|--------|--------|-------|-------------|----------|-------------|----------|
| **LML Aggressive** | **869** | **0.86** | **0.85** | **0.84** | **0.84** | **0.76** |
| AIF Source | ~900 | TBD | TBD | TBD | TBD | TBD |
| LML Standard | ~950 | TBD | TBD | TBD | TBD | TBD |
| JSON IR | 3,838 | 0.85 | 0.79 | 0.80 | 0.81 | 0.70 |
| HTML | 1,217 | 0.83 | 0.80 | 0.80 | 0.81 | 0.71 |
| Raw Markdown | 908 | 0.82 | 0.82 | 0.80 | 0.80 | **0.65** |

**Constraint resistance** (user pressures model to skip steps): LML 0.86 vs Markdown 0.68 (+18pp).

## How It Works

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│ Skill (.aif) │────→│  Compile to  │────→│   Executor   │
│              │     │  6 formats   │     │  (Sonnet)    │
└─────────────┘     │ - Raw MD     │     │              │
                    │ - AIF Source │     │ "Follow this │
┌─────────────┐     │ - LML Aggr.  │     │  skill on    │
│  Scenario   │────→│ - LML Std.   │     │  this task"  │
│  (prompt)   │     │ - HTML       │     └──────┬───────┘
└─────────────┘     │ - JSON IR    │            │
                    └──────────────┘            ▼
                    ┌──────────────┐     ┌──────────────┐
                    │    Scores    │←────│    Judge     │
                    │  per format  │     │  (Sonnet)    │
                    └──────────────┘     │              │
                                        │ Scores 0-1:  │
                                        │ - Steps      │
                                        │ - Constraints │
                                        │ - Contract   │
                                        │ - Overall    │
                                        └──────────────┘
```

### The Protocol

1. **Compile** the same skill to each format (Markdown, AIF Source, LML Aggressive, LML Standard, HTML, JSON)
2. **Execute**: Give the executor LLM the skill as a system prompt + a test scenario as user input
3. **Judge**: A separate judge LLM scores the executor's response against expected behaviors
4. **Compare**: Same skill, same scenario, same models — only the format changes

### Why This Design

- **Same skill content** — format is the only variable
- **Separate judge** — avoids self-evaluation bias
- **Multiple scenarios per skill** — easy bugs (everyone catches) + hard judgment calls (format differences emerge)
- **Difficulty levels** — easy/medium/hard to see where format matters most

## Metrics

| Metric | What it measures | Scale |
|--------|-----------------|-------|
| **Step coverage** | Were the skill's `@step` blocks followed? | 0.0–1.0 |
| **Constraint respect** | Were `@red_flag` / `@verify` constraints honored? | 0.0–1.0 |
| **Output contract** | Does the response match `@output_contract` criteria? | 0.0–1.0 |
| **Overall** | Weighted average of all dimensions | 0.0–1.0 |

## Scenarios

21 scenarios across 5 skills and 5 scenario types:

| Type | Count | What it tests |
|------|-------|---------------|
| Standard | 8 | Traditional skill application |
| Constraint resistance | 3 | User pressures model to skip steps |
| Multi-step | 3 | Ordered workflow compliance |
| Conflicting instructions | 3 | User prompt contradicts skill |
| Edge cases | 4 | Unusual inputs (empty, safe, out-of-scope) |

Skills: code-review, security-guidance, debugging, commit-commands, feature-dev, frontend-design.

## Key Finding: Format Matters Most on Hard Scenarios

On easy scenarios (SQL injection, eval detection), all formats score ~1.0 — the bug is so obvious that format doesn't matter. The gap appears on **hard scenarios** where the model must exercise judgment:

**"Clean Code (Should Approve)" — the differentiator:**

| Format | Overall | What happened |
|--------|---------|---------------|
| LML Aggressive | 0.93 | Correctly approved with praise for test coverage |
| JSON IR | 0.85 | Approved but added unnecessary suggestions |
| HTML | 0.75 | Over-flagged minor style issues as blocking |
| Raw Markdown | 0.62 | Bikeshedded on naming, missed the "approve with praise" intent |

**Why LML wins here:** The `@red_flag` block explicitly says "don't bikeshed on style while missing logic." In LML Aggressive, this appears as `@red_flag:` — a visually distinct tag the LLM can latch onto. In raw Markdown, it's buried in prose.

## New Format Arms: AIF Source and LML Standard

Two additional formats were added to test whether closing-tag styles affect LLM compliance:

| Format | Tag Style | Example | Hypothesis |
|--------|-----------|---------|------------|
| **AIF Source** | `@block...@end` | `@step[order=1]\n  ...\n@end` | Explicit `@end` closing tags may help LLMs track block boundaries better than LML Aggressive's implicit line-based blocks |
| **LML Standard** | `[STEP]...[/STEP]` | `[STEP order=1]\n...\n[/STEP]` | XML-like paired tags are familiar from HTML/XML training data and may improve structure recognition |

**AIF Source** reads the raw `.aif` file directly (no compilation). **LML Standard** uses `aif compile --format lml` which produces `[STEP]...[/STEP]` style tags with full semantic markup.

These complement the existing formats to cover the full spectrum of delimiter styles: no delimiters (Markdown), prefix-only (`@step:` in LML Aggressive), paired tags (`[STEP]...[/STEP]` in LML Standard), explicit close (`@end` in AIF Source), and structural markup (HTML, JSON).

## Token Efficiency

LML Aggressive doesn't just score higher — it uses fewer tokens:

| Format | Tokens | Overall | Compliance per 1K tokens |
|--------|--------|---------|--------------------------|
| **LML Aggressive** | **869** | **0.84** | **0.972** |
| Raw Markdown | 908 | 0.80 | 0.883 |
| HTML | 1,217 | 0.81 | 0.662 |
| JSON IR | 3,838 | 0.81 | 0.211 |

JSON IR achieves comparable compliance but at 4.4x the token cost — terrible efficiency. LML Aggressive delivers the best compliance-per-token ratio.

## Running the Benchmark

### Prerequisites

```bash
# 1. Build AIF CLI (release mode for speed)
cargo build --release -p aif-cli

# 2. Set your API key
export ANTHROPIC_API_KEY=sk-ant-...

# 3. Install Python deps
pip install anthropic
```

### Run

```bash
# Full benchmark (21 scenarios × 6 formats = 126 LLM calls)
python benchmarks/skill-execution/benchmark.py

# Analyze existing results (no API calls)
python benchmarks/skill-execution/analysis.py
```

### Output

The benchmark produces:
- **Terminal output** — live progress + full analysis report
- **results.json** — structured results with scores, timing, and response previews
- **Analysis report** — format summary, token efficiency, difficulty/category breakdowns, pairwise wins

### Adding Scenarios

Edit `scenarios.py` to add new test cases:

```python
{
    "skill_file": "examples/skills/your-skill.aif",
    "name": "Descriptive Scenario Name",
    "category": "your-category",
    "difficulty": "easy|medium|hard",
    "description": "What this scenario tests",
    "prompt": "The user's task prompt...",
    "expected_steps": ["step 1 behavior", "step 2 behavior"],
    "expected_constraints": ["constraint 1", "constraint 2"],
    "output_contract": "what the response should look like",
}
```

## Multi-Run Variance Analysis

Single runs don't tell you if format differences are real or noise. Run multiple times to compute confidence intervals:

```bash
# Run 5 times per scenario×format (expensive: 5 × scenarios × formats LLM calls)
ANTHROPIC_API_KEY=sk-... python benchmarks/skill-execution/multi_run.py --runs 5

# Analyze saved results (no API calls)
python benchmarks/skill-execution/multi_run.py --analyze
```

Reports per-format mean, stddev, 95% confidence interval, and pairwise significance tests (non-overlapping CIs). Also flags high-variance scenario×format combinations (stddev > 0.10) that may need more runs.

## File Structure

```
benchmarks/skill-execution/
├── README.md              # This file
├── benchmark.py           # Main runner: compile → execute → judge → save
├── scenarios.py           # Test scenario definitions (skills × prompts × expectations)
├── analysis.py            # Post-hoc analysis: summaries, breakdowns, efficiency metrics
├── multi_run.py           # Multi-run variance analysis with confidence intervals
├── results.json           # Latest single-run benchmark results
└── multi_run_results.json # Multi-run results (generated, not committed)
```

## Methodology Notes

**Judge calibration:** The judge LLM (Sonnet) evaluates against explicit criteria — not vibes. Each expected step and constraint is listed in the judge prompt, so scoring is deterministic given the response. Variance comes from the executor, not the judge.

**Reproducibility:** Results vary between runs because LLM outputs are stochastic. The benchmark saves response previews in results.json for post-hoc inspection. Run multiple times and average for publication-quality numbers.

**Scenario design:** Easy scenarios establish a baseline (all formats should score ~1.0). Hard scenarios reveal format-dependent behavior. If a format scores poorly on easy scenarios, there's a compilation or prompt assembly bug — not a format quality issue.

**Limitations:**
- 19 scenarios completed (2 remaining due to API credit exhaustion) — larger sample than initial 3-scenario pilot
- Same judge model for all formats — could introduce systematic bias
- Single executor run per format×scenario in `benchmark.py` — use `multi_run.py` for variance estimates
- Scores are relative to the judge's interpretation of "expected" behavior
