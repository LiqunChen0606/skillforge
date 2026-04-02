use aif_core::ast::{Attrs, Block, BlockKind, Document, Inline, ListItem};
use aif_core::span::Span;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Accumulates table rows during parsing.
/// The first row seen inside TableHead becomes headers; subsequent rows become data rows.
#[derive(Debug, Default)]
struct TableAccumulator {
    headers: Vec<Vec<Inline>>,
    rows: Vec<Vec<Vec<Inline>>>,
    current_row: Vec<Vec<Inline>>,
}

/// What kind of block is currently being built.
#[derive(Debug)]
enum BuilderKind {
    /// Top-level collector (the document root or a section body).
    Root,
    /// A heading (level 1–6).
    Heading(u8),
    /// A paragraph.
    Paragraph,
    /// A fenced/indented code block with optional language.
    CodeBlock(Option<String>),
    /// A block quote.
    BlockQuote,
    /// A list. `ordered` is true for ordered lists.
    List { ordered: bool },
    /// A single list item.
    ListItem,
    /// Emphasis inline wrapper.
    Emphasis,
    /// Strong inline wrapper.
    Strong,
    /// Link with destination URL.
    Link(String),
    /// A GFM table.
    Table,
    /// Table head section.
    TableHead,
    /// A table row (header or body).
    TableRow,
    /// A single table cell.
    TableCell,
}

/// Accumulator pushed onto the stack while processing pulldown-cmark events.
#[derive(Debug)]
struct BlockBuilder {
    kind: BuilderKind,
    inlines: Vec<Inline>,
    children: Vec<Block>,
    items: Vec<ListItem>,
    code: String,
    table: TableAccumulator,
}

impl BlockBuilder {
    fn new(kind: BuilderKind) -> Self {
        Self {
            kind,
            inlines: Vec::new(),
            children: Vec::new(),
            items: Vec::new(),
            code: String::new(),
            table: TableAccumulator::default(),
        }
    }
}

