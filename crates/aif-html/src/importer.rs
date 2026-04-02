use aif_core::ast::*;
use aif_core::span::Span;
use ego_tree::NodeRef;
use scraper::{ElementRef, Html, Node, Selector};

/// Result of importing HTML into an AIF Document.
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlImportResult {
    pub document: Document,
    pub mode: ImportMode,
}

/// Whether the import detected AIF-emitted HTML or generic HTML.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportMode {
    AifRoundtrip,
    Generic,
}

/// Import an HTML string into an AIF Document.
///
/// `_strip_chrome` is reserved for future readability extraction.
pub fn import_html(input: &str, _strip_chrome: bool) -> HtmlImportResult {
    let html = Html::parse_document(input);
    let mut doc = Document::new();

    // Extract metadata from <head>
    extract_metadata(&html, &mut doc);

    // Find <body> or fall back to document root
    let body_sel = Selector::parse("body").unwrap();
    let root = html.select(&body_sel).next();

    let blocks = if let Some(body) = root {
        parse_children(body)
    } else {
        // No body tag — parse from root
        let root_el = html.root_element();
        parse_children(root_el)
    };

    doc.blocks = blocks;

    HtmlImportResult {
        document: doc,
        mode: ImportMode::Generic,
    }
}

fn span() -> Span {
    Span::new(0, 0)
}

// ---------------------------------------------------------------------------
// Metadata extraction
// ---------------------------------------------------------------------------

