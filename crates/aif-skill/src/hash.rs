use aif_core::ast::*;
use sha2::{Sha256, Digest};

fn inline_to_text(out: &mut String, inline: &Inline) {
    match inline {
        Inline::Text { text } => out.push_str(text),
        Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
            for i in content { inline_to_text(out, i); }
        }
        Inline::InlineCode { code } => out.push_str(code),
        Inline::Link { text, url } => {
            for i in text { inline_to_text(out, i); }
            out.push_str(url);
        }
        Inline::Image { alt, src } => {
            out.push_str(alt);
            out.push_str(src);
        }
        Inline::Reference { target } => out.push_str(target),
        Inline::SoftBreak => out.push(' '),
        Inline::HardBreak => out.push('\n'),
    }
}

fn normalize_child(out: &mut String, block: &Block) {
    if let BlockKind::SkillBlock { skill_type, attrs, content, children, .. } = &block.kind {
        out.push_str(&format!("{:?}", skill_type));
        for (k, v) in &attrs.pairs {
            if k != "hash" {
                out.push_str(&format!(" {}={}", k, v));
            }
        }
        out.push('\n');
        for inline in content {
            inline_to_text(out, inline);
        }
        for child in children {
            out.push('\n');
            normalize_child(out, child);
        }
    }
}

fn normalize_for_hash(block: &Block) -> String {
    let mut out = String::new();
    if let BlockKind::SkillBlock { content, children, .. } = &block.kind {
        for inline in content {
            inline_to_text(&mut out, inline);
        }
        for child in children {
            out.push('\n');
            normalize_child(&mut out, child);
        }
    }
    out.replace("\r\n", "\n").trim().to_string()
}

/// Compute SHA-256 hash of a skill block's content.
pub fn compute_skill_hash(block: &Block) -> String {
    let normalized = normalize_for_hash(block);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}

/// Verify that a skill block's hash attribute matches its computed hash.
pub fn verify_skill_hash(block: &Block) -> HashVerifyResult {
    if let BlockKind::SkillBlock { attrs, .. } = &block.kind {
        match attrs.get("hash") {
            Some(expected) => {
                let actual = compute_skill_hash(block);
                if expected == actual {
                    HashVerifyResult::Valid
                } else {
                    HashVerifyResult::Mismatch {
                        expected: expected.to_string(),
                        actual,
                    }
                }
            }
            None => HashVerifyResult::NoHash,
        }
    } else {
        HashVerifyResult::NotASkill
    }
}

#[derive(Debug, PartialEq)]
pub enum HashVerifyResult {
    Valid,
    Mismatch { expected: String, actual: String },
    NoHash,
    NotASkill,
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    fn make_skill_with_content(name: &str, text: &str) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), name.into());
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![Inline::Text { text: text.into() }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn compute_hash_deterministic() {
        let skill = make_skill_with_content("test", "Some content here.");
        let hash1 = compute_skill_hash(&skill);
        let hash2 = compute_skill_hash(&skill);
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));
        assert_eq!(hash1.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn different_content_different_hash() {
        let skill1 = make_skill_with_content("test", "Content A");
        let skill2 = make_skill_with_content("test", "Content B");
        assert_ne!(compute_skill_hash(&skill1), compute_skill_hash(&skill2));
    }

    #[test]
    fn verify_valid_hash() {
        let mut skill = make_skill_with_content("test", "Some content.");
        let hash = compute_skill_hash(&skill);
        if let BlockKind::SkillBlock { ref mut attrs, .. } = skill.kind {
            attrs.pairs.insert("hash".into(), hash);
        }
        assert_eq!(verify_skill_hash(&skill), HashVerifyResult::Valid);
    }

    #[test]
    fn verify_tampered_content() {
        let mut skill = make_skill_with_content("test", "Original content.");
        let hash = compute_skill_hash(&skill);
        if let BlockKind::SkillBlock { ref mut attrs, ref mut content, .. } = skill.kind {
            attrs.pairs.insert("hash".into(), hash);
            *content = vec![Inline::Text { text: "Tampered content.".into() }];
        }
        match verify_skill_hash(&skill) {
            HashVerifyResult::Mismatch { .. } => {}
            other => panic!("expected Mismatch, got {:?}", other),
        }
    }

    #[test]
    fn verify_no_hash() {
        let skill = make_skill_with_content("test", "Content.");
        assert_eq!(verify_skill_hash(&skill), HashVerifyResult::NoHash);
    }
}
