use aif_core::ast::*;
use aif_core::span::Span;

#[test]
fn render_skill_block_markdown() {
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Reproduce the bug.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let doc = Document {
        metadata: std::collections::BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "debugging".into());
                    a
                },
                title: None,
                content: vec![],
                children: vec![step],
            },
            span: Span::empty(),
        }],
    };
    let md = aif_markdown::render_markdown(&doc);
    assert!(md.contains("## Steps"), "expected '## Steps' in:\n{}", md);
    assert!(md.contains("1. Reproduce the bug."), "expected '1. Reproduce the bug.' in:\n{}", md);
}
