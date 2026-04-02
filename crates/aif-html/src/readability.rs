//! Readability extraction — simple content root detection and chrome stripping.
//!
//! Strategy (in priority order):
//! 1. `<article>` tag → use as content root
//! 2. `<main>` tag → use as content root
//! 3. `[role="main"]` → use as content root
//! 4. `<body>` fallback → strip chrome tags (nav, header, footer, aside without aif-callout)

use scraper::{ElementRef, Html, Selector};

/// Tags considered page chrome and stripped when using the body fallback.
const CHROME_TAGS: &[&str] = &["nav", "header", "footer"];

/// Result of content root extraction.
pub enum ContentRoot<'a> {
    /// A single element to use as the parsing root (article, main, or role=main).
    Element(ElementRef<'a>),
    /// No semantic root found — use body but filter chrome children.
    BodyFiltered,
    /// No body found at all.
    None,
}

/// Find the best content root in the parsed HTML document.
pub fn find_content_root(html: &Html) -> ContentRoot<'_> {
    // 1. <article>
    let article_sel = Selector::parse("article").unwrap();
    if let Some(el) = html.select(&article_sel).next() {
        return ContentRoot::Element(el);
    }

    // 2. <main>
    let main_sel = Selector::parse("main").unwrap();
    if let Some(el) = html.select(&main_sel).next() {
        return ContentRoot::Element(el);
    }

    // 3. [role="main"]
    let role_sel = Selector::parse("[role=\"main\"]").unwrap();
    if let Some(el) = html.select(&role_sel).next() {
        return ContentRoot::Element(el);
    }

    // 4. <body> fallback
    let body_sel = Selector::parse("body").unwrap();
    if html.select(&body_sel).next().is_some() {
        return ContentRoot::BodyFiltered;
    }

    ContentRoot::None
}

/// Check whether an element is a chrome tag that should be stripped.
pub fn is_chrome_element(el: &ElementRef) -> bool {
    let tag = el.value().name();

    // nav, header, footer are always chrome
    if CHROME_TAGS.contains(&tag) {
        return true;
    }

    // aside is chrome UNLESS it has aif-callout class
    if tag == "aside" {
        let has_aif_callout = el.value().classes().any(|c| c == "aif-callout");
        return !has_aif_callout;
    }

    false
}
