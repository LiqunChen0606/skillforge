# AIF Benchmarks

Comprehensive benchmarks measuring AIF's token efficiency, format fidelity, chunking quality, and skill execution compliance.

## Benchmark Suite

| Benchmark | What it measures | API needed? |
|-----------|-----------------|-------------|
| [document-tokens/](document-tokens/) | Token counts across 13 formats for 10 Wikipedia articles | Yes (Anthropic) |
| [skill-tokens/](skill-tokens/) | Token counts + semantic compliance for 10 AI skills | Yes (Anthropic) |
| [skill-execution/](skill-execution/) | Whether LLMs follow skills better in AIF vs Markdown | Yes (Anthropic) |
| [adversarial/](adversarial/) | Constraint resistance under user pressure (15 scenarios) | Yes (Anthropic) |
| [chunking/](chunking/) | Chunking strategy quality (self-containment, size variance) | No |
| [roundtrip/](roundtrip/) | Format roundtrip fidelity (AIF→X→AIF) | No |

## Key Results

| Finding | Data |
|---------|------|
| Cleaned HTML text vs raw HTML | 544K vs 5.5M tokens (90.1% saved, but zero structure) |
| AIF LML Aggressive vs raw Markdown | 981K vs 1,263K tokens (22% cheaper, full semantic types) |
| AIF LML Aggressive vs flat text | 981K vs 544K (80% more, but typed blocks + roundtrip) |
| Skill execution: LML vs Markdown | 0.84 vs 0.80 overall (+4pp); +18pp on constraint resistance, +11pp on hard scenarios |
| Adversarial resistance | All formats 0.93-1.00 (15 scenarios, 60 runs) |
| JSON roundtrip fidelity | 1.00 (lossless) |
| Markdown roundtrip fidelity | 0.84 |
| HTML roundtrip fidelity | 0.50 (generic mode loses block types) |

## Quick Start

```bash
# Build release (required for benchmarks)
cargo build --release -p aif-cli

# Run without API key
python benchmarks/chunking/benchmark.py
python benchmarks/roundtrip/benchmark.py

# Run with API key
export ANTHROPIC_API_KEY=sk-ant-...
python benchmarks/document-tokens/benchmark.py
python benchmarks/skill-tokens/benchmark.py
python benchmarks/skill-execution/benchmark.py
```

## Dashboard

Open [index.html](index.html) for a unified visual dashboard linking all benchmark reports.
