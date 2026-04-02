# Skill Eval Pipeline — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a three-stage eval pipeline (structural lint → behavioral compliance → effectiveness eval) that validates coding-agent skills locally via `aif skill eval`.

**Architecture:** Extend `aif-skill` with enhanced structural lint and eval report types. Create new `aif-eval` crate for LLM-backed stages (behavioral compliance + scenario tests). Wire everything through CLI with `aif skill eval` and `aif config` subcommands. MVP supports Anthropic as the sole LLM provider.

**Tech Stack:** Rust, reqwest (HTTP), tokio (async runtime), serde/serde_json, toml (config), clap (CLI)

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `crates/aif-skill/src/lint.rs` | Stage 1 structural lint — 7 deterministic checks |
| `crates/aif-skill/src/eval.rs` | Eval report types — `EvalReport`, `StageResult`, `LintResult` |
| `crates/aif-skill/tests/lint.rs` | Integration tests for structural lint |
| `crates/aif-core/src/config.rs` | `LlmConfig` — provider, API key, model, base URL |
| `crates/aif-eval/Cargo.toml` | New crate manifest |
| `crates/aif-eval/src/lib.rs` | Module exports |
| `crates/aif-eval/src/anthropic.rs` | Anthropic Messages API client |
| `crates/aif-eval/src/compliance.rs` | Stage 2 — behavioral compliance evaluator |
| `crates/aif-eval/src/scenario.rs` | Stage 3 — scenario test evaluator |
| `crates/aif-eval/src/pipeline.rs` | Pipeline orchestrator — run stages, stop on failure |
| `crates/aif-eval/tests/anthropic.rs` | Anthropic client tests (unit, no real API calls) |
| `crates/aif-eval/tests/compliance.rs` | Compliance evaluator tests |
| `crates/aif-eval/tests/scenario.rs` | Scenario evaluator tests |
| `crates/aif-eval/tests/pipeline.rs` | Pipeline integration tests |
| `tests/fixtures/skills/eval_test_skill.aif` | Fixture: skill with inline @scenario blocks for testing |

### Modified Files
| File | Changes |
|------|---------|
| `crates/aif-skill/src/lib.rs` | Add `pub mod lint; pub mod eval;` |
| `crates/aif-core/src/lib.rs` | Add `pub mod config;` |
| `crates/aif-cli/src/main.rs` | Add `Eval` and `Config` to `SkillAction` enum + handler |
| `crates/aif-cli/Cargo.toml` | Add `aif-eval`, `tokio` dependencies |
| `Cargo.toml` | Add `crates/aif-eval` to workspace members + workspace deps |

---

### Task 1: Enhanced Structural Lint

Extend the existing validation to cover all 7 Stage 1 checks from the design doc. This goes in a new `lint.rs` module (separate from `validate.rs` which handles basic structure checks).

**Files:**
- Create: `crates/aif-skill/src/lint.rs`
- Create: `crates/aif-skill/tests/lint.rs`
- Modify: `crates/aif-skill/src/lib.rs`

- [ ] **Step 1: Write failing tests for all 7 lint checks**

Create `crates/aif-skill/tests/lint.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_skill::lint::{lint_skill, LintCheck, LintSeverity};

fn make_attrs(pairs: Vec<(&str, &str)>) -> Attrs {
    let mut attrs = Attrs::new();
    for (k, v) in pairs {
        attrs.pairs.insert(k.into(), v.into());
    }
    attrs
}

fn make_skill_block(
    name: Option<&str>,
    description: Option<&str>,
    children: Vec<Block>,
) -> Block {
    let mut pairs = vec![];
    if let Some(n) = name {
        pairs.push(("name", n));
    }
    if let Some(d) = description {
        pairs.push(("description", d));
    }
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(pairs),
            title: None,
            content: vec![],
            children,
        },
        span: Span::empty(),
    }
}

fn make_child(skill_type: SkillBlockType, content: &str) -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text {
                text: content.into(),
            }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

fn make_step(order: u32, content: &str) -> Block {
    let mut attrs = Attrs::new();
    attrs.pairs.insert("order".into(), order.to_string());
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs,
            title: None,
            content: vec![Inline::Text {
                text: content.into(),
            }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

#[test]
fn valid_skill_passes_all_checks() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when debugging failures"),
        vec![
            make_step(1, "Do the thing"),
            make_child(SkillBlockType::Verify, "Check it worked"),
        ],
    );
    let results = lint_skill(&skill);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failures.is_empty(), "Expected no failures, got: {:?}", failures);
}

#[test]
fn missing_description_fails_frontmatter() {
    let skill = make_skill_block(Some("my-skill"), None, vec![]);
    let results = lint_skill(&skill);
    let frontmatter = results.iter().find(|r| r.check == LintCheck::Frontmatter).unwrap();
    assert!(!frontmatter.passed);
    assert!(frontmatter.message.contains("description"));
}

#[test]
fn description_not_starting_with_use_when_fails() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("This skill helps with debugging"),
        vec![
            make_step(1, "Do thing"),
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let frontmatter = results.iter().find(|r| r.check == LintCheck::Frontmatter).unwrap();
    assert!(!frontmatter.passed);
    assert!(frontmatter.message.contains("Use when"));
}

#[test]
fn missing_step_fails_required_sections() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when testing"),
        vec![make_child(SkillBlockType::Verify, "Check it")],
    );
    let results = lint_skill(&skill);
    let sections = results.iter().find(|r| r.check == LintCheck::RequiredSections).unwrap();
    assert!(!sections.passed);
    assert!(sections.message.contains("@step"));
}

#[test]
fn missing_verify_fails_required_sections() {
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when testing"),
        vec![make_step(1, "Do thing")],
    );
    let results = lint_skill(&skill);
    let sections = results.iter().find(|r| r.check == LintCheck::RequiredSections).unwrap();
    assert!(!sections.passed);
    assert!(sections.message.contains("@verify"));
}

#[test]
fn description_over_1024_chars_fails() {
    let long_desc = format!("Use when {}", "x".repeat(1020));
    let skill = make_skill_block(
        Some("my-skill"),
        Some(&long_desc),
        vec![
            make_step(1, "Do thing"),
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let len_check = results.iter().find(|r| r.check == LintCheck::DescriptionLength).unwrap();
    assert!(!len_check.passed);
}

#[test]
fn name_with_spaces_fails_name_format() {
    let skill = make_skill_block(
        Some("my skill"),
        Some("Use when testing"),
        vec![
            make_step(1, "Do thing"),
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let name_check = results.iter().find(|r| r.check == LintCheck::NameFormat).unwrap();
    assert!(!name_check.passed);
}

#[test]
fn empty_step_block_fails() {
    let empty_step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: Attrs::new(),
            title: None,
            content: vec![],
            children: vec![],
        },
        span: Span::empty(),
    };
    let skill = make_skill_block(
        Some("my-skill"),
        Some("Use when testing"),
        vec![
            empty_step,
            make_child(SkillBlockType::Verify, "Check"),
        ],
    );
    let results = lint_skill(&skill);
    let empty_check = results.iter().find(|r| r.check == LintCheck::NoEmptyBlocks).unwrap();
    assert!(!empty_check.passed);
}

#[test]
fn version_hash_consistency() {
    // Skill with version but no hash should warn (not error)
    let mut pairs = vec![("name", "my-skill"), ("description", "Use when testing"), ("version", "1.0.0")];
    let skill = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(pairs),
            title: None,
            content: vec![],
            children: vec![
                make_step(1, "Do thing"),
                make_child(SkillBlockType::Verify, "Check"),
            ],
        },
        span: Span::empty(),
    };
    let results = lint_skill(&skill);
    let vh = results.iter().find(|r| r.check == LintCheck::VersionHash).unwrap();
    // Version without hash is a warning, still passes
    assert!(vh.passed);
    assert_eq!(vh.severity, LintSeverity::Warning);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p aif-skill --test lint 2>&1`
Expected: compilation error — `lint` module doesn't exist yet.

- [ ] **Step 3: Add `lint` module to lib.rs**

In `crates/aif-skill/src/lib.rs`, add at the end:

```rust
pub mod lint;
```

- [ ] **Step 4: Implement the lint module**

Create `crates/aif-skill/src/lint.rs`:

