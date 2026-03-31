use aif_core::ast::*;

/// A recommendation for which output format to use.
#[derive(Debug, Clone, PartialEq)]
pub struct FormatRecommendation {
    pub format: String,
    pub reason: String,
}

/// Analyze a document's structure and recommend the optimal output format.
///
/// Priority order:
/// 1. Skill blocks present → "lml-aggressive" (best TNO from benchmarks)
/// 2. >40% code blocks → "markdown" (preserves code naturally)
/// 3. Semantic blocks present → "lml-conservative" (preserves semantic types)
/// 4. Default → "markdown" (best general token efficiency)
pub fn recommend_format(doc: &Document) -> FormatRecommendation {
    let (total, skill_count, code_count, semantic_count) = count_block_types(&doc.blocks);

    if skill_count > 0 {
        return FormatRecommendation {
            format: "lml-aggressive".into(),
            reason: "Document contains skill blocks; lml-aggressive has the best TNO (0.99) for skills".into(),
        };
    }

    if total > 0 && (code_count as f64 / total as f64) > 0.4 {
        return FormatRecommendation {
            format: "markdown".into(),
            reason: "Document is code-heavy (>40% code blocks); Markdown preserves code naturally".into(),
        };
    }

    if semantic_count > 0 {
        return FormatRecommendation {
            format: "lml-conservative".into(),
            reason: "Document contains semantic blocks; lml-conservative preserves semantic types with abbreviated tags".into(),
        };
    }

    FormatRecommendation {
        format: "markdown".into(),
        reason: "General document; Markdown offers the best token efficiency (+1.9% saved vs baseline)".into(),
    }
}

/// Recommend a format for a specific purpose, overriding structural analysis when appropriate.
///
/// Supported purposes:
/// - "wire" → "binary-wire" (compact for network transport)
/// - "storage" → "json" (structured, queryable)
/// - anything else → falls back to structural analysis via `recommend_format`
pub fn recommend_format_for_purpose(doc: &Document, purpose: &str) -> FormatRecommendation {
    match purpose {
        "wire" => FormatRecommendation {
            format: "binary-wire".into(),
            reason: "Wire transport purpose; binary-wire is ~82% smaller than JSON in bytes".into(),
        },
        "storage" => FormatRecommendation {
            format: "json".into(),
            reason: "Storage purpose; JSON is structured and queryable".into(),
        },
        _ => recommend_format(doc),
    }
}

/// Recursively count block types in a list of blocks.
/// Returns (total, skill_count, code_count, semantic_count).
fn count_block_types(blocks: &[Block]) -> (usize, usize, usize, usize) {
    let mut total = 0;
    let mut skill = 0;
    let mut code = 0;
    let mut semantic = 0;

    for block in blocks {
        total += 1;
        match &block.kind {
            BlockKind::SkillBlock { children, .. } => {
                skill += 1;
                let (t, s, c, sem) = count_block_types(children);
                total += t;
                skill += s;
                code += c;
                semantic += sem;
            }
            BlockKind::CodeBlock { .. } => {
                code += 1;
            }
            BlockKind::SemanticBlock { .. } => {
                semantic += 1;
            }
            BlockKind::Section { children, .. } => {
                let (t, s, c, sem) = count_block_types(children);
                total += t;
                skill += s;
                code += c;
                semantic += sem;
            }
            BlockKind::BlockQuote { content } => {
                let (t, s, c, sem) = count_block_types(content);
                total += t;
                skill += s;
                code += c;
                semantic += sem;
            }
            _ => {}
        }
    }

    (total, skill, code, semantic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    fn span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn count_basics() {
        let blocks = vec![
            Block { kind: BlockKind::Paragraph { content: vec![] }, span: span() },
            Block { kind: BlockKind::CodeBlock { lang: None, attrs: Attrs::new(), code: "x".into() }, span: span() },
        ];
        let (total, skill, code, semantic) = count_block_types(&blocks);
        assert_eq!((total, skill, code, semantic), (2, 0, 1, 0));
    }
}
