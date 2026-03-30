use aif_core::ast::*;
use aif_core::span::Span;

#[test]
fn skill_compact_strips_examples() {
    let example = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Example,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "This is a long example that should be stripped.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let step = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Step,
            attrs: {
                let mut a = Attrs::new();
                a.pairs.insert("order".into(), "1".into());
                a
            },
            title: None,
            content: vec![Inline::Text { text: "Do the thing carefully and thoroughly.".into() }],
            children: vec![],
        },
        span: Span::empty(),
    };
    let verify = Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Verify,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text { text: "Check results.".into() }],
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
                    a.pairs.insert("name".into(), "test".into());
                    a
                },
                title: None,
                content: vec![],
                children: vec![step, verify, example],
            },
            span: Span::empty(),
        }],
    };

    let full = aif_lml::render_lml(&doc);
    let compact = aif_lml::render_lml_skill_compact(&doc);

    // Full mode includes example
    assert!(full.contains("long example"));
    // Compact mode strips example
    assert!(!compact.contains("long example"));
    // Compact preserves verify
    assert!(compact.contains("Check results."));
    // Compact is shorter
    assert!(compact.len() < full.len());
}
