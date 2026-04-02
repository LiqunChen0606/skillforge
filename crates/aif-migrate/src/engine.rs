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
    #[allow(dead_code)]
    config: EngineConfig,
}

impl MigrationEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
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
