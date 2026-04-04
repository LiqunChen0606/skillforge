//! Skill inheritance — resolve `extends` attribute on `@skill` blocks.
//!
//! When a skill declares `@skill[extends="base-debugging"]`, inheritance resolution
//! loads the base skill from the registry and merges blocks: child overrides parent
//! blocks of the same type (matched by SkillBlockType + order attribute for Steps),
//! and inherits all unmatched parent blocks.
//!
//! Supports multi-level inheritance with cycle detection.

use crate::registry::Registry;
use aif_core::ast::*;
use std::collections::HashSet;

/// Errors during inheritance resolution
#[derive(Debug)]
pub enum InheritError {
    /// The `extends` target was not found in the registry
    BaseNotFound(String),
    /// Circular inheritance detected (A extends B extends A)
    CyclicInheritance(Vec<String>),
    /// The base skill file could not be read or parsed
    InvalidBase { name: String, reason: String },
    /// The block is not a valid skill block
    NotASkill,
}

impl std::fmt::Display for InheritError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InheritError::BaseNotFound(name) => {
                write!(f, "Base skill '{}' not found in registry", name)
            }
            InheritError::CyclicInheritance(chain) => {
                write!(f, "Cyclic inheritance: {}", chain.join(" -> "))
            }
            InheritError::InvalidBase { name, reason } => {
                write!(f, "Invalid base skill '{}': {}", name, reason)
            }
            InheritError::NotASkill => write!(f, "Block is not a @skill block"),
        }
    }
}

impl std::error::Error for InheritError {}

/// A key that uniquely identifies a child block within a skill for merging purposes.
/// Step blocks are matched by their `order` attribute; other block types are matched
/// by their SkillBlockType (and optionally by `id` attribute if present).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BlockKey {
    Step(String), // order value
    Typed(String, Option<String>), // (skill_type_name, optional id)
}

fn block_key(block: &Block) -> Option<BlockKey> {
    if let BlockKind::SkillBlock {
        skill_type, attrs, ..
    } = &block.kind
    {
        match skill_type {
            SkillBlockType::Step => {
                let order = attrs.get("order").unwrap_or("0").to_string();
                Some(BlockKey::Step(order))
            }
            _ => {
                let type_name = format!("{:?}", skill_type);
                let id = attrs.get("id").map(|s| s.to_string());
                Some(BlockKey::Typed(type_name, id))
            }
        }
    } else {
        None
    }
}

/// Extract the `extends` attribute from a skill block.
pub fn get_extends(block: &Block) -> Option<String> {
    if let BlockKind::SkillBlock {
        skill_type: SkillBlockType::Skill,
        attrs,
        ..
    } = &block.kind
    {
        attrs.get("extends").map(|s| s.to_string())
    } else {
        None
    }
}

/// Load a skill document from the registry by name.
fn load_skill_from_registry(
    name: &str,
    registry: &Registry,
) -> Result<Block, InheritError> {
    let entry = registry
        .lookup(name)
        .ok_or_else(|| InheritError::BaseNotFound(name.to_string()))?;

    let source = std::fs::read_to_string(&entry.path).map_err(|e| InheritError::InvalidBase {
        name: name.to_string(),
        reason: format!("cannot read file {}: {}", entry.path, e),
    })?;

    // Try JSON first (registry stores compiled docs), then AIF source
    let doc: Document = serde_json::from_str(&source).map_err(|e| InheritError::InvalidBase {
        name: name.to_string(),
        reason: format!("failed to parse {}: {}", entry.path, e),
    })?;

    doc.blocks
        .into_iter()
        .find(|b| {
            matches!(
                &b.kind,
                BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Skill,
                    ..
                }
            )
        })
        .ok_or_else(|| InheritError::InvalidBase {
            name: name.to_string(),
            reason: "no @skill block found".to_string(),
        })
}

