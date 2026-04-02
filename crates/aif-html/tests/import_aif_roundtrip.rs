use aif_core::ast::*;
use aif_core::span::Span;
use aif_html::importer::{import_html, ImportMode};
use aif_html::render_html;

fn span() -> Span {
    Span::new(0, 0)
}

// ===== Task 7: Semantic Blocks and Callouts =====

#[test]
fn test_roundtrip_semantic_block_claim() {
    let html = r#"<body><div class="aif-claim"><p>This is a claim.</p></div></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::SemanticBlock {
            block_type,
            content,
            title,
            ..
        } => {
            assert_eq!(*block_type, SemanticBlockType::Claim);
            assert!(title.is_none());
            assert!(matches!(&content[0], Inline::Text { text } if text == "This is a claim."));
        }
        other => panic!("expected SemanticBlock, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_semantic_block_with_title() {
    let html = r#"<body><div class="aif-evidence"><strong>Key Evidence</strong><p>Supporting data here.</p></div></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::SemanticBlock {
            block_type,
            title,
            content,
            ..
        } => {
            assert_eq!(*block_type, SemanticBlockType::Evidence);
            let title_inlines = title.as_ref().expect("should have title");
            assert!(matches!(&title_inlines[0], Inline::Text { text } if text == "Key Evidence"));
            assert!(matches!(&content[0], Inline::Text { text } if text == "Supporting data here."));
        }
        other => panic!("expected SemanticBlock, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_semantic_block_with_id() {
    let html = r#"<body><div class="aif-claim" id="c1"><p>Claim with ID.</p></div></body>"#;
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::SemanticBlock { attrs, .. } => {
            assert_eq!(attrs.id, Some("c1".into()));
        }
        other => panic!("expected SemanticBlock, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_all_semantic_types() {
    let types = [
        ("aif-claim", SemanticBlockType::Claim),
        ("aif-evidence", SemanticBlockType::Evidence),
        ("aif-definition", SemanticBlockType::Definition),
        ("aif-theorem", SemanticBlockType::Theorem),
        ("aif-assumption", SemanticBlockType::Assumption),
        ("aif-result", SemanticBlockType::Result),
        ("aif-conclusion", SemanticBlockType::Conclusion),
        ("aif-requirement", SemanticBlockType::Requirement),
        ("aif-recommendation", SemanticBlockType::Recommendation),
    ];

    for (class, expected_type) in &types {
        let html = format!(
            r#"<body><div class="{}"><p>Content for {}.</p></div></body>"#,
            class, class
        );
        let result = import_html(&html, false);
        assert_eq!(result.mode, ImportMode::AifRoundtrip);
        match &result.document.blocks[0].kind {
            BlockKind::SemanticBlock { block_type, .. } => {
                assert_eq!(block_type, expected_type, "failed for class {}", class);
            }
            other => panic!("expected SemanticBlock for {}, got {:?}", class, other),
        }
    }
}

#[test]
fn test_roundtrip_callout_note() {
    let html = r#"<body><aside class="aif-callout aif-note"><p>This is a note.</p></aside></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Callout {
            callout_type,
            content,
            ..
        } => {
            assert_eq!(*callout_type, CalloutType::Note);
            assert!(matches!(&content[0], Inline::Text { text } if text == "This is a note."));
        }
        other => panic!("expected Callout, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_all_callout_types() {
    let types = [
        ("aif-note", CalloutType::Note),
        ("aif-warning", CalloutType::Warning),
        ("aif-info", CalloutType::Info),
        ("aif-tip", CalloutType::Tip),
    ];

    for (class, expected_type) in &types {
        let html = format!(
            r#"<body><aside class="aif-callout {}"><p>Content for {}.</p></aside></body>"#,
            class, class
        );
        let result = import_html(&html, false);
        match &result.document.blocks[0].kind {
            BlockKind::Callout { callout_type, .. } => {
                assert_eq!(callout_type, expected_type, "failed for class {}", class);
            }
            other => panic!("expected Callout for {}, got {:?}", class, other),
        }
    }
}

#[test]
fn test_roundtrip_aif_ref() {
    let html = r##"<body><p>See <a class="aif-ref" href="#intro">intro</a> for details.</p></body>"##;
    let result = import_html(html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    // Should have: Text, Reference, Text
    let ref_inline = content
        .iter()
        .find(|i| matches!(i, Inline::Reference { .. }))
        .expect("should find Reference inline");
    match ref_inline {
        Inline::Reference { target } => {
            assert_eq!(target, "intro");
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_roundtrip_aif_footnote() {
    let html = r#"<body><p>Main text<sup class="aif-footnote">footnote content</sup>.</p></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };
    let footnote = content
        .iter()
        .find(|i| matches!(i, Inline::Footnote { .. }))
        .expect("should find Footnote inline");
    match footnote {
        Inline::Footnote { content } => {
            assert!(matches!(&content[0], Inline::Text { text } if text == "footnote content"));
        }
        _ => unreachable!(),
    }
}

// ===== Task 8: Skill Blocks =====

#[test]
fn test_roundtrip_skill_block() {
    let html = r#"<body>
        <div class="aif-skill">
            <h3>Debugging Skill</h3>
            <p>A skill for debugging.</p>
            <div class="aif-precondition">
                <p>When a bug is reported.</p>
            </div>
            <div class="aif-step">
                <h3>Step 1</h3>
                <p>Reproduce the bug.</p>
            </div>
            <div class="aif-verify">
                <p>Check the fix works.</p>
            </div>
        </div>
    </body>"#;
    let result = import_html(html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);
    assert_eq!(result.document.blocks.len(), 1);

    match &result.document.blocks[0].kind {
        BlockKind::SkillBlock {
            skill_type,
            title,
            content,
            children,
            ..
        } => {
            assert_eq!(*skill_type, SkillBlockType::Skill);
            let title_inlines = title.as_ref().expect("should have title");
            assert!(matches!(&title_inlines[0], Inline::Text { text } if text == "Debugging Skill"));
            assert!(matches!(&content[0], Inline::Text { text } if text == "A skill for debugging."));
            assert_eq!(children.len(), 3);

            // Check children types
            assert!(matches!(
                &children[0].kind,
                BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Precondition,
                    ..
                }
            ));
            assert!(matches!(
                &children[1].kind,
                BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Step,
                    ..
                }
            ));
            assert!(matches!(
                &children[2].kind,
                BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Verify,
                    ..
                }
            ));
        }
        other => panic!("expected SkillBlock, got {:?}", other),
    }
}

#[test]
fn test_roundtrip_all_skill_types() {
    let types = [
        ("aif-skill", SkillBlockType::Skill),
        ("aif-step", SkillBlockType::Step),
        ("aif-verify", SkillBlockType::Verify),
        ("aif-precondition", SkillBlockType::Precondition),
        ("aif-output-contract", SkillBlockType::OutputContract),
        ("aif-decision", SkillBlockType::Decision),
        ("aif-tool", SkillBlockType::Tool),
        ("aif-fallback", SkillBlockType::Fallback),
        ("aif-red-flag", SkillBlockType::RedFlag),
        ("aif-example", SkillBlockType::Example),
        ("aif-scenario", SkillBlockType::Scenario),
    ];

    for (class, expected_type) in &types {
        let html = format!(
            r#"<body><div class="{}"><p>Content for {}.</p></div></body>"#,
            class, class
        );
        let result = import_html(&html, false);
        match &result.document.blocks[0].kind {
            BlockKind::SkillBlock { skill_type, .. } => {
                assert_eq!(skill_type, expected_type, "failed for class {}", class);
            }
            other => panic!("expected SkillBlock for {}, got {:?}", class, other),
        }
    }
}

#[test]
fn test_roundtrip_skill_block_with_data_attrs() {
    let html = r#"<body><div class="aif-step" data-aif-order="1"><p>Step content.</p></div></body>"#;
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::SkillBlock { attrs, .. } => {
            assert_eq!(attrs.pairs.get("order").map(|s| s.as_str()), Some("1"));
        }
        other => panic!("expected SkillBlock, got {:?}", other),
    }
}

// ===== Task 9: Full Roundtrip Tests =====

#[test]
fn test_full_roundtrip_mixed_document() {
    // Build a Document with mixed block types
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Roundtrip Test".into());

    // Section
    doc.blocks.push(Block {
        kind: BlockKind::Section {
            attrs: {
                let mut a = Attrs::new();
                a.id = Some("intro".into());
                a
            },
            title: vec![Inline::Text {
                text: "Introduction".into(),
            }],
            children: vec![
                Block {
                    kind: BlockKind::Paragraph {
                        content: vec![Inline::Text {
                            text: "A paragraph.".into(),
                        }],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Claim,
                        attrs: Attrs::new(),
                        title: None,
                        content: vec![Inline::Text {
                            text: "An important claim.".into(),
                        }],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::Callout {
                        callout_type: CalloutType::Warning,
                        attrs: Attrs::new(),
                        content: vec![Inline::Text {
                            text: "Be careful!".into(),
                        }],
                    },
                    span: span(),
                },
            ],
        },
        span: span(),
    });

    // Emit to HTML
    let html = render_html(&doc);

    // Re-import
    let result = import_html(&html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);
    assert_eq!(
        result.document.metadata.get("title").map(|s| s.as_str()),
        Some("Roundtrip Test")
    );

    // Should have one section
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Section {
            attrs,
            title,
            children,
        } => {
            assert_eq!(attrs.id, Some("intro".into()));
            assert!(matches!(&title[0], Inline::Text { text } if text == "Introduction"));
            assert_eq!(children.len(), 3);

            // Paragraph
            assert!(matches!(&children[0].kind, BlockKind::Paragraph { .. }));

            // SemanticBlock (Claim)
            match &children[1].kind {
                BlockKind::SemanticBlock {
                    block_type,
                    content,
                    ..
                } => {
                    assert_eq!(*block_type, SemanticBlockType::Claim);
                    assert!(
                        matches!(&content[0], Inline::Text { text } if text == "An important claim.")
                    );
                }
                other => panic!("expected SemanticBlock, got {:?}", other),
            }

            // Callout (Warning)
            match &children[2].kind {
                BlockKind::Callout {
                    callout_type,
                    content,
                    ..
                } => {
                    assert_eq!(*callout_type, CalloutType::Warning);
                    assert!(
                        matches!(&content[0], Inline::Text { text } if text == "Be careful!")
                    );
                }
                other => panic!("expected Callout, got {:?}", other),
            }
        }
        other => panic!("expected Section, got {:?}", other),
    }
}

#[test]
fn test_full_roundtrip_code_and_table() {
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Code and Table".into());

    // Code block
    doc.blocks.push(Block {
        kind: BlockKind::CodeBlock {
            lang: Some("rust".into()),
            attrs: Attrs::new(),
            code: "fn main() {\n    println!(\"hello\");\n}".into(),
        },
        span: span(),
    });

    // Table
    doc.blocks.push(Block {
        kind: BlockKind::Table {
            attrs: Attrs::new(),
            caption: Some(vec![Inline::Text {
                text: "Results".into(),
            }]),
            headers: vec![
                vec![Inline::Text {
                    text: "Name".into(),
                }],
                vec![Inline::Text {
                    text: "Score".into(),
                }],
            ],
            rows: vec![vec![
                vec![Inline::Text {
                    text: "Alice".into(),
                }],
                vec![Inline::Text {
                    text: "95".into(),
                }],
            ]],
        },
        span: span(),
    });

    // Emit to HTML
    let html = render_html(&doc);

    // Re-import
    let result = import_html(&html, false);

    // Code block
    match &result.document.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert!(code.contains("fn main()"));
            assert!(code.contains("println!"));
        }
        other => panic!("expected CodeBlock, got {:?}", other),
    }

    // Table
    match &result.document.blocks[1].kind {
        BlockKind::Table {
            caption,
            headers,
            rows,
            ..
        } => {
            let cap = caption.as_ref().expect("should have caption");
            assert!(matches!(&cap[0], Inline::Text { text } if text == "Results"));
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].len(), 2);
        }
        other => panic!("expected Table, got {:?}", other),
    }
}

