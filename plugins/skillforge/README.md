# SkillForge — Claude Code Plugin

Auto-lint, version, and sign AI skills using the AIF toolchain.

## Prerequisites

Install the AIF CLI:

```bash
cargo install --path crates/aif-cli
```

## Available Commands

| Command | Description |
|---------|-------------|
| `/lint-skill <file>` | Run 7-point structural lint on an AIF skill |
| `/convert-skill <file> [--to aif\|md]` | Convert between AIF and Markdown skill formats |
| `/sign-skill <file> --key <key>` | Sign a skill with Ed25519 for tamper detection |
| `/verify-skill <file>` | Verify integrity hash and optional signature |

## Example Usage

```
/lint-skill examples/skills/code_review.aif
/convert-skill my-skill.md --to aif
/sign-skill my-skill.aif --key ~/.aif/keys/private.pem
/verify-skill my-skill.aif
```

## How It Works

- **Lint** runs the Stage 1 eval pipeline (7 deterministic structural checks, no LLM required)
- **Convert** uses `aif skill import` and `aif skill export` for bidirectional Markdown/AIF conversion
- **Sign** generates Ed25519 signatures for tamper detection on published skills
- **Verify** checks content hashes and optional cryptographic signatures