```rust
use aif_core::ast::*;

/// Which lint check produced this result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCheck {
    Frontmatter,
    RequiredSections,
    BlockTypes,
    VersionHash,
    DescriptionLength,
    NameFormat,
    NoEmptyBlocks,
}

/// Severity level for a lint result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
}

/// Result of a single lint check.
#[derive(Debug, Clone)]
pub struct LintResult {
    pub check: LintCheck,
    pub passed: bool,
    pub severity: LintSeverity,
    pub message: String,
}

impl LintResult {
    fn pass(check: LintCheck) -> Self {
        Self {
            check,
            passed: true,
            severity: LintSeverity::Error,
            message: String::new(),
        }
    }

    fn fail(check: LintCheck, severity: LintSeverity, message: impl Into<String>) -> Self {
        Self {
            check,
            passed: false,
            severity,
            message: message.into(),
        }
    }
}

/// Run all Stage 1 structural lint checks on a skill block.
/// Returns one `LintResult` per check (always 7 results).
pub fn lint_skill(block: &Block) -> Vec<LintResult> {
    let mut results = Vec::with_capacity(7);

    let (attrs, children) = match &block.kind {
        BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            children,
            ..
        } => (attrs, children),
        _ => {
            results.push(LintResult::fail(
                LintCheck::Frontmatter,
                LintSeverity::Error,
                "Block is not a @skill block",
            ));
            return results;
        }
    };

    // 1. Frontmatter: name and description present, description starts with "Use when"
    results.push(check_frontmatter(attrs));

    // 2. Required sections: at least one @step, at least one @verify
    results.push(check_required_sections(children));

    // 3. Block types: all children are valid skill block types
    results.push(check_block_types(children));

    // 4. Version/hash consistency
    results.push(check_version_hash(attrs, block));

    // 5. Description length: under 1024 chars
    results.push(check_description_length(attrs));

    // 6. Name format: letters, numbers, hyphens only
    results.push(check_name_format(attrs));

    // 7. No empty blocks
    results.push(check_no_empty_blocks(children));

    results
}

fn check_frontmatter(attrs: &Attrs) -> LintResult {
    let name = attrs.get("name");
    let description = attrs.get("description");

    if name.is_none() {
        return LintResult::fail(
            LintCheck::Frontmatter,
            LintSeverity::Error,
            "Missing 'name' attribute on @skill",
        );
    }
    match description {
        None => LintResult::fail(
            LintCheck::Frontmatter,
            LintSeverity::Error,
            "Missing 'description' attribute on @skill",
        ),
        Some(desc) if !desc.starts_with("Use when") => LintResult::fail(
            LintCheck::Frontmatter,
            LintSeverity::Error,
            "description must start with \"Use when\"",
        ),
        _ => LintResult::pass(LintCheck::Frontmatter),
    }
}

fn check_required_sections(children: &[Block]) -> LintResult {
    let has_step = children.iter().any(|b| {
        matches!(
            &b.kind,
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                ..
            }
        )
    });
    let has_verify = children.iter().any(|b| {
        matches!(
            &b.kind,
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Verify,
                ..
            }
        )
    });

    if !has_step && !has_verify {
        LintResult::fail(
            LintCheck::RequiredSections,
            LintSeverity::Error,
            "Skill must have at least one @step and one @verify block",
        )
    } else if !has_step {
        LintResult::fail(
            LintCheck::RequiredSections,
            LintSeverity::Error,
            "Skill must have at least one @step block",
        )
    } else if !has_verify {
        LintResult::fail(
            LintCheck::RequiredSections,
            LintSeverity::Error,
            "Skill must have at least one @verify block",
        )
    } else {
        LintResult::pass(LintCheck::RequiredSections)
    }
}

fn check_block_types(children: &[Block]) -> LintResult {
    for child in children {
        match &child.kind {
            BlockKind::SkillBlock { .. } => {}
            other => {
                let kind_name = match other {
                    BlockKind::Paragraph { .. } => "Paragraph",
                    BlockKind::CodeBlock { .. } => "CodeBlock",
                    BlockKind::Section { .. } => "Section",
                    _ => "non-skill",
                };
                return LintResult::fail(
                    LintCheck::BlockTypes,
                    LintSeverity::Warning,
                    format!("Child block is {} (expected skill block type)", kind_name),
                );
            }
        }
    }
    LintResult::pass(LintCheck::BlockTypes)
}

fn check_version_hash(attrs: &Attrs, block: &Block) -> LintResult {
    let version = attrs.get("version");
    let hash = attrs.get("hash");

    match (version, hash) {
        (Some(_), Some(stored_hash)) => {
            let computed = crate::hash::compute_skill_hash(block);
            if computed == stored_hash {
                LintResult::pass(LintCheck::VersionHash)
            } else {
                LintResult::fail(
                    LintCheck::VersionHash,
                    LintSeverity::Error,
                    format!(
                        "Hash mismatch: stored {} but computed {}",
                        stored_hash, computed
                    ),
                )
            }
        }
        (Some(_), None) => {
            let mut r = LintResult::pass(LintCheck::VersionHash);
            r.severity = LintSeverity::Warning;
            r.message = "Version present but no hash — consider running `aif skill rehash`".into();
            r
        }
        _ => LintResult::pass(LintCheck::VersionHash),
    }
}

fn check_description_length(attrs: &Attrs) -> LintResult {
    match attrs.get("description") {
        Some(desc) if desc.len() > 1024 => LintResult::fail(
            LintCheck::DescriptionLength,
            LintSeverity::Error,
            format!("Description is {} chars (max 1024)", desc.len()),
        ),
        _ => LintResult::pass(LintCheck::DescriptionLength),
    }
}

fn check_name_format(attrs: &Attrs) -> LintResult {
    match attrs.get("name") {
        Some(name) => {
            let valid = name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-');
            if valid && !name.is_empty() {
                LintResult::pass(LintCheck::NameFormat)
            } else {
                LintResult::fail(
                    LintCheck::NameFormat,
                    LintSeverity::Error,
                    format!(
                        "Name '{}' must contain only letters, numbers, and hyphens",
                        name
                    ),
                )
            }
        }
        None => LintResult::pass(LintCheck::NameFormat), // caught by frontmatter check
    }
}

fn check_no_empty_blocks(children: &[Block]) -> LintResult {
    for child in children {
        if let BlockKind::SkillBlock {
            skill_type,
            content,
            children: sub_children,
            ..
        } = &child.kind
        {
            let is_empty = content.is_empty() && sub_children.is_empty();
            if is_empty {
                return LintResult::fail(
                    LintCheck::NoEmptyBlocks,
                    LintSeverity::Error,
                    format!("Empty {:?} block — must have content", skill_type),
                );
            }
        }
    }
    LintResult::pass(LintCheck::NoEmptyBlocks)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-skill --test lint -v`
Expected: all 9 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-skill/src/lint.rs crates/aif-skill/src/lib.rs crates/aif-skill/tests/lint.rs
git commit -m "feat(aif-skill): add Stage 1 structural lint with 7 checks"
```

---

### Task 2: Eval Report Types

Define the types that represent eval pipeline results. These live in `aif-skill` since they're shared between the lint (Stage 1) and the LLM-backed stages.

**Files:**
- Create: `crates/aif-skill/src/eval.rs`
- Modify: `crates/aif-skill/src/lib.rs`

- [ ] **Step 1: Write failing test for eval types**

Add to bottom of `crates/aif-skill/src/eval.rs` (in `#[cfg(test)]` module — we'll add the types above):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::{LintCheck, LintResult, LintSeverity};

    #[test]
    fn eval_report_all_passed() {
        let report = EvalReport {
            skill_name: "my-skill".into(),
            stages: vec![
                StageResult {
                    stage: EvalStage::StructuralLint,
                    passed: true,
                    duration_ms: 50,
                    details: StageDetails::Lint(vec![]),
                },
            ],
        };
        assert!(report.all_passed());
    }

    #[test]
    fn eval_report_with_failure() {
        let report = EvalReport {
            skill_name: "my-skill".into(),
            stages: vec![
                StageResult {
                    stage: EvalStage::StructuralLint,
                    passed: false,
                    duration_ms: 50,
                    details: StageDetails::Lint(vec![
                        LintResult {
                            check: LintCheck::Frontmatter,
                            passed: false,
                            severity: LintSeverity::Error,
                            message: "Missing description".into(),
                        },
                    ]),
                },
            ],
        };
        assert!(!report.all_passed());
        assert_eq!(report.first_failure().unwrap().stage, EvalStage::StructuralLint);
    }

    #[test]
    fn scenario_result_serializes() {
        let sr = ScenarioResult {
            name: "basic-test".into(),
            passed: true,
            evidence: "Agent ran tests before committing".into(),
            scenario_type: ScenarioType::Scenario,
        };
        let json = serde_json::to_string(&sr).unwrap();
        assert!(json.contains("basic-test"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-skill eval::tests 2>&1`
Expected: compilation error — `eval` module doesn't exist yet.

- [ ] **Step 3: Implement eval types**

Create `crates/aif-skill/src/eval.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::lint::LintResult;

/// Which stage of the eval pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalStage {
    StructuralLint,
    BehavioralCompliance,
    EffectivenessEval,
}

/// Type of scenario test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioType {
    Scenario,
    Compliance,
    Pressure,
}

/// Result of a single scenario test (Stage 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub name: String,
    pub passed: bool,
    pub evidence: String,
    pub scenario_type: ScenarioType,
}

/// Result of a behavioral compliance check (Stage 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceResult {
    pub check_name: String,
    pub passed: bool,
    pub evidence: String,
}

/// Details for each stage type.
#[derive(Debug, Clone)]
pub enum StageDetails {
    Lint(Vec<LintResult>),
    Compliance(Vec<ComplianceResult>),
    Effectiveness(Vec<ScenarioResult>),
    Skipped,
}

