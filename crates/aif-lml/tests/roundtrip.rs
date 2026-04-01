use aif_core::ast::*;
use aif_core::span::Span;
use aif_lml::{parse_lml, render_lml_aggressive};
use std::collections::BTreeMap;

#[test]
fn figure_roundtrip() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::Figure {
            attrs: Attrs::new(),
            caption: Some(vec![text("A nice photo")]),
            src: "photo.jpg".to_string(),
            meta: MediaMeta {
                alt: Some("Sunset".to_string()),
                width: Some(1024),
                height: Some(768),
                duration: None,
                mime: None,
                poster: None,
            },
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::Figure { src, meta, caption, .. } => {
            assert_eq!(src, "photo.jpg");
            assert_eq!(meta.alt.as_deref(), Some("Sunset"));
            assert_eq!(meta.width, Some(1024));
            assert_eq!(meta.height, Some(768));
            assert!(meta.duration.is_none());
            let cap = caption.as_ref().unwrap();
            assert_eq!(cap[0], text("A nice photo"));
        }
        other => panic!("Expected Figure, got {:?}", other),
    }
}

#[test]
fn audio_roundtrip() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::Audio {
            attrs: Attrs::new(),
            caption: Some(vec![text("Episode 1")]),
            src: "podcast.mp3".to_string(),
            meta: MediaMeta {
                alt: Some("Podcast episode".to_string()),
                width: None,
                height: None,
                duration: Some(3600.0),
                mime: None,
                poster: None,
            },
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::Audio { src, meta, caption, .. } => {
            assert_eq!(src, "podcast.mp3");
            assert_eq!(meta.alt.as_deref(), Some("Podcast episode"));
            assert_eq!(meta.duration, Some(3600.0));
            let cap = caption.as_ref().unwrap();
            assert_eq!(cap[0], text("Episode 1"));
        }
        other => panic!("Expected Audio, got {:?}", other),
    }
}

#[test]
fn video_roundtrip() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::Video {
            attrs: Attrs::new(),
            caption: Some(vec![text("Tutorial video")]),
            src: "tutorial.mp4".to_string(),
            meta: MediaMeta {
                alt: Some("Tutorial".to_string()),
                width: Some(1920),
                height: Some(1080),
                duration: Some(300.5),
                mime: None,
                poster: Some("thumb.jpg".to_string()),
            },
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::Video { src, meta, caption, .. } => {
            assert_eq!(src, "tutorial.mp4");
            assert_eq!(meta.alt.as_deref(), Some("Tutorial"));
            assert_eq!(meta.width, Some(1920));
            assert_eq!(meta.height, Some(1080));
            assert_eq!(meta.duration, Some(300.5));
            assert_eq!(meta.poster.as_deref(), Some("thumb.jpg"));
            let cap = caption.as_ref().unwrap();
            assert_eq!(cap[0], text("Tutorial video"));
        }
        other => panic!("Expected Video, got {:?}", other),
    }
}

#[test]
fn figure_no_caption_roundtrip() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::Figure {
            attrs: Attrs::new(),
            caption: None,
            src: "diagram.svg".to_string(),
            meta: MediaMeta::default(),
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::Figure { src, caption, .. } => {
            assert_eq!(src, "diagram.svg");
            assert!(caption.is_none());
        }
        other => panic!("Expected Figure, got {:?}", other),
    }
}

fn text(s: &str) -> Inline {
    Inline::Text { text: s.to_string() }
}

fn block(kind: BlockKind) -> Block {
    Block { kind, span: Span::empty() }
}

#[test]
fn paragraph_roundtrip() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::Paragraph {
            content: vec![text("Hello world")],
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::Paragraph { content } => {
            assert_eq!(content, &vec![text("Hello world")]);
        }
        other => panic!("Expected Paragraph, got {:?}", other),
    }
}

