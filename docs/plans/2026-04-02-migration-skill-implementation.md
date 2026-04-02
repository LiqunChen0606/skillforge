# Migration Skill System — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build `aif-migrate` crate — a chunked migration engine that applies typed migration skills to codebases with repair loops and structured reporting.

**Architecture:** New `aif-migrate` workspace crate following `aif-eval` pipeline pattern. Extends `aif-skill` lint for migration profile validation. Reuses `aif-core` config/types. CLI integration via new `Migrate` subcommand.

**Tech Stack:** Rust, tokio (async LLM), reqwest (HTTP), serde (serialization), aif-core/aif-skill/aif-eval dependencies.

**Design doc:** `docs/plans/2026-04-02-migration-skill-system-design.md`

---

### Task 1: Scaffold `aif-migrate` Crate

**Files:**
- Create: `crates/aif-migrate/Cargo.toml`
- Create: `crates/aif-migrate/src/lib.rs`
- Create: `crates/aif-migrate/src/types.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/smoke.rs`:

```rust
use aif_migrate::types::{MigrationConfig, ChunkStatus, ChunkResult, VerificationResult};
use std::path::PathBuf;

#[test]
fn chunk_status_default_variants() {
    // Verify all ChunkStatus variants exist and can be matched
    let statuses = vec![
        ChunkStatus::Success,
        ChunkStatus::PartialSuccess,
        ChunkStatus::Failed,
        ChunkStatus::Skipped,
    ];
    assert_eq!(statuses.len(), 4);
}

#[test]
fn chunk_result_has_expected_fields() {
    let result = ChunkResult {
        chunk_id: "test-001".to_string(),
        files: vec![PathBuf::from("src/main.rs")],
        status: ChunkStatus::Success,
        confidence: 0.95,
        verification: VerificationResult {
            static_checks: vec![],
            semantic_checks: vec![],
            passed: true,
        },
        repair_iterations: 0,
        notes: vec!["Clean migration".to_string()],
    };
    assert_eq!(result.chunk_id, "test-001");
    assert!(result.confidence > 0.9);
    assert!(result.verification.passed);
}

#[test]
fn migration_config_construction() {
    let config = MigrationConfig {
        skill_path: PathBuf::from("skill.aif"),
        source_dir: PathBuf::from("./src"),
        output_dir: PathBuf::from("./migrated"),
        max_repair_iterations: 3,
        file_patterns: vec!["*.rs".to_string()],
    };
    assert_eq!(config.max_repair_iterations, 3);
    assert_eq!(config.file_patterns.len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate`
Expected: FAIL — crate doesn't exist yet.

**Step 3: Add crate to workspace**

Add to root `Cargo.toml` workspace members:
```toml
"crates/aif-migrate",
```

Add workspace dependency:
```toml
aif-migrate = { path = "crates/aif-migrate" }
```

**Step 4: Create `Cargo.toml`**

```toml
[package]
name = "aif-migrate"
version.workspace = true
edition.workspace = true

[dependencies]
aif-core = { workspace = true }
aif-skill = { path = "../aif-skill" }
aif-eval = { path = "../aif-eval" }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["rt", "macros"] }

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
aif-parser = { path = "../aif-parser" }
```

**Step 5: Create `src/lib.rs`**

```rust
pub mod types;
```

**Step 6: Create `src/types.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for a migration run.
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub skill_path: PathBuf,
    pub source_dir: PathBuf,
    pub output_dir: PathBuf,
    pub max_repair_iterations: u32,
    pub file_patterns: Vec<String>,
}

/// Result for a single chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResult {
    pub chunk_id: String,
    pub files: Vec<PathBuf>,
    pub status: ChunkStatus,
    pub confidence: f64,
    pub verification: VerificationResult,
    pub repair_iterations: u32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkStatus {
    Success,
    PartialSuccess,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub static_checks: Vec<StaticCheck>,
    pub semantic_checks: Vec<SemanticCheck>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCheck {
    pub criterion: String,
    pub passed: bool,
    pub reasoning: String,
    pub confidence: f64,
}

/// Full migration report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub skill_name: String,
    pub source_dir: PathBuf,
    pub chunks: Vec<ChunkResult>,
    pub overall_confidence: f64,
    pub unresolved: Vec<String>,
    pub manual_review: Vec<String>,
    #[serde(with = "duration_serde")]
    pub duration: Duration,
}

impl MigrationReport {
    pub fn all_passed(&self) -> bool {
        self.chunks.iter().all(|c| c.status == ChunkStatus::Success)
    }

    pub fn success_rate(&self) -> f64 {
        if self.chunks.is_empty() {
            return 0.0;
        }
        let successes = self.chunks.iter()
            .filter(|c| matches!(c.status, ChunkStatus::Success | ChunkStatus::PartialSuccess))
            .count();
        successes as f64 / self.chunks.len() as f64
    }
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_millis().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}
```

**Step 7: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: 3 tests pass.

**Step 8: Commit**

```bash
git add crates/aif-migrate/ Cargo.toml
git commit -m "feat(aif-migrate): scaffold crate with core types"
```

---

### Task 2: Migration Profile Validation

**Files:**
- Create: `crates/aif-migrate/src/validate.rs`
- Modify: `crates/aif-migrate/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/validate.rs`:

```rust
use aif_parser::parse;
use aif_migrate::validate::{validate_migration_skill, MigrationLintCheck, MigrationLintResult};

fn parse_skill(source: &str) -> aif_core::ast::Document {
    parse(source).expect("parse failed")
}

#[test]
fn valid_migration_skill_passes() {
    let source = r#"
#title: Test Migration

@skill[name="test-migrate", version="1.0", profile=migration]
  @precondition
    Source uses framework X.
  @end

  @step[order=1]
    Replace X with Y.
  @end

  @verify
    No remaining X references.
  @end

  @output_contract
    All files use Y.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    assert!(results.iter().all(|r| r.passed), "All checks should pass: {:?}", results);
}

#[test]
fn missing_precondition_fails() {
    let source = r#"
#title: Bad Migration

@skill[name="bad-migrate", version="1.0", profile=migration]
  @step[order=1]
    Do something.
  @end

  @verify
    Check something.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let precondition_check = results.iter().find(|r| r.check == MigrationLintCheck::HasPrecondition).unwrap();
    assert!(!precondition_check.passed);
}

#[test]
fn missing_steps_fails() {
    let source = r#"
#title: No Steps

@skill[name="no-steps", version="1.0", profile=migration]
  @precondition
    Has framework.
  @end

  @verify
    Check it.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let step_check = results.iter().find(|r| r.check == MigrationLintCheck::HasSteps).unwrap();
    assert!(!step_check.passed);
}

#[test]
fn missing_verify_fails() {
    let source = r#"
#title: No Verify

@skill[name="no-verify", version="1.0", profile=migration]
  @precondition
    Has framework.
  @end

  @step[order=1]
    Migrate it.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let verify_check = results.iter().find(|r| r.check == MigrationLintCheck::HasVerify).unwrap();
    assert!(!verify_check.passed);
}

#[test]
fn missing_output_contract_fails() {
    let source = r#"
#title: No Output

@skill[name="no-output", version="1.0", profile=migration]
  @precondition
    Has framework.
  @end

  @step[order=1]
    Migrate it.
  @end

  @verify
    Check it.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let contract_check = results.iter().find(|r| r.check == MigrationLintCheck::HasOutputContract).unwrap();
    assert!(!contract_check.passed);
}

#[test]
fn not_a_migration_profile_fails() {
    let source = r#"
#title: Regular Skill

@skill[name="regular", version="1.0"]
  @precondition
    Something.
  @end

  @step[order=1]
    Do something.
  @end

  @verify
    Check it.
  @end
@end
"#;
    let doc = parse_skill(source);
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();
    let results = validate_migration_skill(skill_block);
    let profile_check = results.iter().find(|r| r.check == MigrationLintCheck::HasMigrationProfile).unwrap();
    assert!(!profile_check.passed);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate --test validate`