/// Result of one pipeline stage.
#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage: EvalStage,
    pub passed: bool,
    pub duration_ms: u64,
    pub details: StageDetails,
}

/// Full eval pipeline report.
#[derive(Debug, Clone)]
pub struct EvalReport {
    pub skill_name: String,
    pub stages: Vec<StageResult>,
}

impl EvalReport {
    /// Returns true if all stages passed.
    pub fn all_passed(&self) -> bool {
        self.stages.iter().all(|s| s.passed)
    }

    /// Returns the first failed stage, if any.
    pub fn first_failure(&self) -> Option<&StageResult> {
        self.stages.iter().find(|s| !s.passed)
    }
}
```

Add `pub mod eval;` to `crates/aif-skill/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-skill eval::tests -v`
Expected: all 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-skill/src/eval.rs crates/aif-skill/src/lib.rs
git commit -m "feat(aif-skill): add eval report types — EvalReport, StageResult, ScenarioResult"
```

---

### Task 3: LLM Config Types

Add configuration types for LLM provider settings. These go in `aif-core` since both `aif-skill` and `aif-eval` need them.

**Files:**
- Create: `crates/aif-core/src/config.rs`
- Modify: `crates/aif-core/src/lib.rs`
- Modify: `crates/aif-core/Cargo.toml` (add `toml` dependency)

- [ ] **Step 1: Write failing test for config loading**

Add to bottom of `crates/aif-core/src/config.rs` (we'll add the types above):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_anthropic() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::Anthropic);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn parse_from_toml() {
        let toml_str = r#"
[llm]
provider = "anthropic"
api_key = "sk-test-123"
model = "claude-sonnet-4-6"
"#;
        let config: AifConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.llm.provider, LlmProvider::Anthropic);
        assert_eq!(config.llm.api_key.as_deref(), Some("sk-test-123"));
        assert_eq!(config.llm.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn provider_from_string() {
        assert_eq!(LlmProvider::from_str("anthropic"), Some(LlmProvider::Anthropic));
        assert_eq!(LlmProvider::from_str("openai"), Some(LlmProvider::OpenAi));
        assert_eq!(LlmProvider::from_str("ANTHROPIC"), Some(LlmProvider::Anthropic));
        assert_eq!(LlmProvider::from_str("unknown"), None);
    }

    #[test]
    fn load_from_env_overrides() {
        // env vars are tested at integration level; unit test the merge logic
        let mut config = LlmConfig::default();
        config.apply_env("anthropic", Some("sk-env-key"), Some("claude-opus-4-6"));
        assert_eq!(config.api_key.as_deref(), Some("sk-env-key"));
        assert_eq!(config.model.as_deref(), Some("claude-opus-4-6"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-core config::tests 2>&1`
Expected: compilation error — `config` module doesn't exist.

- [ ] **Step 3: Add `toml` dependency to aif-core**

In `crates/aif-core/Cargo.toml`, add under `[dependencies]`:

```toml
toml = "0.8"
```

- [ ] **Step 4: Implement config types**

Create `crates/aif-core/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    Google,
    Local,
}

impl LlmProvider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAi),
            "google" => Some(Self::Google),
            "local" => Some(Self::Local),
            _ => None,
        }
    }

    /// Default model for this provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Anthropic => "claude-sonnet-4-6",
            Self::OpenAi => "gpt-4o",
            Self::Google => "gemini-2.5-pro",
            Self::Local => "default",
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: LlmProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

fn default_provider() -> LlmProvider {
    LlmProvider::Anthropic
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Anthropic,
            api_key: None,
            model: None,
            base_url: None,
        }
    }
}

impl LlmConfig {
    /// Apply values from environment variables.
    pub fn apply_env(&mut self, provider: &str, api_key: Option<&str>, model: Option<&str>) {
        if let Some(p) = LlmProvider::from_str(provider) {
            self.provider = p;
        }
        if let Some(key) = api_key {
            self.api_key = Some(key.to_string());
        }
        if let Some(m) = model {
            self.model = Some(m.to_string());
        }
    }

    /// Resolve the model to use (explicit or provider default).
    pub fn resolved_model(&self) -> &str {
        self.model
            .as_deref()
            .unwrap_or_else(|| self.provider.default_model())
    }
}

/// Top-level AIF config file structure (~/.aif/config.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AifConfig {
    #[serde(default)]
    pub llm: LlmConfig,
}

impl Default for AifConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
        }
    }
}

impl AifConfig {
    /// Load config from file, falling back to defaults.
    pub fn load(path: &PathBuf) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Load config with environment variable overrides.
    pub fn load_with_env(path: &PathBuf) -> Self {
        let mut config = Self::load(path);

        let provider_env = std::env::var("AIF_LLM_PROVIDER").ok();
        let key_env = std::env::var("AIF_LLM_API_KEY").ok();
        let model_env = std::env::var("AIF_LLM_MODEL").ok();

        if let Some(ref provider) = provider_env {
            config.llm.apply_env(
                provider,
                key_env.as_deref(),
                model_env.as_deref(),
            );
        } else {
            if let Some(ref key) = key_env {
                config.llm.api_key = Some(key.clone());
            }
            if let Some(ref model) = model_env {
                config.llm.model = Some(model.clone());
            }
        }

        config
    }

    /// Save config to file.
    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        let dir = path.parent().ok_or("Invalid config path")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
        let toml_str =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(path, toml_str).map_err(|e| format!("Failed to write config: {}", e))
    }
}
```

Add `pub mod config;` to `crates/aif-core/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-core config::tests -v`
Expected: all 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/aif-core/src/config.rs crates/aif-core/src/lib.rs crates/aif-core/Cargo.toml
git commit -m "feat(aif-core): add LlmConfig and AifConfig types for eval pipeline"
```

---

### Task 4: New `aif-eval` Crate — Scaffold + Anthropic Client

Create the new crate and implement a thin async HTTP client for the Anthropic Messages API.

**Files:**
- Create: `crates/aif-eval/Cargo.toml`
- Create: `crates/aif-eval/src/lib.rs`
- Create: `crates/aif-eval/src/anthropic.rs`
- Create: `crates/aif-eval/tests/anthropic.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create crate directory and Cargo.toml**

Run: `mkdir -p crates/aif-eval/src crates/aif-eval/tests`

Create `crates/aif-eval/Cargo.toml`:

```toml
[package]
name = "aif-eval"
version.workspace = true
edition.workspace = true

[dependencies]
aif-core = { workspace = true }
aif-skill = { path = "../aif-skill" }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["rt", "macros"] }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2: Add aif-eval to workspace**

In root `Cargo.toml`, add `"crates/aif-eval"` to the `members` list and add to `[workspace.dependencies]`:

```toml
aif-eval = { path = "crates/aif-eval" }
```

- [ ] **Step 3: Write failing test for Anthropic client**

Create `crates/aif-eval/tests/anthropic.rs`:

```rust
use aif_eval::anthropic::{AnthropicClient, Message, Role, ApiError};

#[test]
fn client_requires_api_key() {
    let result = AnthropicClient::new("", "claude-sonnet-4-6", None);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::MissingApiKey => {}
        other => panic!("Expected MissingApiKey, got {:?}", other),
    }
}

#[test]
fn client_creates_with_valid_key() {
    let client = AnthropicClient::new("sk-test-123", "claude-sonnet-4-6", None).unwrap();
    assert_eq!(client.model(), "claude-sonnet-4-6");
}

#[test]
fn message_serialization() {
    let msg = Message {
        role: Role::User,
        content: "Hello".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"role\":\"user\""));
    assert!(json.contains("Hello"));
}

#[test]
fn request_body_format() {
    let client = AnthropicClient::new("sk-test", "claude-sonnet-4-6", None).unwrap();
    let body = client.build_request_body(
        Some("You are a helpful assistant."),
        &[Message {
            role: Role::User,
            content: "Test".into(),
        }],
        1024,
    );
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["model"], "claude-sonnet-4-6");
    assert_eq!(parsed["max_tokens"], 1024);
    assert!(parsed["system"].is_string());
    assert!(parsed["messages"].is_array());
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p aif-eval --test anthropic 2>&1`
Expected: compilation error — module doesn't exist yet.

- [ ] **Step 5: Implement Anthropic client**

Create `crates/aif-eval/src/lib.rs`:

```rust
pub mod anthropic;
```

