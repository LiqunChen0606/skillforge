# Skill Eval Pipeline — "GitHub Actions for Coding-Agent Skills"

## Vision

A three-stage quality pipeline that makes authoring, validating, and publishing coding-agent skills as easy as writing and shipping code. The pipeline is one thing that runs in three contexts: local dev loop, CI on push, and registry gate on publish.

## Design Decisions

| Question | Decision |
|----------|----------|
| Safety model | D: Three-stage pipeline — Structural, Behavioral, Effectiveness |
| Test case types | C+D: Scenarios, compliance checks, AND pressure tests |
| Authoring UX | D: Pipeline IS the feedback loop (failures are actionable) |
| Entry points | D: Natural language, examples, guided interview, or template — all converge |
| Canonical format | D: AIF native with SKILL.md export for compatibility |
| Where it runs | D: Local + CI + registry gate (same pipeline, different triggers) |
| LLM access | User configures API keys for their preferred provider(s) |

## Architecture

```
                    ┌─────────────────────────────────────┐
                    │          ENTRY POINTS                │
                    │                                      │
                    │  Natural language  "Make a skill     │
                    │  Examples + intent  that ensures..." │
                    │  Guided interview                    │
                    │  Template + LLM fill                 │
                    │  Hand-authored AIF                   │
                    └──────────────┬──────────────────────┘
                                   │
                                   ▼
                    ┌──────────────────────────────────────┐
                    │         SKILL DRAFT (.aif)           │
                    │  AIF native, typed @skill blocks     │
                    └──────────────┬──────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────┐
                    │     STAGE 1: STRUCTURAL LINT         │
                    │                                      │
                    │  - Required sections present?        │
                    │  - Frontmatter valid?                │
                    │  - Skill block types correct?        │
                    │  - Version/hash consistent?          │
                    │  - Description follows conventions?  │
                    │                                      │
                    │  Fast, no LLM needed, deterministic  │
                    └──────────────┬──────────────────────┘
                                   │ pass
                    ┌──────────────▼──────────────────────┐
                    │   STAGE 2: BEHAVIORAL COMPLIANCE     │
                    │                                      │
                    │  For each compliance check:          │
                    │  1. Dispatch agent WITH skill loaded │
                    │  2. Give agent a task that should    │
                    │     trigger skill behavior           │
                    │  3. Verify agent followed the skill  │
                    │     (did it run tests? did it ask    │
                    │     before acting? etc.)             │
                    │                                      │
                    │  Requires LLM, moderate cost         │
                    └──────────────┬──────────────────────┘
                                   │ pass
                    ┌──────────────▼──────────────────────┐
                    │   STAGE 3: EFFECTIVENESS EVAL        │
                    │                                      │
                    │  Three test types:                   │
                    │                                      │
                    │  A. Scenarios — "Given task X,       │
                    │     agent should produce Y"          │
                    │                                      │
                    │  B. Compliance — "Agent must follow  │
                    │     steps in order, not skip any"    │
                    │                                      │
                    │  C. Pressure tests — "Agent gets     │
                    │     tempted to skip the skill.       │
                    │     Does it resist?"                 │
                    │                                      │
                    │  Requires LLM, highest cost          │
                    └──────────────┬──────────────────────┘
                                   │ pass
                    ┌──────────────▼──────────────────────┐
                    │        VALIDATED SKILL                │
                    │                                      │
                    │  .aif (canonical) + .md (export)     │
                    │  Ready for: local use, publish,      │
                    │  or registry submission               │
                    └─────────────────────────────────────┘
```

## Pipeline Contexts

The same pipeline runs in three places:

| Context | Trigger | Stages | Purpose |
|---------|---------|--------|---------|
| **Local** | `aif skill eval my-skill.aif` | All 3 | Authoring feedback loop |
| **CI** | Push / PR (GitHub Actions) | All 3 | PR validation gate |
| **Registry** | `aif skill publish` | All 3 | Publish quality gate |

### Local Dev Loop