Expected: FAIL — `validate` module doesn't exist.

**Step 3: Implement `validate.rs`**

```rust
use aif_core::ast::{Block, BlockKind, SkillBlockType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationLintCheck {
    HasMigrationProfile,
    HasPrecondition,
    HasSteps,
    HasVerify,
    HasOutputContract,
}

#[derive(Debug, Clone)]
pub struct MigrationLintResult {
    pub check: MigrationLintCheck,
    pub passed: bool,
    pub message: String,
}

/// Validate that a skill block conforms to the migration profile requirements.
pub fn validate_migration_skill(skill_block: &Block) -> Vec<MigrationLintResult> {
    let mut results = Vec::new();

    // Check migration profile attribute
    let has_profile = skill_block.attrs.as_ref()
        .map(|a| a.pairs.iter().any(|(k, v)| k == "profile" && v == "migration"))
        .unwrap_or(false);
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasMigrationProfile,
        passed: has_profile,
        message: if has_profile {
            "Skill has profile=migration attribute".to_string()
        } else {
            "Skill missing profile=migration attribute".to_string()
        },
    });

    // Extract children from Skill block
    let children = match &skill_block.kind {
        BlockKind::Skill { children, .. } => children,
        _ => return results,
    };

    let has_precondition = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { block_type: SkillBlockType::Precondition, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasPrecondition,
        passed: has_precondition,
        message: if has_precondition {
            "@precondition block present".to_string()
        } else {
            "Missing @precondition block — migration skills must specify when to apply".to_string()
        },
    });

    let has_steps = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { block_type: SkillBlockType::Step, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasSteps,
        passed: has_steps,
        message: if has_steps {
            "At least one @step block present".to_string()
        } else {
            "Missing @step blocks — migration skills must have at least one step".to_string()
        },
    });

    let has_verify = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { block_type: SkillBlockType::Verify, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasVerify,
        passed: has_verify,
        message: if has_verify {
            "@verify block present".to_string()
        } else {
            "Missing @verify block — migration skills must define verification criteria".to_string()
        },
    });

    let has_output_contract = children.iter().any(|b| matches!(&b.kind,
        BlockKind::SkillBlock { block_type: SkillBlockType::OutputContract, .. }));
    results.push(MigrationLintResult {
        check: MigrationLintCheck::HasOutputContract,
        passed: has_output_contract,
        message: if has_output_contract {
            "@output_contract block present".to_string()
        } else {
            "Missing @output_contract block — migration skills must define success criteria".to_string()
        },
    });

    results
}
```

**Step 4: Update `lib.rs`**

```rust
pub mod types;
pub mod validate;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/aif-migrate/
git commit -m "feat(aif-migrate): add migration profile validation"
```

---

### Task 3: Source File Chunking

**Files:**
- Create: `crates/aif-migrate/src/chunk.rs`
- Modify: `crates/aif-migrate/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/chunk.rs`:

```rust
use aif_migrate::chunk::{SourceChunk, ChunkStrategy, chunk_source_files};
use std::path::PathBuf;
use std::collections::HashMap;

fn make_files(entries: &[(&str, &str)]) -> HashMap<PathBuf, String> {
    entries.iter().map(|(p, c)| (PathBuf::from(p), c.to_string())).collect()
}

#[test]
fn file_per_chunk_creates_one_chunk_per_file() {
    let files = make_files(&[
        ("src/a.rs", "fn a() {}"),
        ("src/b.rs", "fn b() {}"),
        ("src/c.rs", "fn c() {}"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::FilePerChunk);
    assert_eq!(chunks.len(), 3);
    for chunk in &chunks {
        assert_eq!(chunk.files.len(), 1);
    }
}

#[test]
fn directory_chunk_groups_by_parent_dir() {
    let files = make_files(&[
        ("src/components/a.rs", "fn a() {}"),
        ("src/components/b.rs", "fn b() {}"),
        ("src/utils/c.rs", "fn c() {}"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::DirectoryChunk);
    assert_eq!(chunks.len(), 2);
    let comp_chunk = chunks.iter().find(|c| c.chunk_id.contains("components")).unwrap();
    assert_eq!(comp_chunk.files.len(), 2);
}

#[test]
fn token_budget_respects_limit() {
    // Each file has ~10 tokens. Budget of 25 should create 2 chunks for 3 files.
    let files = make_files(&[
        ("a.rs", "fn a() { let x = 1; }"),
        ("b.rs", "fn b() { let y = 2; }"),
        ("c.rs", "fn c() { let z = 3; }"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::TokenBudget { max_tokens: 25 });
    assert!(chunks.len() >= 2, "Should split into multiple chunks, got {}", chunks.len());
    for chunk in &chunks {
        let total_tokens: usize = chunk.files.iter()
            .map(|(_, content)| estimate_tokens(content))
            .sum();
        assert!(total_tokens <= 25, "Chunk exceeds token budget: {}", total_tokens);
    }
}

fn estimate_tokens(text: &str) -> usize {
    // Match the crate's BPE estimate
    (text.split_whitespace().count() as f64 * 1.3).ceil() as usize
}

#[test]
fn empty_files_returns_empty_chunks() {
    let files: HashMap<PathBuf, String> = HashMap::new();
    let chunks = chunk_source_files(&files, ChunkStrategy::FilePerChunk);
    assert!(chunks.is_empty());
}

#[test]
fn chunk_ids_are_unique() {
    let files = make_files(&[
        ("a.rs", "fn a() {}"),
        ("b.rs", "fn b() {}"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::FilePerChunk);
    let ids: Vec<&str> = chunks.iter().map(|c| c.chunk_id.as_str()).collect();
    let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), unique.len(), "Chunk IDs must be unique");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate --test chunk`
Expected: FAIL — `chunk` module doesn't exist.

**Step 3: Implement `chunk.rs`**

