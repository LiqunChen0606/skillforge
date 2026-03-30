use aif_core::ast::*;

pub fn emit_html(doc: &Document) -> String {
    let mut out = String::new();
    let title = doc.metadata.get("title").cloned().unwrap_or_default();

    out.push_str("<!DOCTYPE html>\n");
    out.push_str("<html lang=\"en\">\n<head>\n");
    out.push_str(&format!(
        "  <meta charset=\"utf-8\">\n  <title>{}</title>\n",
        escape_html(&title)
    ));
    if let Some(summary) = doc.metadata.get("summary") {
        out.push_str(&format!(
            "  <meta name=\"description\" content=\"{}\">\n",
            escape_html(summary)
        ));
    }
    out.push_str("</head>\n<body>\n");

    for block in &doc.blocks {
        emit_block(&mut out, block, 2);
    }

    out.push_str("</body>\n</html>\n");
    out
}

fn emit_block(out: &mut String, block: &Block, heading_level: u8) {
    match &block.kind {
        BlockKind::Section {
            attrs,
            title,
            children,
        } => {
            let level = heading_level.min(6);
            if let Some(id) = &attrs.id {
                out.push_str(&format!("<section id=\"{}\">", escape_html(id)));
            } else {
                out.push_str("<section>");
            }
            out.push_str(&format!("<h{}>", level));
            emit_inlines(out, title);
            out.push_str(&format!("</h{}>", level));
            for child in children {
                emit_block(out, child, heading_level.saturating_add(1));
            }
            out.push_str("</section>\n");
        }
        BlockKind::Paragraph { content } => {
            out.push_str("<p>");
            emit_inlines(out, content);
            out.push_str("</p>\n");
        }
        BlockKind::SemanticBlock {
            block_type,
            attrs,
            title,
            content,
        } => {
            let type_name = semantic_block_type_name(block_type);
            if let Some(id) = &attrs.id {
                out.push_str(&format!(
                    "<div class=\"aif-{}\" id=\"{}\">",
                    type_name,
                    escape_html(id)
                ));
            } else {
                out.push_str(&format!("<div class=\"aif-{}\">", type_name));
            }
            if let Some(title_inlines) = title {
                out.push_str("<strong>");
                emit_inlines(out, title_inlines);
                out.push_str("</strong>");
            }
            out.push_str("<p>");
            emit_inlines(out, content);
            out.push_str("</p>");
            out.push_str("</div>\n");
        }
        BlockKind::Callout {
            callout_type,
            attrs: _,
            content,
        } => {
            let type_name = callout_type_name(callout_type);
            out.push_str(&format!(
                "<aside class=\"aif-callout aif-{}\">",
                type_name
            ));
            out.push_str("<p>");
            emit_inlines(out, content);
            out.push_str("</p>");
            out.push_str("</aside>\n");
        }
        BlockKind::Table {
            attrs,
            caption,
            headers,
            rows,
        } => {
            if let Some(id) = &attrs.id {
                out.push_str(&format!("<table id=\"{}\">", escape_html(id)));
            } else {
                out.push_str("<table>");
            }
            if let Some(cap) = caption {
                out.push_str("<caption>");
                emit_inlines(out, cap);
                out.push_str("</caption>");
            }
            if !headers.is_empty() {
                out.push_str("<thead><tr>");
                for cell in headers {
                    out.push_str("<th>");
                    emit_inlines(out, cell);
                    out.push_str("</th>");
                }
                out.push_str("</tr></thead>");
            }
            if !rows.is_empty() {
                out.push_str("<tbody>");
                for row in rows {
                    out.push_str("<tr>");
                    for cell in row {
                        out.push_str("<td>");
                        emit_inlines(out, cell);
                        out.push_str("</td>");
                    }
                    out.push_str("</tr>");
                }
                out.push_str("</tbody>");
            }
            out.push_str("</table>\n");
        }
        BlockKind::Figure {
            attrs,
            caption,
            src,
        } => {
            if let Some(id) = &attrs.id {
                out.push_str(&format!("<figure id=\"{}\">", escape_html(id)));
            } else {
                out.push_str("<figure>");
            }
            out.push_str(&format!("<img src=\"{}\" alt=\"\">", escape_html(src)));
            if let Some(cap) = caption {
                out.push_str("<figcaption>");
                emit_inlines(out, cap);
                out.push_str("</figcaption>");
            }
            out.push_str("</figure>\n");
        }
        BlockKind::CodeBlock {
            lang, attrs: _, code,
        } => {
            if let Some(language) = lang {
                out.push_str(&format!(
                    "<pre><code class=\"language-{}\">",
                    escape_html(language)
                ));
            } else {
                out.push_str("<pre><code>");
            }
            out.push_str(&escape_html(code));
            out.push_str("</code></pre>\n");
        }
        BlockKind::BlockQuote { content } => {
            out.push_str("<blockquote>\n");
            for child in content {
                emit_block(out, child, heading_level);
            }
            out.push_str("</blockquote>\n");
        }
        BlockKind::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            out.push_str(&format!("<{}>", tag));
            for item in items {
                out.push_str("<li>");
                emit_inlines(out, &item.content);
                for child in &item.children {
                    emit_block(out, child, heading_level);
                }
                out.push_str("</li>");
            }
            out.push_str(&format!("</{}>", tag));
            out.push('\n');
        }
        BlockKind::SkillBlock {
            skill_type,
            attrs,
            title,
            content,
            children,
        } => {
            let class = skill_block_class(skill_type);
            if let Some(id) = &attrs.id {
                out.push_str(&format!(
                    "<div class=\"{}\" id=\"{}\">",
                    class,
                    escape_html(id)
                ));
            } else {
                out.push_str(&format!("<div class=\"{}\">", class));
            }
            if let Some(title_inlines) = title {
                out.push_str("<h3>");
                emit_inlines(out, title_inlines);
                out.push_str("</h3>");
            }
            if !content.is_empty() {
                out.push_str("<p>");
                emit_inlines(out, content);
                out.push_str("</p>");
            }
            for child in children {
                emit_block(out, child, heading_level);
            }
            out.push_str("</div>\n");
        }
        BlockKind::ThematicBreak => {
            out.push_str("<hr>\n");
        }
    }
}