/// Merge child skill with parent skill. Child blocks override parent blocks
/// when they match by BlockKey. Unmatched parent blocks are inherited.
fn merge_skill_blocks(parent: &Block, child: &Block) -> Result<Block, InheritError> {
    let (parent_attrs, parent_children) = match &parent.kind {
        BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            children,
            ..
        } => (attrs, children),
        _ => return Err(InheritError::NotASkill),
    };

    let (child_attrs, child_title, child_content, child_children) = match &child.kind {
        BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            title,
            content,
            children,
        } => (attrs, title, content, children),
        _ => return Err(InheritError::NotASkill),
    };

    // Build set of child block keys for override detection
    let child_keys: HashSet<BlockKey> = child_children.iter().filter_map(block_key).collect();

    // Start with parent blocks that aren't overridden by the child
    let mut merged_children: Vec<Block> = Vec::new();
    for parent_block in parent_children {
        if let Some(key) = block_key(parent_block) {
            if !child_keys.contains(&key) {
                merged_children.push(parent_block.clone());
            }
        } else {
            // Non-skill blocks from parent are inherited
            merged_children.push(parent_block.clone());
        }
    }

    // Add all child blocks
    for child_block in child_children {
        merged_children.push(child_block.clone());
    }

    // Sort step blocks by order to maintain consistent ordering
    merged_children.sort_by(|a, b| {
        let order_a = step_order(a);
        let order_b = step_order(b);
        match (order_a, order_b) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    // Merge attrs: child overrides parent, except `extends` and `name` come from child
    let mut merged_attrs = parent_attrs.clone();
    merged_attrs.id = child_attrs.id.clone().or(merged_attrs.id);
    for (key, value) in &child_attrs.pairs {
        if key != "extends" {
            merged_attrs.pairs.insert(key.clone(), value.clone());
        }
    }

    Ok(Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: merged_attrs,
            title: child_title.clone(),
            content: child_content.clone(),
            children: merged_children,
        },
        span: child.span,
    })
}

fn step_order(block: &Block) -> Option<u32> {
    if let BlockKind::SkillBlock {
        skill_type: SkillBlockType::Step,
        attrs,
        ..
    } = &block.kind
    {
        attrs.get("order").and_then(|s| s.parse().ok())
    } else {
        None
    }
}

