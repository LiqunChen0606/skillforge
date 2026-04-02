use aif_core::ast::*;
use aif_core::span::Span;
use aif_lml::{parse_lml, render_lml, render_lml_aggressive, render_lml_compressed, render_lml_hybrid};
use std::collections::BTreeMap;

fn text(s: &str) -> Inline {
    Inline::Text { text: s.to_string() }
}

fn block(kind: BlockKind) -> Block {
    Block { kind, span: Span::empty() }
}

fn sample_table() -> Block {
    block(BlockKind::Table {
        attrs: {
            let mut a = Attrs::new();
            a.id = Some("t1".to_string());
            a
        },
        caption: Some(vec![text("Sales Data")]),
        headers: vec![vec![text("Product")], vec![text("Revenue")]],
        rows: vec![
            vec![vec![text("Widget A")], vec![text("$100")]],
            vec![vec![text("Widget B")], vec![text("$200")]],
        ],
    })
}

fn table_no_caption() -> Block {
    block(BlockKind::Table {
        attrs: Attrs::new(),
        caption: None,
        headers: vec![vec![text("Name")], vec![text("Age")]],
        rows: vec![
            vec![vec![text("Alice")], vec![text("30")]],
        ],
    })
}

// ── Gap 1: Emitter tests ───────────────────────────────────────────

#[test]
fn emit_table_standard_mode() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![sample_table()],
    };
    let lml = render_lml(&doc);
    assert!(lml.contains("[TABLE id=t1] Sales Data"), "missing table header, got:\n{}", lml);
    assert!(lml.contains("| Product | Revenue |"), "missing header row, got:\n{}", lml);
    assert!(lml.contains("| --- | --- |"), "missing separator, got:\n{}", lml);
    assert!(lml.contains("| Widget A | $100 |"), "missing data row 1, got:\n{}", lml);
    assert!(lml.contains("| Widget B | $200 |"), "missing data row 2, got:\n{}", lml);
    assert!(lml.contains("[/TABLE]"), "missing closing tag, got:\n{}", lml);
}

#[test]
fn emit_table_aggressive_mode() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![sample_table()],
    };
    let lml = render_lml_aggressive(&doc);
    assert!(lml.contains("@table: Sales Data"), "missing table header, got:\n{}", lml);
    assert!(lml.contains("| Product | Revenue |"), "missing header row, got:\n{}", lml);
    assert!(lml.contains("| --- | --- |"), "missing separator, got:\n{}", lml);
    assert!(lml.contains("| Widget A | $100 |"), "missing data row 1, got:\n{}", lml);
    assert!(lml.contains("| Widget B | $200 |"), "missing data row 2, got:\n{}", lml);
}

#[test]
fn emit_table_no_caption() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![table_no_caption()],
    };
    let lml = render_lml_aggressive(&doc);
    assert!(lml.contains("@table:"), "missing @table: line, got:\n{}", lml);
    assert!(lml.contains("| Name | Age |"), "missing header row, got:\n{}", lml);
    assert!(lml.contains("| Alice | 30 |"), "missing data row, got:\n{}", lml);
}

// ── Gap 2: Parser roundtrip tests ──────────────────────────────────

#[test]
fn parse_table_aggressive() {
    let input = "@table: Sales Data\n| Product | Revenue |\n| --- | --- |\n| Widget A | $100 |\n| Widget B | $200 |\n\nNext paragraph\n";
    let doc = parse_lml(input).unwrap();
    assert_eq!(doc.blocks.len(), 2, "expected table + paragraph, got: {:?}", doc.blocks);
    match &doc.blocks[0].kind {
        BlockKind::Table { caption, headers, rows, .. } => {
            let cap = caption.as_ref().expect("expected caption");
            assert_eq!(cap, &vec![text("Sales Data")]);
            assert_eq!(headers.len(), 2);
            assert_eq!(headers[0], vec![text("Product")]);
            assert_eq!(headers[1], vec![text("Revenue")]);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][0], vec![text("Widget A")]);
            assert_eq!(rows[0][1], vec![text("$100")]);
            assert_eq!(rows[1][0], vec![text("Widget B")]);
            assert_eq!(rows[1][1], vec![text("$200")]);
        }
        other => panic!("Expected Table, got {:?}", other),
    }
}

