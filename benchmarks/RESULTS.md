# SkillForge Benchmark Results — Consolidated Summary

All results from 7 benchmark suites. Single source of truth for citing numbers.

## 1. Skill Execution Quality (the main claim)

**126 runs: 21 scenarios × 6 formats, claude-sonnet-4-6, April 2026**

| Format | Tokens | Overall | Steps | Constraints | Contract | Multi-Step |
|--------|--------|---------|-------|-------------|----------|------------|
| **LML Aggressive** | **861** | **0.88** | 0.89 | **0.89** | 0.89 | **0.81** |
| JSON IR | 3,838 | 0.87 | 0.85 | 0.87 | 0.87 | 0.74 |
| LML Standard | 928 | 0.86 | 0.86 | 0.88 | 0.86 | 0.74 |
| Raw Markdown | 901 | 0.84 | 0.87 | 0.86 | 0.85 | 0.72 |
| HTML | 1,217 | 0.84 | 0.84 | 0.87 | 0.84 | 0.73 |
| AIF Source | 1,024 | 0.82 | 0.82 | 0.85 | 0.82 | 0.72 |

**Key findings:**
- LML Aggressive: +4pp overall vs Markdown, +9pp on multi-step workflows
- All formats score ~0.95 on standard/easy scenarios — gap only appears on hard tasks
- JSON IR scores well but at 4.4× the token cost — worst efficiency
- AIF Source syntax (with `@end` tags) scores lowest — closing tags don't help LLMs

**Pairwise wins:** 15 ties, 2 JSON wins, 2 AIF Source wins, 1 LML wins, 1 LML Standard wins

## 2. Adversarial Resistance

**60 runs: 15 scenarios × 4 formats, claude-sonnet-4-6**

All formats score 0.93-1.00 on adversarial resistance. One outlier scenario ("Skip Security Checks") where LML scored 0.00 (needs investigation).

**Honest assessment:** In this run, the adversarial resistance advantage was NOT reproduced from the initial pilot. All formats resisted pressure comparably.

## 3. Document Token Efficiency

**10 Wikipedia articles, 13 formats, Anthropic token counting API**

| Format | Total Tokens | vs Raw HTML | Structure |
|--------|-------------|-------------|-----------|
| Cleaned HTML text | 543,584 | +90.1% saved | None |
| Raw PDF text | 561,449 | +89.8% saved | None |
| **LML Aggressive** | **980,626** | **+82.2% saved** | **Full semantic** |
| Raw Markdown | 1,263,434 | +77.0% saved | Basic |
| Raw HTML (baseline) | 5,500,132 | — | Full + chrome |

**Key findings:**
- LML Aggressive: 22% fewer tokens than Markdown with full semantic types
- 80% more tokens than flat text extraction (the price of structure)
- "82% vs Raw HTML" is real but raw HTML is an inflated baseline

## 4. Roundtrip Fidelity

**40 .aif files × 3 formats (HTML, Markdown, JSON)**

| Format | Overall Fidelity | Block Types | Semantic Types | Metadata |
|--------|-----------------|-------------|----------------|----------|
| JSON | 1.00 | 1.00 | 1.00 | 1.00 |
| Markdown | 0.84 | 0.66 | 0.93 | 0.81 |
| HTML | 0.50 | 0.00 | 0.93 | 0.65 |

**Key findings:**
- JSON roundtrip is lossless
- Markdown loses some block types but preserves most semantic types
- HTML generic mode loses all block types (AIF roundtrip mode is lossless)

## 5. Chunking Quality

**50 .aif files × 4 strategies**

| Strategy | Avg Chunks | Avg Tokens | Self-Contained | Size CV |
|----------|-----------|------------|----------------|---------|
| Token-budget | 40 | 4,718 | 8% | 0.00 |
| Fixed-blocks | 49 | 3,852 | 3% | 0.04 |
| Semantic | 64 | 2,949 | 5% | 0.04 |
| Section | 69 | 2,735 | 4% | 0.08 |

## 6. Skill Token Efficiency

**10 AI skills × 11 formats, Anthropic token counting API**

| Format | Total Tokens | vs SKILL.md | Compliance | TNO |
|--------|-------------|-------------|------------|-----|
| Markdown (RT) | 38,755 | +1.9% saved | 100% | 1.05 |
| SKILL.md | 39,506 | — | — | — |
| LML Aggressive | 39,514 | ~0% | 100% | 0.99 |

Skills are already token-dense — savings are marginal (<2%). The value of AIF for skills is semantic typing, not token reduction.

## Caveats

- All LLM-based benchmarks (execution, adversarial) are stochastic — single runs per scenario×format
- No confidence intervals (multi-run script exists but not yet executed)
- Judge model (Sonnet) may have systematic biases
- 21 scenarios is a moderate sample — more scenarios would increase confidence
- Adversarial results did NOT reproduce the +18pp constraint resistance from the initial 3-scenario pilot