#[test]
fn skill_blocks_parsing() {
    let input = r#"@skill(name=debugging):
@pre: Check if error is reproducible
@step(order=1): Reproduce the bug
@step(order=2): Isolate the cause
@verify: Confirm the fix resolves the issue
"#;
    let doc = parse_lml(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);

    match &doc.blocks[0].kind {
        BlockKind::SkillBlock {
            skill_type,
            attrs,
            children,
            ..
        } => {
            assert_eq!(*skill_type, SkillBlockType::Skill);
            assert_eq!(attrs.get("name"), Some("debugging"));
            assert_eq!(children.len(), 4);

            // Check @pre
            match &children[0].kind {
                BlockKind::SkillBlock { skill_type, content, .. } => {
                    assert_eq!(*skill_type, SkillBlockType::Precondition);
                    assert_eq!(content, &vec![text("Check if error is reproducible")]);
                }
                other => panic!("Expected Precondition, got {:?}", other),
            }

            // Check @step(order=1)
            match &children[1].kind {
                BlockKind::SkillBlock { skill_type, attrs, content, .. } => {
                    assert_eq!(*skill_type, SkillBlockType::Step);
                    assert_eq!(attrs.get("order"), Some("1"));
                    assert_eq!(content, &vec![text("Reproduce the bug")]);
                }
                other => panic!("Expected Step, got {:?}", other),
            }

            // Check @step(order=2)
            match &children[2].kind {
                BlockKind::SkillBlock { skill_type, attrs, content, .. } => {
                    assert_eq!(*skill_type, SkillBlockType::Step);
                    assert_eq!(attrs.get("order"), Some("2"));
                    assert_eq!(content, &vec![text("Isolate the cause")]);
                }
                other => panic!("Expected Step, got {:?}", other),
            }

            // Check @verify
            match &children[3].kind {
                BlockKind::SkillBlock { skill_type, content, .. } => {
                    assert_eq!(*skill_type, SkillBlockType::Verify);
                    assert_eq!(content, &vec![text("Confirm the fix resolves the issue")]);
                }
                other => panic!("Expected Verify, got {:?}", other),
            }
        }
        other => panic!("Expected SkillBlock, got {:?}", other),
    }
}

#[test]
fn code_block_roundtrip() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::CodeBlock {
            lang: Some("python".to_string()),
            attrs: Attrs::new(),
            code: "print('hello')\n".to_string(),
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), Some("python"));
            assert_eq!(code, "print('hello')\n");
        }
        other => panic!("Expected CodeBlock, got {:?}", other),
    }
}