/// Resolve inheritance for a skill block, loading base skills from the registry.
///
/// Follows the `extends` chain until a skill without `extends` is found,
/// detecting cycles along the way. Returns the fully merged skill block.
pub fn resolve_inheritance(
    skill_block: &Block,
    registry: &Registry,
) -> Result<Block, InheritError> {
    let extends = match get_extends(skill_block) {
        Some(name) => name,
        None => return Ok(skill_block.clone()), // No inheritance to resolve
    };

    // Get the child's name for cycle detection
    let child_name = if let BlockKind::SkillBlock {
        skill_type: SkillBlockType::Skill,
        attrs,
        ..
    } = &skill_block.kind
    {
        attrs.get("name").unwrap_or("unnamed").to_string()
    } else {
        return Err(InheritError::NotASkill);
    };

    let mut chain = vec![child_name.clone()];
    let mut visited = HashSet::new();
    visited.insert(child_name);

    // Walk the inheritance chain
    let mut current_base_name = extends;
    let mut bases: Vec<Block> = Vec::new();

    loop {
        if !visited.insert(current_base_name.clone()) {
            chain.push(current_base_name);
            return Err(InheritError::CyclicInheritance(chain));
        }
        chain.push(current_base_name.clone());

        let base_block = load_skill_from_registry(&current_base_name, registry)?;

        match get_extends(&base_block) {
            Some(next_base) => {
                bases.push(base_block);
                current_base_name = next_base;
            }
            None => {
                bases.push(base_block);
                break;
            }
        }
    }

    // Merge from bottom up: deepest base first, then each intermediate, then child
    // bases is [immediate_parent, grandparent, ...], so reverse to get root-first
    bases.reverse();

    let mut result = bases.remove(0);
    for base in bases {
        result = merge_skill_blocks(&result, &base)?;
    }

    // Finally merge with the child skill
    merge_skill_blocks(&result, skill_block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;
    use std::io::Write;

    fn make_skill_block(name: &str, extends: Option<&str>, children: Vec<Block>) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), name.into());
        attrs.pairs.insert("version".into(), "1.0.0".into());
        if let Some(base) = extends {
            attrs.pairs.insert("extends".into(), base.into());
        }
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children,
            },
            span: Span::empty(),
        }
    }

    fn make_step(order: u32, text: &str) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("order".into(), order.to_string());
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text {
                    text: text.to_string(),
                }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    fn make_verify(text: &str) -> Block {
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Verify,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text {
                    text: text.to_string(),
                }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    fn make_precondition(text: &str) -> Block {
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Precondition,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text {
                    text: text.to_string(),
                }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn no_extends_returns_original() {
        let skill = make_skill_block("my-skill", None, vec![make_step(1, "do something")]);
        let registry = Registry::new(std::path::PathBuf::from("/tmp/test_reg.json"));
        let result = resolve_inheritance(&skill, &registry).unwrap();
        assert_eq!(result, skill);
    }

    #[test]
    fn extends_not_found_returns_error() {
        let skill = make_skill_block("child", Some("nonexistent"), vec![]);
        let registry = Registry::new(std::path::PathBuf::from("/tmp/test_reg.json"));
        let result = resolve_inheritance(&skill, &registry);
        assert!(matches!(result, Err(InheritError::BaseNotFound(_))));
    }

    #[test]
    fn simple_inheritance_merges_blocks() {
        let dir = std::env::temp_dir().join("aif_inherit_test_simple");
        let _ = std::fs::create_dir_all(&dir);

        // Create base skill with step 1, verify, precondition
        let base = make_skill_block(
            "base-debugging",
            None,
            vec![
                make_step(1, "base step 1"),
                make_step(2, "base step 2"),
                make_verify("base verify"),
                make_precondition("base precondition"),
            ],
        );
        let base_doc = Document {
            metadata: Default::default(),
            blocks: vec![base],
        };
        let base_path = dir.join("base.json");
        let mut f = std::fs::File::create(&base_path).unwrap();
        f.write_all(serde_json::to_string(&base_doc).unwrap().as_bytes())
            .unwrap();

        let mut registry = Registry::new(dir.join("registry.json"));
        registry.register(
            "base-debugging",
            "1.0.0",
            "sha256:aaa",
            base_path.to_str().unwrap(),
        );

        // Child overrides step 1 and inherits step 2, verify, precondition
        let child = make_skill_block(
            "my-debugging",
            Some("base-debugging"),
            vec![make_step(1, "child step 1")],
        );

        let resolved = resolve_inheritance(&child, &registry).unwrap();

        if let BlockKind::SkillBlock {
            attrs, children, ..
        } = &resolved.kind
        {
            assert_eq!(attrs.get("name").unwrap(), "my-debugging");

            // Should have 4 blocks: step 1 (child), step 2 (parent), verify (parent), precondition (parent)
            assert_eq!(children.len(), 4);

            // Check step 1 came from child
            let step1 = children
                .iter()
                .find(|b| {
                    if let BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Step,
                        attrs,
                        ..
                    } = &b.kind
                    {
                        attrs.get("order") == Some("1")
                    } else {
                        false
                    }
                })
                .unwrap();
            if let BlockKind::SkillBlock { content, .. } = &step1.kind {
                assert_eq!(
                    content,
                    &vec![Inline::Text {
                        text: "child step 1".into()
                    }]
                );
            }

            // Check step 2 came from parent
            let step2 = children
                .iter()
                .find(|b| {
                    if let BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Step,
                        attrs,
                        ..
                    } = &b.kind
                    {
                        attrs.get("order") == Some("2")
                    } else {
                        false
                    }
                })
                .unwrap();
            if let BlockKind::SkillBlock { content, .. } = &step2.kind {
                assert_eq!(
                    content,
                    &vec![Inline::Text {
                        text: "base step 2".into()
                    }]
                );
            }
        } else {
            panic!("Expected SkillBlock");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cyclic_inheritance_detected() {
        let dir = std::env::temp_dir().join("aif_inherit_test_cycle");
        let _ = std::fs::create_dir_all(&dir);

        // A extends B, B extends A
        let skill_a = make_skill_block("skill-a", Some("skill-b"), vec![]);
        let doc_a = Document {
            metadata: Default::default(),
            blocks: vec![skill_a.clone()],
        };

        let skill_b = make_skill_block("skill-b", Some("skill-a"), vec![]);
        let doc_b = Document {
            metadata: Default::default(),
            blocks: vec![skill_b],
        };

        let path_a = dir.join("a.json");
        let path_b = dir.join("b.json");
        std::fs::write(&path_a, serde_json::to_string(&doc_a).unwrap()).unwrap();
        std::fs::write(&path_b, serde_json::to_string(&doc_b).unwrap()).unwrap();

        let mut registry = Registry::new(dir.join("registry.json"));
        registry.register("skill-a", "1.0.0", "sha256:aaa", path_a.to_str().unwrap());
        registry.register("skill-b", "1.0.0", "sha256:bbb", path_b.to_str().unwrap());

        let result = resolve_inheritance(&skill_a, &registry);
        assert!(matches!(result, Err(InheritError::CyclicInheritance(_))));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn multi_level_inheritance() {
        let dir = std::env::temp_dir().join("aif_inherit_test_multi");
        let _ = std::fs::create_dir_all(&dir);

        // grandparent -> parent -> child
        let grandparent = make_skill_block(
            "grandparent",
            None,
            vec![
                make_step(1, "gp step 1"),
                make_step(2, "gp step 2"),
                make_verify("gp verify"),
            ],
        );
        let gp_doc = Document {
            metadata: Default::default(),
            blocks: vec![grandparent],
        };

        let parent = make_skill_block(
            "parent",
            Some("grandparent"),
            vec![
                make_step(1, "parent step 1"), // overrides gp step 1
                make_precondition("parent precondition"),
            ],
        );
        let p_doc = Document {
            metadata: Default::default(),
            blocks: vec![parent],
        };

        let gp_path = dir.join("gp.json");
        let p_path = dir.join("parent.json");
        std::fs::write(&gp_path, serde_json::to_string(&gp_doc).unwrap()).unwrap();
        std::fs::write(&p_path, serde_json::to_string(&p_doc).unwrap()).unwrap();

        let mut registry = Registry::new(dir.join("registry.json"));
        registry.register(
            "grandparent",
            "1.0.0",
            "sha256:gp",
            gp_path.to_str().unwrap(),
        );
        registry.register("parent", "1.0.0", "sha256:p", p_path.to_str().unwrap());

        let child = make_skill_block(
            "child",
            Some("parent"),
            vec![make_step(2, "child step 2")], // overrides gp step 2
        );

        let resolved = resolve_inheritance(&child, &registry).unwrap();
        if let BlockKind::SkillBlock {
            attrs, children, ..
        } = &resolved.kind
        {
            assert_eq!(attrs.get("name").unwrap(), "child");
            // Should have: step 1 (parent), step 2 (child), verify (gp), precondition (parent)
            assert_eq!(children.len(), 4);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_extends_extracts_attribute() {
        let skill = make_skill_block("child", Some("base"), vec![]);
        assert_eq!(get_extends(&skill), Some("base".to_string()));

        let skill_no_extends = make_skill_block("standalone", None, vec![]);
        assert_eq!(get_extends(&skill_no_extends), None);
    }

    #[test]
    fn merge_preserves_child_attrs() {
        let mut parent_attrs = Attrs::new();
        parent_attrs.pairs.insert("name".into(), "parent".into());
        parent_attrs
            .pairs
            .insert("version".into(), "1.0.0".into());
        parent_attrs
            .pairs
            .insert("description".into(), "Parent description".into());

        let parent = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: parent_attrs,
                title: None,
                content: vec![],
                children: vec![],
            },
            span: Span::empty(),
        };

        let mut child_attrs = Attrs::new();
        child_attrs.pairs.insert("name".into(), "child".into());
        child_attrs
            .pairs
            .insert("version".into(), "2.0.0".into());
        child_attrs
            .pairs
            .insert("extends".into(), "parent".into());

        let child = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: child_attrs,
                title: None,
                content: vec![],
                children: vec![],
            },
            span: Span::empty(),
        };

        let merged = merge_skill_blocks(&parent, &child).unwrap();
        if let BlockKind::SkillBlock { attrs, .. } = &merged.kind {
            // Child name wins
            assert_eq!(attrs.get("name").unwrap(), "child");
            // Child version wins
            assert_eq!(attrs.get("version").unwrap(), "2.0.0");
            // Parent description is inherited
            assert_eq!(
                attrs.get("description").unwrap(),
                "Parent description"
            );
            // extends should NOT be in the merged result
            assert!(attrs.get("extends").is_none());
        }
    }
}