```bash
# Author writes/edits skill
aif skill eval my-skill.aif

# Output:
# STAGE 1: STRUCTURAL LINT ............ PASS (0.2s)
# STAGE 2: BEHAVIORAL COMPLIANCE ...... FAIL (12s)
#   ✗ Agent skipped step 3 when task seemed simple
#   → Suggestion: Add red flag for "this is too simple to need process"
# STAGE 3: EFFECTIVENESS EVAL ......... SKIPPED (stage 2 failed)
#
# 1 of 3 stages passed. Fix behavioral compliance to continue.
```

### CI Integration

```yaml
# .github/workflows/skill-eval.yml (generated by `aif skill ci-init`)
name: Skill Eval
on: [push, pull_request]
jobs:
  eval:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: aif-project/skill-eval-action@v1
        with:
          skills: "skills/*.aif"
          llm-provider: ${{ secrets.LLM_PROVIDER }}
          api-key: ${{ secrets.LLM_API_KEY }}
```

### Registry Gate

```bash
aif skill publish my-skill.aif
# Runs full pipeline remotely
# Registry rejects skills that fail any stage
```

## Authoring Flow

### Any Entry Point → Draft → Pipeline → Iterate

```
User: "I want a skill that makes agents always run tests before claiming done"
  │
  ▼ (LLM generates draft from natural language)
  │
Draft: my-skill.aif
  │
  ▼ (pipeline runs)
  │
Feedback: "Agent skipped verification under time pressure"
  │
  ▼ (LLM or user edits skill to add pressure resistance)
  │
Draft v2: my-skill.aif
  │
  ▼ (pipeline runs)
  │
Result: ALL STAGES PASS → ready to use/publish
```

### Entry Points

1. **Natural language** — User describes intent, LLM generates full skill
2. **Examples + intent** — User provides good/bad agent behavior examples
3. **Guided interview** — System asks 5-7 questions, generates from answers
4. **Template** — User fills template, optionally asks LLM to complete it
5. **Hand-authored AIF** — User writes `.aif` directly (tutorials available)

All produce an `.aif` file that enters the same pipeline.

## Test Case Format

Test cases are defined inline in the skill or in a companion `tests/` directory:

```aif
@skill[name="verification-before-completion", version="1.0"]

  @step[order=1]
    Run the verification command before claiming success.
  @end

  @verify
    @scenario[name="basic-compliance"]
      @precondition
        Agent has just finished implementing a feature.
      @end
      @step[order=1]
        Give agent a task: "Add a hello-world function and commit."
      @end
      @output_contract
        Agent must run test suite before committing.
      @end
    @end

    @scenario[name="pressure-resistance", type=pressure]
      @precondition
        Agent is told "this is urgent, skip testing."
      @end
      @step[order=1]
        Give agent a task with time pressure framing.
      @end
      @output_contract
        Agent must STILL run tests. Skill overrides urgency.
      @end
    @end
  @end
@end
```

## LLM Provider Configuration

```bash
# First-time setup
aif config set llm.provider anthropic   # or openai, google, etc.
aif config set llm.api-key sk-...       # stored in ~/.aif/config.toml
aif config set llm.model claude-sonnet-4-6  # optional, has defaults

# Or via environment variables
export AIF_LLM_PROVIDER=anthropic
export AIF_LLM_API_KEY=sk-...
```

Supported in config file (`~/.aif/config.toml`):

```toml
[llm]
provider = "anthropic"    # anthropic | openai | google | local
api_key = "sk-..."        # or use env var AIF_LLM_API_KEY
model = "claude-sonnet-4-6"  # optional, provider-specific default
base_url = ""             # optional, for local/custom endpoints
```

## Stage Details

### Stage 1: Structural Lint (no LLM)

Fast, deterministic checks:

| Check | Description |
|-------|-------------|
| `frontmatter` | `name` and `description` present, description starts with "Use when" |
| `required-sections` | At least one `@step`, at least one `@verify` |
| `block-types` | All skill block types are valid (`step`, `verify`, `precondition`, etc.) |
| `version-hash` | Version and hash are consistent (if present) |
| `description-length` | Description under 1024 chars, no workflow summary |
| `name-format` | Letters, numbers, hyphens only |
| `no-empty-blocks` | No empty `@step` or `@verify` blocks |

### Stage 2: Behavioral Compliance (LLM required)

For each compliance check defined in the skill's `@verify` section:

1. Spin up a sandboxed agent with the skill loaded
2. Give it a task designed to trigger the skill's rules
3. Observe: did the agent follow the skill?
4. Report: which rules were followed, which were violated

**Default compliance checks** (run even if skill doesn't define custom ones):

- Agent acknowledges skill is loaded
- Agent follows steps in declared order
- Agent doesn't skip steps marked as mandatory

### Stage 3: Effectiveness Eval (LLM required)

Three test types, all defined by the skill author:

| Type | Purpose | Example |
|------|---------|---------|
| **Scenario** | Does the skill produce correct outcomes? | "Agent should run tests before committing" |
| **Compliance** | Does the agent follow the process? | "Agent must ask before destructive actions" |
| **Pressure** | Does the skill hold under adversarial conditions? | "Agent told to skip testing — does it resist?" |

Each test produces a pass/fail with evidence (agent transcript excerpt showing compliance or violation).

## Output Formats

### Validated Skill

```bash
# AIF canonical format (source of truth)
my-skill.aif

# SKILL.md export (for Claude Code, Codex, etc.)
aif skill export my-skill.aif -o my-skill.md
```

### Eval Report

```bash
aif skill eval my-skill.aif --report json    # machine-readable
aif skill eval my-skill.aif --report text    # human-readable (default)
aif skill eval my-skill.aif --report html    # rich report
```

## CLI Commands (New)

```bash
# Eval pipeline
aif skill eval <skill.aif> [--stage 1|2|3] [--report text|json|html]
aif skill eval <skill.aif> --watch          # re-run on file change

# Authoring helpers
aif skill generate "natural language description"
aif skill generate --from-examples examples/
aif skill generate --interview

# CI integration
aif skill ci-init [--provider github|gitlab]  # generate CI config

# LLM configuration
aif config set llm.provider <provider>
aif config set llm.api-key <key>
aif config set llm.model <model>
aif config list
```

## Implementation Crates

| Crate | New/Modified | Purpose |
|-------|-------------|---------|
| `aif-skill` | Modified | Add eval pipeline, structural lint, test case types |
| `aif-eval` | **New** | Behavioral compliance + effectiveness eval (LLM integration) |
| `aif-cli` | Modified | New `eval`, `generate`, `ci-init`, `config` subcommands |
| `aif-core` | Minor | Config types for LLM provider settings |

## MVP Scope

**Phase 1 (MVP):** Local eval only

- Stage 1: Structural lint (extend existing `aif skill verify`)
- Stage 2: Behavioral compliance (basic, 1 LLM provider)
- Stage 3: Effectiveness eval (scenario tests only)
- CLI: `aif skill eval`, `aif config`
- One LLM provider (Anthropic)

**Phase 2:** Full authoring + CI

- All entry points (`generate`, `--interview`, templates)
- CI config generation (`ci-init`)
- Multiple LLM providers
- Pressure tests
- `--watch` mode

**Phase 3:** Registry integration

- Registry gate on publish
- Remote eval execution
- Eval result caching
- Cross-skill dependency validation

## Success Criteria

1. **Authoring friction reduced** — User goes from idea to validated skill in < 10 minutes
2. **Quality gate works** — Skills that pass the pipeline produce measurably better agent behavior than skills that don't
3. **Pipeline is fast enough for dev loop** — Stage 1 < 1s, Stage 2 < 30s, Stage 3 < 2min
4. **Entry points converge** — All authoring methods produce identical pipeline input
5. **CI is turnkey** — `aif skill ci-init` produces working GitHub Actions config
