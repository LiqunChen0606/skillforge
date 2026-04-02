use aif_core::ast::*;
use aif_html::importer::{import_html, ImportMode};

// ===== Task 1: Basic Paragraph Import =====

#[test]
fn import_single_paragraph() {
    let result = import_html("<html><body><p>Hello world</p></body></html>", false);
    assert_eq!(result.mode, ImportMode::Generic);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Paragraph { content } => {
            assert_eq!(content.len(), 1);
            assert!(matches!(&content[0], Inline::Text { text } if text == "Hello world"));
        }
        _ => panic!("expected Paragraph"),
    }
}

#[test]
fn import_multiple_paragraphs() {
    let html = "<html><body><p>First</p><p>Second</p><p>Third</p></body></html>";
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 3);
}

#[test]
fn import_empty_body() {
    let result = import_html("<html><body></body></html>", false);
    assert_eq!(result.document.blocks.len(), 0);
}

// ===== Task 2: Inline Element Parsing =====

#[test]
fn import_strong_text() {
    let result = import_html("<body><p><strong>bold</strong></p></body>", false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    assert!(matches!(&content[0], Inline::Strong { content } if matches!(&content[0], Inline::Text { text } if text == "bold")));
}

#[test]
fn import_b_tag() {
    let result = import_html("<body><p><b>bold</b></p></body>", false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    assert!(matches!(&content[0], Inline::Strong { .. }));
}

#[test]
fn import_emphasis_text() {
    let result = import_html("<body><p><em>italic</em></p></body>", false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    assert!(matches!(&content[0], Inline::Emphasis { content } if matches!(&content[0], Inline::Text { text } if text == "italic")));
}

#[test]
fn import_i_tag() {
    let result = import_html("<body><p><i>italic</i></p></body>", false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    assert!(matches!(&content[0], Inline::Emphasis { .. }));
}

#[test]
fn import_inline_code() {
    let result = import_html("<body><p><code>let x = 1;</code></p></body>", false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    assert!(matches!(&content[0], Inline::InlineCode { code } if code == "let x = 1;"));
}

#[test]
fn import_link() {
    let result = import_html(r#"<body><p><a href="https://example.com">click</a></p></body>"#, false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    match &content[0] {
        Inline::Link { text, url } => {
            assert_eq!(url, "https://example.com");
            assert!(matches!(&text[0], Inline::Text { text } if text == "click"));
        }
        _ => panic!("expected Link"),
    }
}

#[test]
fn import_inline_image() {
    let result = import_html(r#"<body><p><img src="photo.jpg" alt="A photo"></p></body>"#, false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    match &content[0] {
        Inline::Image { alt, src } => {
            assert_eq!(src, "photo.jpg");
            assert_eq!(alt, "A photo");
        }
        _ => panic!("expected Image"),
    }
}

#[test]
fn import_hard_break() {
    let result = import_html("<body><p>line1<br>line2</p></body>", false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    assert_eq!(content.len(), 3);
    assert!(matches!(&content[1], Inline::HardBreak));
}

#[test]
fn import_mixed_inlines() {
    let html = r#"<body><p>Hello <strong>bold</strong> and <em>italic</em> and <code>code</code></p></body>"#;
    let result = import_html(html, false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    // Should have: Text, Strong, Text, Emphasis, Text, InlineCode
    assert!(content.len() >= 5);
    assert!(matches!(&content[0], Inline::Text { text } if text == "Hello "));
    assert!(matches!(&content[1], Inline::Strong { .. }));
}

#[test]
fn import_unknown_inline_element_recurses() {
    let html = "<body><p><span>inside span</span></p></body>";
    let result = import_html(html, false);
    let content = extract_paragraph_content(&result.document.blocks[0]);
    // <span> should recurse, yielding the text
    assert!(content.iter().any(|i| matches!(i, Inline::Text { text } if text == "inside span")));
}

// ===== Task 3: Headings, Sections, Metadata =====

#[test]
fn import_explicit_section_with_heading() {
    let html = "<body><section><h2>Title</h2><p>Content</p></section></body>";
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            assert!(matches!(&title[0], Inline::Text { text } if text == "Title"));
            assert_eq!(children.len(), 1);
            assert!(matches!(&children[0].kind, BlockKind::Paragraph { .. }));
        }
        _ => panic!("expected Section"),
    }
}

#[test]
fn import_section_with_id() {
    let html = r#"<body><section id="intro"><h2>Intro</h2></section></body>"#;
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Section { attrs, .. } => {
            assert_eq!(attrs.id, Some("intro".into()));
        }
        _ => panic!("expected Section"),
    }
}

#[test]
fn import_nested_sections() {
    let html = r#"<body>
        <section><h2>Outer</h2>
            <section><h3>Inner</h3><p>Content</p></section>
        </section>
    </body>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            assert!(matches!(&title[0], Inline::Text { text } if text == "Outer"));
            assert_eq!(children.len(), 1);
            match &children[0].kind {
                BlockKind::Section { title: inner_title, children: inner_children, .. } => {
                    assert!(matches!(&inner_title[0], Inline::Text { text } if text == "Inner"));
                    assert_eq!(inner_children.len(), 1);
                }
                _ => panic!("expected nested Section"),
            }
        }
        _ => panic!("expected Section"),
    }
}

#[test]
fn import_bare_heading_creates_section() {
    let html = "<body><h2>Title</h2><p>Paragraph under heading</p></body>";
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            assert!(matches!(&title[0], Inline::Text { text } if text == "Title"));
            assert_eq!(children.len(), 1);
        }
        _ => panic!("expected Section from bare heading"),
    }
}