#[test]
fn list_roundtrip_unordered() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::List {
            ordered: false,
            items: vec![
                ListItem { content: vec![text("alpha")], children: vec![] },
                ListItem { content: vec![text("beta")], children: vec![] },
            ],
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].content, vec![text("alpha")]);
            assert_eq!(items[1].content, vec![text("beta")]);
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn list_roundtrip_ordered() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::List {
            ordered: true,
            items: vec![
                ListItem { content: vec![text("first")], children: vec![] },
                ListItem { content: vec![text("second")], children: vec![] },
                ListItem { content: vec![text("third")], children: vec![] },
            ],
        })],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(ordered);
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].content, vec![text("first")]);
            assert_eq!(items[2].content, vec![text("third")]);
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn metadata_parsing() {
    let input = "#title: My Skill Guide\n#author: Test Author\n#version: 1.0\n\nSome content\n";
    let doc = parse_lml(input).unwrap();
    assert_eq!(doc.metadata.get("title").unwrap(), "My Skill Guide");
    assert_eq!(doc.metadata.get("author").unwrap(), "Test Author");
    assert_eq!(doc.metadata.get("version").unwrap(), "1.0");
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn mixed_content() {
    // Test parsing a mixed document with metadata, headings, code, lists,
    // and thematic breaks. Note: heading-based sections consume subsequent
    // non-heading blocks as children (inherent to markdown-style syntax).
    let input = r#"#title: Test

# Introduction

Welcome to the guide.

# Code Examples

```bash
echo hello
```

# Items

- item one
- item two

---
"#;
    let parsed = parse_lml(input).unwrap();

    assert_eq!(parsed.metadata.get("title").unwrap(), "Test");

    // Three sections + thematic break at end
    // The --- appears after the last section's content ends
    // In heading-based parsing, sections consume content until the next
    // same-or-shallower heading, so the thematic break is inside the last section.
    assert_eq!(parsed.blocks.len(), 3);

    // Section 1: Introduction
    match &parsed.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            assert_eq!(title, &vec![text("Introduction")]);
            assert_eq!(children.len(), 1);
            match &children[0].kind {
                BlockKind::Paragraph { content } => {
                    assert_eq!(content, &vec![text("Welcome to the guide.")]);
                }
                other => panic!("Expected Paragraph, got {:?}", other),
            }
        }
        other => panic!("Expected Section, got {:?}", other),
    }

    // Section 2: Code Examples
    match &parsed.blocks[1].kind {
        BlockKind::Section { title, children, .. } => {
            assert_eq!(title, &vec![text("Code Examples")]);
            assert_eq!(children.len(), 1);
            match &children[0].kind {
                BlockKind::CodeBlock { lang, code, .. } => {
                    assert_eq!(lang.as_deref(), Some("bash"));
                    assert_eq!(code, "echo hello\n");
                }
                other => panic!("Expected CodeBlock, got {:?}", other),
            }
        }
        other => panic!("Expected Section, got {:?}", other),
    }

    // Section 3: Items (contains list + thematic break)
    match &parsed.blocks[2].kind {
        BlockKind::Section { title, children, .. } => {
            assert_eq!(title, &vec![text("Items")]);
            assert!(children.len() >= 1);
            match &children[0].kind {
                BlockKind::List { ordered, items } => {
                    assert!(!ordered);
                    assert_eq!(items.len(), 2);
                }
                other => panic!("Expected List, got {:?}", other),
            }
        }
        other => panic!("Expected Section, got {:?}", other),
    }
}

#[test]
fn blockquote_roundtrip() {
    let input = "> This is a quote\n> with two lines\n\n";
    let doc = parse_lml(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::BlockQuote { content } => {
            assert_eq!(content.len(), 1);
            match &content[0].kind {
                BlockKind::Paragraph { content: c } => {
                    assert_eq!(c, &vec![text("This is a quote with two lines")]);
                }
                other => panic!("Expected Paragraph in blockquote, got {:?}", other),
            }
        }
        other => panic!("Expected BlockQuote, got {:?}", other),
    }
}

#[test]
fn heading_depth() {
    let input = "# Top Level\n\nParagraph under top.\n\n## Nested\n\nNested content.\n";
    let doc = parse_lml(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            assert_eq!(title, &vec![text("Top Level")]);
            // Should contain: paragraph, nested section
            assert_eq!(children.len(), 2);
            match &children[1].kind {
                BlockKind::Section { title: nested_title, .. } => {
                    assert_eq!(nested_title, &vec![text("Nested")]);
                }
                other => panic!("Expected nested Section, got {:?}", other),
            }
        }
        other => panic!("Expected Section, got {:?}", other),
    }
}

#[test]
fn all_skill_directives() {
    let directives = vec![
        ("@step: Do something", SkillBlockType::Step),
        ("@verify: Check it", SkillBlockType::Verify),
        ("@pre: Before this", SkillBlockType::Precondition),
        ("@output: Expected result", SkillBlockType::OutputContract),
        ("@decision: Choose path", SkillBlockType::Decision),
        ("@tool: Use hammer", SkillBlockType::Tool),
        ("@fallback: Plan B", SkillBlockType::Fallback),
        ("@redflag: Watch out", SkillBlockType::RedFlag),
        ("@example: Like this", SkillBlockType::Example),
    ];

    for (input, expected_type) in directives {
        let doc = parse_lml(input).unwrap();
        match &doc.blocks[0].kind {
            BlockKind::SkillBlock { skill_type, .. } => {
                assert_eq!(*skill_type, expected_type, "Failed for input: {}", input);
            }
            other => panic!("Expected SkillBlock for '{}', got {:?}", input, other),
        }
    }
}