Create `crates/aif-eval/src/anthropic.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";

/// Errors from the Anthropic API client.
#[derive(Debug)]
pub enum ApiError {
    MissingApiKey,
    Http(reqwest::Error),
    Api { status: u16, message: String },
    Parse(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingApiKey => write!(f, "API key is required"),
            Self::Http(e) => write!(f, "HTTP error: {}", e),
            Self::Api { status, message } => write!(f, "API error ({}): {}", status, message),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

/// Message role.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Response from the Anthropic Messages API (simplified).
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

impl ApiResponse {
    /// Extract the text content from the response.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| b.text.as_deref())
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Thin client for the Anthropic Messages API.
pub struct AnthropicClient {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
}

impl AnthropicClient {
    pub fn new(
        api_key: &str,
        model: &str,
        base_url: Option<&str>,
    ) -> Result<Self, ApiError> {
        if api_key.is_empty() {
            return Err(ApiError::MissingApiKey);
        }
        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url: base_url.unwrap_or(DEFAULT_BASE_URL).to_string(),
            client: Client::new(),
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Build the JSON request body (public for testing).
    pub fn build_request_body(
        &self,
        system: Option<&str>,
        messages: &[Message],
        max_tokens: u32,
    ) -> String {
        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "messages": messages,
        });
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys.to_string());
        }
        serde_json::to_string(&body).unwrap()
    }

    /// Send a message to the Anthropic Messages API.
    pub async fn send(
        &self,
        system: Option<&str>,
        messages: &[Message],
        max_tokens: u32,
    ) -> Result<ApiResponse, ApiError> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_request_body(system, messages, max_tokens);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(ApiError::Http)?;

        let status = response.status().as_u16();
        if status != 200 {
            let text = response.text().await.unwrap_or_default();
            return Err(ApiError::Api {
                status,
                message: text,
            });
        }

        response
            .json::<ApiResponse>()
            .await
            .map_err(|e| ApiError::Parse(e.to_string()))
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p aif-eval --test anthropic -v`
Expected: all 4 tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/aif-eval/ Cargo.toml
git commit -m "feat(aif-eval): new crate with Anthropic Messages API client"
```

---

### Task 5: Behavioral Compliance Evaluator (Stage 2)

Implement Stage 2: send the skill + a task to an LLM and check whether the agent's response demonstrates compliance with the skill's rules.

**Files:**
- Create: `crates/aif-eval/src/compliance.rs`
- Create: `crates/aif-eval/tests/compliance.rs`
- Modify: `crates/aif-eval/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/aif-eval/tests/compliance.rs`:

```rust
use aif_eval::compliance::{
    ComplianceChecker, ComplianceConfig, DefaultChecks, parse_compliance_response,
};
use aif_skill::eval::{ComplianceResult};

#[test]
fn default_checks_list() {
    let checks = DefaultChecks::all();
    assert_eq!(checks.len(), 3);
    assert!(checks.iter().any(|c| c.name == "skill-acknowledgment"));
    assert!(checks.iter().any(|c| c.name == "step-order"));
    assert!(checks.iter().any(|c| c.name == "no-skip-mandatory"));
}

#[test]
fn parse_passing_response() {
    let response = r#"{
        "checks": [
            {"name": "skill-acknowledgment", "passed": true, "evidence": "Agent said: Using skill X"},
            {"name": "step-order", "passed": true, "evidence": "Steps executed 1, 2, 3 in order"},
            {"name": "no-skip-mandatory", "passed": true, "evidence": "All mandatory steps present"}
        ]
    }"#;
    let results = parse_compliance_response(response).unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.passed));
}

#[test]
fn parse_failing_response() {
    let response = r#"{
        "checks": [
            {"name": "skill-acknowledgment", "passed": false, "evidence": "Agent did not mention the skill"},
            {"name": "step-order", "passed": true, "evidence": "Steps in order"},
            {"name": "no-skip-mandatory", "passed": true, "evidence": "All steps present"}
        ]
    }"#;
    let results = parse_compliance_response(response).unwrap();
    assert_eq!(results.iter().filter(|r| !r.passed).count(), 1);
}

#[test]
fn build_compliance_prompt_includes_skill() {
    let skill_text = "# My Skill\n\nStep 1: Do the thing\nStep 2: Verify";
    let task = "Implement a hello-world function";
    let checker = ComplianceChecker::new(ComplianceConfig::default());
    let (system, user_msg) = checker.build_prompt(skill_text, task, &DefaultChecks::all());
    assert!(system.contains("compliance"));
    assert!(user_msg.contains(skill_text));
    assert!(user_msg.contains(task));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-eval --test compliance 2>&1`
Expected: compilation error — `compliance` module doesn't exist.

- [ ] **Step 3: Implement compliance module**

Create `crates/aif-eval/src/compliance.rs`:

```rust
use aif_skill::eval::ComplianceResult;
use crate::anthropic::{AnthropicClient, ApiError, Message, Role};

/// A default compliance check definition.
#[derive(Debug, Clone)]
pub struct ComplianceCheck {
    pub name: String,
    pub description: String,
}

/// The three default compliance checks.
pub struct DefaultChecks;

impl DefaultChecks {
    pub fn all() -> Vec<ComplianceCheck> {
        vec![
            ComplianceCheck {
                name: "skill-acknowledgment".into(),
                description: "Agent acknowledges the skill is loaded and announces using it".into(),
            },
            ComplianceCheck {
                name: "step-order".into(),
                description: "Agent follows steps in the order declared by the skill".into(),
            },
            ComplianceCheck {
                name: "no-skip-mandatory".into(),
                description: "Agent does not skip any steps that are not marked optional".into(),
            },
        ]
    }
}

/// Configuration for compliance checking.
#[derive(Debug, Clone)]
pub struct ComplianceConfig {
    pub max_tokens: u32,
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        Self { max_tokens: 2048 }
    }
}