#[test]
fn import_bare_headings_nested() {
    let html = "<body><h2>A</h2><p>P1</p><h3>B</h3><p>P2</p><h2>C</h2><p>P3</p></body>";
    let result = import_html(html, false);
    // Should create: Section(A, [P1, Section(B, [P2])]), Section(C, [P3])
    assert_eq!(result.document.blocks.len(), 2);
    match &result.document.blocks[0].kind {
        BlockKind::Section { title, children, .. } => {
            assert!(matches!(&title[0], Inline::Text { text } if text == "A"));
            // Children: P1 and Section(B)
            assert_eq!(children.len(), 2);
            assert!(matches!(&children[0].kind, BlockKind::Paragraph { .. }));
            assert!(matches!(&children[1].kind, BlockKind::Section { .. }));
        }
        _ => panic!("expected Section"),
    }
}

#[test]
fn import_title_metadata() {
    let html = "<html><head><title>My Page</title></head><body></body></html>";
    let result = import_html(html, false);
    assert_eq!(result.document.metadata.get("title").map(|s| s.as_str()), Some("My Page"));
}

#[test]
fn import_description_metadata() {
    let html = r#"<html><head><meta name="description" content="A summary"></head><body></body></html>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.metadata.get("summary").map(|s| s.as_str()), Some("A summary"));
}

// ===== Task 4: Code Blocks, Block Quotes, Lists, Thematic Breaks =====

#[test]
fn import_code_block_with_language() {
    let html = r#"<body><pre><code class="language-rust">fn main() {}</code></pre></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        }
        _ => panic!("expected CodeBlock"),
    }
}

#[test]
fn import_code_block_without_language() {
    let html = "<body><pre><code>plain code</code></pre></body>";
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::CodeBlock { lang, code, .. } => {
            assert_eq!(lang.as_deref(), None);
            assert_eq!(code, "plain code");
        }
        _ => panic!("expected CodeBlock"),
    }
}

#[test]
fn import_blockquote() {
    let html = "<body><blockquote><p>Quoted text</p></blockquote></body>";
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::BlockQuote { content } => {
            assert_eq!(content.len(), 1);
            assert!(matches!(&content[0].kind, BlockKind::Paragraph { .. }));
        }
        _ => panic!("expected BlockQuote"),
    }
}

#[test]
fn import_unordered_list() {
    let html = "<body><ul><li>One</li><li>Two</li></ul></body>";
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[0].content[0], Inline::Text { text } if text == "One"));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn import_ordered_list() {
    let html = "<body><ol><li>First</li><li>Second</li></ol></body>";
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::List { ordered, items } => {
            assert!(ordered);
            assert_eq!(items.len(), 2);
        }
        _ => panic!("expected ordered List"),
    }
}

