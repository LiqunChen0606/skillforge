use aif_core::ast::*;
use aif_core::span::Span;
use std::collections::BTreeMap;

use aif_binary::token_opt::{decode, encode};

/// Helper: encode then decode, comparing the result.
/// Note: spans are lost (always Span(0,0) after roundtrip).
/// Unknown future byte values for SemanticBlockType/CalloutType fall back
/// to Claim/Note for forward-compatibility, but all known variants roundtrip exactly.
fn roundtrip(doc: &Document) -> Document {
    let bytes = encode(doc);
    decode(&bytes).expect("decode failed")
}

fn sp() -> Span {
    Span::new(0, 0)
}

#[test]
fn roundtrip_simple_paragraph_with_metadata() {
    let doc = Document {
        metadata: {
            let mut m = BTreeMap::new();
            m.insert("title".into(), "Hello".into());
            m.insert("author".into(), "Test".into());
            m
        },
        blocks: vec![Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Text {
                    text: "Hello world".into(),
                }],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    assert_eq!(decoded.metadata, doc.metadata);
    assert_eq!(decoded.blocks.len(), 1);
    if let BlockKind::Paragraph { content } = &decoded.blocks[0].kind {
        assert_eq!(content.len(), 1);
        if let Inline::Text { text } = &content[0] {
            assert_eq!(text, "Hello world");
        } else {
            panic!("expected Text inline");
        }
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn roundtrip_skill_block_with_children() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "debugging".into());
                    a
                },
                title: Some(vec![Inline::Text {
                    text: "Debug Skill".into(),
                }]),
                content: vec![Inline::Text {
                    text: "A debugging skill".into(),
                }],
                children: vec![
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Precondition,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text {
                                text: "When a bug is reported".into(),
                            }],
                            children: vec![],
                        },
                        span: sp(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Step,
                            attrs: {
                                let mut a = Attrs::new();
                                a.pairs.insert("order".into(), "1".into());
                                a
                            },
                            title: None,
                            content: vec![Inline::Text {
                                text: "Reproduce the bug".into(),
                            }],
                            children: vec![],
                        },
                        span: sp(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Verify,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text {
                                text: "Bug is fixed".into(),
                            }],
                            children: vec![],
                        },
                        span: sp(),
                    },
                ],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    assert_eq!(decoded.blocks.len(), 1);
    if let BlockKind::SkillBlock {
        skill_type,
        attrs,
        title,
        content,
        children,
    } = &decoded.blocks[0].kind
    {
        assert_eq!(*skill_type, SkillBlockType::Skill);
        assert_eq!(attrs.get("name"), Some("debugging"));
        assert!(title.is_some());
        assert_eq!(content.len(), 1);
        assert_eq!(children.len(), 3);

        // Check child types
        if let BlockKind::SkillBlock { skill_type, .. } = &children[0].kind {
            assert_eq!(*skill_type, SkillBlockType::Precondition);
        } else {
            panic!("expected SkillBlock child");
        }
        if let BlockKind::SkillBlock { skill_type, .. } = &children[1].kind {
            assert_eq!(*skill_type, SkillBlockType::Step);
        } else {
            panic!("expected SkillBlock child");
        }
        if let BlockKind::SkillBlock { skill_type, .. } = &children[2].kind {
            assert_eq!(*skill_type, SkillBlockType::Verify);
        } else {
            panic!("expected SkillBlock child");
        }
    } else {
        panic!("expected SkillBlock");
    }
}

#[test]
fn roundtrip_code_block() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::CodeBlock {
                lang: Some("rust".into()),
                attrs: {
                    let mut a = Attrs::new();
                    a.id = Some("example".into());
                    a
                },
                code: "fn main() {}".into(),
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::CodeBlock { lang, attrs, code } = &decoded.blocks[0].kind {
        assert_eq!(lang.as_deref(), Some("rust"));
        assert_eq!(attrs.id.as_deref(), Some("example"));
        assert_eq!(code, "fn main() {}");
    } else {
        panic!("expected CodeBlock");
    }
}

#[test]
fn roundtrip_code_block_no_lang() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::CodeBlock {
                lang: None,
                attrs: Attrs::new(),
                code: "some code".into(),
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::CodeBlock { lang, .. } = &decoded.blocks[0].kind {
        assert_eq!(*lang, None);
    } else {
        panic!("expected CodeBlock");
    }
}

