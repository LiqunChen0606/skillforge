use aif_core::ast::*;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
}

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

/// Run all 7 structural lint checks on a skill block.
/// Returns one `LintResult` per check.
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

    results.push(check_frontmatter(attrs));
    results.push(check_required_sections(children));
    results.push(check_block_types(children));
    results.push(check_version_hash(attrs, block));
    results.push(check_description_length(attrs));
    results.push(check_name_format(attrs));
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
        None => LintResult::pass(LintCheck::NameFormat),
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