#[test]
fn parse_table_no_caption() {
    let input = "@table:\n| Name | Age |\n| --- | --- |\n| Alice | 30 |\n\n";
    let doc = parse_lml(input).unwrap();
    assert_eq!(doc.blocks.len(), 1);
    match &doc.blocks[0].kind {
        BlockKind::Table { caption, headers, rows, .. } => {
            assert!(caption.is_none());
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
        }
        other => panic!("Expected Table, got {:?}", other),
    }
}

#[test]
fn table_roundtrip_aggressive() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![sample_table()],
    };
    let lml = render_lml_aggressive(&doc);
    let parsed = parse_lml(&lml).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0].kind {
        BlockKind::Table { caption, headers, rows, .. } => {
            let cap = caption.as_ref().expect("expected caption");
            assert_eq!(cap, &vec![text("Sales Data")]);
            assert_eq!(headers.len(), 2);
            assert_eq!(headers[0], vec![text("Product")]);
            assert_eq!(headers[1], vec![text("Revenue")]);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][0], vec![text("Widget A")]);
            assert_eq!(rows[1][1], vec![text("$200")]);
        }
        other => panic!("Expected Table, got {:?}", other),
    }
}

// ── Gap 3: Compression tests ──────────────────────────────────────

#[test]
fn compression_includes_table_cell_content() {
    let repeated_text = "This is a long repeated phrase that appears in table cells and elsewhere";
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![
            block(BlockKind::Table {
                attrs: Attrs::new(),
                caption: None,
                headers: vec![vec![text(repeated_text)]],
                rows: vec![
                    vec![vec![text(repeated_text)]],
                ],
            }),
            block(BlockKind::Paragraph {
                content: vec![text(repeated_text)],
            }),
        ],
    };
    let output = render_lml_compressed(&doc);
    // The repeated text should be deduplicated (appears 3x: header, row cell, paragraph)
    assert!(output.contains("~dict:"), "expected dictionary for repeated table cell text, got:\n{}", output);
    assert!(output.contains("~ref:"), "expected dictionary references, got:\n{}", output);
}

#[test]
fn compression_table_emits_rows() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::Table {
            attrs: Attrs::new(),
            caption: Some(vec![text("Test Table")]),
            headers: vec![vec![text("Col A")], vec![text("Col B")]],
            rows: vec![vec![vec![text("val1")], vec![text("val2")]]],
        })],
    };
    let output = render_lml_compressed(&doc);
    assert!(output.contains("| Col A | Col B |"), "missing header row in compressed output, got:\n{}", output);
    assert!(output.contains("| val1 | val2 |"), "missing data row in compressed output, got:\n{}", output);
}

// ── Gap 4: Hybrid tests ──────────────────────────────────────────

#[test]
fn hybrid_table_emits_rows() {
    let doc = Document {
        metadata: BTreeMap::new(),
        blocks: vec![block(BlockKind::Table {
            attrs: Attrs::new(),
            caption: Some(vec![text("Hybrid Table")]),
            headers: vec![vec![text("X")], vec![text("Y")]],
            rows: vec![vec![vec![text("1")], vec![text("2")]]],
        })],
    };
    let output = render_lml_hybrid(&doc);
    assert!(output.contains("| X | Y |"), "missing header row in hybrid output, got:\n{}", output);
    assert!(output.contains("| --- | --- |"), "missing separator in hybrid output, got:\n{}", output);
    assert!(output.contains("| 1 | 2 |"), "missing data row in hybrid output, got:\n{}", output);
}
