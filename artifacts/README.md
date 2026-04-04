# SkillForge Artifacts

Typed artifact skills that generate structured outputs from benchmark data and skill definitions.

## Available Artifacts

| Artifact | Input | Output | Command |
|----------|-------|--------|---------|
| [Benchmark Workbook](benchmark-workbook/) | benchmarks/*/results.json | Excel (.xlsx) | `python artifacts/benchmark-workbook/generate.py` |
| [Skill Diagram](skill-diagram/) | .aif skill file | Mermaid / SVG | `python artifacts/skill-diagram/generate.py <skill.aif>` |

## What Are Artifact Skills?

Artifact skills are a specialized class of AIF skills whose job is to generate structured business artifacts — spreadsheets, decks, diagrams, reports — from typed inputs. Each artifact skill defines its input schema, generation steps, verification rules, and output contract.

See [benchmark-workbook/benchmark-workbook.aif](benchmark-workbook/benchmark-workbook.aif) for an example.