```rust
use std::collections::HashMap;
use std::path::PathBuf;

const BPE_TOKENS_PER_WORD: f64 = 1.3;

#[derive(Debug, Clone)]
pub enum ChunkStrategy {
    FilePerChunk,
    DirectoryChunk,
    TokenBudget { max_tokens: usize },
}

#[derive(Debug, Clone)]
pub struct SourceChunk {
    pub chunk_id: String,
    /// (file_path, content) pairs in this chunk.
    pub files: Vec<(PathBuf, String)>,
}

pub fn estimate_tokens(text: &str) -> usize {
    (text.split_whitespace().count() as f64 * BPE_TOKENS_PER_WORD).ceil() as usize
}

pub fn chunk_source_files(
    files: &HashMap<PathBuf, String>,
    strategy: ChunkStrategy,
) -> Vec<SourceChunk> {
    if files.is_empty() {
        return Vec::new();
    }

    // Sort for deterministic ordering
    let mut sorted: Vec<_> = files.iter().collect();
    sorted.sort_by_key(|(p, _)| p.clone());

    match strategy {
        ChunkStrategy::FilePerChunk => {
            sorted.iter().enumerate().map(|(i, (path, content))| {
                SourceChunk {
                    chunk_id: format!("file-{:04}-{}", i, path.display()),
                    files: vec![((*path).clone(), content.to_string())],
                }
            }).collect()
        }
        ChunkStrategy::DirectoryChunk => {
            let mut by_dir: HashMap<String, Vec<(PathBuf, String)>> = HashMap::new();
            for (path, content) in &sorted {
                let dir = path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string());
                by_dir.entry(dir).or_default().push(((*path).clone(), content.to_string()));
            }
            let mut dirs: Vec<_> = by_dir.into_iter().collect();
            dirs.sort_by(|a, b| a.0.cmp(&b.0));
            dirs.into_iter().enumerate().map(|(i, (dir, files))| {
                SourceChunk {
                    chunk_id: format!("dir-{:04}-{}", i, dir),
                    files,
                }
            }).collect()
        }
        ChunkStrategy::TokenBudget { max_tokens } => {
            let mut chunks = Vec::new();
            let mut current_files = Vec::new();
            let mut current_tokens = 0usize;

            for (path, content) in &sorted {
                let file_tokens = estimate_tokens(content);
                if !current_files.is_empty() && current_tokens + file_tokens > max_tokens {
                    chunks.push(SourceChunk {
                        chunk_id: format!("budget-{:04}", chunks.len()),
                        files: std::mem::take(&mut current_files),
                    });
                    current_tokens = 0;
                }
                current_files.push(((*path).clone(), content.to_string()));
                current_tokens += file_tokens;
            }
            if !current_files.is_empty() {
                chunks.push(SourceChunk {
                    chunk_id: format!("budget-{:04}", chunks.len()),
                    files: current_files,
                });
            }
            chunks
        }
    }
}
```

**Step 4: Update `lib.rs`**

```rust
pub mod types;
pub mod validate;
pub mod chunk;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/aif-migrate/
git commit -m "feat(aif-migrate): add source file chunking with 3 strategies"
```

---

### Task 4: Static Verification

**Files:**
- Create: `crates/aif-migrate/src/verify.rs`
- Modify: `crates/aif-migrate/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/verify.rs`:

```rust
use aif_migrate::verify::{run_static_checks, StaticCheckSpec};
use aif_migrate::types::StaticCheck;

#[test]
fn pattern_absence_check_passes_when_pattern_missing() {
    let content = "import { describe } from 'vitest';\nvi.fn();";
    let spec = StaticCheckSpec::PatternAbsence {
        name: "no jest calls".to_string(),
        pattern: "jest\\.".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert_eq!(result.len(), 1);
    assert!(result[0].passed);
}

#[test]
fn pattern_absence_check_fails_when_pattern_present() {
    let content = "import jest from 'jest';\njest.fn();";
    let spec = StaticCheckSpec::PatternAbsence {
        name: "no jest calls".to_string(),
        pattern: "jest\\.".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(!result[0].passed);
    assert!(result[0].detail.contains("jest."));
}

#[test]
fn pattern_presence_check_passes_when_found() {
    let content = "import { describe } from 'vitest';";
    let spec = StaticCheckSpec::PatternPresence {
        name: "vitest imports".to_string(),
        pattern: "from 'vitest'".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(result[0].passed);
}

#[test]
fn pattern_presence_check_fails_when_missing() {
    let content = "import { describe } from 'jest';";
    let spec = StaticCheckSpec::PatternPresence {
        name: "vitest imports".to_string(),
        pattern: "from 'vitest'".to_string(),
    };
    let result = run_static_checks(content, &[spec]);
    assert!(!result[0].passed);
}

#[test]
fn multiple_checks_run_independently() {
    let content = "import { vi } from 'vitest';\nvi.fn();";
    let specs = vec![
        StaticCheckSpec::PatternPresence {
            name: "has vitest".to_string(),
            pattern: "vitest".to_string(),
        },
        StaticCheckSpec::PatternAbsence {
            name: "no jest".to_string(),
            pattern: "jest".to_string(),
        },
    ];
    let results = run_static_checks(content, &specs);
    assert_eq!(results.len(), 2);
    assert!(results[0].passed); // vitest present
    assert!(results[1].passed); // jest absent
}

#[test]
fn extract_check_specs_from_verify_text() {
    use aif_migrate::verify::extract_static_specs;

    let verify_text = r#"
No remaining `jest.` calls in test files.
All test files import from 'vitest'.
"#;
    let specs = extract_static_specs(verify_text);
    // Should extract pattern-based specs from verify text
    assert!(!specs.is_empty(), "Should extract at least one check spec");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate --test verify`
Expected: FAIL — `verify` module doesn't exist.

**Step 3: Implement `verify.rs`**

```rust
use crate::types::StaticCheck;
use regex::Regex;

/// Specification for a static check extracted from @verify blocks.
#[derive(Debug, Clone)]
pub enum StaticCheckSpec {
    /// Fail if pattern is found in content.
    PatternAbsence { name: String, pattern: String },
    /// Fail if pattern is NOT found in content.
    PatternPresence { name: String, pattern: String },
}

/// Run static checks against file content.
pub fn run_static_checks(content: &str, specs: &[StaticCheckSpec]) -> Vec<StaticCheck> {
    specs.iter().map(|spec| {
        match spec {
            StaticCheckSpec::PatternAbsence { name, pattern } => {
                let re = Regex::new(pattern).unwrap_or_else(|_| Regex::new(&regex::escape(pattern)).unwrap());
                let found: Vec<&str> = re.find_iter(content).map(|m| m.as_str()).collect();
                StaticCheck {
                    name: name.clone(),
                    passed: found.is_empty(),
                    detail: if found.is_empty() {
                        format!("Pattern '{}' not found (good)", pattern)
                    } else {
                        format!("Found forbidden pattern '{}': {}", pattern, found.join(", "))
                    },
                }
            }
            StaticCheckSpec::PatternPresence { name, pattern } => {
                let found = content.contains(pattern);
                StaticCheck {
                    name: name.clone(),
                    passed: found,
                    detail: if found {
                        format!("Required pattern '{}' found", pattern)
                    } else {
                        format!("Required pattern '{}' not found", pattern)
                    },
                }
            }
        }
    }).collect()
}

/// Extract static check specs from @verify block text.
/// Heuristic: lines with backtick-quoted patterns become absence/presence checks.
pub fn extract_static_specs(verify_text: &str) -> Vec<StaticCheckSpec> {
    let mut specs = Vec::new();
    let backtick_re = Regex::new(r"`([^`]+)`").unwrap();

    for line in verify_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();
        let patterns: Vec<String> = backtick_re.captures_iter(trimmed)
            .map(|c| c[1].to_string())
            .collect();

        for pattern in patterns {
            if lower.contains("no ") || lower.contains("not ") || lower.contains("no remaining") {
                specs.push(StaticCheckSpec::PatternAbsence {
                    name: trimmed.to_string(),
                    pattern,
                });
            } else {
                specs.push(StaticCheckSpec::PatternPresence {
                    name: trimmed.to_string(),
                    pattern,
                });
            }
        }
    }

    specs
}
```

**Step 4: Add `regex` dependency to `Cargo.toml`**

Add to `[dependencies]`:
```toml
regex = "1"
```

**Step 5: Update `lib.rs`**

```rust
pub mod types;
pub mod validate;
pub mod chunk;
pub mod verify;
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: All tests pass.

**Step 7: Commit**

```bash
git add crates/aif-migrate/
git commit -m "feat(aif-migrate): add static verification with pattern checks"
```

---

### Task 5: LLM-Based Migration (Apply + Semantic Verify)

**Files:**
- Create: `crates/aif-migrate/src/apply.rs`
- Modify: `crates/aif-migrate/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/apply.rs`:

```rust
use aif_migrate::apply::{build_migration_prompt, build_semantic_verify_prompt, parse_migration_response, parse_semantic_response};
use aif_migrate::types::SemanticCheck;

#[test]
fn build_migration_prompt_includes_skill_steps_and_source() {
    let steps = vec![
        "Replace jest.fn() with vi.fn()".to_string(),
        "Update imports to vitest".to_string(),
    ];
    let source = "import { jest } from '@jest/globals';\njest.fn();";
    let prompt = build_migration_prompt(&steps, source, None);
    assert!(prompt.contains("Replace jest.fn() with vi.fn()"));
    assert!(prompt.contains("Update imports to vitest"));
    assert!(prompt.contains("jest.fn()"));
    assert!(prompt.contains("import"));
}

#[test]
fn build_migration_prompt_includes_repair_context() {
    let steps = vec!["Migrate imports".to_string()];
    let source = "old code";
    let repair = Some("Previous attempt failed: missing vitest import".to_string());
    let prompt = build_migration_prompt(&steps, source, repair.as_deref());
    assert!(prompt.contains("Previous attempt failed"));
}

#[test]
fn parse_migration_response_extracts_code_block() {
    let response = r#"Here's the migrated code:

```
import { vi } from 'vitest';
vi.fn();
```

I replaced the jest imports with vitest."#;
    let code = parse_migration_response(response);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("import { vi } from 'vitest'"));
    assert!(code.contains("vi.fn()"));
}

#[test]
fn parse_migration_response_handles_no_code_block() {
    let response = "I can't migrate this code because it's too complex.";
    let code = parse_migration_response(response);
    assert!(code.is_none());
}

#[test]
fn build_semantic_verify_prompt_includes_criteria() {
    let original = "jest.fn()";
    let migrated = "vi.fn()";
    let criteria = vec![
        "No remaining jest calls".to_string(),
        "Vitest imports present".to_string(),
    ];
    let prompt = build_semantic_verify_prompt(original, migrated, &criteria);
    assert!(prompt.contains("No remaining jest calls"));
    assert!(prompt.contains("Vitest imports present"));
    assert!(prompt.contains("jest.fn()"));
    assert!(prompt.contains("vi.fn()"));
}

#[test]
fn parse_semantic_response_extracts_checks() {
    let response = r#"## Criterion 1: No remaining jest calls
**PASS** — confidence: 0.95
The migrated code contains no references to jest.

## Criterion 2: Vitest imports present
**PASS** — confidence: 0.90
The code correctly imports from vitest.
"#;
    let criteria = vec![
        "No remaining jest calls".to_string(),
        "Vitest imports present".to_string(),
    ];
    let checks = parse_semantic_response(response, &criteria);
    assert_eq!(checks.len(), 2);
    assert!(checks[0].passed);
    assert!(checks[1].passed);
}

#[test]
fn parse_semantic_response_handles_failures() {
    let response = r#"## Criterion 1: No remaining jest calls
**FAIL** — confidence: 0.85
Found `jest.mock` on line 5.
"#;
    let criteria = vec!["No remaining jest calls".to_string()];
    let checks = parse_semantic_response(response, &criteria);
    assert_eq!(checks.len(), 1);
    assert!(!checks[0].passed);
    assert!(checks[0].reasoning.contains("jest.mock"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate --test apply`
Expected: FAIL — `apply` module doesn't exist.

**Step 3: Implement `apply.rs`**

```rust
use crate::types::SemanticCheck;
use regex::Regex;

/// Build the LLM prompt for migrating source code using skill steps.
pub fn build_migration_prompt(steps: &[String], source: &str, repair_context: Option<&str>) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are a code migration assistant. Apply the following migration steps to the source code.\n\n");

    prompt.push_str("## Migration Steps\n\n");
    for (i, step) in steps.iter().enumerate() {
        prompt.push_str(&format!("{}. {}\n", i + 1, step));
    }

    prompt.push_str("\n## Source Code\n\n```\n");
    prompt.push_str(source);
    prompt.push_str("\n```\n\n");

    if let Some(context) = repair_context {
        prompt.push_str("## Repair Context\n\n");
        prompt.push_str("A previous migration attempt failed. Here's what went wrong:\n\n");
        prompt.push_str(context);
        prompt.push_str("\n\nPlease fix these issues in your migration.\n\n");
    }

    prompt.push_str("Output ONLY the migrated code in a single code block. Do not include explanations before the code block.\n");
    prompt
}

/// Extract code from a migration response. Looks for fenced code blocks.
pub fn parse_migration_response(response: &str) -> Option<String> {
    let re = Regex::new(r"(?s)```(?:\w*)\n(.*?)```").unwrap();
    re.captures(response).map(|c| c[1].trim().to_string())
}

/// Build the LLM prompt for semantic verification of migrated code.
pub fn build_semantic_verify_prompt(original: &str, migrated: &str, criteria: &[String]) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are a code migration verifier. Check whether the migrated code satisfies each criterion.\n\n");

    prompt.push_str("## Original Code\n\n```\n");
    prompt.push_str(original);
    prompt.push_str("\n```\n\n");

    prompt.push_str("## Migrated Code\n\n```\n");
    prompt.push_str(migrated);
    prompt.push_str("\n```\n\n");

    prompt.push_str("## Verification Criteria\n\n");
    for (i, criterion) in criteria.iter().enumerate() {
        prompt.push_str(&format!("{}. {}\n", i + 1, criterion));
    }

    prompt.push_str("\nFor each criterion, respond with this exact format:\n\n");
    prompt.push_str("## Criterion N: <criterion text>\n");
    prompt.push_str("**PASS** or **FAIL** — confidence: <0.0-1.0>\n");
    prompt.push_str("<reasoning>\n\n");
    prompt
}

/// Parse the LLM semantic verification response into structured checks.
pub fn parse_semantic_response(response: &str, criteria: &[String]) -> Vec<SemanticCheck> {
    let section_re = Regex::new(r"(?m)^## Criterion \d+:.*$").unwrap();
    let pass_re = Regex::new(r"(?i)\*\*PASS\*\*").unwrap();
    let fail_re = Regex::new(r"(?i)\*\*FAIL\*\*").unwrap();
    let confidence_re = Regex::new(r"confidence:\s*([\d.]+)").unwrap();

    // Split response into sections
    let section_starts: Vec<usize> = section_re.find_iter(response).map(|m| m.start()).collect();

    let mut checks = Vec::new();
    for (i, criterion) in criteria.iter().enumerate() {
        let section_text = if i < section_starts.len() {
            let start = section_starts[i];
            let end = section_starts.get(i + 1).copied().unwrap_or(response.len());
            &response[start..end]
        } else {
            ""
        };

        let passed = pass_re.is_match(section_text) && !fail_re.is_match(section_text);
        let confidence = confidence_re.captures(section_text)
            .and_then(|c| c[1].parse::<f64>().ok())
            .unwrap_or(0.5);

        // Reasoning is everything after the PASS/FAIL line
        let reasoning = section_text.lines().skip(2).collect::<Vec<_>>().join("\n").trim().to_string();

        checks.push(SemanticCheck {
            criterion: criterion.clone(),
            passed,
            reasoning,
            confidence,
        });
    }

    checks
}
```

**Step 4: Update `lib.rs`**

```rust
pub mod types;
pub mod validate;
pub mod chunk;
pub mod verify;
pub mod apply;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/aif-migrate/
git commit -m "feat(aif-migrate): add LLM prompt building and response parsing for migration and verification"
```

---

### Task 6: Repair Loop

**Files:**
- Create: `crates/aif-migrate/src/repair.rs`
- Modify: `crates/aif-migrate/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/repair.rs`:

```rust
use aif_migrate::repair::{build_repair_context, RepairOutcome, RepairState};
use aif_migrate::types::{StaticCheck, SemanticCheck, VerificationResult};