fn emit_inlines(out: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        emit_inline(out, inline);
    }
}

fn emit_inline(out: &mut String, inline: &Inline) {
    match inline {
        Inline::Text { text } => {
            out.push_str(&escape_html(text));
        }
        Inline::Strong { content } => {
            out.push_str("<strong>");
            emit_inlines(out, content);
            out.push_str("</strong>");
        }
        Inline::Emphasis { content } => {
            out.push_str("<em>");
            emit_inlines(out, content);
            out.push_str("</em>");
        }
        Inline::InlineCode { code } => {
            out.push_str("<code>");
            out.push_str(&escape_html(code));
            out.push_str("</code>");
        }
        Inline::Link { text, url } => {
            out.push_str(&format!("<a href=\"{}\">", escape_html(url)));
            emit_inlines(out, text);
            out.push_str("</a>");
        }
        Inline::Reference { target } => {
            out.push_str(&format!(
                "<a class=\"aif-ref\" href=\"#{}\">{}</a>",
                escape_html(target),
                escape_html(target)
            ));
        }
        Inline::Footnote { content } => {
            out.push_str("<sup class=\"aif-footnote\">");
            emit_inlines(out, content);
            out.push_str("</sup>");
        }
        Inline::SoftBreak => {
            out.push('\n');
        }
        Inline::HardBreak => {
            out.push_str("<br>\n");
        }
    }
}

fn skill_block_class(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "aif-skill",
        SkillBlockType::Step => "aif-step",
        SkillBlockType::Verify => "aif-verify",
        SkillBlockType::Precondition => "aif-precondition",
        SkillBlockType::OutputContract => "aif-output-contract",
        SkillBlockType::Decision => "aif-decision",
        SkillBlockType::Tool => "aif-tool",
        SkillBlockType::Fallback => "aif-fallback",
        SkillBlockType::RedFlag => "aif-red-flag",
        SkillBlockType::Example => "aif-example",
    }
}

fn semantic_block_type_name(t: &SemanticBlockType) -> &'static str {
    match t {
        SemanticBlockType::Claim => "claim",
        SemanticBlockType::Evidence => "evidence",
        SemanticBlockType::Definition => "definition",
        SemanticBlockType::Theorem => "theorem",
        SemanticBlockType::Assumption => "assumption",
        SemanticBlockType::Result => "result",
        SemanticBlockType::Conclusion => "conclusion",
        SemanticBlockType::Requirement => "requirement",
        SemanticBlockType::Recommendation => "recommendation",
    }
}

fn callout_type_name(t: &CalloutType) -> &'static str {
    match t {
        CalloutType::Note => "note",
        CalloutType::Warning => "warning",
        CalloutType::Info => "info",
        CalloutType::Tip => "tip",
    }
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
        assert_eq!(escape_html("say \"hi\""), "say &quot;hi&quot;");
        assert_eq!(escape_html("plain"), "plain");
    }
}
