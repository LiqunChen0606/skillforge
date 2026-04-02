# Skill Execution Quality Benchmark

Does the format you present a skill in actually affect how well the LLM follows it? This benchmark answers that question empirically.

## TL;DR

**LML Aggressive scores 10 percentage points higher than raw Markdown** (0.97 vs 0.87 overall compliance) at 5% fewer tokens. The explicit typed tags (`@step:`, `@verify:`, `@red_flag:`) help the LLM identify and follow each instruction block — especially for constraint respect and output contract adherence.

| Format | Tokens | Steps | Constraints | Contract | **Overall** |
|--------|--------|-------|-------------|----------|-------------|
| **LML Aggressive** | **1,012** | **1.00** | **0.95** | **0.97** | **0.97** |
| JSON IR | 4,732 | 1.00 | 0.92 | 0.97 | 0.95 |
| HTML | 1,485 | 0.95 | 0.88 | 0.93 | 0.91 |
| Raw Markdown | 1,067 | 0.97 | 0.85 | 0.87 | 0.87 |

## How It Works

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│ Skill (.aif) │────→│  Compile to  │────→│   Executor   │
│              │     │  4 formats   │     │  (Sonnet)    │
└─────────────┘     │ - Raw MD     │     │              │
                    │ - LML Aggr.  │     │ "Follow this │
┌─────────────┐     │ - HTML       │     │  skill on    │
│  Scenario   │────→│ - JSON IR    │     │  this task"  │
│  (prompt)   │     └──────────────┘     └──────┬───────┘
└─────────────┘                                 │
                                                ▼
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

1. **Compile** the same skill to each format (Markdown, LML, HTML, JSON)
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

### Code Review Skill (3 scenarios)

| Scenario | Difficulty | What it tests |
|----------|-----------|---------------|
| SQL Injection Bug | Easy | Can the model find an obvious security flaw? |
| Clean Code (Should Approve) | Hard | Does the model avoid over-flagging good code? |
| Race Condition in Counter | Medium | Can the model identify a concurrency bug? |

### Security Guidance Skill (2 scenarios)

| Scenario | Difficulty | What it tests |
|----------|-----------|---------------|
| eval() in User Input | Easy | Classic eval injection detection |
| Shell Injection via Template | Medium | Subtler shell=True vulnerability |

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

## Token Efficiency

LML Aggressive doesn't just score higher — it uses fewer tokens:

| Format | Tokens | Overall | Compliance per 1K tokens |
|--------|--------|---------|--------------------------|
| **LML Aggressive** | **1,012** | **0.97** | **0.959** |
| Raw Markdown | 1,067 | 0.87 | 0.815 |
| HTML | 1,485 | 0.91 | 0.613 |
| JSON IR | 4,732 | 0.95 | 0.201 |

JSON IR achieves high compliance (0.95) but at 4.7x the token cost — terrible efficiency. LML Aggressive delivers the best compliance-per-token ratio by a wide margin.

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
# Full benchmark (5 scenarios × 4 formats = 20 LLM calls)
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

## File Structure

```
benchmarks/skill-execution/
├── README.md          # This file
├── benchmark.py       # Main runner: compile → execute → judge → save
├── scenarios.py       # Test scenario definitions (skills × prompts × expectations)
├── analysis.py        # Post-hoc analysis: summaries, breakdowns, efficiency metrics
└── results.json       # Latest benchmark results (committed for reference)
```

## Methodology Notes

**Judge calibration:** The judge LLM (Sonnet) evaluates against explicit criteria — not vibes. Each expected step and constraint is listed in the judge prompt, so scoring is deterministic given the response. Variance comes from the executor, not the judge.

**Reproducibility:** Results vary between runs because LLM outputs are stochastic. The benchmark saves response previews in results.json for post-hoc inspection. Run multiple times and average for publication-quality numbers.

**Scenario design:** Easy scenarios establish a baseline (all formats should score ~1.0). Hard scenarios reveal format-dependent behavior. If a format scores poorly on easy scenarios, there's a compilation or prompt assembly bug — not a format quality issue.

**Limitations:**
- 5 scenarios is a small sample — add more for higher confidence
- Same judge model for all formats — could introduce systematic bias
- Single executor run per format×scenario — no variance estimate
- Scores are relative to the judge's interpretation of "expected" behavior