fn extract_metadata(html: &Html, doc: &mut Document) {
    // <title>
    let title_sel = Selector::parse("title").unwrap();
    if let Some(el) = html.select(&title_sel).next() {
        let text = el.text().collect::<String>();
        let text = text.trim().to_string();
        if !text.is_empty() {
            doc.metadata.insert("title".into(), text);
        }
    }

    // <meta name="description" content="...">
    let meta_sel = Selector::parse("meta[name=description]").unwrap();
    if let Some(el) = html.select(&meta_sel).next() {
        if let Some(content) = el.value().attr("content") {
            let content = content.trim().to_string();
            if !content.is_empty() {
                doc.metadata.insert("summary".into(), content);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Block parsing
// ---------------------------------------------------------------------------

/// Parse the direct children of an element into blocks.
fn parse_children(parent: ElementRef) -> Vec<Block> {
    let raw = collect_raw_elements(parent);
    group_headings_into_sections(raw)
}

/// A raw element before heading grouping.
enum RawBlock {
    Parsed(Block),
    Heading {
        level: u8,
        title: Vec<Inline>,
        id: Option<String>,
    },
}

/// First pass: convert each direct child into a RawBlock.
fn collect_raw_elements(parent: ElementRef) -> Vec<RawBlock> {
    let mut raw = Vec::new();

    for child in parent.children() {
        if let Some(el) = ElementRef::wrap(child) {
            let tag = el.value().name();
            match tag {
                "p" => {
                    let content = parse_inlines(el);
                    if !content.is_empty() {
                        raw.push(RawBlock::Parsed(Block {
                            kind: BlockKind::Paragraph { content },
                            span: span(),
                        }));
                    }
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let level = tag[1..].parse::<u8>().unwrap_or(2);
                    let title = parse_inlines(el);
                    let id = el.value().attr("id").map(String::from);
                    raw.push(RawBlock::Heading { level, title, id });
                }
                "section" => {
                    let block = parse_section(el);
                    raw.push(RawBlock::Parsed(block));
                }
                "pre" => {
                    let block = parse_pre(el);
                    raw.push(RawBlock::Parsed(block));
                }
                "blockquote" => {
                    let content = parse_children(el);
                    raw.push(RawBlock::Parsed(Block {
                        kind: BlockKind::BlockQuote { content },
                        span: span(),
                    }));
                }
                "ul" => {
                    let items = parse_list_items(el);
                    raw.push(RawBlock::Parsed(Block {
                        kind: BlockKind::List {
                            ordered: false,
                            items,
                        },
                        span: span(),
                    }));
                }
                "ol" => {
                    let items = parse_list_items(el);
                    raw.push(RawBlock::Parsed(Block {
                        kind: BlockKind::List {
                            ordered: true,
                            items,
                        },
                        span: span(),
                    }));
                }
                "hr" => {
                    raw.push(RawBlock::Parsed(Block {
                        kind: BlockKind::ThematicBreak,
                        span: span(),
                    }));
                }
                "table" => {
                    let block = parse_table(el);
                    raw.push(RawBlock::Parsed(block));
                }
                "figure" => {
                    let block = parse_figure(el);
                    raw.push(RawBlock::Parsed(block));
                }
                "audio" => {
                    let block = parse_audio(el);
                    raw.push(RawBlock::Parsed(block));
                }
                "video" => {
                    let block = parse_video(el);
                    raw.push(RawBlock::Parsed(block));
                }
                "div" | "main" | "article" | "header" | "footer" | "nav" | "aside" => {
                    // Transparent containers — recurse into children
                    let children = collect_raw_elements(el);
                    raw.extend(children);
                }
                _ => {
                    // Unknown block element: try to extract inline content as paragraph
                    let content = parse_inlines(el);
                    if !content.is_empty() {
                        raw.push(RawBlock::Parsed(Block {
                            kind: BlockKind::Paragraph { content },
                            span: span(),
                        }));
                    }
                }
            }
        }
        // Skip text nodes at block level (whitespace between elements)
    }

    raw
}

/// Second pass: group bare headings with following content into Section blocks.
fn group_headings_into_sections(raw: Vec<RawBlock>) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0;

    while i < raw.len() {
        match &raw[i] {
            RawBlock::Heading { .. } => {
                // Extract heading info
                let (level, title, id) = if let RawBlock::Heading { level, title, id } = &raw[i] {
                    (*level, title.clone(), id.clone())
                } else {
                    unreachable!()
                };

                // Gather following siblings until next heading of same or higher level
                let mut j = i + 1;
                while j < raw.len() {
                    match &raw[j] {
                        RawBlock::Heading {
                            level: next_level, ..
                        } if *next_level <= level => {
                            break;
                        }
                        _ => {
                            j += 1;
                        }
                    }
                }

                // Recursively group sub-headings within the range [i+1..j)
                let sub_raw: Vec<&RawBlock> = raw[i + 1..j].iter().collect();
                let children = group_headings_from_refs(&sub_raw);

                let mut attrs = Attrs::new();
                attrs.id = id;

                blocks.push(Block {
                    kind: BlockKind::Section {
                        attrs,
                        title,
                        children,
                    },
                    span: span(),
                });

                i = j;
            }
            RawBlock::Parsed(_) => {
                // Move parsed block out — we need to handle ownership carefully
                // Since we can't move from raw (borrowed), we'll reconstruct
                if let RawBlock::Parsed(block) = &raw[i] {
                    blocks.push(block.clone());
                }
                i += 1;
            }
        }
    }

    blocks
}

/// Helper: group headings from a slice of references (for nested heading grouping).
fn group_headings_from_refs(raw: &[&RawBlock]) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0;

    while i < raw.len() {
        match raw[i] {
            RawBlock::Heading { level, title, id } => {
                let level = *level;
                let title = title.clone();
                let id = id.clone();

                let mut j = i + 1;
                while j < raw.len() {
                    if let RawBlock::Heading {
                        level: next_level, ..
                    } = raw[j]
                    {
                        if *next_level <= level {
                            break;
                        }
                    }
                    j += 1;
                }

                let sub: Vec<&RawBlock> = raw[i + 1..j].iter().copied().collect();
                let children = group_headings_from_refs(&sub);

                let mut attrs = Attrs::new();
                attrs.id = id;

                blocks.push(Block {
                    kind: BlockKind::Section {
                        attrs,
                        title,
                        children,
                    },
                    span: span(),
                });

                i = j;
            }
            RawBlock::Parsed(block) => {
                blocks.push(block.clone());
                i += 1;
            }
        }
    }

    blocks
}

// ---------------------------------------------------------------------------
// Section parsing (explicit <section> elements)
// ---------------------------------------------------------------------------

fn parse_section(el: ElementRef) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    // Find the first heading child for section title
    let mut title = Vec::new();
    let mut children = Vec::new();
    let mut found_heading = false;

    for child in el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            let tag = child_el.value().name();
            if !found_heading && matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                title = parse_inlines(child_el);
                found_heading = true;
                continue;
            }

            // Parse remaining children as blocks
            let child_blocks = parse_single_element(child_el);
            children.extend(child_blocks);
        }
    }

    Block {
        kind: BlockKind::Section {
            attrs,
            title,
            children,
        },
        span: span(),
    }
}