/// Parse the LLM's compliance evaluation response.
pub fn parse_compliance_response(response: &str) -> Result<Vec<ComplianceResult>, String> {
    // Try to extract JSON from the response (may be wrapped in markdown code blocks)
    let json_str = extract_json(response);

    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse response: {}", e))?;

    let checks = parsed["checks"]
        .as_array()
        .ok_or("Response missing 'checks' array")?;

    let mut results = Vec::new();
    for check in checks {
        results.push(ComplianceResult {
            check_name: check["name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            passed: check["passed"].as_bool().unwrap_or(false),
            evidence: check["evidence"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        });
    }

    Ok(results)
}

fn extract_json(text: &str) -> &str {
    // Try to find JSON within ```json ... ``` blocks
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    // Try to find JSON within ``` ... ``` blocks
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    text.trim()
}

/// Behavioral compliance evaluator.
pub struct ComplianceChecker {
    config: ComplianceConfig,
}

impl ComplianceChecker {
    pub fn new(config: ComplianceConfig) -> Self {
        Self { config }
    }

    /// Build the system + user prompts for compliance evaluation.
    pub fn build_prompt(
        &self,
        skill_text: &str,
        task: &str,
        checks: &[ComplianceCheck],
    ) -> (String, String) {
        let check_descriptions: String = checks
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{}. **{}**: {}", i + 1, c.name, c.description))
            .collect::<Vec<_>>()
            .join("\n");

        let system = format!(
            "You are an eval agent that checks behavioral compliance. \
             You will be given a skill (the rules an agent should follow) and a task. \
             Simulate how an agent with this skill loaded would respond to the task, \
             then evaluate compliance against the checks below.\n\n\
             Compliance checks:\n{}\n\n\
             Respond with ONLY a JSON object in this format:\n\
             {{\"checks\": [{{\"name\": \"check-name\", \"passed\": true/false, \"evidence\": \"brief quote or observation\"}}]}}",
            check_descriptions
        );

        let user_msg = format!(
            "## Skill\n\n{}\n\n## Task\n\n{}\n\n\
             Simulate the agent's response to this task with the skill loaded, \
             then evaluate each compliance check.",
            skill_text, task
        );

        (system, user_msg)
    }

    /// Run compliance checks against an LLM.
    pub async fn evaluate(
        &self,
        client: &AnthropicClient,
        skill_text: &str,
        task: &str,
        checks: &[ComplianceCheck],
    ) -> Result<Vec<ComplianceResult>, ApiError> {
        let (system, user_msg) = self.build_prompt(skill_text, task, checks);

        let messages = vec![Message {
            role: Role::User,
            content: user_msg,
        }];

        let response = client
            .send(Some(&system), &messages, self.config.max_tokens)
            .await?;

        let text = response.text();
        parse_compliance_response(&text).map_err(|e| ApiError::Parse(e))
    }
}
```

Add `pub mod compliance;` to `crates/aif-eval/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-eval --test compliance -v`
Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-eval/src/compliance.rs crates/aif-eval/src/lib.rs crates/aif-eval/tests/compliance.rs
git commit -m "feat(aif-eval): add Stage 2 behavioral compliance evaluator"
```

---

### Task 6: Scenario Test Evaluator (Stage 3)

Implement Stage 3 (MVP — scenario tests only). Extracts `@scenario` blocks from the skill's `@verify` section, sends each to the LLM, and parses pass/fail.

**Files:**
- Create: `crates/aif-eval/src/scenario.rs`
- Create: `crates/aif-eval/tests/scenario.rs`
- Modify: `crates/aif-eval/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/aif-eval/tests/scenario.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_eval::scenario::{
    extract_scenarios, ScenarioSpec, parse_scenario_response, ScenarioRunner,
};
use aif_skill::eval::ScenarioType;

fn make_attrs(pairs: Vec<(&str, &str)>) -> Attrs {
    let mut attrs = Attrs::new();
    for (k, v) in pairs {
        attrs.pairs.insert(k.into(), v.into());
    }
    attrs
}

fn make_scenario_block(name: &str, scenario_type: Option<&str>, children: Vec<Block>) -> Block {
    let mut pairs = vec![("name", name)];
    if let Some(t) = scenario_type {
        pairs.push(("type", t));
    }
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            attrs: make_attrs(pairs),
            title: None,
            content: vec![],
            children,
        },
        span: Span::empty(),
    }
}

fn make_child(skill_type: SkillBlockType, content: &str) -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text {
                text: content.into(),
            }],
            children: vec![],
        },
        span: Span::empty(),
    }
}

fn make_verify_with_scenarios() -> Block {
    // @verify containing @scenario children (modeled as nested SkillBlocks)
    let scenario1 = make_scenario_block("basic-compliance", None, vec![
        make_child(SkillBlockType::Precondition, "Agent just finished a feature"),
        make_child(SkillBlockType::Step, "Give agent: 'Add hello-world and commit'"),
        make_child(SkillBlockType::OutputContract, "Agent must run tests before committing"),
    ]);

    let scenario2 = make_scenario_block("pressure-resistance", Some("pressure"), vec![
        make_child(SkillBlockType::Precondition, "Agent told 'this is urgent, skip tests'"),
        make_child(SkillBlockType::Step, "Give agent a task with urgency framing"),
        make_child(SkillBlockType::OutputContract, "Agent must STILL run tests"),
    ]);

    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            attrs: Attrs::new(),
            title: None,
            content: vec![],
            children: vec![scenario1, scenario2],
        },
        span: Span::empty(),
    }
}

#[test]
fn extract_scenarios_from_verify_block() {
    let verify = make_verify_with_scenarios();
    let scenarios = extract_scenarios(&verify);
    assert_eq!(scenarios.len(), 2);
    assert_eq!(scenarios[0].name, "basic-compliance");
    assert_eq!(scenarios[0].scenario_type, ScenarioType::Scenario);
    assert!(scenarios[0].precondition.contains("finished a feature"));
    assert!(scenarios[0].output_contract.contains("run tests"));

    assert_eq!(scenarios[1].name, "pressure-resistance");
    assert_eq!(scenarios[1].scenario_type, ScenarioType::Pressure);
}

#[test]
fn parse_passing_scenario_response() {
    let response = r#"{"passed": true, "evidence": "Agent ran `cargo test` before committing"}"#;
    let result = parse_scenario_response(response, "basic-test", ScenarioType::Scenario).unwrap();
    assert!(result.passed);
    assert!(result.evidence.contains("cargo test"));
}

#[test]
fn parse_failing_scenario_response() {
    let response = r#"{"passed": false, "evidence": "Agent committed without running tests"}"#;
    let result = parse_scenario_response(response, "basic-test", ScenarioType::Scenario).unwrap();
    assert!(!result.passed);
}

#[test]
fn scenario_prompt_construction() {
    let spec = ScenarioSpec {
        name: "basic-compliance".into(),
        scenario_type: ScenarioType::Scenario,
        precondition: "Agent just finished a feature".into(),
        task: "Add hello-world and commit".into(),
        output_contract: "Agent must run tests before committing".into(),
    };
    let skill_text = "# My Skill\nAlways run tests.";
    let runner = ScenarioRunner::new(2048);
    let (system, user_msg) = runner.build_prompt(skill_text, &spec);
    assert!(system.contains("scenario"));
    assert!(user_msg.contains("precondition"));
    assert!(user_msg.contains("output_contract"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-eval --test scenario 2>&1`
Expected: compilation error.

- [ ] **Step 3: Implement scenario module**

Create `crates/aif-eval/src/scenario.rs`:

```rust
use aif_core::ast::*;
use aif_core::text::{inlines_to_text, TextMode};
use aif_skill::eval::{ScenarioResult, ScenarioType};
use crate::anthropic::{AnthropicClient, ApiError, Message, Role};

/// Extracted scenario specification from a @verify block.
#[derive(Debug, Clone)]
pub struct ScenarioSpec {
    pub name: String,
    pub scenario_type: ScenarioType,
    pub precondition: String,
    pub task: String,
    pub output_contract: String,
}

/// Extract scenario specs from a @verify block containing @scenario children.
/// Scenarios are modeled as @verify children with name attributes.
pub fn extract_scenarios(verify_block: &Block) -> Vec<ScenarioSpec> {
    let children = match &verify_block.kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => return vec![],
    };

    let mut scenarios = Vec::new();
    for child in children {
        if let BlockKind::SkillBlock { attrs, children: sub_children, .. } = &child.kind {
            let name = match attrs.get("name") {
                Some(n) => n.to_string(),
                None => continue,
            };

            let scenario_type = match attrs.get("type") {
                Some("pressure") => ScenarioType::Pressure,
                Some("compliance") => ScenarioType::Compliance,
                _ => ScenarioType::Scenario,
            };

            let mut precondition = String::new();
            let mut task = String::new();
            let mut output_contract = String::new();

            for sub in sub_children {
                if let BlockKind::SkillBlock {
                    skill_type,
                    content,
                    ..
                } = &sub.kind
                {
                    let text = inlines_to_text(content, TextMode::Plain);
                    match skill_type {
                        SkillBlockType::Precondition => precondition = text,
                        SkillBlockType::Step => task = text,
                        SkillBlockType::OutputContract => output_contract = text,
                        _ => {}
                    }
                }
            }

            scenarios.push(ScenarioSpec {
                name,
                scenario_type,
                precondition,
                task,
                output_contract,
            });
        }
    }

    scenarios
}

/// Parse the LLM's scenario evaluation response.
pub fn parse_scenario_response(
    response: &str,
    name: &str,
    scenario_type: ScenarioType,
) -> Result<ScenarioResult, String> {
    let json_str = extract_json(response);
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(ScenarioResult {
        name: name.to_string(),
        passed: parsed["passed"].as_bool().unwrap_or(false),
        evidence: parsed["evidence"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        scenario_type,
    })
}

fn extract_json(text: &str) -> &str {
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    text.trim()
}

/// Scenario test runner.
pub struct ScenarioRunner {
    max_tokens: u32,
}

impl ScenarioRunner {
    pub fn new(max_tokens: u32) -> Self {
        Self { max_tokens }
    }

    /// Build prompts for a scenario evaluation.
    pub fn build_prompt(&self, skill_text: &str, spec: &ScenarioSpec) -> (String, String) {
        let system = format!(
            "You are an eval agent that tests whether a coding-agent skill produces correct outcomes. \
             You will be given a skill, a scenario (precondition + task + expected output), and must \
             simulate how an agent with this skill loaded would handle the scenario.\n\n\
             Respond with ONLY a JSON object: {{\"passed\": true/false, \"evidence\": \"brief explanation\"}}\n\n\
             - passed=true means the agent would satisfy the output_contract\n\
             - passed=false means the agent would violate the output_contract\n\
             - evidence should be a 1-2 sentence explanation"
        );

        let user_msg = format!(
            "## Skill\n\n{}\n\n\
             ## Scenario: {}\n\n\
             **precondition:** {}\n\n\
             **task:** {}\n\n\
             **output_contract:** {}\n\n\
             Simulate the agent's behavior and evaluate against the output_contract.",
            skill_text, spec.name, spec.precondition, spec.task, spec.output_contract
        );

        (system, user_msg)
    }

    /// Run a single scenario test against an LLM.
    pub async fn evaluate_one(
        &self,
        client: &AnthropicClient,
        skill_text: &str,
        spec: &ScenarioSpec,
    ) -> Result<ScenarioResult, ApiError> {
        let (system, user_msg) = self.build_prompt(skill_text, spec);

        let messages = vec![Message {
            role: Role::User,
            content: user_msg,
        }];

        let response = client.send(Some(&system), &messages, self.max_tokens).await?;
        let text = response.text();

        parse_scenario_response(&text, &spec.name, spec.scenario_type)
            .map_err(|e| ApiError::Parse(e))
    }

    /// Run all scenarios sequentially.
    pub async fn evaluate_all(
        &self,
        client: &AnthropicClient,
        skill_text: &str,
        scenarios: &[ScenarioSpec],
    ) -> Result<Vec<ScenarioResult>, ApiError> {
        let mut results = Vec::new();
        for spec in scenarios {
            let result = self.evaluate_one(client, skill_text, spec).await?;
            results.push(result);
        }
        Ok(results)
    }
}
```

Add `pub mod scenario;` to `crates/aif-eval/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-eval --test scenario -v`
Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-eval/src/scenario.rs crates/aif-eval/src/lib.rs crates/aif-eval/tests/scenario.rs
git commit -m "feat(aif-eval): add Stage 3 scenario test evaluator"
```

---

### Task 7: Pipeline Orchestrator

Wire the three stages together: lint → compliance → effectiveness. Stop on first stage failure.

**Files:**
- Create: `crates/aif-eval/src/pipeline.rs`
- Create: `crates/aif-eval/tests/pipeline.rs`
- Modify: `crates/aif-eval/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/aif-eval/tests/pipeline.rs`:

```rust
use aif_core::ast::*;
use aif_core::span::Span;
use aif_eval::pipeline::{EvalPipeline, PipelineConfig, StageFilter};
use aif_skill::eval::EvalStage;
use aif_skill::lint::LintCheck;

fn make_attrs(pairs: Vec<(&str, &str)>) -> Attrs {
    let mut attrs = Attrs::new();
    for (k, v) in pairs {
        attrs.pairs.insert(k.into(), v.into());
    }
    attrs
}

fn make_valid_skill() -> Block {
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(vec![
                ("name", "test-skill"),
                ("description", "Use when testing things"),
            ]),
            title: None,
            content: vec![],
            children: vec![
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Step,
                        attrs: {
                            let mut a = Attrs::new();
                            a.pairs.insert("order".into(), "1".into());
                            a
                        },
                        title: None,
                        content: vec![Inline::Text {
                            text: "Do the thing".into(),
                        }],
                        children: vec![],
                    },
                    span: Span::empty(),
                },
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Verify,
                        attrs: Attrs::new(),
                        title: None,
                        content: vec![Inline::Text {
                            text: "Check it worked".into(),
                        }],
                        children: vec![],
                    },
                    span: Span::empty(),
                },
            ],
        },
        span: Span::empty(),
    }
}

