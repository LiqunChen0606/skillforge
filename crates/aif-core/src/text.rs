use crate::ast::Inline;

/// Controls how inline formatting is rendered to text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextMode {
    /// Strip all formatting, plain text only (for token counting, search indexing).
    Plain,
    /// Markdown-formatted text (preserves `**`, `*`, backticks, links, images).
    Markdown,
    /// Render-oriented text: plain text but preserves backticks for code,
    /// reference notation `[ref:target]`, and footnote notation `[^...]`.
    Render,
}

/// Convert a slice of `Inline` elements to a `String` according to the given `TextMode`.
///
/// This is the single canonical implementation, replacing the previously duplicated
/// functions in `aif-markdown`, `aif-pdf/chunk/splitter`, and `aif-pdf/export/renderer`.
pub fn inlines_to_text(inlines: &[Inline], mode: TextMode) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),

            Inline::Strong { content } => match mode {
                TextMode::Markdown => {
                    out.push_str("**");
                    out.push_str(&inlines_to_text(content, mode));
                    out.push_str("**");
                }
                _ => out.push_str(&inlines_to_text(content, mode)),
            },

            Inline::Emphasis { content } => match mode {
                TextMode::Markdown => {
                    out.push('*');
                    out.push_str(&inlines_to_text(content, mode));
                    out.push('*');
                }
                _ => out.push_str(&inlines_to_text(content, mode)),
            },

            Inline::InlineCode { code } => match mode {
                TextMode::Plain => out.push_str(code),
                TextMode::Markdown | TextMode::Render => {
                    out.push('`');
                    out.push_str(code);
                    out.push('`');
                }
            },

            Inline::Link { text, url } => match mode {
                TextMode::Markdown => {
                    out.push('[');
                    out.push_str(&inlines_to_text(text, mode));
                    out.push_str("](");
                    out.push_str(url);
                    out.push(')');
                }
                _ => out.push_str(&inlines_to_text(text, mode)),
            },

            Inline::Image { alt, src } => match mode {
                TextMode::Markdown => {
                    out.push_str("![");
                    out.push_str(alt);
                    out.push_str("](");
                    out.push_str(src);
                    out.push(')');
                }
                _ => out.push_str(alt),
            },

            Inline::Reference { target } => match mode {
                TextMode::Markdown => {
                    out.push_str(&format!("[{}](#{})", target, target));
                }
                TextMode::Render => {
                    out.push_str("[ref:");
                    out.push_str(target);
                    out.push(']');
                }
                TextMode::Plain => {}
            },

            Inline::Footnote { content } => match mode {
                TextMode::Markdown | TextMode::Render => {
                    out.push_str("[^");
                    out.push_str(&inlines_to_text(content, mode));
                    out.push(']');
                }
                TextMode::Plain => {}
            },

            Inline::SoftBreak => match mode {
                TextMode::Markdown => out.push('\n'),
                _ => out.push(' '),
            },

            Inline::HardBreak => match mode {
                TextMode::Markdown => out.push_str("  \n"),
                TextMode::Render => out.push('\n'),
                TextMode::Plain => out.push(' '),
            },
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_inlines() -> Vec<Inline> {
        vec![
            Inline::Text {
                text: "Hello ".into(),
            },
            Inline::Strong {
                content: vec![Inline::Text {
                    text: "world".into(),
                }],
            },
        ]
    }

    #[test]
    fn plain_strips_formatting() {
        let inlines = sample_inlines();
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "Hello world");
    }

    #[test]
    fn markdown_preserves_strong() {
        let inlines = sample_inlines();
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Markdown),
            "Hello **world**"
        );
    }

    #[test]
    fn render_strips_strong() {
        let inlines = sample_inlines();
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Render),
            "Hello world"
        );
    }

    #[test]
    fn markdown_emphasis() {
        let inlines = vec![Inline::Emphasis {
            content: vec![Inline::Text {
                text: "italic".into(),
            }],
        }];
        assert_eq!(inlines_to_text(&inlines, TextMode::Markdown), "*italic*");
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "italic");
    }

    #[test]
    fn inline_code_modes() {
        let inlines = vec![Inline::InlineCode {
            code: "foo".into(),
        }];
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "foo");
        assert_eq!(inlines_to_text(&inlines, TextMode::Markdown), "`foo`");
        assert_eq!(inlines_to_text(&inlines, TextMode::Render), "`foo`");
    }

    #[test]
    fn link_modes() {
        let inlines = vec![Inline::Link {
            text: vec![Inline::Text {
                text: "click".into(),
            }],
            url: "http://example.com".into(),
        }];
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Markdown),
            "[click](http://example.com)"
        );
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "click");
        assert_eq!(inlines_to_text(&inlines, TextMode::Render), "click");
    }

    #[test]
    fn image_modes() {
        let inlines = vec![Inline::Image {
            alt: "photo".into(),
            src: "img.png".into(),
        }];
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Markdown),
            "![photo](img.png)"
        );
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "photo");
    }

    #[test]
    fn reference_modes() {
        let inlines = vec![Inline::Reference {
            target: "sec1".into(),
        }];
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Markdown),
            "[sec1](#sec1)"
        );
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Render),
            "[ref:sec1]"
        );
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "");
    }

    #[test]
    fn footnote_modes() {
        let inlines = vec![Inline::Footnote {
            content: vec![Inline::Text {
                text: "note".into(),
            }],
        }];
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Markdown),
            "[^note]"
        );
        assert_eq!(
            inlines_to_text(&inlines, TextMode::Render),
            "[^note]"
        );
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "");
    }

    #[test]
    fn break_modes() {
        let inlines = vec![Inline::SoftBreak, Inline::HardBreak];
        assert_eq!(inlines_to_text(&inlines, TextMode::Markdown), "\n  \n");
        assert_eq!(inlines_to_text(&inlines, TextMode::Plain), "  ");
        assert_eq!(inlines_to_text(&inlines, TextMode::Render), " \n");
    }
}
