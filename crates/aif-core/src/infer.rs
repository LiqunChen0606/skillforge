//! Semantic inference engine — upgrades untyped blocks to typed SemanticBlocks
//! based on pattern-matching rules.

use crate::ast::{
    Attrs, Block, BlockKind, CalloutType, Inline, SemanticBlockType,
};
use crate::text::{inlines_to_text, TextMode};
use std::collections::BTreeMap;

/// Configuration for semantic inference.
#[derive(Debug, Clone)]
pub struct InferConfig {
    /// Minimum confidence threshold; matches below this are ignored.
    pub min_confidence: f64,
    /// Strategy to use for inference.
    pub strategy: InferStrategy,
}

impl Default for InferConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            strategy: InferStrategy::Pattern,
        }
    }
}

/// Inference strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferStrategy {
    /// Pattern-based heuristic rules.
    Pattern,
}

/// A rule that attempts to infer a semantic type for a block.
pub trait InferRule: Send + Sync {
    /// Human-readable rule name (used in `_aif_infer_rule` attr).
    fn name(&self) -> &str;
    /// If the rule matches, returns the semantic type and a confidence score.
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)>;
}

// ---------------------------------------------------------------------------
// Helper: check if inlines contain a Link
// ---------------------------------------------------------------------------

fn inlines_contain_link(inlines: &[Inline]) -> bool {
    for inline in inlines {
        match inline {
            Inline::Link { .. } => return true,
            Inline::Strong { content }
            | Inline::Emphasis { content }
            | Inline::Footnote { content } => {
                if inlines_contain_link(content) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn blocks_contain_link(blocks: &[Block]) -> bool {
    for block in blocks {
        match &block.kind {
            BlockKind::Paragraph { content } => {
                if inlines_contain_link(content) {
                    return true;
                }
            }
            BlockKind::BlockQuote { content } => {
                if blocks_contain_link(content) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn blocks_to_plain_text(blocks: &[Block]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        if let BlockKind::Paragraph { content } = &block.kind {
            parts.push(inlines_to_text(content, TextMode::Plain));
        }
    }
    parts.join(" ")
}

fn count_sentences(text: &str) -> usize {
    text.matches(|c| c == '.' || c == '!' || c == '?').count().max(1)
}

// ---------------------------------------------------------------------------
// 8 Pattern Rules
// ---------------------------------------------------------------------------

/// BlockQuote containing a Link inline -> Evidence (0.70)
pub struct BlockquoteWithCitation;

impl InferRule for BlockquoteWithCitation {
    fn name(&self) -> &str {
        "blockquote_with_citation"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::BlockQuote { content } = &block.kind {
            if blocks_contain_link(content) {
                return Some((SemanticBlockType::Evidence, 0.70));
            }
        }
        None
    }
}

/// BlockQuote with no link and < 3 sentences -> Claim (0.55)
pub struct BlockquoteShortClaim;

impl InferRule for BlockquoteShortClaim {
    fn name(&self) -> &str {
        "blockquote_short_claim"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::BlockQuote { content } = &block.kind {
            if blocks_contain_link(content) {
                return None;
            }
            let text = blocks_to_plain_text(content);
            if count_sentences(&text) < 3 {
                return Some((SemanticBlockType::Claim, 0.55));
            }
        }
        None
    }
}

/// Paragraph starting with "we define" / "is defined as" / "definition:" -> Definition (0.80)
pub struct ParagraphDefinition;

impl InferRule for ParagraphDefinition {
    fn name(&self) -> &str {
        "paragraph_definition"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::Paragraph { content } = &block.kind {
            let text = inlines_to_text(content, TextMode::Plain).to_lowercase();
            let trimmed = text.trim_start();
            if trimmed.starts_with("we define")
                || trimmed.starts_with("is defined as")
                || trimmed.starts_with("definition:")
            {
                return Some((SemanticBlockType::Definition, 0.80));
            }
        }
        None
    }
}

/// Paragraph starting with "we recommend" / "it is recommended" -> Recommendation (0.75)
pub struct ParagraphRecommendation;

impl InferRule for ParagraphRecommendation {
    fn name(&self) -> &str {
        "paragraph_recommendation"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::Paragraph { content } = &block.kind {
            let text = inlines_to_text(content, TextMode::Plain).to_lowercase();
            let trimmed = text.trim_start();
            if trimmed.starts_with("we recommend")
                || trimmed.starts_with("it is recommended")
            {
                return Some((SemanticBlockType::Recommendation, 0.75));
            }
        }
        None
    }
}

/// Paragraph starting with "we conclude" / "in conclusion" / "therefore" -> Conclusion (0.75)
pub struct ParagraphConclusion;

impl InferRule for ParagraphConclusion {
    fn name(&self) -> &str {
        "paragraph_conclusion"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::Paragraph { content } = &block.kind {
            let text = inlines_to_text(content, TextMode::Plain).to_lowercase();
            let trimmed = text.trim_start();
            if trimmed.starts_with("we conclude")
                || trimmed.starts_with("in conclusion")
                || trimmed.starts_with("therefore")
            {
                return Some((SemanticBlockType::Conclusion, 0.75));
            }
        }
        None
    }
}

/// Paragraph starting with "we assume" / "assuming that" -> Assumption (0.70)
pub struct ParagraphAssumption;

impl InferRule for ParagraphAssumption {
    fn name(&self) -> &str {
        "paragraph_assumption"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::Paragraph { content } = &block.kind {
            let text = inlines_to_text(content, TextMode::Plain).to_lowercase();
            let trimmed = text.trim_start();
            if trimmed.starts_with("we assume")
                || trimmed.starts_with("assuming that")
            {
                return Some((SemanticBlockType::Assumption, 0.70));
            }
        }
        None
    }
}

/// Paragraph starting with "the result shows" / "results indicate" -> Result (0.65)
pub struct ParagraphResult;

impl InferRule for ParagraphResult {
    fn name(&self) -> &str {
        "paragraph_result"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::Paragraph { content } = &block.kind {
            let text = inlines_to_text(content, TextMode::Plain).to_lowercase();
            let trimmed = text.trim_start();
            if trimmed.starts_with("the result shows")
                || trimmed.starts_with("results indicate")
            {
                return Some((SemanticBlockType::Result, 0.65));
            }
        }
        None
    }
}

/// Callout[Warning] containing "requirement" / "must" / "shall" -> Requirement (0.60)
pub struct CalloutRequirement;

impl InferRule for CalloutRequirement {
    fn name(&self) -> &str {
        "callout_requirement"
    }
    fn try_infer(&self, block: &Block) -> Option<(SemanticBlockType, f64)> {
        if let BlockKind::Callout {
            callout_type: CalloutType::Warning,
            content,
            ..
        } = &block.kind
        {
            let text = inlines_to_text(content, TextMode::Plain).to_lowercase();
            if text.contains("requirement")
                || text.contains("must")
                || text.contains("shall")
            {
                return Some((SemanticBlockType::Requirement, 0.60));
            }
        }
        None
    }
}

/// Returns the default set of 8 pattern-based inference rules.
pub fn default_rules() -> Vec<Box<dyn InferRule>> {
    vec![
        Box::new(BlockquoteWithCitation),
        Box::new(BlockquoteShortClaim),
        Box::new(ParagraphDefinition),
        Box::new(ParagraphRecommendation),
        Box::new(ParagraphConclusion),
        Box::new(ParagraphAssumption),
        Box::new(ParagraphResult),
        Box::new(CalloutRequirement),
    ]
}

// ---------------------------------------------------------------------------
// annotate_semantics — walk document and upgrade matching blocks in-place
// ---------------------------------------------------------------------------

/// Walk a document and upgrade untyped blocks (Paragraph, BlockQuote, Callout)
/// to SemanticBlocks when pattern rules match above the configured confidence.
pub fn annotate_semantics(doc: &mut Document, config: &InferConfig) {
    let rules = default_rules();
    annotate_blocks(&mut doc.blocks, &rules, config);
}

use crate::ast::Document;

fn annotate_blocks(blocks: &mut Vec<Block>, rules: &[Box<dyn InferRule>], config: &InferConfig) {
    for block in blocks.iter_mut() {
        // First recurse into children
        match &mut block.kind {
            BlockKind::Section { children, .. } => {
                annotate_blocks(children, rules, config);
            }
            BlockKind::BlockQuote { content } => {
                annotate_blocks(content, rules, config);
            }
            _ => {}
        }

        // Skip blocks that shouldn't be inferred
        match &block.kind {
            BlockKind::Paragraph { .. }
            | BlockKind::BlockQuote { .. }
            | BlockKind::Callout { .. } => {}
            _ => continue,
        }

        // Run all rules, pick highest confidence
        let mut best: Option<(SemanticBlockType, f64, &str)> = None;
        for rule in rules {
            if let Some((stype, conf)) = rule.try_infer(block) {
                if conf >= config.min_confidence {
                    if best.as_ref().map_or(true, |b| conf > b.1) {
                        best = Some((stype, conf, rule.name()));
                    }
                }
            }
        }

        if let Some((block_type, confidence, rule_name)) = best {
            // Extract content from the original block
            let content = extract_content(&block.kind);

            let mut pairs = BTreeMap::new();
            pairs.insert("_aif_inferred".to_string(), "true".to_string());
            pairs.insert(
                "_aif_confidence".to_string(),
                format!("{:.2}", confidence),
            );
            pairs.insert("_aif_infer_rule".to_string(), rule_name.to_string());

            block.kind = BlockKind::SemanticBlock {
                block_type,
                attrs: Attrs { id: None, pairs },
                title: None,
                content,
            };
        }
    }
}

fn extract_content(kind: &BlockKind) -> Vec<Inline> {
    match kind {
        BlockKind::Paragraph { content } => content.clone(),
        BlockKind::BlockQuote { content } => {
            // Flatten child paragraph content
            let mut inlines = Vec::new();
            for block in content {
                if let BlockKind::Paragraph { content: c } = &block.kind {
                    if !inlines.is_empty() {
                        inlines.push(Inline::SoftBreak);
                    }
                    inlines.extend(c.clone());
                }
            }
            inlines
        }
        BlockKind::Callout { content, .. } => content.clone(),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    use crate::span::Span;

    fn text(s: &str) -> Inline {
        Inline::Text { text: s.into() }
    }

    fn para(inlines: Vec<Inline>) -> Block {
        Block {
            kind: BlockKind::Paragraph { content: inlines },
            span: Span::new(0, 0),
        }
    }

    fn blockquote(children: Vec<Block>) -> Block {
        Block {
            kind: BlockKind::BlockQuote { content: children },
            span: Span::new(0, 0),
        }
    }

    fn callout(ct: CalloutType, inlines: Vec<Inline>) -> Block {
        Block {
            kind: BlockKind::Callout {
                callout_type: ct,
                attrs: Attrs::new(),
                content: inlines,
            },
            span: Span::new(0, 0),
        }
    }

    #[test]
    fn default_config() {
        let config = InferConfig::default();
        assert!((config.min_confidence - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.strategy, InferStrategy::Pattern);
    }

    #[test]
    fn blockquote_with_link_inferred_as_evidence() {
        let rule = BlockquoteWithCitation;
        let bq = blockquote(vec![para(vec![
            text("See "),
            Inline::Link {
                text: vec![text("source")],
                url: "https://example.com".into(),
            },
        ])]);
        let result = rule.try_infer(&bq);
        assert_eq!(result, Some((SemanticBlockType::Evidence, 0.70)));
    }

    #[test]
    fn short_blockquote_inferred_as_claim() {
        let rule = BlockquoteShortClaim;
        let bq = blockquote(vec![para(vec![text("The sky is blue.")])]);
        let result = rule.try_infer(&bq);
        assert_eq!(result, Some((SemanticBlockType::Claim, 0.55)));
    }

    #[test]
    fn paragraph_definition_detected() {
        let rule = ParagraphDefinition;
        let p = para(vec![text("We define X as the set of all Y.")]);
        assert_eq!(
            rule.try_infer(&p),
            Some((SemanticBlockType::Definition, 0.80))
        );

        let p2 = para(vec![text("Definition: X is a mapping from A to B.")]);
        assert_eq!(
            rule.try_infer(&p2),
            Some((SemanticBlockType::Definition, 0.80))
        );
    }

    #[test]
    fn paragraph_conclusion_detected() {
        let rule = ParagraphConclusion;
        let p = para(vec![text("We conclude that the hypothesis holds.")]);
        assert_eq!(
            rule.try_infer(&p),
            Some((SemanticBlockType::Conclusion, 0.75))
        );

        let p2 = para(vec![text("Therefore, X must be true.")]);
        assert_eq!(
            rule.try_infer(&p2),
            Some((SemanticBlockType::Conclusion, 0.75))
        );
    }

    #[test]
    fn plain_paragraph_not_inferred() {
        let rules = default_rules();
        let p = para(vec![text("This is just a normal paragraph.")]);
        for rule in &rules {
            assert!(rule.try_infer(&p).is_none(), "rule {} matched", rule.name());
        }
    }

    #[test]
    fn callout_requirement_detected() {
        let rule = CalloutRequirement;
        let c = callout(
            CalloutType::Warning,
            vec![text("The system must handle errors gracefully.")],
        );
        assert_eq!(
            rule.try_infer(&c),
            Some((SemanticBlockType::Requirement, 0.60))
        );
    }

    #[test]
    fn callout_note_not_inferred_as_requirement() {
        let rule = CalloutRequirement;
        // Note callout, even with "must", should NOT match (only Warning matches)
        let c = callout(
            CalloutType::Note,
            vec![text("You must remember this.")],
        );
        assert!(rule.try_infer(&c).is_none());
    }

    #[test]
    fn annotate_upgrades_blockquote_to_claim() {
        let mut doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![blockquote(vec![para(vec![text("A short claim.")])])],
        };
        let config = InferConfig::default();
        annotate_semantics(&mut doc, &config);

        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0].kind {
            BlockKind::SemanticBlock {
                block_type, attrs, ..
            } => {
                assert_eq!(*block_type, SemanticBlockType::Claim);
                assert_eq!(attrs.pairs.get("_aif_inferred").unwrap(), "true");
                assert_eq!(attrs.pairs.get("_aif_confidence").unwrap(), "0.55");
                assert_eq!(
                    attrs.pairs.get("_aif_infer_rule").unwrap(),
                    "blockquote_short_claim"
                );
            }
            other => panic!("expected SemanticBlock, got {:?}", other),
        }
    }

    #[test]
    fn annotate_skips_existing_semantic_blocks() {
        let mut doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::SemanticBlock {
                    block_type: SemanticBlockType::Theorem,
                    attrs: Attrs::new(),
                    title: None,
                    content: vec![text("Existing theorem.")],
                },
                span: Span::new(0, 0),
            }],
        };
        let config = InferConfig::default();
        annotate_semantics(&mut doc, &config);

        // Should remain unchanged
        match &doc.blocks[0].kind {
            BlockKind::SemanticBlock { block_type, attrs, .. } => {
                assert_eq!(*block_type, SemanticBlockType::Theorem);
                assert!(attrs.pairs.get("_aif_inferred").is_none());
            }
            other => panic!("expected SemanticBlock, got {:?}", other),
        }
    }

    #[test]
    fn annotate_respects_min_confidence() {
        // BlockquoteShortClaim gives 0.55; with min_confidence=0.90 it should be skipped
        let mut doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![blockquote(vec![para(vec![text("A short claim.")])])],
        };
        let config = InferConfig {
            min_confidence: 0.90,
            strategy: InferStrategy::Pattern,
        };
        annotate_semantics(&mut doc, &config);

        // Should remain as BlockQuote
        match &doc.blocks[0].kind {
            BlockKind::BlockQuote { .. } => {}
            other => panic!("expected BlockQuote to remain, got {:?}", other),
        }
    }
}