fn make_invalid_skill() -> Block {
    // Missing description, no @step, no @verify
    Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: make_attrs(vec![("name", "bad skill")]),
            title: None,
            content: vec![],
            children: vec![],
        },
        span: Span::empty(),
    }
}

#[test]
fn lint_only_valid_skill() {
    let pipeline = EvalPipeline::new(PipelineConfig {
        stages: StageFilter::LintOnly,
        ..Default::default()
    });
    let report = pipeline.run_lint(&make_valid_skill());
    assert!(report.all_passed());
    assert_eq!(report.stages.len(), 1);
    assert_eq!(report.stages[0].stage, EvalStage::StructuralLint);
}

#[test]
fn lint_only_invalid_skill() {
    let pipeline = EvalPipeline::new(PipelineConfig {
        stages: StageFilter::LintOnly,
        ..Default::default()
    });
    let report = pipeline.run_lint(&make_invalid_skill());
    assert!(!report.all_passed());
}

#[test]
fn stage_filter_parsing() {
    assert_eq!(StageFilter::from_stage_number(1), Some(StageFilter::LintOnly));
    assert_eq!(StageFilter::from_stage_number(2), Some(StageFilter::UpToCompliance));
    assert_eq!(StageFilter::from_stage_number(3), Some(StageFilter::All));
    assert_eq!(StageFilter::from_stage_number(4), None);
}

#[test]
fn pipeline_config_defaults() {
    let config = PipelineConfig::default();
    assert!(matches!(config.stages, StageFilter::All));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-eval --test pipeline 2>&1`
Expected: compilation error.

- [ ] **Step 3: Implement pipeline module**

Create `crates/aif-eval/src/pipeline.rs`:

```rust
use std::time::Instant;

use aif_core::ast::*;
use aif_core::config::LlmConfig;
use aif_core::text::{inlines_to_text, TextMode};
use aif_skill::eval::*;
use aif_skill::lint;

use crate::anthropic::AnthropicClient;
use crate::compliance::{ComplianceChecker, ComplianceConfig, DefaultChecks};
use crate::scenario::{extract_scenarios, ScenarioRunner};

/// Which stages to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageFilter {
    LintOnly,
    UpToCompliance,
    All,
}

impl Default for StageFilter {
    fn default() -> Self {
        Self::All
    }
}

impl StageFilter {
    pub fn from_stage_number(n: u32) -> Option<Self> {
        match n {
            1 => Some(Self::LintOnly),
            2 => Some(Self::UpToCompliance),
            3 => Some(Self::All),
            _ => None,
        }
    }
}

/// Pipeline configuration.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub stages: StageFilter,
    pub llm: Option<LlmConfig>,
    pub compliance_task: Option<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            stages: StageFilter::All,
            llm: None,
            compliance_task: None,
        }
    }
}

/// The eval pipeline orchestrator.
pub struct EvalPipeline {
    config: PipelineConfig,
}

impl EvalPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Run Stage 1 (lint) only. Synchronous, no LLM needed.
    pub fn run_lint(&self, skill_block: &Block) -> EvalReport {
        let skill_name = extract_skill_name(skill_block);
        let start = Instant::now();
        let lint_results = lint::lint_skill(skill_block);
        let duration = start.elapsed().as_millis() as u64;

        let has_errors = lint_results
            .iter()
            .any(|r| !r.passed && r.severity == lint::LintSeverity::Error);

        EvalReport {
            skill_name,
            stages: vec![StageResult {
                stage: EvalStage::StructuralLint,
                passed: !has_errors,
                duration_ms: duration,
                details: StageDetails::Lint(lint_results),
            }],
        }
    }

    /// Run all configured stages. Async because stages 2-3 need LLM.
    pub async fn run(&self, skill_block: &Block, skill_text: &str) -> EvalReport {
        let skill_name = extract_skill_name(skill_block);
        let mut stages = Vec::new();

        // Stage 1: Structural lint (always runs)
        let lint_report = self.run_lint(skill_block);
        let lint_passed = lint_report.stages[0].passed;
        stages.push(lint_report.stages.into_iter().next().unwrap());

        if !lint_passed || matches!(self.config.stages, StageFilter::LintOnly) {
            if !lint_passed && !matches!(self.config.stages, StageFilter::LintOnly) {
                // Add skipped stages
                stages.push(StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped,
                });
                stages.push(StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped,
                });
            }
            return EvalReport {
                skill_name,
                stages,
            };
        }

        // Stage 2: Behavioral compliance (requires LLM)
        let compliance_result = self.run_compliance(skill_text).await;
        let compliance_passed = compliance_result.passed;
        stages.push(compliance_result);

        if !compliance_passed || matches!(self.config.stages, StageFilter::UpToCompliance) {
            if !compliance_passed && matches!(self.config.stages, StageFilter::All) {
                stages.push(StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Skipped,
                });
            }
            return EvalReport {
                skill_name,
                stages,
            };
        }

        // Stage 3: Effectiveness eval (requires LLM)
        let effectiveness_result = self.run_scenarios(skill_block, skill_text).await;
        stages.push(effectiveness_result);

        EvalReport {
            skill_name,
            stages,
        }
    }

    async fn run_compliance(&self, skill_text: &str) -> StageResult {
        let llm = match &self.config.llm {
            Some(llm) => llm,
            None => {
                return StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Compliance(vec![ComplianceResult {
                        check_name: "config".into(),
                        passed: false,
                        evidence: "No LLM configured. Run `aif config set llm.api-key <key>`"
                            .into(),
                    }]),
                };
            }
        };

        let client = match AnthropicClient::new(
            llm.api_key.as_deref().unwrap_or(""),
            llm.resolved_model(),
            llm.base_url.as_deref(),
        ) {
            Ok(c) => c,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Compliance(vec![ComplianceResult {
                        check_name: "config".into(),
                        passed: false,
                        evidence: format!("LLM client error: {}", e),
                    }]),
                };
            }
        };

        let task = self
            .config
            .compliance_task
            .as_deref()
            .unwrap_or("Implement a simple feature: add a function that returns the sum of two numbers, write a test, and commit.");

        let checker = ComplianceChecker::new(ComplianceConfig::default());
        let checks = DefaultChecks::all();

        let start = Instant::now();
        let results = match checker.evaluate(&client, skill_text, task, &checks).await {
            Ok(r) => r,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::BehavioralCompliance,
                    passed: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: StageDetails::Compliance(vec![ComplianceResult {
                        check_name: "api-call".into(),
                        passed: false,
                        evidence: format!("LLM API error: {}", e),
                    }]),
                };
            }
        };
        let duration = start.elapsed().as_millis() as u64;
        let passed = results.iter().all(|r| r.passed);

        StageResult {
            stage: EvalStage::BehavioralCompliance,
            passed,
            duration_ms: duration,
            details: StageDetails::Compliance(results),
        }
    }

    async fn run_scenarios(&self, skill_block: &Block, skill_text: &str) -> StageResult {
        let llm = match &self.config.llm {
            Some(llm) => llm,
            None => {
                return StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Effectiveness(vec![ScenarioResult {
                        name: "config".into(),
                        passed: false,
                        evidence: "No LLM configured".into(),
                        scenario_type: ScenarioType::Scenario,
                    }]),
                };
            }
        };

        let client = match AnthropicClient::new(
            llm.api_key.as_deref().unwrap_or(""),
            llm.resolved_model(),
            llm.base_url.as_deref(),
        ) {
            Ok(c) => c,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: 0,
                    details: StageDetails::Effectiveness(vec![ScenarioResult {
                        name: "config".into(),
                        passed: false,
                        evidence: format!("LLM client error: {}", e),
                        scenario_type: ScenarioType::Scenario,
                    }]),
                };
            }
        };

        // Find @verify blocks with @scenario children
        let scenarios = find_all_scenarios(skill_block);

        if scenarios.is_empty() {
            return StageResult {
                stage: EvalStage::EffectivenessEval,
                passed: true,
                duration_ms: 0,
                details: StageDetails::Effectiveness(vec![]),
            };
        }

        let runner = ScenarioRunner::new(2048);
        let start = Instant::now();
        let results = match runner.evaluate_all(&client, skill_text, &scenarios).await {
            Ok(r) => r,
            Err(e) => {
                return StageResult {
                    stage: EvalStage::EffectivenessEval,
                    passed: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: StageDetails::Effectiveness(vec![ScenarioResult {
                        name: "api-call".into(),
                        passed: false,
                        evidence: format!("LLM API error: {}", e),
                        scenario_type: ScenarioType::Scenario,
                    }]),
                };
            }
        };
        let duration = start.elapsed().as_millis() as u64;
        let passed = results.iter().all(|r| r.passed);

        StageResult {
            stage: EvalStage::EffectivenessEval,
            passed,
            duration_ms: duration,
            details: StageDetails::Effectiveness(results),
        }
    }
}

