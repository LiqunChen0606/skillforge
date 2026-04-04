use aif_core::ast::*;
use aif_core::span::Span;
use ego_tree::NodeRef;
use scraper::{ElementRef, Html, Node, Selector};
use std::sync::OnceLock;

use crate::readability::{find_content_root, is_chrome_element, ContentRoot};

// ---------------------------------------------------------------------------
// Cached CSS selectors (parsed once via OnceLock)
// ---------------------------------------------------------------------------

fn sel_body() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("body").unwrap())
}
fn sel_title() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("title").unwrap())
}
fn sel_meta_description() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("meta[name=description]").unwrap())
}
fn sel_code() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("code").unwrap())
}
fn sel_caption() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("caption").unwrap())
}
fn sel_thead() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("thead").unwrap())
}
fn sel_tr() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("tr").unwrap())
}
fn sel_th() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("th").unwrap())
}
fn sel_tbody() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("tbody").unwrap())
}
fn sel_td() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("td").unwrap())
}
fn sel_img() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("img").unwrap())
}
fn sel_figcaption() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("figcaption").unwrap())
}
fn sel_source() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("source").unwrap())
}
fn sel_p() -> &'static Selector {
    static SEL: OnceLock<Selector> = OnceLock::new();
    SEL.get_or_init(|| Selector::parse("p").unwrap())
}

// ---------------------------------------------------------------------------
// AIF class detection helpers
// ---------------------------------------------------------------------------

fn detect_semantic_type(classes: &[&str]) -> Option<SemanticBlockType> {
    for class in classes {
        match *class {
            "aif-claim" => return Some(SemanticBlockType::Claim),
            "aif-evidence" => return Some(SemanticBlockType::Evidence),
            "aif-definition" => return Some(SemanticBlockType::Definition),
            "aif-theorem" => return Some(SemanticBlockType::Theorem),
            "aif-assumption" => return Some(SemanticBlockType::Assumption),
            "aif-result" => return Some(SemanticBlockType::Result),
            "aif-conclusion" => return Some(SemanticBlockType::Conclusion),
            "aif-requirement" => return Some(SemanticBlockType::Requirement),
            "aif-recommendation" => return Some(SemanticBlockType::Recommendation),
            _ => {}
        }
    }
    None
}

fn detect_skill_type(classes: &[&str]) -> Option<SkillBlockType> {
    for class in classes {
        match *class {
            "aif-skill" => return Some(SkillBlockType::Skill),
            "aif-step" => return Some(SkillBlockType::Step),
            "aif-verify" => return Some(SkillBlockType::Verify),
            "aif-precondition" => return Some(SkillBlockType::Precondition),
            "aif-output-contract" => return Some(SkillBlockType::OutputContract),
            "aif-decision" => return Some(SkillBlockType::Decision),
            "aif-tool" => return Some(SkillBlockType::Tool),
            "aif-fallback" => return Some(SkillBlockType::Fallback),
            "aif-red-flag" => return Some(SkillBlockType::RedFlag),
            "aif-example" => return Some(SkillBlockType::Example),
            "aif-scenario" => return Some(SkillBlockType::Scenario),
            _ => {}
        }
    }
    None
}

