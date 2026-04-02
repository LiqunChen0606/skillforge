use aif_html::render_html;
use aif_parser::parse;

#[test]
fn html_contains_doctype_and_structure() {
    let input = "#title: Test\n#summary: Summary\n\nHello.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<html lang=\"en\">"));
    assert!(html.contains("<title>Test</title>"));
    assert!(html.contains("<meta name=\"description\" content=\"Summary\">"));
    assert!(html.contains("</body>"));
    assert!(html.contains("</html>"));
}

#[test]
fn html_section_with_id() {
    let input = "@section[id=intro]: Introduction\nContent here.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<section id=\"intro\">"));
    assert!(html.contains("<h2>Introduction</h2>"));
    assert!(html.contains("</section>"));
}

#[test]
fn html_semantic_blocks() {
    let types = ["claim", "evidence", "definition", "theorem",
                 "assumption", "result", "conclusion", "requirement", "recommendation"];
    for t in &types {
        let input = format!("@{}[id=test]\nContent.\n", t);
        let doc = parse(&input).unwrap();
        let html = render_html(&doc);
        assert!(html.contains(&format!("class=\"aif-{}\"", t)),
            "Expected aif-{} class in HTML", t);
        assert!(html.contains("id=\"test\""));
    }
}

#[test]
fn html_callout_types() {
    for t in &["note", "warning", "info", "tip"] {
        let input = format!("@callout[type={}]\nCallout content.\n", t);
        let doc = parse(&input).unwrap();
        let html = render_html(&doc);
        assert!(html.contains("aif-callout"), "Expected aif-callout class");
        assert!(html.contains(&format!("aif-{}", t)), "Expected aif-{} class", t);
    }
}

#[test]
fn html_inline_formatting() {
    let input = "This is **bold** and *italic* and `code`.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<strong>bold</strong>"));
    assert!(html.contains("<em>italic</em>"));
    assert!(html.contains("<code>code</code>"));
}

#[test]
fn html_link() {
    let input = "Visit [Example](https://example.com) for more.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<a href=\"https://example.com\">Example</a>"));
}

#[test]
fn html_table_structure() {
    let input = "@table[id=t1]: Data\n| Name | Value |\n| A | 1 |\n| B | 2 |\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<table id=\"t1\">"));
    assert!(html.contains("<caption>Data</caption>"));
    assert!(html.contains("<th>Name</th>"));
    assert!(html.contains("<th>Value</th>"));
    assert!(html.contains("<td>A</td>"));
    assert!(html.contains("<td>1</td>"));
}

#[test]
fn html_figure() {
    let input = "@figure[id=fig1, src=image.png]: A caption\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<figure id=\"fig1\">"));
    assert!(html.contains("<img src=\"image.png\""));
    assert!(html.contains("<figcaption>A caption</figcaption>"));
}

#[test]
fn html_ordered_list() {
    let input = "1. First\n2. Second\n3. Third\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<ol>"));
    assert!(html.contains("<li>First</li>"));
    assert!(html.contains("</ol>"));
}

#[test]
fn html_unordered_list() {
    let input = "- Alpha\n- Beta\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>Alpha</li>"));
    assert!(html.contains("</ul>"));
}

#[test]
fn html_code_block_with_language() {
    let input = "```rust\nfn main() {}\n```\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("class=\"language-rust\""));
    assert!(html.contains("fn main() {}"));
}

#[test]
fn html_escapes_special_chars() {
    let input = "Use <div> tags & \"quotes\" in HTML.\n";
    let doc = parse(input).unwrap();
    let html = render_html(&doc);
    assert!(html.contains("&lt;div&gt;"));
    assert!(html.contains("&amp;"));
    assert!(html.contains("&quot;quotes&quot;"));
}

#[test]
fn html_wiki_article_fixture() {
    let input = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/documents/wiki_article.aif")
    ).unwrap();
    let doc = parse(&input).unwrap();
    let html = render_html(&doc);

    // Check document structure
    assert!(html.contains("<title>Photosynthesis</title>"));
    assert!(html.contains("<section id=\"overview\">"));
    assert!(html.contains("<section id=\"equation\">"));
    assert!(html.contains("<section id=\"stages\">"));
    assert!(html.contains("<section id=\"light-reactions\">"));
    assert!(html.contains("<section id=\"calvin-cycle\">"));
    assert!(html.contains("<section id=\"factors\">"));
    assert!(html.contains("<section id=\"importance\">"));

    // Semantic blocks
    assert!(html.contains("aif-definition"));
    assert!(html.contains("aif-claim"));
    assert!(html.contains("aif-evidence"));
    assert!(html.contains("aif-conclusion"));

    // Callouts
    assert!(html.contains("aif-callout aif-note"));
    assert!(html.contains("aif-callout aif-warning"));

    // Table
    assert!(html.contains("<table id=\"factors-table\">"));
    assert!(html.contains("<th>Factor</th>"));

    // Figure
    assert!(html.contains("<figure id=\"fig-leaf\">"));
    assert!(html.contains("leaf-cross-section.png"));

    // Inline formatting
    assert!(html.contains("<strong>light energy</strong>"));
    assert!(html.contains("<em>chemical energy</em>"));
    assert!(html.contains("<code>ATP</code>"));
}