#[test]
fn test_full_roundtrip_skill_document() {
    let mut doc = Document::new();
    doc.metadata.insert("title".into(), "Debug Skill".into());

    doc.blocks.push(Block {
        kind: BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs: {
                let mut a = Attrs::new();
                a.id = Some("debug".into());
                a
            },
            title: Some(vec![Inline::Text {
                text: "Debugging".into(),
            }]),
            content: vec![Inline::Text {
                text: "A debugging workflow.".into(),
            }],
            children: vec![
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Precondition,
                        attrs: Attrs::new(),
                        title: None,
                        content: vec![Inline::Text {
                            text: "When a bug occurs.".into(),
                        }],
                        children: vec![],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Step,
                        attrs: Attrs::new(),
                        title: Some(vec![Inline::Text {
                            text: "Reproduce".into(),
                        }]),
                        content: vec![Inline::Text {
                            text: "Try to reproduce the bug.".into(),
                        }],
                        children: vec![],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Verify,
                        attrs: Attrs::new(),
                        title: None,
                        content: vec![Inline::Text {
                            text: "Confirm the fix.".into(),
                        }],
                        children: vec![],
                    },
                    span: span(),
                },
            ],
        },
        span: span(),
    });

    // Emit to HTML
    let html = render_html(&doc);

    // Re-import
    let result = import_html(&html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);

    match &result.document.blocks[0].kind {
        BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        } => {
            assert_eq!(*skill_type, SkillBlockType::Skill);
            assert_eq!(attrs.id, Some("debug".into()));
            let title_inlines = title.as_ref().expect("should have title");
            assert!(matches!(&title_inlines[0], Inline::Text { text } if text == "Debugging"));
            assert!(
                matches!(&content[0], Inline::Text { text } if text == "A debugging workflow.")
            );
            assert_eq!(children.len(), 3);

            // Precondition
            match &children[0].kind {
                BlockKind::SkillBlock {
                    skill_type,
                    content,
                    ..
                } => {
                    assert_eq!(*skill_type, SkillBlockType::Precondition);
                    assert!(
                        matches!(&content[0], Inline::Text { text } if text == "When a bug occurs.")
                    );
                }
                other => panic!("expected SkillBlock(Precondition), got {:?}", other),
            }

            // Step
            match &children[1].kind {
                BlockKind::SkillBlock {
                    skill_type,
                    title,
                    content,
                    ..
                } => {
                    assert_eq!(*skill_type, SkillBlockType::Step);
                    let t = title.as_ref().expect("step should have title");
                    assert!(matches!(&t[0], Inline::Text { text } if text == "Reproduce"));
                    assert!(
                        matches!(&content[0], Inline::Text { text } if text == "Try to reproduce the bug.")
                    );
                }
                other => panic!("expected SkillBlock(Step), got {:?}", other),
            }

            // Verify
            match &children[2].kind {
                BlockKind::SkillBlock {
                    skill_type,
                    content,
                    ..
                } => {
                    assert_eq!(*skill_type, SkillBlockType::Verify);
                    assert!(
                        matches!(&content[0], Inline::Text { text } if text == "Confirm the fix.")
                    );
                }
                other => panic!("expected SkillBlock(Verify), got {:?}", other),
            }
        }
        other => panic!("expected SkillBlock(Skill), got {:?}", other),
    }
}