/// Parse a single element into zero or more blocks.
fn parse_single_element(el: ElementRef) -> Vec<Block> {
    let tag = el.value().name();
    match tag {
        "p" => {
            let content = parse_inlines(el);
            if content.is_empty() {
                vec![]
            } else {
                vec![Block {
                    kind: BlockKind::Paragraph { content },
                    span: span(),
                }]
            }
        }
        "section" => vec![parse_section(el)],
        "pre" => vec![parse_pre(el)],
        "blockquote" => {
            let content = parse_children(el);
            vec![Block {
                kind: BlockKind::BlockQuote { content },
                span: span(),
            }]
        }
        "ul" => {
            let items = parse_list_items(el);
            vec![Block {
                kind: BlockKind::List {
                    ordered: false,
                    items,
                },
                span: span(),
            }]
        }
        "ol" => {
            let items = parse_list_items(el);
            vec![Block {
                kind: BlockKind::List {
                    ordered: true,
                    items,
                },
                span: span(),
            }]
        }
        "hr" => vec![Block {
            kind: BlockKind::ThematicBreak,
            span: span(),
        }],
        "table" => vec![parse_table(el)],
        "figure" => vec![parse_figure(el)],
        "audio" => vec![parse_audio(el)],
        "video" => vec![parse_video(el)],
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            // Bare heading inside section — create nested section
            let title = parse_inlines(el);
            let id = el.value().attr("id").map(String::from);
            let mut attrs = Attrs::new();
            attrs.id = id;
            vec![Block {
                kind: BlockKind::Section {
                    attrs,
                    title,
                    children: vec![],
                },
                span: span(),
            }]
        }
        "div" | "main" | "article" | "header" | "footer" | "nav" | "aside" => {
            parse_children(el)
        }
        _ => {
            let content = parse_inlines(el);
            if content.is_empty() {
                vec![]
            } else {
                vec![Block {
                    kind: BlockKind::Paragraph { content },
                    span: span(),
                }]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pre/Code block parsing
// ---------------------------------------------------------------------------

fn parse_pre(el: ElementRef) -> Block {
    let code_sel = Selector::parse("code").unwrap();
    if let Some(code_el) = el.select(&code_sel).next() {
        let lang = extract_language(code_el);
        let code = code_el.text().collect::<String>();
        Block {
            kind: BlockKind::CodeBlock {
                lang,
                attrs: Attrs::new(),
                code,
            },
            span: span(),
        }
    } else {
        // <pre> without <code>
        let code = el.text().collect::<String>();
        Block {
            kind: BlockKind::CodeBlock {
                lang: None,
                attrs: Attrs::new(),
                code,
            },
            span: span(),
        }
    }
}

fn extract_language(code_el: ElementRef) -> Option<String> {
    for class in code_el.value().classes() {
        if let Some(lang) = class.strip_prefix("language-") {
            return Some(lang.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// List parsing
// ---------------------------------------------------------------------------

fn parse_list_items(list_el: ElementRef) -> Vec<ListItem> {
    let mut items = Vec::new();

    // Only select direct children <li> elements
    for li in list_el.children().filter_map(ElementRef::wrap) {
        if li.value().name() != "li" {
            continue;
        }

        let mut content = Vec::new();
        let mut children = Vec::new();

        for child in li.children() {
            if let Some(child_el) = ElementRef::wrap(child) {
                let tag = child_el.value().name();
                match tag {
                    "ul" => {
                        let sub_items = parse_list_items(child_el);
                        children.push(Block {
                            kind: BlockKind::List {
                                ordered: false,
                                items: sub_items,
                            },
                            span: span(),
                        });
                    }
                    "ol" => {
                        let sub_items = parse_list_items(child_el);
                        children.push(Block {
                            kind: BlockKind::List {
                                ordered: true,
                                items: sub_items,
                            },
                            span: span(),
                        });
                    }
                    _ => {
                        // Inline elements within the list item
                        let inlines = parse_inline_node(child);
                        content.extend(inlines);
                    }
                }
            } else if let Some(text) = child.value().as_text() {
                let t = text.to_string();
                if !t.trim().is_empty() {
                    content.push(Inline::Text { text: t });
                }
            }
        }

        items.push(ListItem { content, children });
    }

    items
}

// ---------------------------------------------------------------------------
// Table parsing
// ---------------------------------------------------------------------------

fn parse_table(el: ElementRef) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut caption = None;
    let mut headers: Vec<Vec<Inline>> = Vec::new();
    let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();

    // Caption
    let caption_sel = Selector::parse("caption").unwrap();
    if let Some(cap_el) = el.select(&caption_sel).next() {
        let inlines = parse_inlines(cap_el);
        if !inlines.is_empty() {
            caption = Some(inlines);
        }
    }

    // Headers from <thead>
    let thead_sel = Selector::parse("thead").unwrap();
    if let Some(thead) = el.select(&thead_sel).next() {
        let tr_sel = Selector::parse("tr").unwrap();
        if let Some(tr) = thead.select(&tr_sel).next() {
            let th_sel = Selector::parse("th").unwrap();
            for th in tr.select(&th_sel) {
                headers.push(parse_inlines(th));
            }
        }
    }

    // Rows from <tbody> or directly
    let tbody_sel = Selector::parse("tbody").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();

    if let Some(tbody) = el.select(&tbody_sel).next() {
        for tr in tbody.select(&tr_sel) {
            let mut row = Vec::new();
            for td in tr.select(&td_sel) {
                row.push(parse_inlines(td));
            }
            rows.push(row);
        }
    } else {
        // No <tbody>, look for direct <tr> children
        for tr in el.select(&tr_sel) {
            // Skip if this is in thead
            if tr.parent().and_then(|p| {
                ElementRef::wrap(p).map(|e| e.value().name() == "thead")
            }).unwrap_or(false) {
                continue;
            }
            let mut row = Vec::new();
            for td in tr.select(&td_sel) {
                row.push(parse_inlines(td));
            }
            if !row.is_empty() {
                rows.push(row);
            }
        }
    }

    Block {
        kind: BlockKind::Table {
            attrs,
            caption,
            headers,
            rows,
        },
        span: span(),
    }
}

// ---------------------------------------------------------------------------
// Figure parsing
// ---------------------------------------------------------------------------

fn parse_figure(el: ElementRef) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut src = String::new();
    let mut meta = MediaMeta::default();
    let mut caption = None;

    let img_sel = Selector::parse("img").unwrap();
    if let Some(img) = el.select(&img_sel).next() {
        src = img.value().attr("src").unwrap_or("").to_string();
        meta.alt = img.value().attr("alt").map(String::from);
        meta.width = img.value().attr("width").and_then(|w| w.parse().ok());
        meta.height = img.value().attr("height").and_then(|h| h.parse().ok());
    }

    let figcaption_sel = Selector::parse("figcaption").unwrap();
    if let Some(cap_el) = el.select(&figcaption_sel).next() {
        let inlines = parse_inlines(cap_el);
        if !inlines.is_empty() {
            caption = Some(inlines);
        }
    }

    Block {
        kind: BlockKind::Figure {
            attrs,
            caption,
            src,
            meta,
        },
        span: span(),
    }
}

// ---------------------------------------------------------------------------
// Audio parsing
// ---------------------------------------------------------------------------

fn parse_audio(el: ElementRef) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut src = el.value().attr("src").unwrap_or("").to_string();
    let mut meta = MediaMeta::default();
    let mut caption = None;

    // Check for <source> child
    let source_sel = Selector::parse("source").unwrap();
    if let Some(source_el) = el.select(&source_sel).next() {
        if src.is_empty() {
            src = source_el.value().attr("src").unwrap_or("").to_string();
        }
        meta.mime = source_el.value().attr("type").map(String::from);
    }

    // Caption from <p> child
    let p_sel = Selector::parse("p").unwrap();
    if let Some(p_el) = el.select(&p_sel).next() {
        let inlines = parse_inlines(p_el);
        if !inlines.is_empty() {
            caption = Some(inlines);
        }
    }

    Block {
        kind: BlockKind::Audio {
            attrs,
            caption,
            src,
            meta,
        },
        span: span(),
    }
}

// ---------------------------------------------------------------------------
// Video parsing
// ---------------------------------------------------------------------------

fn parse_video(el: ElementRef) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let src = el.value().attr("src").unwrap_or("").to_string();
    let mut meta = MediaMeta::default();
    meta.width = el.value().attr("width").and_then(|w| w.parse().ok());
    meta.height = el.value().attr("height").and_then(|h| h.parse().ok());
    meta.poster = el.value().attr("poster").map(String::from);
    let mut caption = None;

    // Caption from <p> child
    let p_sel = Selector::parse("p").unwrap();
    if let Some(p_el) = el.select(&p_sel).next() {
        let inlines = parse_inlines(p_el);
        if !inlines.is_empty() {
            caption = Some(inlines);
        }
    }

    Block {
        kind: BlockKind::Video {
            attrs,
            caption,
            src,
            meta,
        },
        span: span(),
    }
}

// ---------------------------------------------------------------------------
// Inline parsing
// ---------------------------------------------------------------------------

/// Parse all inline content from an element's children.
pub fn parse_inlines(parent: ElementRef) -> Vec<Inline> {
    let mut inlines = Vec::new();

    for child in parent.children() {
        let node_inlines = parse_inline_node(child);
        inlines.extend(node_inlines);
    }

    inlines
}

/// Parse a single DOM node into inline elements.
fn parse_inline_node(node: NodeRef<'_, Node>) -> Vec<Inline> {
    if let Some(text) = node.value().as_text() {
        let t: String = text.to_string();
        if t.is_empty() {
            return vec![];
        }
        return vec![Inline::Text { text: t }];
    }

    if let Some(el) = ElementRef::wrap(node) {
        let tag = el.value().name();
        match tag {
            "strong" | "b" => {
                let content = parse_inlines(el);
                vec![Inline::Strong { content }]
            }
            "em" | "i" => {
                let content = parse_inlines(el);
                vec![Inline::Emphasis { content }]
            }
            "code" => {
                let code = el.text().collect::<String>();
                vec![Inline::InlineCode { code }]
            }
            "a" => {
                let url = el.value().attr("href").unwrap_or("").to_string();
                let text = parse_inlines(el);
                vec![Inline::Link { text, url }]
            }
            "img" => {
                let src = el.value().attr("src").unwrap_or("").to_string();
                let alt = el.value().attr("alt").unwrap_or("").to_string();
                vec![Inline::Image { alt, src }]
            }
            "br" => {
                vec![Inline::HardBreak]
            }
            _ => {
                // Unknown inline element — recurse into children
                parse_inlines(el)
            }
        }
    } else {
        vec![]
    }
}