pub fn import(input: &str) -> Document {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(input, opts);

    let mut doc = Document::new();
    let mut stack: Vec<BlockBuilder> = vec![BlockBuilder::new(BuilderKind::Root)];

    for event in parser {
        match event {
            Event::Start(tag) => {
                let kind = match tag {
                    Tag::Heading { level, .. } => BuilderKind::Heading(heading_level_to_u8(level)),
                    Tag::Paragraph => BuilderKind::Paragraph,
                    Tag::CodeBlock(cb_kind) => {
                        let lang = match cb_kind {
                            CodeBlockKind::Fenced(info) => {
                                let s = info.to_string();
                                if s.is_empty() { None } else { Some(s) }
                            }
                            CodeBlockKind::Indented => None,
                        };
                        BuilderKind::CodeBlock(lang)
                    }
                    Tag::BlockQuote(_) => BuilderKind::BlockQuote,
                    Tag::List(first_item) => BuilderKind::List {
                        ordered: first_item.is_some(),
                    },
                    Tag::Item => BuilderKind::ListItem,
                    Tag::Emphasis => BuilderKind::Emphasis,
                    Tag::Strong => BuilderKind::Strong,
                    Tag::Link { dest_url, .. } => BuilderKind::Link(dest_url.to_string()),
                    Tag::Image { dest_url, .. } => BuilderKind::Link(format!("!img:{}", dest_url)),
                    Tag::Table(_alignments) => BuilderKind::Table,
                    Tag::TableHead => BuilderKind::TableHead,
                    Tag::TableRow => BuilderKind::TableRow,
                    Tag::TableCell => BuilderKind::TableCell,
                    _ => BuilderKind::Root,
                };
                stack.push(BlockBuilder::new(kind));
            }

            Event::End(tag_end) => {
                let mut builder = stack.pop().expect("stack underflow");
                let parent = stack.last_mut().expect("stack underflow on parent");

                match tag_end {
                    TagEnd::Heading(_level) => {
                        let level = match builder.kind {
                            BuilderKind::Heading(l) => l,
                            _ => 1,
                        };
                        // Extract title text for metadata if this is H1
                        if level == 1 {
                            let title_text = inlines_to_plain_text(&builder.inlines);
                            if !title_text.is_empty() {
                                doc.metadata
                                    .entry("title".to_string())
                                    .or_insert(title_text);
                            }
                        }
                        let block = Block {
                            kind: BlockKind::Section {
                                attrs: Attrs::new(),
                                title: builder.inlines,
                                children: builder.children,
                            },
                            span: Span::empty(),
                        };
                        parent.children.push(block);
                    }

                    TagEnd::Paragraph => {
                        let block = Block {
                            kind: BlockKind::Paragraph {
                                content: builder.inlines,
                            },
                            span: Span::empty(),
                        };
                        parent.children.push(block);
                    }

                    TagEnd::CodeBlock => {
                        let lang = match builder.kind {
                            BuilderKind::CodeBlock(l) => l,
                            _ => None,
                        };
                        let block = Block {
                            kind: BlockKind::CodeBlock {
                                lang,
                                attrs: Attrs::new(),
                                code: builder.code,
                            },
                            span: Span::empty(),
                        };
                        parent.children.push(block);
                    }

                    TagEnd::BlockQuote(_) => {
                        let block = Block {
                            kind: BlockKind::BlockQuote {
                                content: builder.children,
                            },
                            span: Span::empty(),
                        };
                        parent.children.push(block);
                    }

                    TagEnd::List(_) => {
                        let ordered = match builder.kind {
                            BuilderKind::List { ordered } => ordered,
                            _ => false,
                        };
                        let block = Block {
                            kind: BlockKind::List {
                                ordered,
                                items: builder.items,
                            },
                            span: Span::empty(),
                        };
                        parent.children.push(block);
                    }

                    TagEnd::Item => {
                        let item = ListItem {
                            content: builder.inlines,
                            children: builder.children,
                        };
                        parent.items.push(item);
                    }

                    TagEnd::Emphasis => {
                        let inline = Inline::Emphasis {
                            content: builder.inlines,
                        };
                        parent.inlines.push(inline);
                    }

                    TagEnd::Strong => {
                        let inline = Inline::Strong {
                            content: builder.inlines,
                        };
                        parent.inlines.push(inline);
                    }

                    TagEnd::Link => {
                        let url = match builder.kind {
                            BuilderKind::Link(u) => u,
                            _ => String::new(),
                        };
                        let inline = Inline::Link {
                            text: builder.inlines,
                            url,
                        };
                        parent.inlines.push(inline);
                    }

                    TagEnd::Image => {
                        let src = match builder.kind {
                            BuilderKind::Link(u) => u.strip_prefix("!img:").unwrap_or(&u).to_string(),
                            _ => String::new(),
                        };
                        let alt = inlines_to_plain_text(&builder.inlines);
                        parent.inlines.push(Inline::Image { alt, src });
                    }

                    TagEnd::TableCell => {
                        // Cell finished: push its inlines as a complete cell
                        // into the parent row builder's current_row accumulator.
                        parent.table.current_row.push(builder.inlines);
                    }

                    TagEnd::TableRow => {
                        // Row finished: take all accumulated cells and push as a complete row
                        // into the parent (TableHead or Table builder).
                        let row_cells = std::mem::take(&mut builder.table.current_row);
                        parent.table.rows.push(row_cells);
                    }

                    TagEnd::TableHead => {
                        // The header section is done. In pulldown_cmark 0.12, cells are
                        // directly inside TableHead (no TableRow wrapper), so header cells
                        // are accumulated in builder.table.current_row.
                        let headers = std::mem::take(&mut builder.table.current_row);
                        parent.table.headers = headers;
                    }

                    TagEnd::Table => {
                        let block = Block {
                            kind: BlockKind::Table {
                                attrs: Attrs::new(),
                                caption: None,
                                headers: builder.table.headers,
                                rows: builder.table.rows,
                            },
                            span: Span::empty(),
                        };
                        parent.children.push(block);
                    }

                    _ => {
                        // For any unhandled tag, merge children/inlines upward.
                        parent.inlines.extend(builder.inlines);
                        parent.children.extend(builder.children);
                    }
                }
            }

            Event::Text(text) => {
                let top = stack.last_mut().expect("stack empty");
                match &top.kind {
                    BuilderKind::CodeBlock(_) => {
                        top.code.push_str(&text);
                    }
                    _ => {
                        top.inlines.push(Inline::Text {
                            text: text.to_string(),
                        });
                    }
                }
            }

            Event::Code(code) => {
                let top = stack.last_mut().expect("stack empty");
                top.inlines.push(Inline::InlineCode {
                    code: code.to_string(),
                });
            }

            Event::SoftBreak => {
                let top = stack.last_mut().expect("stack empty");
                top.inlines.push(Inline::SoftBreak);
            }

            Event::HardBreak => {
                let top = stack.last_mut().expect("stack empty");
                top.inlines.push(Inline::HardBreak);
            }

            Event::Rule => {
                let top = stack.last_mut().expect("stack empty");
                top.children.push(Block {
                    kind: BlockKind::ThematicBreak,
                    span: Span::empty(),
                });
            }

            _ => {
                // Ignore other events (HTML, footnotes, etc.)
            }
        }
    }

    // Collect blocks from the root builder.
    let root = stack.pop().expect("root missing");
    doc.blocks = root.children;

    // Provenance: record import source metadata
    doc.metadata
        .insert("_aif_source_format".into(), "markdown".into());
    doc.metadata
        .insert("_aif_import_mode".into(), "generic".into());

    doc
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn inlines_to_plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::InlineCode { code } => out.push_str(code),
            Inline::Emphasis { content } => out.push_str(&inlines_to_plain_text(content)),
            Inline::Strong { content } => out.push_str(&inlines_to_plain_text(content)),
            Inline::Link { text, .. } => out.push_str(&inlines_to_plain_text(text)),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out
}
