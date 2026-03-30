use aif_core::ast::*;
use aif_core::span::Span;

#[test]
fn render_skill_block_html() {
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
    let html = aif_html::render_html(&doc);
    assert!(html.contains("aif-skill"));
    assert!(html.contains("aif-step"));
    assert!(html.contains("Reproduce the bug."));
}