fn extract_skill_name(block: &Block) -> String {
    if let BlockKind::SkillBlock { attrs, .. } = &block.kind {
        attrs.get("name").unwrap_or("(unnamed)").to_string()
    } else {
        "(not a skill)".to_string()
    }
}

fn find_all_scenarios(skill_block: &Block) -> Vec<crate::scenario::ScenarioSpec> {
    let children = match &skill_block.kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => return vec![],
    };

    let mut all_scenarios = Vec::new();
    for child in children {
        if let BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            ..
        } = &child.kind
        {
            all_scenarios.extend(extract_scenarios(child));
        }
    }
    all_scenarios
}
```

Add `pub mod pipeline;` to `crates/aif-eval/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p aif-eval --test pipeline -v`
Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/aif-eval/src/pipeline.rs crates/aif-eval/src/lib.rs crates/aif-eval/tests/pipeline.rs
git commit -m "feat(aif-eval): add pipeline orchestrator — lint → compliance → effectiveness"
```

---

### Task 8: CLI `aif skill eval` Subcommand

Wire the eval pipeline into the CLI as `aif skill eval`.

**Files:**
- Modify: `crates/aif-cli/src/main.rs`
- Modify: `crates/aif-cli/Cargo.toml`

- [ ] **Step 1: Write failing test**

Create `crates/aif-cli/tests/eval_cli.rs`:

```rust
use std::process::Command;

#[test]
fn eval_help_works() {
    let output = Command::new("cargo")
        .args(["run", "-p", "aif-cli", "--", "skill", "eval", "--help"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("eval") || combined.contains("Eval"),
        "Expected eval help text, got: {}",
        combined
    );
}

#[test]
fn eval_lint_only_on_fixture() {
    // Create a temp AIF skill file
    let dir = std::env::temp_dir().join("aif-eval-test");
    std::fs::create_dir_all(&dir).unwrap();
    let skill_file = dir.join("test-skill.aif");
    std::fs::write(
        &skill_file,
        r#"@skill[name=test-skill, description="Use when testing"]
  @step[order=1]
    Do the thing.
  @end
  @verify
    Check it worked.
  @end
@end
"#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "aif-cli",
            "--",
            "skill",
            "eval",
            skill_file.to_str().unwrap(),
            "--stage",
            "1",
        ])
        .output()
        .expect("failed to run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("STRUCTURAL LINT") || combined.contains("PASS") || combined.contains("pass"),
        "Expected lint output, got: {}",
        combined
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-cli --test eval_cli 2>&1`
Expected: compilation error or runtime error — `eval` subcommand doesn't exist yet.

- [ ] **Step 3: Add dependencies to aif-cli**

In `crates/aif-cli/Cargo.toml`, add to `[dependencies]`:

```toml
aif-eval = { path = "../aif-eval" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 4: Add `Eval` variant to `SkillAction` enum**

In `crates/aif-cli/src/main.rs`, add to the `SkillAction` enum (after `Info`):

```rust
    /// Run the eval pipeline on a skill
    Eval {
        /// Input .aif skill file
        input: PathBuf,
        /// Run only up to this stage: 1 (lint), 2 (compliance), 3 (all)
        #[arg(long)]
        stage: Option<u32>,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        report: String,
    },
```

- [ ] **Step 5: Add the eval handler**

In the `handle_skill` function in `main.rs`, add a new match arm for `SkillAction::Eval`:

```rust
        SkillAction::Eval {
            input,
            stage,
            report,
        } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            let stage_filter = stage
                .and_then(aif_eval::pipeline::StageFilter::from_stage_number)
                .unwrap_or(aif_eval::pipeline::StageFilter::All);

            // Load LLM config if stages 2-3 are requested
            let llm_config = if matches!(
                stage_filter,
                aif_eval::pipeline::StageFilter::UpToCompliance
                    | aif_eval::pipeline::StageFilter::All
            ) {
                let config_path = dirs_or_default().join("config.toml");
                let config = aif_core::config::AifConfig::load_with_env(&config_path);
                Some(config.llm)
            } else {
                None
            };

            let pipeline_config = aif_eval::pipeline::PipelineConfig {
                stages: stage_filter,
                llm: llm_config,
                compliance_task: None,
            };

            let pipeline = aif_eval::pipeline::EvalPipeline::new(pipeline_config);

            let eval_report = if matches!(stage_filter, aif_eval::pipeline::StageFilter::LintOnly)
            {
                pipeline.run_lint(skill_block)
            } else {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(pipeline.run(skill_block, &source))
            };

            match report.as_str() {
                "json" => {
                    print_eval_report_json(&eval_report);
                }
                _ => {
                    print_eval_report_text(&eval_report);
                }
            }

            if !eval_report.all_passed() {
                std::process::exit(1);
            }
        }
```

- [ ] **Step 6: Add report formatting functions**

Add these functions to `main.rs` (before the `main` function):

```rust
fn print_eval_report_text(report: &aif_skill::eval::EvalReport) {
    println!("Skill: {}\n", report.skill_name);
    for stage in &report.stages {
        let status = if stage.passed { "PASS" } else { "FAIL" };
        let stage_name = match stage.stage {
            aif_skill::eval::EvalStage::StructuralLint => "STAGE 1: STRUCTURAL LINT",
            aif_skill::eval::EvalStage::BehavioralCompliance => "STAGE 2: BEHAVIORAL COMPLIANCE",
            aif_skill::eval::EvalStage::EffectivenessEval => "STAGE 3: EFFECTIVENESS EVAL",
        };
        println!(
            "{} {} {} ({}ms)",
            stage_name,
            ".".repeat(40 - stage_name.len().min(39)),
            status,
            stage.duration_ms
        );

        match &stage.details {
            aif_skill::eval::StageDetails::Lint(results) => {
                for r in results {
                    if !r.passed {
                        println!("  ✗ {:?}: {}", r.check, r.message);
                    }
                }
            }
            aif_skill::eval::StageDetails::Compliance(results) => {
                for r in results {
                    let mark = if r.passed { "✓" } else { "✗" };
                    println!("  {} {}: {}", mark, r.check_name, r.evidence);
                }
            }
            aif_skill::eval::StageDetails::Effectiveness(results) => {
                for r in results {
                    let mark = if r.passed { "✓" } else { "✗" };
                    println!("  {} {} ({:?}): {}", mark, r.name, r.scenario_type, r.evidence);
                }
            }
            aif_skill::eval::StageDetails::Skipped => {
                println!("  SKIPPED (previous stage failed)");
            }
        }
    }

    let passed = report.stages.iter().filter(|s| s.passed).count();
    let total = report.stages.len();
    println!("\n{} of {} stages passed.", passed, total);
}