#[test]
fn import_nested_list() {
    let html = "<body><ul><li>Parent<ul><li>Child</li></ul></li></ul></body>";
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::List { items, .. } => {
            assert_eq!(items.len(), 1);
            assert!(matches!(&items[0].content[0], Inline::Text { text } if text == "Parent"));
            assert_eq!(items[0].children.len(), 1);
            match &items[0].children[0].kind {
                BlockKind::List { ordered, items: sub_items } => {
                    assert!(!ordered);
                    assert_eq!(sub_items.len(), 1);
                }
                _ => panic!("expected nested List"),
            }
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn import_thematic_break() {
    let html = "<body><p>Before</p><hr><p>After</p></body>";
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 3);
    assert!(matches!(&result.document.blocks[1].kind, BlockKind::ThematicBreak));
}

// ===== Task 5: Tables =====

#[test]
fn import_table_with_headers() {
    let html = r#"<body><table>
        <thead><tr><th>Name</th><th>Age</th></tr></thead>
        <tbody><tr><td>Alice</td><td>30</td></tr></tbody>
    </table></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Table { headers, rows, .. } => {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].len(), 2);
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn import_table_with_caption() {
    let html = "<body><table><caption>My Table</caption><tbody><tr><td>Cell</td></tr></tbody></table></body>";
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Table { caption, .. } => {
            let cap = caption.as_ref().expect("should have caption");
            assert!(matches!(&cap[0], Inline::Text { text } if text == "My Table"));
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn import_table_with_id() {
    let html = r#"<body><table id="data"><tbody><tr><td>X</td></tr></tbody></table></body>"#;
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Table { attrs, .. } => {
            assert_eq!(attrs.id, Some("data".into()));
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn import_table_without_thead() {
    let html = "<body><table><tr><td>A</td><td>B</td></tr><tr><td>C</td><td>D</td></tr></table></body>";
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Table { headers, rows, .. } => {
            assert!(headers.is_empty());
            assert_eq!(rows.len(), 2);
        }
        _ => panic!("expected Table"),
    }
}

// ===== Task 6: Media Blocks =====

#[test]
fn import_figure() {
    let html = r#"<body><figure id="fig1">
        <img src="photo.jpg" alt="Sunset" width="800" height="600">
        <figcaption>A sunset photo</figcaption>
    </figure></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Figure { attrs, caption, src, meta } => {
            assert_eq!(attrs.id, Some("fig1".into()));
            assert_eq!(src, "photo.jpg");
            assert_eq!(meta.alt, Some("Sunset".into()));
            assert_eq!(meta.width, Some(800));
            assert_eq!(meta.height, Some(600));
            let cap = caption.as_ref().expect("should have caption");
            assert!(matches!(&cap[0], Inline::Text { text } if text == "A sunset photo"));
        }
        _ => panic!("expected Figure"),
    }
}

#[test]
fn import_audio() {
    let html = r#"<body><audio id="track1"><source src="song.mp3" type="audio/mpeg"><p>Song title</p></audio></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Audio { attrs, caption, src, meta } => {
            assert_eq!(attrs.id, Some("track1".into()));
            assert_eq!(src, "song.mp3");
            assert_eq!(meta.mime, Some("audio/mpeg".into()));
            let cap = caption.as_ref().expect("should have caption");
            assert!(matches!(&cap[0], Inline::Text { text } if text == "Song title"));
        }
        _ => panic!("expected Audio"),
    }
}

#[test]
fn import_audio_with_src_attr() {
    let html = r#"<body><audio src="track.ogg"></audio></body>"#;
    let result = import_html(html, false);
    match &result.document.blocks[0].kind {
        BlockKind::Audio { src, .. } => {
            assert_eq!(src, "track.ogg");
        }
        _ => panic!("expected Audio"),
    }
}

#[test]
fn import_video() {
    let html = r#"<body><video id="vid1" src="clip.mp4" width="1920" height="1080" poster="thumb.jpg">
        <p>Video description</p>
    </video></body>"#;
    let result = import_html(html, false);
    assert_eq!(result.document.blocks.len(), 1);
    match &result.document.blocks[0].kind {
        BlockKind::Video { attrs, caption, src, meta } => {
            assert_eq!(attrs.id, Some("vid1".into()));
            assert_eq!(src, "clip.mp4");
            assert_eq!(meta.width, Some(1920));
            assert_eq!(meta.height, Some(1080));
            assert_eq!(meta.poster, Some("thumb.jpg".into()));
            let cap = caption.as_ref().expect("should have caption");
            assert!(matches!(&cap[0], Inline::Text { text } if text == "Video description"));
        }
        _ => panic!("expected Video"),
    }
}

// ===== Integration / Combined Tests =====

#[test]
fn import_full_document() {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Document</title>
    <meta name="description" content="A test page">
</head>
<body>
    <h1>Welcome</h1>
    <p>First paragraph with <strong>bold</strong> text.</p>
    <pre><code class="language-python">print("hello")</code></pre>
    <ul><li>Item A</li><li>Item B</li></ul>
    <hr>
    <table>
        <thead><tr><th>Col1</th></tr></thead>
        <tbody><tr><td>Val1</td></tr></tbody>
    </table>
</body>
</html>"#;

    let result = import_html(html, false);
    assert_eq!(result.document.metadata.get("title").map(|s| s.as_str()), Some("Test Document"));
    assert_eq!(result.document.metadata.get("summary").map(|s| s.as_str()), Some("A test page"));
    // h1 groups everything into a section
    assert!(!result.document.blocks.is_empty());
}

// ===== Helpers =====

fn extract_paragraph_content(block: &Block) -> &Vec<Inline> {
    match &block.kind {
        BlockKind::Paragraph { content } => content,
        _ => panic!("expected Paragraph, got {:?}", block.kind),
    }
}
