use aif_core::ast::*;

#[test]
fn test_strip_chrome_removes_nav() {
    let html = r#"<html><body>
        <nav><a href="/">Home</a><a href="/about">About</a></nav>
        <main>
            <article>
                <h1>Real Article</h1>
                <p>This is the actual content that matters.</p>
            </article>
        </main>
        <footer><p>Copyright 2025</p></footer>
    </body></html>"#;
    let result = aif_html::import_html(html, true);
    let json = serde_json::to_string(&result.document).unwrap();
    assert!(!json.contains("Home"));
    assert!(!json.contains("Copyright"));
    assert!(json.contains("Real Article"));
    assert!(json.contains("actual content"));
}

#[test]
fn test_strip_chrome_uses_article_tag() {
    let html = r#"<html><body>
        <div class="sidebar">Ads here</div>
        <article>
            <p>Main content paragraph one.</p>
            <p>Main content paragraph two.</p>
        </article>
        <div class="sidebar">More ads</div>
    </body></html>"#;
    let result = aif_html::import_html(html, true);
    assert!(result.document.blocks.len() >= 2);
    let json = serde_json::to_string(&result.document).unwrap();
    assert!(!json.contains("Ads here"));
    assert!(json.contains("Main content paragraph one"));
}

#[test]
fn test_strip_chrome_uses_main_tag() {
    let html = r#"<html><body>
        <header><h1>Site Title</h1></header>
        <main>
            <p>Important content.</p>
        </main>
        <footer><p>Footer stuff</p></footer>
    </body></html>"#;
    let result = aif_html::import_html(html, true);
    let json = serde_json::to_string(&result.document).unwrap();
    assert!(!json.contains("Footer stuff"));
    assert!(json.contains("Important content"));
}

#[test]
fn test_no_strip_chrome_keeps_everything() {
    let html = r#"<html><body>
        <nav><a href="/">Home</a></nav>
        <p>Content</p>
        <footer><p>Footer</p></footer>
    </body></html>"#;
    let result = aif_html::import_html(html, false);
    assert!(result.document.blocks.len() >= 2);
}