#[test]
fn roundtrip_list() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::List {
                ordered: true,
                items: vec![
                    ListItem {
                        content: vec![Inline::Text {
                            text: "First".into(),
                        }],
                        children: vec![],
                    },
                    ListItem {
                        content: vec![Inline::Text {
                            text: "Second".into(),
                        }],
                        children: vec![Block {
                            kind: BlockKind::Paragraph {
                                content: vec![Inline::Text {
                                    text: "Sub-paragraph".into(),
                                }],
                            },
                            span: sp(),
                        }],
                    },
                ],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::List { ordered, items } = &decoded.blocks[0].kind {
        assert!(*ordered);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].children.len(), 0);
        assert_eq!(items[1].children.len(), 1);
    } else {
        panic!("expected List");
    }
}

#[test]
fn roundtrip_blockquote() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::BlockQuote {
                content: vec![Block {
                    kind: BlockKind::Paragraph {
                        content: vec![Inline::Text {
                            text: "Quoted text".into(),
                        }],
                    },
                    span: sp(),
                }],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::BlockQuote { content } = &decoded.blocks[0].kind {
        assert_eq!(content.len(), 1);
    } else {
        panic!("expected BlockQuote");
    }
}

#[test]
fn roundtrip_thematic_break() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::ThematicBreak,
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    assert!(matches!(decoded.blocks[0].kind, BlockKind::ThematicBreak));
}