#[test]
fn build_repair_context_includes_failures() {
    let verification = VerificationResult {
        static_checks: vec![
            StaticCheck {
                name: "no jest".to_string(),
                passed: false,
                detail: "Found jest.mock on line 5".to_string(),
            },
            StaticCheck {
                name: "has vitest".to_string(),
                passed: true,
                detail: "vitest import found".to_string(),
            },
        ],
        semantic_checks: vec![
            SemanticCheck {
                criterion: "Behavior preserved".to_string(),
                passed: false,
                reasoning: "Timer mocking semantics differ".to_string(),
                confidence: 0.6,
            },
        ],
        passed: false,
    };
    let fallback = Some("If timer mocking fails, preserve original and flag for review.".to_string());
    let context = build_repair_context(&verification, fallback.as_deref());
    assert!(context.contains("no jest"));
    assert!(context.contains("jest.mock on line 5"));
    assert!(context.contains("Timer mocking semantics differ"));
    assert!(context.contains("preserve original"));
    // Should NOT include passing checks
    assert!(!context.contains("has vitest"));
}

#[test]
fn repair_state_tracks_iterations() {
    let mut state = RepairState::new(3);
    assert_eq!(state.iteration(), 0);
    assert!(state.can_retry());

    state.record_attempt(false);
    assert_eq!(state.iteration(), 1);
    assert!(state.can_retry());

    state.record_attempt(false);
    state.record_attempt(false);
    assert_eq!(state.iteration(), 3);
    assert!(!state.can_retry());
}

#[test]
fn repair_state_stops_on_success() {
    let mut state = RepairState::new(3);
    state.record_attempt(true);
    assert_eq!(state.outcome(), RepairOutcome::Fixed);
}

#[test]
fn repair_state_exhausted_after_max() {
    let mut state = RepairState::new(2);
    state.record_attempt(false);
    state.record_attempt(false);
    assert_eq!(state.outcome(), RepairOutcome::Exhausted);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate --test repair`
Expected: FAIL — `repair` module doesn't exist.

**Step 3: Implement `repair.rs`**

```rust
use crate::types::VerificationResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairOutcome {
    /// Haven't started or still retrying.
    Pending,
    /// Verification passed after repair.
    Fixed,
    /// Max iterations reached without passing.
    Exhausted,
}

#[derive(Debug)]
pub struct RepairState {
    max_iterations: u32,
    attempts: u32,
    last_passed: bool,
}

impl RepairState {
    pub fn new(max_iterations: u32) -> Self {
        Self { max_iterations, attempts: 0, last_passed: false }
    }

    pub fn iteration(&self) -> u32 {
        self.attempts
    }

    pub fn can_retry(&self) -> bool {
        !self.last_passed && self.attempts < self.max_iterations
    }

    pub fn record_attempt(&mut self, passed: bool) {
        self.attempts += 1;
        self.last_passed = passed;
    }

    pub fn outcome(&self) -> RepairOutcome {
        if self.last_passed {
            RepairOutcome::Fixed
        } else if self.attempts >= self.max_iterations {
            RepairOutcome::Exhausted
        } else {
            RepairOutcome::Pending
        }
    }
}

/// Build repair context string from failed verification results.
/// Only includes failing checks to focus the LLM on what needs fixing.
pub fn build_repair_context(verification: &VerificationResult, fallback_text: Option<&str>) -> String {
    let mut context = String::new();

    let failed_static: Vec<_> = verification.static_checks.iter().filter(|c| !c.passed).collect();
    if !failed_static.is_empty() {
        context.push_str("## Failed Static Checks\n\n");
        for check in failed_static {
            context.push_str(&format!("- **{}**: {}\n", check.name, check.detail));
        }
        context.push('\n');
    }

    let failed_semantic: Vec<_> = verification.semantic_checks.iter().filter(|c| !c.passed).collect();
    if !failed_semantic.is_empty() {
        context.push_str("## Failed Semantic Checks\n\n");
        for check in failed_semantic {
            context.push_str(&format!("- **{}**: {} (confidence: {:.2})\n",
                check.criterion, check.reasoning, check.confidence));
        }
        context.push('\n');
    }

    if let Some(fallback) = fallback_text {
        context.push_str("## Fallback Guidance\n\n");
        context.push_str(fallback);
        context.push('\n');
    }

    context
}
```

**Step 4: Update `lib.rs`**

```rust
pub mod types;
pub mod validate;
pub mod chunk;
pub mod verify;
pub mod apply;
pub mod repair;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/aif-migrate/
git commit -m "feat(aif-migrate): add repair loop state machine and context builder"
```

---

### Task 7: Migration Report Generation

**Files:**
- Create: `crates/aif-migrate/src/report.rs`
- Modify: `crates/aif-migrate/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/report.rs`:

```rust
use aif_migrate::report::generate_report_document;
use aif_migrate::types::*;
use std::path::PathBuf;
use std::time::Duration;

fn make_report() -> MigrationReport {
    MigrationReport {
        skill_name: "jest-to-vitest".to_string(),
        source_dir: PathBuf::from("./src"),
        chunks: vec![
            ChunkResult {
                chunk_id: "chunk-0".to_string(),
                files: vec![PathBuf::from("src/a.test.ts")],
                status: ChunkStatus::Success,
                confidence: 0.98,
                verification: VerificationResult {
                    static_checks: vec![StaticCheck {
                        name: "no jest".to_string(), passed: true, detail: "Clean".to_string(),
                    }],
                    semantic_checks: vec![],
                    passed: true,
                },
                repair_iterations: 0,
                notes: vec![],
            },
            ChunkResult {
                chunk_id: "chunk-1".to_string(),
                files: vec![PathBuf::from("src/b.test.ts")],
                status: ChunkStatus::Failed,
                confidence: 0.40,
                verification: VerificationResult {
                    static_checks: vec![],
                    semantic_checks: vec![SemanticCheck {
                        criterion: "timer mocking".to_string(),
                        passed: false,
                        reasoning: "Not convertible".to_string(),
                        confidence: 0.3,
                    }],
                    passed: false,
                },
                repair_iterations: 3,
                notes: vec!["Needs manual review".to_string()],
            },
        ],
        overall_confidence: 0.69,
        unresolved: vec!["Snapshot files need regeneration".to_string()],
        manual_review: vec!["src/b.test.ts".to_string()],
        duration: Duration::from_secs(120),
    }
}

#[test]
fn report_generates_valid_aif_document() {
    let report = make_report();
    let doc = generate_report_document(&report);
    // Should have metadata
    assert!(doc.metadata.iter().any(|(k, _)| k == "title"));
    assert!(doc.metadata.iter().any(|(k, _)| k == "author"));
    // Should have blocks
    assert!(!doc.blocks.is_empty());
}

#[test]
fn report_includes_summary_section() {
    let report = make_report();
    let doc = generate_report_document(&report);
    // Should have a section titled "Summary"
    let has_summary = doc.blocks.iter().any(|b| {
        if let aif_core::ast::BlockKind::Section { heading, .. } = &b.kind {
            aif_core::text::inlines_to_text(heading, aif_core::text::TextMode::Plain).contains("Summary")
        } else {
            false
        }
    });
    assert!(has_summary, "Report should contain a Summary section");
}

#[test]
fn report_includes_failed_chunks() {
    let report = make_report();
    let doc = generate_report_document(&report);
    // Serialize to check content
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("chunk-1"), "Report should mention failed chunk");
    assert!(json.contains("Failed"), "Report should mention failure status");
}