#[test]
fn test_generic_mode_for_non_aif_html() {
    let html = "<body><p>Just a paragraph.</p></body>";
    let result = import_html(html, false);
    assert_eq!(result.mode, ImportMode::Generic);
}

#[test]
fn test_generic_div_flattens_in_aif_mode() {
    // A div without aif classes should still flatten its children
    let html = r#"<body><div class="aif-claim"><p>A claim.</p></div><div><p>Normal div content.</p></div></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.mode, ImportMode::AifRoundtrip);
    assert_eq!(result.document.blocks.len(), 2);
    assert!(matches!(
        &result.document.blocks[0].kind,
        BlockKind::SemanticBlock { .. }
    ));
    assert!(matches!(
        &result.document.blocks[1].kind,
        BlockKind::Paragraph { .. }
    ));
}

#[test]
fn test_roundtrip_reference_and_footnote_in_paragraph() {
    let html = r##"<body><p>See <a class="aif-ref" href="#sec1">sec1</a> and note<sup class="aif-footnote">extra info</sup>.</p></body>"##;
    let result = import_html(html, false);
    let content = match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => content,
        other => panic!("expected Paragraph, got {:?}", other),
    };

    let has_ref = content
        .iter()
        .any(|i| matches!(i, Inline::Reference { target } if target == "sec1"));
    let has_footnote = content.iter().any(|i| matches!(i, Inline::Footnote { .. }));

    assert!(has_ref, "should contain Reference inline");
    assert!(has_footnote, "should contain Footnote inline");
}