#[test]
fn roundtrip_table() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Table {
                attrs: Attrs::new(),
                caption: Some(vec![Inline::Text {
                    text: "My Table".into(),
                }]),
                headers: vec![
                    vec![Inline::Text { text: "A".into() }],
                    vec![Inline::Text { text: "B".into() }],
                ],
                rows: vec![vec![
                    vec![Inline::Text {
                        text: "1".into(),
                    }],
                    vec![Inline::Text {
                        text: "2".into(),
                    }],
                ]],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Table {
        caption,
        headers,
        rows,
        ..
    } = &decoded.blocks[0].kind
    {
        assert!(caption.is_some());
        assert_eq!(headers.len(), 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 2);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn roundtrip_figure() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Figure {
                attrs: Attrs::new(),
                caption: Some(vec![Inline::Text {
                    text: "A figure".into(),
                }]),
                src: "image.png".into(),
                meta: MediaMeta::default(),
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Figure {
        caption, src, ..
    } = &decoded.blocks[0].kind
    {
        assert!(caption.is_some());
        assert_eq!(src, "image.png");
    } else {
        panic!("expected Figure");
    }
}

#[test]
fn roundtrip_semantic_block() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::SemanticBlock {
                block_type: SemanticBlockType::Claim,
                attrs: Attrs::new(),
                title: Some(vec![Inline::Text {
                    text: "My Claim".into(),
                }]),
                content: vec![Inline::Text {
                    text: "This is a claim".into(),
                }],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::SemanticBlock {
        block_type,
        title,
        content,
        ..
    } = &decoded.blocks[0].kind
    {
        assert_eq!(*block_type, SemanticBlockType::Claim);
        assert!(title.is_some());
        assert_eq!(content.len(), 1);
    } else {
        panic!("expected SemanticBlock");
    }
}

#[test]
fn roundtrip_callout() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Callout {
                callout_type: CalloutType::Note,
                attrs: Attrs::new(),
                content: vec![Inline::Text {
                    text: "Take note".into(),
                }],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Callout {
        callout_type,
        content,
        ..
    } = &decoded.blocks[0].kind
    {
        assert_eq!(*callout_type, CalloutType::Note);
        assert_eq!(content.len(), 1);
    } else {
        panic!("expected Callout");
    }
}

#[test]
fn roundtrip_section_with_children() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Section {
                attrs: {
                    let mut a = Attrs::new();
                    a.id = Some("intro".into());
                    a
                },
                title: vec![Inline::Text {
                    text: "Introduction".into(),
                }],
                children: vec![Block {
                    kind: BlockKind::Paragraph {
                        content: vec![Inline::Text {
                            text: "First paragraph".into(),
                        }],
                    },
                    span: sp(),
                }],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Section {
        attrs,
        title,
        children,
    } = &decoded.blocks[0].kind
    {
        assert_eq!(attrs.id.as_deref(), Some("intro"));
        assert_eq!(title.len(), 1);
        assert_eq!(children.len(), 1);
    } else {
        panic!("expected Section");
    }
}

#[test]
fn roundtrip_all_inline_types() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Paragraph {
                content: vec![
                    Inline::Text {
                        text: "plain ".into(),
                    },
                    Inline::Emphasis {
                        content: vec![Inline::Text {
                            text: "italic".into(),
                        }],
                    },
                    Inline::Strong {
                        content: vec![Inline::Text {
                            text: "bold".into(),
                        }],
                    },
                    Inline::InlineCode {
                        code: "code()".into(),
                    },
                    Inline::Link {
                        text: vec![Inline::Text {
                            text: "click".into(),
                        }],
                        url: "https://example.com".into(),
                    },
                    Inline::Reference {
                        target: "ref1".into(),
                    },
                    Inline::Footnote {
                        content: vec![Inline::Text {
                            text: "note".into(),
                        }],
                    },
                    Inline::SoftBreak,
                    Inline::HardBreak,
                ],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Paragraph { content } = &decoded.blocks[0].kind {
        assert_eq!(content.len(), 9);
        assert!(matches!(&content[0], Inline::Text { text } if text == "plain "));
        assert!(matches!(&content[1], Inline::Emphasis { .. }));
        assert!(matches!(&content[2], Inline::Strong { .. }));
        assert!(matches!(&content[3], Inline::InlineCode { code } if code == "code()"));
        assert!(matches!(&content[4], Inline::Link { url, .. } if url == "https://example.com"));
        assert!(matches!(&content[5], Inline::Reference { target } if target == "ref1"));
        assert!(matches!(&content[6], Inline::Footnote { .. }));
        assert!(matches!(&content[7], Inline::SoftBreak));
        assert!(matches!(&content[8], Inline::HardBreak));
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn roundtrip_all_skill_types() {
    let skill_types = vec![
        SkillBlockType::Skill,
        SkillBlockType::Step,
        SkillBlockType::Verify,
        SkillBlockType::Precondition,
        SkillBlockType::OutputContract,
        SkillBlockType::Decision,
        SkillBlockType::Tool,
        SkillBlockType::Fallback,
        SkillBlockType::RedFlag,
        SkillBlockType::Example,
    ];
    for st in skill_types {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::SkillBlock {
                    skill_type: st.clone(),
                    attrs: Attrs::new(),
                    title: None,
                    content: vec![Inline::Text {
                        text: "test".into(),
                    }],
                    children: vec![],
                },
                span: sp(),
            }],
        };
        let decoded = roundtrip(&doc);
        if let BlockKind::SkillBlock { skill_type, .. } = &decoded.blocks[0].kind {
            assert_eq!(*skill_type, st, "skill type mismatch for {:?}", st);
        } else {
            panic!("expected SkillBlock");
        }
    }
}

#[test]
fn decode_rejects_bad_magic() {
    let result = decode(b"XX\x01\x00\x00");
    assert!(result.is_err());
}

#[test]
fn decode_rejects_bad_version() {
    let result = decode(b"AT\x02\x00\x00");
    assert!(result.is_err());
}

#[test]
fn decode_rejects_short_data() {
    let result = decode(b"AT");
    assert!(result.is_err());
}

#[test]
fn roundtrip_audio() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Audio {
                attrs: Attrs::new(),
                caption: Some(vec![Inline::Text {
                    text: "My audio".into(),
                }]),
                src: "audio.mp3".into(),
                meta: MediaMeta::default(),
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Audio { caption, src, .. } = &decoded.blocks[0].kind {
        assert!(caption.is_some());
        assert_eq!(src, "audio.mp3");
    } else {
        panic!("expected Audio");
    }
}

#[test]
fn roundtrip_video() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Video {
                attrs: Attrs::new(),
                caption: Some(vec![Inline::Text {
                    text: "My video".into(),
                }]),
                src: "video.mp4".into(),
                meta: MediaMeta::default(),
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Video { caption, src, .. } = &decoded.blocks[0].kind {
        assert!(caption.is_some());
        assert_eq!(src, "video.mp4");
    } else {
        panic!("expected Video");
    }
}

#[test]
fn roundtrip_figure_with_media_meta() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Figure {
                attrs: Attrs::new(),
                caption: Some(vec![Inline::Text { text: "Photo".into() }]),
                src: "photo.jpg".into(),
                meta: MediaMeta {
                    alt: Some("A sunset".into()),
                    width: Some(1920),
                    height: Some(1080),
                    duration: None,
                    mime: Some("image/jpeg".into()),
                    poster: None,
                },
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Figure { meta, src, .. } = &decoded.blocks[0].kind {
        assert_eq!(src, "photo.jpg");
        assert_eq!(meta.alt.as_deref(), Some("A sunset"));
        assert_eq!(meta.width, Some(1920));
        assert_eq!(meta.height, Some(1080));
        assert_eq!(meta.mime.as_deref(), Some("image/jpeg"));
        assert!(meta.poster.is_none());
        assert!(meta.duration.is_none());
    } else {
        panic!("expected Figure");
    }
}

#[test]
fn roundtrip_video_with_full_meta() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Video {
                attrs: Attrs::new(),
                caption: None,
                src: "vid.mp4".into(),
                meta: MediaMeta {
                    alt: Some("A video".into()),
                    width: Some(1280),
                    height: Some(720),
                    duration: Some(120.5),
                    mime: Some("video/mp4".into()),
                    poster: Some("thumb.jpg".into()),
                },
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Video { meta, .. } = &decoded.blocks[0].kind {
        assert_eq!(meta.alt.as_deref(), Some("A video"));
        assert_eq!(meta.width, Some(1280));
        assert_eq!(meta.height, Some(720));
        assert_eq!(meta.duration, Some(120.5));
        assert_eq!(meta.mime.as_deref(), Some("video/mp4"));
        assert_eq!(meta.poster.as_deref(), Some("thumb.jpg"));
    } else {
        panic!("expected Video");
    }
}

#[test]
fn roundtrip_inline_image() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![Block {
            kind: BlockKind::Paragraph {
                content: vec![Inline::Image {
                    alt: "photo".into(),
                    src: "img.png".into(),
                }],
            },
            span: sp(),
        }],
    };
    let decoded = roundtrip(&doc);
    if let BlockKind::Paragraph { content } = &decoded.blocks[0].kind {
        assert_eq!(content.len(), 1);
        if let Inline::Image { alt, src } = &content[0] {
            assert_eq!(alt, "photo");
            assert_eq!(src, "img.png");
        } else {
            panic!("expected Image inline");
        }
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn roundtrip_all_semantic_block_types() {
    let all_types = vec![
        SemanticBlockType::Claim,
        SemanticBlockType::Evidence,
        SemanticBlockType::Definition,
        SemanticBlockType::Theorem,
        SemanticBlockType::Assumption,
        SemanticBlockType::Result,
        SemanticBlockType::Conclusion,
        SemanticBlockType::Requirement,
        SemanticBlockType::Recommendation,
    ];
    for bt in all_types {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::SemanticBlock {
                    block_type: bt.clone(),
                    attrs: Attrs::new(),
                    title: Some(vec![Inline::Text {
                        text: format!("{:?} title", bt),
                    }]),
                    content: vec![Inline::Text {
                        text: format!("{:?} content", bt),
                    }],
                },
                span: sp(),
            }],
        };
        let decoded = roundtrip(&doc);
        if let BlockKind::SemanticBlock {
            block_type, title, content, ..
        } = &decoded.blocks[0].kind
        {
            assert_eq!(
                *block_type, bt,
                "SemanticBlockType roundtrip failed for {:?}",
                bt
            );
            // Verify title and content survived too
            if let Some(title_inlines) = title {
                if let Inline::Text { text } = &title_inlines[0] {
                    assert_eq!(*text, format!("{:?} title", bt));
                }
            } else {
                panic!("expected title for {:?}", bt);
            }
            if let Inline::Text { text } = &content[0] {
                assert_eq!(*text, format!("{:?} content", bt));
            }
        } else {
            panic!("expected SemanticBlock for {:?}", bt);
        }
    }
}

#[test]
fn roundtrip_all_callout_types() {
    let all_types = vec![
        CalloutType::Note,
        CalloutType::Warning,
        CalloutType::Info,
        CalloutType::Tip,
    ];
    for ct in all_types {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::Callout {
                    callout_type: ct.clone(),
                    attrs: Attrs::new(),
                    content: vec![Inline::Text {
                        text: format!("{:?} callout content", ct),
                    }],
                },
                span: sp(),
            }],
        };
        let decoded = roundtrip(&doc);
        if let BlockKind::Callout {
            callout_type, content, ..
        } = &decoded.blocks[0].kind
        {
            assert_eq!(
                *callout_type, ct,
                "CalloutType roundtrip failed for {:?}",
                ct
            );
            if let Inline::Text { text } = &content[0] {
                assert_eq!(*text, format!("{:?} callout content", ct));
            }
        } else {
            panic!("expected Callout for {:?}", ct);
        }
    }
}