#[test]
fn report_includes_manual_review_section() {
    let report = make_report();
    let doc = generate_report_document(&report);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Manual Review"), "Report should have manual review section");
    assert!(json.contains("src/b.test.ts"));
}

#[test]
fn report_all_passed_false_for_mixed() {
    let report = make_report();
    assert!(!report.all_passed());
}

#[test]
fn report_success_rate_correct() {
    let report = make_report();
    assert!((report.success_rate() - 0.5).abs() < 0.01);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate --test report`
Expected: FAIL — `report` module doesn't exist.

**Step 3: Implement `report.rs`**

```rust
use crate::types::{MigrationReport, ChunkResult, ChunkStatus};
use aif_core::ast::*;

/// Generate an AIF Document from a MigrationReport.
pub fn generate_report_document(report: &MigrationReport) -> Document {
    let mut metadata = vec![
        ("title".to_string(), format!("Migration Report — {}", report.skill_name)),
        ("author".to_string(), "aif-migrate".to_string()),
    ];

    let mut blocks = Vec::new();

    // Summary section
    let summary_text = format!(
        "Migrated {} chunks from {}.\nOverall confidence: {:.2}.\nDuration: {}s.\nSuccess rate: {:.0}%.",
        report.chunks.len(),
        report.source_dir.display(),
        report.overall_confidence,
        report.duration.as_secs(),
        report.success_rate() * 100.0,
    );
    blocks.push(make_section("Summary", vec![make_paragraph(&summary_text)]));

    // Results by chunk
    let mut chunk_blocks = Vec::new();
    for chunk in &report.chunks {
        let callout_type = match chunk.status {
            ChunkStatus::Success => CalloutType::Note,
            ChunkStatus::PartialSuccess => CalloutType::Warning,
            ChunkStatus::Failed => CalloutType::Warning,
            ChunkStatus::Skipped => CalloutType::Note,
        };
        let status_label = match chunk.status {
            ChunkStatus::Success => "Success",
            ChunkStatus::PartialSuccess => "Partial Success",
            ChunkStatus::Failed => "Failed",
            ChunkStatus::Skipped => "Skipped",
        };
        let files_str = chunk.files.iter()
            .map(|f| f.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let text = format!(
            "Chunk {} ({}): {} — confidence {:.2}, {} repair iterations",
            chunk.chunk_id, files_str, status_label, chunk.confidence, chunk.repair_iterations,
        );
        let mut callout_children = vec![make_paragraph(&text)];
        for note in &chunk.notes {
            callout_children.push(make_paragraph(note));
        }
        chunk_blocks.push(Block {
            kind: BlockKind::Callout {
                callout_type,
                children: callout_children,
            },
            attrs: None,
            span: None,
        });
    }
    blocks.push(make_section("Results by Chunk", chunk_blocks));

    // Manual review section
    if !report.manual_review.is_empty() {
        let items: Vec<Block> = report.manual_review.iter()
            .map(|item| make_paragraph(&format!("- {}", item)))
            .collect();
        blocks.push(make_section("Manual Review Required", items));
    }

    // Unresolved issues
    if !report.unresolved.is_empty() {
        let items: Vec<Block> = report.unresolved.iter()
            .map(|item| make_paragraph(&format!("- {}", item)))
            .collect();
        blocks.push(make_section("Unresolved Issues", items));
    }

    Document { metadata, blocks }
}

fn make_section(title: &str, children: Vec<Block>) -> Block {
    Block {
        kind: BlockKind::Section {
            heading: vec![Inline::Text(title.to_string())],
            children,
        },
        attrs: None,
        span: None,
    }
}

fn make_paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph(vec![Inline::Text(text.to_string())]),
        attrs: None,
        span: None,
    }
}
```

**Step 4: Update `lib.rs`**

```rust
pub mod types;
pub mod validate;
pub mod chunk;
pub mod verify;
pub mod apply;
pub mod repair;
pub mod report;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/aif-migrate/
git commit -m "feat(aif-migrate): add AIF report generation from migration results"
```

---

### Task 8: Pipeline Orchestrator

**Files:**
- Create: `crates/aif-migrate/src/engine.rs`
- Modify: `crates/aif-migrate/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/aif-migrate/tests/engine.rs`:

```rust
use aif_migrate::engine::{MigrationEngine, EngineConfig};
use aif_migrate::chunk::ChunkStrategy;
use std::path::PathBuf;

#[test]
fn engine_config_defaults() {
    let config = EngineConfig {
        max_repair_iterations: 3,
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    };
    assert_eq!(config.max_repair_iterations, 3);
}