fn detect_callout_type(classes: &[&str]) -> Option<CalloutType> {
    for class in classes {
        match *class {
            "aif-note" => return Some(CalloutType::Note),
            "aif-warning" => return Some(CalloutType::Warning),
            "aif-info" => return Some(CalloutType::Info),
            "aif-tip" => return Some(CalloutType::Tip),
            _ => {}
        }
    }
    None
}

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
/// When `strip_chrome` is true, applies readability extraction:
/// 1. If `<article>` exists, use it as content root
/// 2. Else if `<main>` exists, use it as content root
/// 3. Else if `[role="main"]` exists, use it as content root
/// 4. Else use `<body>` but strip nav, header, footer, aside (non-aif-callout)
pub fn import_html(input: &str, strip_chrome: bool) -> HtmlImportResult {
    let html = Html::parse_document(input);
    let mut doc = Document::new();

    // Auto-detect AIF roundtrip mode by checking for aif-* CSS classes on elements
    let mode = if input.contains("class=\"aif-") || input.contains("class='aif-") {
        ImportMode::AifRoundtrip
    } else {
        ImportMode::Generic
    };

    let aif_mode = mode == ImportMode::AifRoundtrip;

    // Extract metadata from <head>
    extract_metadata(&html, &mut doc);

    let blocks = if strip_chrome {
        match find_content_root(&html) {
            ContentRoot::Element(el) => parse_children(el, aif_mode),
            ContentRoot::BodyFiltered => {
                if let Some(body) = html.select(sel_body()).next() {
                    parse_children_filtered(body, aif_mode)
                } else {
                    vec![]
                }
            }
            ContentRoot::None => {
                let root_el = html.root_element();
                parse_children(root_el, aif_mode)
            }
        }
    } else {
        // Original behavior — find <body> or fall back to document root
        let root = html.select(sel_body()).next();

        if let Some(body) = root {
            parse_children(body, aif_mode)
        } else {
            let root_el = html.root_element();
            parse_children(root_el, aif_mode)
        }
    };

    doc.blocks = blocks;

    // Provenance: record import source metadata
    doc.metadata.insert(
        "_aif_source_format".into(),
        "html".into(),
    );
    doc.metadata.insert(
        "_aif_import_mode".into(),
        match mode {
            ImportMode::AifRoundtrip => "aif-roundtrip",
            ImportMode::Generic => if strip_chrome { "readability" } else { "generic" },
        }
        .into(),
    );

    HtmlImportResult {
        document: doc,
        mode,
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
    if let Some(el) = html.select(sel_title()).next() {
        let text = el.text().collect::<String>();
        let text = text.trim().to_string();
        if !text.is_empty() {
            doc.metadata.insert("title".into(), text);
        }
    }

    // <meta name="description" content="...">
    if let Some(el) = html.select(sel_meta_description()).next() {
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
fn parse_children(parent: ElementRef, aif_mode: bool) -> Vec<Block> {
    let raw = collect_raw_elements(parent, aif_mode);
    group_headings_into_sections(raw)
}

/// Parse the direct children of an element into blocks, skipping chrome elements.
fn parse_children_filtered(parent: ElementRef, aif_mode: bool) -> Vec<Block> {
    let raw = collect_raw_elements_filtered(parent, aif_mode);
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
fn collect_raw_elements(parent: ElementRef, aif_mode: bool) -> Vec<RawBlock> {
    let mut raw = Vec::new();

    for child in parent.children() {
        if let Some(el) = ElementRef::wrap(child) {
            let tag = el.value().name();
            match tag {
                "p" => {
                    let content = parse_inlines(el, aif_mode);
                    if !content.is_empty() {
                        raw.push(RawBlock::Parsed(Block {
                            kind: BlockKind::Paragraph { content },
                            span: span(),
                        }));
                    }
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let level = tag[1..].parse::<u8>().unwrap_or(2);
                    let title = parse_inlines(el, aif_mode);
                    let id = el.value().attr("id").map(String::from);
                    raw.push(RawBlock::Heading { level, title, id });
                }
                "section" => {
                    let block = parse_section(el, aif_mode);
                    raw.push(RawBlock::Parsed(block));
                }
                "pre" => {
                    let block = parse_pre(el);
                    raw.push(RawBlock::Parsed(block));
                }
                "blockquote" => {
                    let content = parse_children(el, aif_mode);
                    raw.push(RawBlock::Parsed(Block {
                        kind: BlockKind::BlockQuote { content },
                        span: span(),
                    }));
                }
                "ul" => {
                    let items = parse_list_items(el, aif_mode);
                    raw.push(RawBlock::Parsed(Block {
                        kind: BlockKind::List {
                            ordered: false,
                            items,
                        },
                        span: span(),
                    }));
                }
                "ol" => {
                    let items = parse_list_items(el, aif_mode);
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
                    let block = parse_table(el, aif_mode);
                    raw.push(RawBlock::Parsed(block));
                }
                "figure" => {
                    let block = parse_figure(el, aif_mode);
                    raw.push(RawBlock::Parsed(block));
                }
                "audio" => {
                    let block = parse_audio(el, aif_mode);
                    raw.push(RawBlock::Parsed(block));
                }
                "video" => {
                    let block = parse_video(el, aif_mode);
                    raw.push(RawBlock::Parsed(block));
                }
                "div" | "main" | "article" | "header" | "footer" | "nav" => {
                    if aif_mode && tag == "div" {
                        let classes: Vec<&str> = el.value().classes().collect();
                        if let Some(block) = try_parse_aif_div(el, &classes, aif_mode) {
                            raw.push(RawBlock::Parsed(block));
                            continue;
                        }
                    }
                    // Transparent containers — recurse into children
                    let children = collect_raw_elements(el, aif_mode);
                    raw.extend(children);
                }
                "aside" => {
                    if aif_mode {
                        let classes: Vec<&str> = el.value().classes().collect();
                        if classes.contains(&"aif-callout") {
                            let block = parse_aif_callout(el, &classes, aif_mode);
                            raw.push(RawBlock::Parsed(block));
                            continue;
                        }
                    }
                    // Generic aside — treat as transparent container
                    let children = collect_raw_elements(el, aif_mode);
                    raw.extend(children);
                }
                _ => {
                    // Unknown block element: try to extract inline content as paragraph
                    let content = parse_inlines(el, aif_mode);
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

/// First pass with chrome filtering: skip nav, header, footer, and non-aif aside at top level.
fn collect_raw_elements_filtered(parent: ElementRef, aif_mode: bool) -> Vec<RawBlock> {
    let mut raw = Vec::new();

    for child in parent.children() {
        if let Some(el) = ElementRef::wrap(child) {
            if is_chrome_element(&el) {
                continue;
            }
            // Delegate to the same per-element logic used by collect_raw_elements
            let tag = el.value().name();
            match tag {
                "p" => {
                    let content = parse_inlines(el, aif_mode);
                    if !content.is_empty() {
                        raw.push(RawBlock::Parsed(Block {
                            kind: BlockKind::Paragraph { content },
                            span: span(),
                        }));
                    }
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let level = tag[1..].parse::<u8>().unwrap_or(2);
                    let title = parse_inlines(el, aif_mode);
                    let id = el.value().attr("id").map(String::from);
                    raw.push(RawBlock::Heading { level, title, id });
                }
                _ => {
                    // For all other tags, delegate to collect_raw_elements logic
                    // by re-parsing this single element as if it were a child
                    let sub = collect_raw_elements_single(el, aif_mode);
                    raw.extend(sub);
                }
            }
        }
    }

    raw
}

/// Parse a single element into RawBlock entries (used by filtered path).
fn collect_raw_elements_single(el: ElementRef, aif_mode: bool) -> Vec<RawBlock> {
    let tag = el.value().name();
    match tag {
        "p" => {
            let content = parse_inlines(el, aif_mode);
            if content.is_empty() {
                vec![]
            } else {
                vec![RawBlock::Parsed(Block {
                    kind: BlockKind::Paragraph { content },
                    span: span(),
                })]
            }
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level = tag[1..].parse::<u8>().unwrap_or(2);
            let title = parse_inlines(el, aif_mode);
            let id = el.value().attr("id").map(String::from);
            vec![RawBlock::Heading { level, title, id }]
        }
        "section" => vec![RawBlock::Parsed(parse_section(el, aif_mode))],
        "pre" => vec![RawBlock::Parsed(parse_pre(el))],
        "blockquote" => {
            let content = parse_children(el, aif_mode);
            vec![RawBlock::Parsed(Block {
                kind: BlockKind::BlockQuote { content },
                span: span(),
            })]
        }
        "ul" => {
            let items = parse_list_items(el, aif_mode);
            vec![RawBlock::Parsed(Block {
                kind: BlockKind::List {
                    ordered: false,
                    items,
                },
                span: span(),
            })]
        }
        "ol" => {
            let items = parse_list_items(el, aif_mode);
            vec![RawBlock::Parsed(Block {
                kind: BlockKind::List {
                    ordered: true,
                    items,
                },
                span: span(),
            })]
        }
        "hr" => vec![RawBlock::Parsed(Block {
            kind: BlockKind::ThematicBreak,
            span: span(),
        })],
        "table" => vec![RawBlock::Parsed(parse_table(el, aif_mode))],
        "figure" => vec![RawBlock::Parsed(parse_figure(el, aif_mode))],
        "audio" => vec![RawBlock::Parsed(parse_audio(el, aif_mode))],
        "video" => vec![RawBlock::Parsed(parse_video(el, aif_mode))],
        "div" | "main" | "article" | "header" | "footer" | "nav" => {
            if aif_mode && tag == "div" {
                let classes: Vec<&str> = el.value().classes().collect();
                if let Some(block) = try_parse_aif_div(el, &classes, aif_mode) {
                    return vec![RawBlock::Parsed(block)];
                }
            }
            collect_raw_elements(el, aif_mode)
        }
        "aside" => {
            if aif_mode {
                let classes: Vec<&str> = el.value().classes().collect();
                if classes.contains(&"aif-callout") {
                    return vec![RawBlock::Parsed(parse_aif_callout(el, &classes, aif_mode))];
                }
            }
            collect_raw_elements(el, aif_mode)
        }
        _ => {
            let content = parse_inlines(el, aif_mode);
            if content.is_empty() {
                vec![]
            } else {
                vec![RawBlock::Parsed(Block {
                    kind: BlockKind::Paragraph { content },
                    span: span(),
                })]
            }
        }
    }
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

                let sub: Vec<&RawBlock> = raw[i + 1..j].to_vec();
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

fn parse_section(el: ElementRef, aif_mode: bool) -> Block {
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
                title = parse_inlines(child_el, aif_mode);
                found_heading = true;
                continue;
            }

            // Parse remaining children as blocks
            let child_blocks = parse_single_element(child_el, aif_mode);
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
fn parse_single_element(el: ElementRef, aif_mode: bool) -> Vec<Block> {
    let tag = el.value().name();
    match tag {
        "p" => {
            let content = parse_inlines(el, aif_mode);
            if content.is_empty() {
                vec![]
            } else {
                vec![Block {
                    kind: BlockKind::Paragraph { content },
                    span: span(),
                }]
            }
        }
        "section" => vec![parse_section(el, aif_mode)],
        "pre" => vec![parse_pre(el)],
        "blockquote" => {
            let content = parse_children(el, aif_mode);
            vec![Block {
                kind: BlockKind::BlockQuote { content },
                span: span(),
            }]
        }
        "ul" => {
            let items = parse_list_items(el, aif_mode);
            vec![Block {
                kind: BlockKind::List {
                    ordered: false,
                    items,
                },
                span: span(),
            }]
        }
        "ol" => {
            let items = parse_list_items(el, aif_mode);
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
        "table" => vec![parse_table(el, aif_mode)],
        "figure" => vec![parse_figure(el, aif_mode)],
        "audio" => vec![parse_audio(el, aif_mode)],
        "video" => vec![parse_video(el, aif_mode)],
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            // Bare heading inside section — create nested section
            let title = parse_inlines(el, aif_mode);
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
        "div" | "main" | "article" | "header" | "footer" | "nav" => {
            if aif_mode && tag == "div" {
                let classes: Vec<&str> = el.value().classes().collect();
                if let Some(block) = try_parse_aif_div(el, &classes, aif_mode) {
                    return vec![block];
                }
            }
            parse_children(el, aif_mode)
        }
        "aside" => {
            if aif_mode {
                let classes: Vec<&str> = el.value().classes().collect();
                if classes.contains(&"aif-callout") {
                    return vec![parse_aif_callout(el, &classes, aif_mode)];
                }
            }
            parse_children(el, aif_mode)
        }
        _ => {
            let content = parse_inlines(el, aif_mode);
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
    if let Some(code_el) = el.select(sel_code()).next() {
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

fn parse_list_items(list_el: ElementRef, aif_mode: bool) -> Vec<ListItem> {
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
                        let sub_items = parse_list_items(child_el, aif_mode);
                        children.push(Block {
                            kind: BlockKind::List {
                                ordered: false,
                                items: sub_items,
                            },
                            span: span(),
                        });
                    }
                    "ol" => {
                        let sub_items = parse_list_items(child_el, aif_mode);
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
                        let inlines = parse_inline_node(child, aif_mode);
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

fn parse_table(el: ElementRef, aif_mode: bool) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut caption = None;
    let mut headers: Vec<Vec<Inline>> = Vec::new();
    let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();

    // Caption
    if let Some(cap_el) = el.select(sel_caption()).next() {
        let inlines = parse_inlines(cap_el, aif_mode);
        if !inlines.is_empty() {
            caption = Some(inlines);
        }
    }

    // Headers from <thead>
    if let Some(thead) = el.select(sel_thead()).next() {
        if let Some(tr) = thead.select(sel_tr()).next() {
            for th in tr.select(sel_th()) {
                headers.push(parse_inlines(th, aif_mode));
            }
        }
    }

    // Rows from <tbody> or directly
    if let Some(tbody) = el.select(sel_tbody()).next() {
        for tr in tbody.select(sel_tr()) {
            let mut row = Vec::new();
            for td in tr.select(sel_td()) {
                row.push(parse_inlines(td, aif_mode));
            }
            rows.push(row);
        }
    } else {
        // No <tbody>, look for direct <tr> children
        for tr in el.select(sel_tr()) {
            // Skip if this is in thead
            if tr.parent().and_then(|p| {
                ElementRef::wrap(p).map(|e| e.value().name() == "thead")
            }).unwrap_or(false) {
                continue;
            }
            let mut row = Vec::new();
            for td in tr.select(sel_td()) {
                row.push(parse_inlines(td, aif_mode));
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

fn parse_figure(el: ElementRef, aif_mode: bool) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut src = String::new();
    let mut meta = MediaMeta::default();
    let mut caption = None;

    if let Some(img) = el.select(sel_img()).next() {
        src = img.value().attr("src").unwrap_or("").to_string();
        meta.alt = img.value().attr("alt").map(String::from);
        meta.width = img.value().attr("width").and_then(|w| w.parse().ok());
        meta.height = img.value().attr("height").and_then(|h| h.parse().ok());
    }

    if let Some(cap_el) = el.select(sel_figcaption()).next() {
        let inlines = parse_inlines(cap_el, aif_mode);
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

fn parse_audio(el: ElementRef, aif_mode: bool) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut src = el.value().attr("src").unwrap_or("").to_string();
    let mut meta = MediaMeta::default();
    let mut caption = None;

    // Check for <source> child
    if let Some(source_el) = el.select(sel_source()).next() {
        if src.is_empty() {
            src = source_el.value().attr("src").unwrap_or("").to_string();
        }
        meta.mime = source_el.value().attr("type").map(String::from);
    }

    // Caption from <p> child
    if let Some(p_el) = el.select(sel_p()).next() {
        let inlines = parse_inlines(p_el, aif_mode);
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

fn parse_video(el: ElementRef, aif_mode: bool) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let src = el.value().attr("src").unwrap_or("").to_string();
    let meta = MediaMeta {
        width: el.value().attr("width").and_then(|w| w.parse().ok()),
        height: el.value().attr("height").and_then(|h| h.parse().ok()),
        poster: el.value().attr("poster").map(String::from),
        ..MediaMeta::default()
    };
    let mut caption = None;

    // Caption from <p> child
    if let Some(p_el) = el.select(sel_p()).next() {
        let inlines = parse_inlines(p_el, aif_mode);
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
pub fn parse_inlines(parent: ElementRef, aif_mode: bool) -> Vec<Inline> {
    let mut inlines = Vec::new();

    for child in parent.children() {
        let node_inlines = parse_inline_node(child, aif_mode);
        inlines.extend(node_inlines);
    }

    inlines
}

/// Parse a single DOM node into inline elements.
fn parse_inline_node(node: NodeRef<'_, Node>, aif_mode: bool) -> Vec<Inline> {
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
                let content = parse_inlines(el, aif_mode);
                vec![Inline::Strong { content }]
            }
            "em" | "i" => {
                let content = parse_inlines(el, aif_mode);
                vec![Inline::Emphasis { content }]
            }
            "code" => {
                let code = el.text().collect::<String>();
                vec![Inline::InlineCode { code }]
            }
            "a" => {
                if aif_mode {
                    let classes: Vec<&str> = el.value().classes().collect();
                    if classes.contains(&"aif-ref") {
                        let href = el.value().attr("href").unwrap_or("");
                        let target = href.strip_prefix('#').unwrap_or(href).to_string();
                        return vec![Inline::Reference { target }];
                    }
                }
                let url = el.value().attr("href").unwrap_or("").to_string();
                let text = parse_inlines(el, aif_mode);
                vec![Inline::Link { text, url }]
            }
            "sup" => {
                if aif_mode {
                    let classes: Vec<&str> = el.value().classes().collect();
                    if classes.contains(&"aif-footnote") {
                        let content = parse_inlines(el, aif_mode);
                        return vec![Inline::Footnote { content }];
                    }
                }
                // Unknown inline element — recurse into children
                parse_inlines(el, aif_mode)
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
                parse_inlines(el, aif_mode)
            }
        }
    } else {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// AIF block parsers
// ---------------------------------------------------------------------------

/// Try to parse a <div> with AIF classes into a SemanticBlock or SkillBlock.
/// Returns None if no AIF class is detected.
fn try_parse_aif_div(el: ElementRef, classes: &[&str], aif_mode: bool) -> Option<Block> {
    // Check semantic types first
    if let Some(block_type) = detect_semantic_type(classes) {
        return Some(parse_aif_semantic_block(el, block_type, aif_mode));
    }

    // Then check skill types
    if let Some(skill_type) = detect_skill_type(classes) {
        return Some(parse_aif_skill_block(el, skill_type, aif_mode));
    }

    None
}

/// Parse a <div class="aif-{type}"> into a SemanticBlock.
fn parse_aif_semantic_block(el: ElementRef, block_type: SemanticBlockType, aif_mode: bool) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut title = None;
    let mut content = Vec::new();

    for child in el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            let tag = child_el.value().name();
            match tag {
                "strong" if title.is_none() => {
                    title = Some(parse_inlines(child_el, aif_mode));
                }
                "p" => {
                    content = parse_inlines(child_el, aif_mode);
                }
                _ => {}
            }
        }
    }

    Block {
        kind: BlockKind::SemanticBlock {
            block_type,
            attrs,
            title,
            content,
        },
        span: span(),
    }
}

/// Parse an <aside class="aif-callout aif-{type}"> into a Callout.
fn parse_aif_callout(el: ElementRef, classes: &[&str], aif_mode: bool) -> Block {
    let callout_type = detect_callout_type(classes).unwrap_or(CalloutType::Note);
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    let mut content = Vec::new();

    for child in el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            if child_el.value().name() == "p" {
                content = parse_inlines(child_el, aif_mode);
                break;
            }
        }
    }

    Block {
        kind: BlockKind::Callout {
            callout_type,
            attrs,
            content,
        },
        span: span(),
    }
}

/// Parse a <div class="aif-{skill-type}"> into a SkillBlock.
fn parse_aif_skill_block(el: ElementRef, skill_type: SkillBlockType, aif_mode: bool) -> Block {
    let mut attrs = Attrs::new();
    attrs.id = el.value().attr("id").map(String::from);

    // Copy data-aif-* attributes to attrs.pairs
    for (name, value) in el.value().attrs() {
        if let Some(key) = name.strip_prefix("data-aif-") {
            attrs.pairs.insert(key.to_string(), value.to_string());
        }
    }

    let mut title = None;
    let mut content = Vec::new();
    let mut children = Vec::new();

    for child in el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            let tag = child_el.value().name();
            match tag {
                "h3" if title.is_none() => {
                    title = Some(parse_inlines(child_el, aif_mode));
                }
                "p" if content.is_empty() => {
                    content = parse_inlines(child_el, aif_mode);
                }
                "div" => {
                    let child_classes: Vec<&str> = child_el.value().classes().collect();
                    if let Some(block) = try_parse_aif_div(child_el, &child_classes, aif_mode) {
                        children.push(block);
                    } else {
                        // Generic div — recurse
                        let sub_blocks = parse_children(child_el, aif_mode);
                        children.extend(sub_blocks);
                    }
                }
                "aside" => {
                    let child_classes: Vec<&str> = child_el.value().classes().collect();
                    if child_classes.contains(&"aif-callout") {
                        children.push(parse_aif_callout(child_el, &child_classes, aif_mode));
                    } else {
                        let sub_blocks = parse_children(child_el, aif_mode);
                        children.extend(sub_blocks);
                    }
                }
                _ => {
                    // Other block elements inside skill block
                    let sub_blocks = parse_single_element(child_el, aif_mode);
                    children.extend(sub_blocks);
                }
            }
        }
    }

    Block {
        kind: BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        },
        span: span(),
    }
}
