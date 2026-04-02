use aif_core::ast::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillManifest {
    pub skills: Vec<SkillEntry>,
    pub generated: String,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillEntry {
    pub name: String,
    pub version: Option<String>,
    pub hash: String,
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub blocks: Vec<String>,
    pub path: String,
}

use crate::hash::compute_skill_hash;

fn skill_type_tag(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "skill",
        SkillBlockType::Step => "step",
        SkillBlockType::Verify => "verify",
        SkillBlockType::Precondition => "precondition",
        SkillBlockType::OutputContract => "output_contract",
        SkillBlockType::Decision => "decision",
        SkillBlockType::Tool => "tool",
        SkillBlockType::Fallback => "fallback",
        SkillBlockType::RedFlag => "red_flag",
        SkillBlockType::Example => "example",
        SkillBlockType::Scenario => "scenario",
    }
}

pub fn skill_to_entry(block: &Block, path: &str) -> Option<SkillEntry> {
    if let BlockKind::SkillBlock { skill_type: SkillBlockType::Skill, attrs, children, .. } = &block.kind {
        let name = attrs.get("name")?.to_string();
        let version = attrs.get("version").map(|s| s.to_string());
        let hash = compute_skill_hash(block);
        let tags: Vec<String> = attrs.get("tags")
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        let priority = attrs.get("priority").map(|s| s.to_string());

        let mut blocks = Vec::new();
        for child in children {
            if let BlockKind::SkillBlock { skill_type, .. } = &child.kind {
                let tag = skill_type_tag(skill_type).to_string();
                if !blocks.contains(&tag) {
                    blocks.push(tag);
                }
            }
        }

        Some(SkillEntry { name, version, hash, tags, priority, blocks, path: path.to_string() })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    #[test]
    fn generate_entry_from_skill() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "debugging".into());
        attrs.pairs.insert("version".into(), "1.0".into());
        attrs.pairs.insert("tags".into(), "process,troubleshooting".into());
        attrs.pairs.insert("priority".into(), "high".into());

        let step = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("order".into(), "1".into());
                    a
                },
                title: None,
                content: vec![Inline::Text { text: "Do something.".into() }],
                children: vec![],
            },
            span: Span::empty(),
        };

        let verify = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Verify,
                attrs: Attrs::new(),
                title: None,
                content: vec![Inline::Text { text: "Check it.".into() }],
                children: vec![],
            },
            span: Span::empty(),
        };

        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children: vec![step, verify],
            },
            span: Span::empty(),
        };

        let entry = skill_to_entry(&skill, "skills/debugging.aif").unwrap();
        assert_eq!(entry.name, "debugging");
        assert_eq!(entry.version, Some("1.0".into()));
        assert_eq!(entry.tags, vec!["process", "troubleshooting"]);
        assert_eq!(entry.priority, Some("high".into()));
        assert_eq!(entry.blocks, vec!["step", "verify"]);
        assert_eq!(entry.path, "skills/debugging.aif");
        assert!(entry.hash.starts_with("sha256:"));
    }

    #[test]
    fn non_skill_block_returns_none() {
        let block = Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text { text: "Not a skill.".into() }],
            },
            span: Span::empty(),
        };
        assert!(skill_to_entry(&block, "foo.aif").is_none());
    }
}