#[test]
fn engine_validates_skill_before_running() {
    // Parse a skill that's missing profile=migration
    let source = r#"
#title: Not a Migration

@skill[name="regular", version="1.0"]
  @step[order=1]
    Do something.
  @end
@end
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();

    let engine = MigrationEngine::new(EngineConfig {
        max_repair_iterations: 3,
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let validation = engine.validate_skill(skill_block);
    assert!(!validation.is_valid(), "Should reject non-migration skill");
}

#[test]
fn engine_validates_valid_migration_skill() {
    let source = r#"
#title: Test

@skill[name="test", version="1.0", profile=migration]
  @precondition
    Has framework.
  @end

  @step[order=1]
    Migrate it.
  @end

  @verify
    Check it.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();

    let engine = MigrationEngine::new(EngineConfig {
        max_repair_iterations: 3,
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let validation = engine.validate_skill(skill_block);
    assert!(validation.is_valid(), "Should accept valid migration skill: {:?}", validation);
}

#[test]
fn engine_extracts_steps_from_skill() {
    let source = r#"
#title: Test

@skill[name="test", version="1.0", profile=migration]
  @precondition
    When to use.
  @end

  @step[order=1]
    First step.
  @end

  @step[order=2]
    Second step.
  @end

  @verify
    Check it.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();

    let engine = MigrationEngine::new(EngineConfig {
        max_repair_iterations: 3,
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let steps = engine.extract_steps(skill_block);
    assert_eq!(steps.len(), 2);
    assert!(steps[0].contains("First step"));
    assert!(steps[1].contains("Second step"));
}

#[test]
fn engine_extracts_verify_criteria() {
    let source = r#"
#title: Test

@skill[name="test", version="1.0", profile=migration]
  @precondition
    When to use.
  @end

  @step[order=1]
    Migrate.
  @end

  @verify
    No remaining `old_api` calls.
    All files import `new_api`.
  @end

  @output_contract
    Done.
  @end
@end
"#;
    let doc = aif_parser::parse(source).unwrap();
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(b.kind, aif_core::ast::BlockKind::Skill { .. })
    }).unwrap();

    let engine = MigrationEngine::new(EngineConfig {
        max_repair_iterations: 3,
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });
    let criteria = engine.extract_verify_criteria(skill_block);
    assert!(!criteria.is_empty(), "Should extract verification criteria");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p aif-migrate --test engine`
Expected: FAIL — `engine` module doesn't exist.

**Step 3: Implement `engine.rs`**

```rust
use crate::chunk::ChunkStrategy;
use crate::validate::{validate_migration_skill, MigrationLintResult};
use aif_core::ast::{Block, BlockKind, SkillBlockType};
use aif_core::text::{inlines_to_text, TextMode};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub max_repair_iterations: u32,
    pub chunk_strategy: ChunkStrategy,
    pub dry_run: bool,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub checks: Vec<MigrationLintResult>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

pub struct MigrationEngine {
    config: EngineConfig,
}

impl MigrationEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    /// Validate that a skill block is a valid migration skill.
    pub fn validate_skill(&self, skill_block: &Block) -> ValidationResult {
        let checks = validate_migration_skill(skill_block);
        ValidationResult { checks }
    }

    /// Extract migration step text from a skill block.
    pub fn extract_steps(&self, skill_block: &Block) -> Vec<String> {
        let children = match &skill_block.kind {
            BlockKind::Skill { children, .. } => children,
            _ => return Vec::new(),
        };
        children.iter()
            .filter_map(|b| {
                if let BlockKind::SkillBlock { block_type: SkillBlockType::Step, children, .. } = &b.kind {
                    let text: String = children.iter()
                        .filter_map(|child| {
                            if let BlockKind::Paragraph(inlines) = &child.kind {
                                Some(inlines_to_text(inlines, TextMode::Plain))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text.is_empty() { None } else { Some(text) }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract verification criteria text from @verify blocks.
    pub fn extract_verify_criteria(&self, skill_block: &Block) -> Vec<String> {
        let children = match &skill_block.kind {
            BlockKind::Skill { children, .. } => children,
            _ => return Vec::new(),
        };
        children.iter()
            .filter_map(|b| {
                if let BlockKind::SkillBlock { block_type: SkillBlockType::Verify, children, .. } = &b.kind {
                    let text: String = children.iter()
                        .filter_map(|child| {
                            if let BlockKind::Paragraph(inlines) = &child.kind {
                                Some(inlines_to_text(inlines, TextMode::Plain))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text.is_empty() { None } else { Some(text) }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract fallback text from @fallback blocks.
    pub fn extract_fallback(&self, skill_block: &Block) -> Option<String> {
        let children = match &skill_block.kind {
            BlockKind::Skill { children, .. } => children,
            _ => return None,
        };
        children.iter()
            .find_map(|b| {
                if let BlockKind::SkillBlock { block_type: SkillBlockType::Fallback, children, .. } = &b.kind {
                    let text: String = children.iter()
                        .filter_map(|child| {
                            if let BlockKind::Paragraph(inlines) = &child.kind {
                                Some(inlines_to_text(inlines, TextMode::Plain))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text.is_empty() { None } else { Some(text) }
                } else {
                    None
                }
            })
    }
}
```

**Step 4: Update `lib.rs`**

```rust
pub mod types;
pub mod validate;
pub mod chunk;
pub mod verify;
pub mod apply;
pub mod repair;
pub mod report;
pub mod engine;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p aif-migrate`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/aif-migrate/
git commit -m "feat(aif-migrate): add pipeline engine with skill validation and extraction"
```

---

### Task 9: CLI Integration

**Files:**
- Modify: `crates/aif-cli/Cargo.toml`
- Modify: `crates/aif-cli/src/main.rs`

**Step 1: Write the failing test**

Test CLI integration manually since it's wiring:

Run: `cargo run -p aif-cli -- migrate --help`
Expected: FAIL — no `migrate` subcommand.

**Step 2: Add `aif-migrate` dependency to `aif-cli/Cargo.toml`**

Add to `[dependencies]`:
```toml
aif-migrate = { path = "../aif-migrate" }
```

**Step 3: Add `Migrate` variant to `Commands` enum in `main.rs`**

Add to the `Commands` enum:
```rust
/// Run code migrations using migration skills
Migrate {
    #[command(subcommand)]
    action: MigrateAction,
},
```

Add the `MigrateAction` enum:
```rust
#[derive(Subcommand)]
enum MigrateAction {
    /// Validate a migration skill
    Validate {
        /// Path to migration skill .aif file
        input: PathBuf,
    },
    /// Run a migration
    Run {
        /// Path to migration skill .aif file
        #[arg(long)]
        skill: PathBuf,
        /// Source directory to migrate
        #[arg(long)]
        source: PathBuf,
        /// Output directory for migrated files
        #[arg(short, long, default_value = "./migrated")]
        output: PathBuf,
        /// Chunking strategy: file, directory, token-budget
        #[arg(long, default_value = "file")]
        strategy: String,
        /// Max repair iterations per chunk
        #[arg(long, default_value = "3")]
        max_repairs: u32,
        /// Report format: text or json
        #[arg(long, default_value = "text")]
        report: String,
    },
}
```

**Step 4: Add match arm for `Migrate` in `main()`**

```rust
Commands::Migrate { action } => match action {
    MigrateAction::Validate { input } => {
        let source = std::fs::read_to_string(&input)
            .unwrap_or_else(|e| { eprintln!("Error reading {}: {}", input.display(), e); std::process::exit(1); });
        let doc = aif_parser::parse(&source)
            .unwrap_or_else(|e| { eprintln!("Parse error: {:?}", e); std::process::exit(1); });
        let skill_block = doc.blocks.iter()
            .find(|b| matches!(b.kind, aif_core::ast::BlockKind::Skill { .. }))
            .unwrap_or_else(|| { eprintln!("No @skill block found"); std::process::exit(1); });

        let results = aif_migrate::validate::validate_migration_skill(skill_block);
        let all_passed = results.iter().all(|r| r.passed);

        for r in &results {
            let icon = if r.passed { "PASS" } else { "FAIL" };
            eprintln!("  [{}] {:?}: {}", icon, r.check, r.message);
        }

        if all_passed {
            eprintln!("\nMigration skill validation passed.");
        } else {
            eprintln!("\nMigration skill validation failed.");
            std::process::exit(1);
        }
    }
    MigrateAction::Run { skill, source, output, strategy, max_repairs, report } => {
        eprintln!("Migration engine requires LLM configuration.");
        eprintln!("Configure with: aif config set llm.api-key <key>");
        eprintln!();

        // Load config
        let config_path = dirs::home_dir()
            .map(|h| h.join(".aif/config.toml"))
            .unwrap_or_else(|| PathBuf::from(".aif/config.toml"));
        let aif_config = aif_core::config::AifConfig::load_with_env(&config_path);

        if aif_config.llm.api_key.is_none() {
            eprintln!("Error: No LLM API key configured.");
            eprintln!("Set via: aif config set llm.api-key <key>");
            eprintln!("Or env: AIF_LLM_API_KEY=<key>");
            std::process::exit(1);
        }

        let chunk_strategy = match strategy.as_str() {
            "file" => aif_migrate::chunk::ChunkStrategy::FilePerChunk,
            "directory" => aif_migrate::chunk::ChunkStrategy::DirectoryChunk,
            "token-budget" => aif_migrate::chunk::ChunkStrategy::TokenBudget { max_tokens: 4000 },
            other => {
                eprintln!("Unknown strategy: {}. Use: file, directory, token-budget", other);
                std::process::exit(1);
            }
        };

        eprintln!("Migration engine (async pipeline) not yet wired — validation available via 'aif migrate validate'.");
        eprintln!("Skill: {}", skill.display());
        eprintln!("Source: {}", source.display());
        eprintln!("Output: {}", output.display());
        eprintln!("Strategy: {}", strategy);
        eprintln!("Max repairs: {}", max_repairs);

        // TODO: Wire async pipeline in Task 10
    }
},
```

**Step 5: Verify CLI help works**

Run: `cargo run -p aif-cli -- migrate --help`
Expected: Shows `validate` and `run` subcommands.

Run: `cargo run -p aif-cli -- migrate validate --help`
Expected: Shows input argument.

**Step 6: Commit**

```bash
git add crates/aif-cli/
git commit -m "feat(aif-cli): add 'aif migrate validate' and 'aif migrate run' subcommands"
```

---

### Task 10: End-to-End Integration Test

**Files:**
- Create: `crates/aif-migrate/tests/fixtures/jest-to-vitest.aif`
- Create: `crates/aif-migrate/tests/integration.rs`

**Step 1: Create test fixture**

Create `crates/aif-migrate/tests/fixtures/jest-to-vitest.aif`:

```aif
#title: Jest to Vitest Migration
#author: test

@skill[name="jest-to-vitest", version="1.0", profile=migration]
  @precondition
    Repository uses Jest as test runner.
    Test files use describe, it, expect patterns.
  @end

  @step[order=1]
    Replace Jest imports with Vitest equivalents.
  @end

  @step[order=2]
    Replace jest.fn() with vi.fn().
    Replace jest.mock() with vi.mock().
    Replace jest.spyOn() with vi.spyOn().
  @end

  @verify
    No remaining `jest.` calls in test files.
    All test files import from `vitest`.
  @end

  @fallback
    If a test file uses Jest-specific timer mocking,
    preserve the original and flag for manual review.
  @end

  @output_contract
    All test files migrated to Vitest API.
    Test pass rate >= original pass rate.
  @end
@end
```

**Step 2: Write the integration test**

```rust
use aif_migrate::chunk::{chunk_source_files, ChunkStrategy};
use aif_migrate::engine::{EngineConfig, MigrationEngine};
use aif_migrate::report::generate_report_document;
use aif_migrate::types::*;
use aif_migrate::verify::{extract_static_specs, run_static_checks};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

fn load_fixture_skill() -> aif_core::ast::Block {
    let source = include_str!("fixtures/jest-to-vitest.aif");
    let doc = aif_parser::parse(source).expect("fixture parse failed");
    doc.blocks.into_iter()
        .find(|b| matches!(b.kind, aif_core::ast::BlockKind::Skill { .. }))
        .expect("no skill block in fixture")
}

#[test]
fn full_validation_pipeline() {
    let skill = load_fixture_skill();
    let engine = MigrationEngine::new(EngineConfig {
        max_repair_iterations: 3,
        chunk_strategy: ChunkStrategy::FilePerChunk,
        dry_run: false,
    });

    // 1. Validate skill
    let validation = engine.validate_skill(&skill);
    assert!(validation.is_valid(), "Fixture skill should be valid: {:?}", validation);

    // 2. Extract steps
    let steps = engine.extract_steps(&skill);
    assert_eq!(steps.len(), 2, "Should have 2 steps");

    // 3. Extract verify criteria
    let criteria = engine.extract_verify_criteria(&skill);
    assert!(!criteria.is_empty(), "Should have verify criteria");

    // 4. Extract fallback
    let fallback = engine.extract_fallback(&skill);
    assert!(fallback.is_some(), "Should have fallback");
    assert!(fallback.unwrap().contains("timer mocking"));
}

#[test]
fn chunking_and_static_verification_pipeline() {
    // Simulate: chunk files → verify migrated content
    let source_files: HashMap<PathBuf, String> = [
        (PathBuf::from("src/a.test.ts"), "import { vi } from 'vitest';\nvi.fn();".to_string()),
        (PathBuf::from("src/b.test.ts"), "import { vi } from 'vitest';\nvi.mock('./db');".to_string()),
    ].into_iter().collect();

    let chunks = chunk_source_files(&source_files, ChunkStrategy::FilePerChunk);
    assert_eq!(chunks.len(), 2);

    // Run static checks on each chunk's migrated content
    let verify_text = "No remaining `jest.` calls in test files.\nAll test files import from `vitest`.";
    let specs = extract_static_specs(verify_text);

    for chunk in &chunks {
        for (_, content) in &chunk.files {
            let results = run_static_checks(content, &specs);
            assert!(results.iter().all(|r| r.passed),
                "Migrated content should pass all static checks: {:?}", results);
        }
    }
}

#[test]
fn report_generation_end_to_end() {
    let report = MigrationReport {
        skill_name: "jest-to-vitest".to_string(),
        source_dir: PathBuf::from("./src"),
        chunks: vec![
            ChunkResult {
                chunk_id: "file-0000-src/a.test.ts".to_string(),
                files: vec![PathBuf::from("src/a.test.ts")],
                status: ChunkStatus::Success,
                confidence: 0.95,
                verification: VerificationResult {
                    static_checks: vec![
                        StaticCheck { name: "no jest".to_string(), passed: true, detail: "Clean".to_string() },
                    ],
                    semantic_checks: vec![],
                    passed: true,
                },
                repair_iterations: 0,
                notes: vec![],
            },
        ],
        overall_confidence: 0.95,
        unresolved: vec![],
        manual_review: vec![],
        duration: Duration::from_secs(5),
    };

    let doc = generate_report_document(&report);

    // Compile to HTML to verify it's valid AIF
    let html = aif_html::compile(&doc);
    assert!(html.contains("Migration Report"));
    assert!(html.contains("jest-to-vitest"));
    assert!(html.contains("Success"));

    // Compile to JSON to verify structure
    let json = serde_json::to_string_pretty(&doc).unwrap();
    assert!(json.contains("Summary"));
}
```

**Step 3: Add `aif-html` dev-dependency**

Add to `crates/aif-migrate/Cargo.toml` `[dev-dependencies]`:
```toml
aif-html = { path = "../aif-html" }
```

**Step 4: Run integration tests**

Run: `cargo test -p aif-migrate --test integration`
Expected: All tests pass.

**Step 5: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All workspace tests pass.

**Step 6: Commit**

```bash
git add crates/aif-migrate/
git commit -m "test(aif-migrate): add integration tests with fixture skill"
```

---

### Task 11: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Add `aif-migrate` to the workspace crates table**

Add row:
```
| `aif-migrate` | Migration engine — chunked pipeline, repair loops, static+LLM verification, AIF report generation |
```

**Step 2: Add key types**

Add to Key Types section:
```
- `MigrationConfig` / `ChunkResult` / `MigrationReport` — migration pipeline types (in `aif-migrate::types`)
- `SourceChunk` / `ChunkStrategy` — source file chunking (in `aif-migrate::chunk`)
- `MigrationEngine` / `EngineConfig` — pipeline orchestrator (in `aif-migrate::engine`)
- `StaticCheckSpec` — pattern-based static verification (in `aif-migrate::verify`)
```

**Step 3: Add CLI commands**

Add to CLI Commands section:
```bash
# Migration
aif migrate validate input.aif                # Validate migration skill profile
aif migrate run --skill s.aif --source ./src --output ./migrated [--strategy file|directory|token-budget] [--max-repairs 3] [--report text|json]
```

**Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md with aif-migrate crate and migration CLI commands"
```

---

## Execution Order Summary

| Task | Module | Description | Dependencies |
|------|--------|-------------|--------------|
| 1 | types | Core types + crate scaffold | None |
| 2 | validate | Migration profile lint | Task 1 |
| 3 | chunk | Source file chunking | Task 1 |
| 4 | verify | Static pattern checks | Task 1 |
| 5 | apply | LLM prompt/response parsing | Task 1 |
| 6 | repair | Repair loop state machine | Task 1, 4 |
| 7 | report | AIF report generation | Task 1 |
| 8 | engine | Pipeline orchestrator | Tasks 2-7 |
| 9 | CLI | Subcommand wiring | Task 8 |
| 10 | integration | End-to-end tests | Tasks 1-9 |
| 11 | docs | CLAUDE.md update | Task 10 |

Tasks 2-7 can be done in parallel (they depend only on Task 1). Task 8 depends on all of them. Tasks 9-11 are sequential.