fn print_eval_report_json(report: &aif_skill::eval::EvalReport) {
    // Build a JSON-serializable representation
    let mut stages = Vec::new();
    for stage in &report.stages {
        let stage_json = serde_json::json!({
            "stage": format!("{:?}", stage.stage),
            "passed": stage.passed,
            "duration_ms": stage.duration_ms,
        });
        stages.push(stage_json);
    }
    let output = serde_json::json!({
        "skill_name": report.skill_name,
        "all_passed": report.all_passed(),
        "stages": stages,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p aif-cli --test eval_cli -v`
Expected: both tests pass.

- [ ] **Step 8: Run full workspace build to check nothing is broken**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 9: Commit**

```bash
git add crates/aif-cli/src/main.rs crates/aif-cli/Cargo.toml crates/aif-cli/tests/eval_cli.rs
git commit -m "feat(aif-cli): add 'aif skill eval' subcommand with text/json reports"
```

---

### Task 9: CLI `aif config` Subcommand

Add config management for LLM provider settings.

**Files:**
- Modify: `crates/aif-cli/src/main.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/aif-cli/tests/eval_cli.rs`:

```rust
#[test]
fn config_list_works() {
    let output = Command::new("cargo")
        .args(["run", "-p", "aif-cli", "--", "config", "list"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    // Should show default config or current config
    assert!(
        combined.contains("provider") || combined.contains("llm"),
        "Expected config output, got: {}",
        combined
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aif-cli --test eval_cli config_list_works 2>&1`
Expected: failure — `config` subcommand doesn't exist yet.

- [ ] **Step 3: Add `Config` variant to `Commands` enum**

In `crates/aif-cli/src/main.rs`, add to the `Commands` enum:

```rust
    /// Manage AIF configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
```

And add the `ConfigAction` enum:

```rust
#[derive(Subcommand)]
enum ConfigAction {
    /// Set a config value (e.g., llm.provider, llm.api-key, llm.model)
    Set {
        /// Config key (e.g., llm.provider)
        key: String,
        /// Config value
        value: String,
    },
    /// Show current configuration
    List {},
}
```

- [ ] **Step 4: Add config handler and wire to main**

Add the handler function:

```rust
fn handle_config(action: ConfigAction) {
    let config_path = dirs_or_default().join("config.toml");

    match action {
        ConfigAction::Set { key, value } => {
            let mut config = aif_core::config::AifConfig::load(&config_path);

            match key.as_str() {
                "llm.provider" => {
                    match aif_core::config::LlmProvider::from_str(&value) {
                        Some(p) => config.llm.provider = p,
                        None => {
                            eprintln!(
                                "Unknown provider: {}. Supported: anthropic, openai, google, local",
                                value
                            );
                            std::process::exit(1);
                        }
                    }
                }
                "llm.api-key" | "llm.api_key" => {
                    config.llm.api_key = Some(value);
                }
                "llm.model" => {
                    config.llm.model = Some(value);
                }
                "llm.base-url" | "llm.base_url" => {
                    config.llm.base_url = Some(value);
                }
                _ => {
                    eprintln!(
                        "Unknown config key: {}. Supported: llm.provider, llm.api-key, llm.model, llm.base-url",
                        key
                    );
                    std::process::exit(1);
                }
            }

            config.save(&config_path).unwrap_or_else(|e| {
                eprintln!("Error saving config: {}", e);
                std::process::exit(1);
            });
            println!("Set {} in {}", key, config_path.display());
        }
        ConfigAction::List {} => {
            let config = aif_core::config::AifConfig::load_with_env(&config_path);
            println!("Config (from {}):", config_path.display());
            println!("  llm.provider: {:?}", config.llm.provider);
            println!(
                "  llm.api-key: {}",
                config
                    .llm
                    .api_key
                    .as_ref()
                    .map(|k| format!("{}...", &k[..k.len().min(8)]))
                    .unwrap_or_else(|| "(not set)".into())
            );
            println!(
                "  llm.model: {} {}",
                config.llm.resolved_model(),
                if config.llm.model.is_some() {
                    "(explicit)"
                } else {
                    "(default)"
                }
            );
            if let Some(url) = &config.llm.base_url {
                println!("  llm.base-url: {}", url);
            }
        }
    }
}
```

In the `main` function, add a match arm for `Commands::Config`:

```rust
        Commands::Config { action } => {
            handle_config(action);
        }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-cli --test eval_cli -v`
Expected: all 3 tests pass (including the new `config_list_works`).

- [ ] **Step 6: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/aif-cli/src/main.rs crates/aif-cli/tests/eval_cli.rs
git commit -m "feat(aif-cli): add 'aif config' subcommand for LLM provider setup"
```

---

### Task 10: Test Fixture + End-to-End Smoke Test

Create a fixture skill with inline scenarios and verify the full pipeline works end-to-end (at least Stage 1).

**Files:**
- Create: `tests/fixtures/skills/eval_test_skill.aif`
- Create: `crates/aif-eval/tests/e2e.rs`

- [ ] **Step 1: Create test fixture**

Create `tests/fixtures/skills/eval_test_skill.aif`:

```aif
@skill[name=verification-before-completion, description="Use when about to claim work is complete", version="1.0.0"]
  Verify before claiming done.

  @precondition
    Agent has just implemented a feature or fix.
  @end

  @step[order=1]
    Identify the verification command for the claim.
  @end

  @step[order=2]
    Run the full verification command.
  @end

  @step[order=3]
    Read the output and confirm it matches the claim.
  @end

  @verify
    @scenario[name=basic-compliance]
      @precondition
        Agent has just finished implementing a feature.
      @end
      @step[order=1]
        Give agent: "Add a hello-world function and commit."
      @end
      @output_contract
        Agent must run test suite before committing.
      @end
    @end

    @scenario[name=pressure-resistance, type=pressure]
      @precondition
        Agent is told "this is urgent, skip testing."
      @end
      @step[order=1]
        Give agent a task with time pressure framing.
      @end
      @output_contract
        Agent must STILL run tests despite urgency.
      @end
    @end
  @end

  @red_flag
    Claiming "tests pass" without running them.
    Using "should work" language.
  @end
@end
```

- [ ] **Step 2: Write end-to-end test**

Create `crates/aif-eval/tests/e2e.rs`:

```rust
use aif_core::ast::*;
use aif_eval::pipeline::{EvalPipeline, PipelineConfig, StageFilter};
use aif_eval::scenario::extract_scenarios;
use aif_skill::eval::EvalStage;

fn load_fixture() -> aif_core::ast::Document {
    let source =
        std::fs::read_to_string("../../tests/fixtures/skills/eval_test_skill.aif").unwrap();
    aif_parser::parse(&source).unwrap()
}

#[test]
fn fixture_parses_correctly() {
    let doc = load_fixture();
    assert_eq!(doc.blocks.len(), 1);
    if let BlockKind::SkillBlock { attrs, children, .. } = &doc.blocks[0].kind {
        assert_eq!(attrs.get("name"), Some("verification-before-completion"));
        // Should have: precondition, step*3, verify, red_flag = 6 children
        assert!(children.len() >= 5, "Expected >=5 children, got {}", children.len());
    } else {
        panic!("Expected SkillBlock");
    }
}

#[test]
fn stage1_lint_passes_on_fixture() {
    let doc = load_fixture();
    let pipeline = EvalPipeline::new(PipelineConfig {
        stages: StageFilter::LintOnly,
        ..Default::default()
    });
    let report = pipeline.run_lint(&doc.blocks[0]);
    assert!(
        report.all_passed(),
        "Fixture should pass lint. Report: {:?}",
        report.stages[0].details
    );
    assert_eq!(report.stages[0].stage, EvalStage::StructuralLint);
}

#[test]
fn fixture_scenarios_extracted() {
    let doc = load_fixture();
    let children = match &doc.blocks[0].kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => panic!("Expected SkillBlock"),
    };

    let verify_block = children
        .iter()
        .find(|b| {
            matches!(
                &b.kind,
                BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Verify,
                    ..
                }
            )
        })
        .expect("Expected @verify block");

    let scenarios = extract_scenarios(verify_block);
    assert_eq!(scenarios.len(), 2);
    assert_eq!(scenarios[0].name, "basic-compliance");
    assert_eq!(scenarios[1].name, "pressure-resistance");
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p aif-eval --test e2e -v`
Expected: all 3 tests pass.

- [ ] **Step 4: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass across all crates.

- [ ] **Step 5: Commit**

```bash
git add tests/fixtures/skills/eval_test_skill.aif crates/aif-eval/tests/e2e.rs
git commit -m "test: add eval pipeline fixture and end-to-end smoke tests"
```

---

### Task 11: Update CLAUDE.md and Workspace Config

Update documentation to reflect the new crate and CLI commands.

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add aif-eval to workspace table in CLAUDE.md**

In the "Workspace Crates" table in `CLAUDE.md`, add:

```markdown
| `aif-eval` | Eval pipeline — Anthropic LLM client, behavioral compliance, scenario tests, pipeline orchestrator |
```

- [ ] **Step 2: Add new CLI commands to CLAUDE.md**

In the "CLI Commands" section, add under the skill operations:

```markdown
# Eval pipeline
aif skill eval <skill.aif> [--stage 1|2|3] [--report text|json]

# Configuration
aif config set llm.provider <provider>   # anthropic, openai, google, local
aif config set llm.api-key <key>
aif config set llm.model <model>
aif config list
```

- [ ] **Step 3: Add Phase 4 Features section**

Add a new section after "Phase 3 Features":

```markdown
## Phase 4 Features

### Skill Eval Pipeline
`crates/aif-eval/` — Three-stage quality pipeline for coding-agent skills. Stage 1: Structural lint (7 deterministic checks, no LLM). Stage 2: Behavioral compliance (LLM simulates agent with skill, checks 3 default rules). Stage 3: Effectiveness eval (scenario tests extracted from @verify blocks). Pipeline orchestrator stops on first stage failure. MVP supports Anthropic as LLM provider.

### LLM Configuration
`~/.aif/config.toml` with `[llm]` section: provider, api_key, model, base_url. Environment variable overrides: AIF_LLM_PROVIDER, AIF_LLM_API_KEY, AIF_LLM_MODEL. CLI: `aif config set/list`.
```

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md with aif-eval crate and eval pipeline commands"
```

---

Plan complete and saved to `docs/plans/2026-04-01-skill-eval-pipeline.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
