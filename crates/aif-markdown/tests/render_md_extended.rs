use aif_markdown::render_markdown;
use aif_parser::parse;

#[test]
fn render_md_preserves_title() {
    let input = "#title: My Document\n\nA paragraph.\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.starts_with("# My Document"));
}

#[test]
fn render_md_sections_as_headings() {
    let input = "@section[id=s1]: First Section\nContent.\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("## First Section"));
}

#[test]
fn render_md_inline_formatting() {
    let input = "Text with **bold** and *italic* and `code`.\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("**bold**"));
    assert!(md.contains("*italic*"));
    assert!(md.contains("`code`"));
}

#[test]
fn render_md_link() {
    let input = "Visit [Example](https://example.com) today.\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("[Example](https://example.com)"));
}

#[test]
fn render_md_code_block() {
    let input = "```rust\nfn main() {}\n```\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("```rust"));
    assert!(md.contains("fn main() {}"));
    assert!(md.contains("```"));
}

#[test]
fn render_md_unordered_list() {
    let input = "- Alpha\n- Beta\n- Gamma\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("- Alpha"));
    assert!(md.contains("- Beta"));
    assert!(md.contains("- Gamma"));
}

#[test]
fn render_md_ordered_list() {
    let input = "1. First\n2. Second\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("1. First"));
    assert!(md.contains("2. Second"));
}

#[test]
fn render_md_semantic_block() {
    let input = "@claim[id=c1]\nThis is a claim.\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("**Claim:**"));
    assert!(md.contains("This is a claim."));
}

#[test]
fn render_md_callout() {
    let input = "@callout[type=warning]\nBe careful!\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("> **Warning:**"));
    assert!(md.contains("Be careful!"));
}

#[test]
fn render_md_table() {
    let input = "@table[id=t1]: Data\n| Name | Value |\n| A | 1 |\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("| Name |"));
    assert!(md.contains("| --- |"));
    assert!(md.contains("| A |"));
}

#[test]
fn render_md_figure() {
    let input = "@figure[id=fig1, src=photo.jpg]: A photo\n";
    let doc = parse(input).unwrap();
    let md = render_markdown(&doc);
    assert!(md.contains("![A photo](photo.jpg)"));
}

#[test]
fn render_md_wiki_article() {
    let input = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/documents/wiki_article.aif")
    ).unwrap();
    let doc = parse(&input).unwrap();
    let md = render_markdown(&doc);

    assert!(md.starts_with("# Photosynthesis"));
    assert!(md.contains("## Overview"));
    assert!(md.contains("**light energy**"));
    assert!(md.contains("*chemical energy*"));
    assert!(md.contains("`ATP`"));
    assert!(md.contains("**Definition:**"));
    assert!(md.contains("**Claim:**"));
    assert!(md.contains("> **Note:**"));
    assert!(md.contains("[RuBisCO](https://en.wikipedia.org/wiki/RuBisCO)"));
}
