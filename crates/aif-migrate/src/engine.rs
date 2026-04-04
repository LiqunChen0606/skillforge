use crate::chunk::chunk_source_files;
use crate::repair::{RepairState, RepairOutcome};
use crate::types::{
    ChunkResult, ChunkStatus, MigrationConfig, MigrationReport, VerificationResult,
};
use crate::validate::{validate_migration_skill, MigrationLintResult};
use crate::verify::{run_static_checks, extract_static_specs};
use aif_core::ast::{Block, BlockKind, SkillBlockType};
use aif_core::text::{inlines_to_text, TextMode};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

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
    config: MigrationConfig,
}

impl MigrationEngine {
    pub fn new(config: MigrationConfig) -> Self {
        Self { config }
    }

    /// Run the full migration pipeline: validate → chunk → apply → verify → repair → report.
    ///
    /// The `apply_fn` callback simulates the LLM call: given (steps, source_code, repair_context),
    /// it returns the migrated code. Pass `None` for repair_context on the first attempt.
    /// This design allows both real LLM integration and deterministic testing.
    pub fn run<F>(
        &self,
        skill_block: &Block,
        source_files: &HashMap<PathBuf, String>,
        apply_fn: F,
    ) -> Result<MigrationReport, String>
    where
        F: Fn(&[String], &str, Option<&str>) -> Option<String>,
    {
        let start = Instant::now();

        // 1. Validate skill
        let validation = self.validate_skill(skill_block);
        if !validation.is_valid() {
            let msgs: Vec<String> = validation.checks.iter()
                .filter(|c| !c.passed)
                .map(|c| c.message.clone())
                .collect();
            return Err(format!("Skill validation failed: {}", msgs.join("; ")));
        }

        // 2. Extract steps and verify criteria
        let steps = self.extract_steps(skill_block);
        let verify_criteria = self.extract_verify_criteria(skill_block);
        let fallback = self.extract_fallback(skill_block);

        // 3. Extract static check specs from verify text
        let verify_text = verify_criteria.join("\n");
        let static_specs = extract_static_specs(&verify_text);

        // 4. Chunk source files
        let chunks = chunk_source_files(source_files, self.config.chunk_strategy.clone());

        // 5. Process each chunk
        let mut chunk_results = Vec::new();
        let mut unresolved = Vec::new();
        let mut manual_review = Vec::new();

        for chunk in &chunks {
            // Collect warnings from chunking
            for w in &chunk.warnings {
                manual_review.push(w.clone());
            }

            let source = chunk.files.iter()
                .map(|(path, content)| format!("// File: {}\n{}", path.display(), content))
                .collect::<Vec<_>>()
                .join("\n\n");

            let mut repair_state = RepairState::new(self.config.max_repair_iterations);
            let mut last_verification = VerificationResult {
                static_checks: Vec::new(),
                semantic_checks: Vec::new(),
                passed: false,
            };
            let mut migrated_code = None;
            let mut notes = Vec::new();

            // Initial attempt
            let repair_ctx: Option<String> = None;
            match apply_fn(&steps, &source, repair_ctx.as_deref()) {
                Some(code) => {
                    let static_results = run_static_checks(&code, &static_specs);
                    let all_passed = static_results.iter().all(|c| c.passed);
                    last_verification = VerificationResult {
                        static_checks: static_results,
                        semantic_checks: Vec::new(),
                        passed: all_passed,
                    };
                    repair_state.record_attempt(all_passed);
                    migrated_code = Some(code);
                }
                None => {
                    notes.push("LLM returned no code block".to_string());
                    repair_state.record_attempt(false);
                }
            }

            // Repair loop
            while repair_state.can_retry() {
                let ctx = crate::repair::build_repair_context(
                    &last_verification,
                    fallback.as_deref(),
                );
                match apply_fn(&steps, &source, Some(&ctx)) {
                    Some(code) => {
                        let static_results = run_static_checks(&code, &static_specs);
                        let all_passed = static_results.iter().all(|c| c.passed);
                        last_verification = VerificationResult {
                            static_checks: static_results,
                            semantic_checks: Vec::new(),
                            passed: all_passed,
                        };
                        repair_state.record_attempt(all_passed);
                        migrated_code = Some(code);
                    }
                    None => {
                        notes.push("Repair attempt returned no code block".to_string());
                        repair_state.record_attempt(false);
                    }
                }
            }

            let mut status = match repair_state.outcome() {
                RepairOutcome::Fixed => ChunkStatus::Success,
                RepairOutcome::Exhausted => {
                    unresolved.push(format!("Chunk '{}' exhausted repair loop", chunk.chunk_id));
                    ChunkStatus::Failed
                }
                RepairOutcome::Pending => {
                    // Should not happen after the loop, but handle gracefully
                    if last_verification.passed {
                        ChunkStatus::Success
                    } else {
                        ChunkStatus::Failed
                    }
                }
            };

            let confidence = if last_verification.passed { 0.9 } else { 0.3 };

            // Write output if we have migrated code and not dry-run
            if let Some(ref code) = migrated_code {
                if !self.config.dry_run {
                    if chunk.files.len() > 1 {
                        // Multi-file chunk: LLM returns a single response for concatenated
                        // input, so we can only reliably write to the first file.
                        let (first_path, _) = &chunk.files[0];
                        let out_path = self.config.output_dir.join(first_path);
                        if let Some(parent) = out_path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                notes.push(format!(
                                    "Failed to create directory '{}': {}",
                                    parent.display(), e
                                ));
                                status = ChunkStatus::Failed;
                            }
                        }
                        if let Err(e) = std::fs::write(&out_path, code) {
                            notes.push(format!(
                                "Failed to write '{}': {}",
                                out_path.display(), e
                            ));
                            status = ChunkStatus::Failed;
                        }
                        let skipped: Vec<_> = chunk.files[1..].iter()
                            .map(|(p, _)| p.display().to_string())
                            .collect();
                        notes.push(format!(
                            "Multi-file chunk: wrote combined output to '{}'; \
                             skipped writing to: {}",
                            first_path.display(),
                            skipped.join(", ")
                        ));
                    } else if let Some((path, _)) = chunk.files.first() {
                        let out_path = self.config.output_dir.join(path);
                        if let Some(parent) = out_path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                notes.push(format!(
                                    "Failed to create directory '{}': {}",
                                    parent.display(), e
                                ));
                                status = ChunkStatus::Failed;
                            }
                        }
                        if let Err(e) = std::fs::write(&out_path, code) {
                            notes.push(format!(
                                "Failed to write '{}': {}",
                                out_path.display(), e
                            ));
                            status = ChunkStatus::Failed;
                        }
                    }
                }
            }

            chunk_results.push(ChunkResult {
                chunk_id: chunk.chunk_id.clone(),
                files: chunk.files.iter().map(|(p, _)| p.clone()).collect(),
                status,
                confidence,
                verification: last_verification,
                repair_iterations: repair_state.iteration(),
                notes,
            });
        }

        let overall_confidence = if chunk_results.is_empty() {
            0.0
        } else {
            chunk_results.iter().map(|c| c.confidence).sum::<f64>() / chunk_results.len() as f64
        };

        Ok(MigrationReport {
            skill_name: extract_skill_name(skill_block),
            source_dir: self.config.source_dir.clone(),
            chunks: chunk_results,
            overall_confidence,
            unresolved,
            manual_review,
            duration: start.elapsed(),
        })
    }

    pub fn validate_skill(&self, skill_block: &Block) -> ValidationResult {
        let checks = validate_migration_skill(skill_block);
        ValidationResult { checks }
    }

    pub fn extract_steps(&self, skill_block: &Block) -> Vec<String> {
        let children = match &skill_block.kind {
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                children,
                ..
            } => children,
            _ => return Vec::new(),
        };
        children
            .iter()
            .filter_map(|b| {
                if let BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Step,
                    children,
                    content,
                    ..
                } = &b.kind
                {
                    // Collect text from both content inlines and paragraph children
                    let mut parts = Vec::new();
                    let content_text = inlines_to_text(content, TextMode::Plain);
                    if !content_text.is_empty() {
                        parts.push(content_text);
                    }
                    for child in children {
                        if let BlockKind::Paragraph { content } = &child.kind {
                            parts.push(inlines_to_text(content, TextMode::Plain));
                        }
                    }
                    let text = parts.join("\n");
                    if text.is_empty() {
                        None
                    } else {
                        Some(text)
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn extract_verify_criteria(&self, skill_block: &Block) -> Vec<String> {
        let children = match &skill_block.kind {
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                children,
                ..
            } => children,
            _ => return Vec::new(),
        };
        children
            .iter()
            .filter_map(|b| {
                if let BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Verify,
                    children,
                    content,
                    ..
                } = &b.kind
                {
                    let mut parts = Vec::new();
                    let content_text = inlines_to_text(content, TextMode::Plain);
                    if !content_text.is_empty() {
                        parts.push(content_text);
                    }
                    for child in children {
                        if let BlockKind::Paragraph { content } = &child.kind {
                            parts.push(inlines_to_text(content, TextMode::Plain));
                        }
                    }
                    let text = parts.join("\n");
                    if text.is_empty() {
                        None
                    } else {
                        Some(text)
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn extract_fallback(&self, skill_block: &Block) -> Option<String> {
        let children = match &skill_block.kind {
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                children,
                ..
            } => children,
            _ => return None,
        };
        children.iter().find_map(|b| {
            if let BlockKind::SkillBlock {
                skill_type: SkillBlockType::Fallback,
                children,
                content,
                ..
            } = &b.kind
            {
                let mut parts = Vec::new();
                let content_text = inlines_to_text(content, TextMode::Plain);
                if !content_text.is_empty() {
                    parts.push(content_text);
                }
                for child in children {
                    if let BlockKind::Paragraph { content } = &child.kind {
                        parts.push(inlines_to_text(content, TextMode::Plain));
                    }
                }
                let text = parts.join("\n");
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            } else {
                None
            }
        })
    }
}

fn extract_skill_name(skill_block: &Block) -> String {
    match &skill_block.kind {
        BlockKind::SkillBlock { attrs, .. } => {
            attrs.pairs.iter()
                .find(|(k, _)| k.as_str() == "name")
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "unnamed".to_string())
        }
        _ => "unnamed".to_string(),
    }
}
