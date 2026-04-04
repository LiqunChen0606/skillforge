//! Semantic inference engine — upgrades untyped blocks to typed SemanticBlocks
//! based on pattern-matching rules and optional LLM classification.

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
#[derive(Debug, Clone)]
pub enum InferStrategy {
    /// Pattern-based heuristic rules.
    Pattern,
    /// LLM-assisted classification (pattern rules first, then LLM for unmatched).
    Llm(crate::config::LlmConfig),
}

impl PartialEq for InferStrategy {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (InferStrategy::Pattern, InferStrategy::Pattern)
                | (InferStrategy::Llm(_), InferStrategy::Llm(_))
        )
    }
}

impl Eq for InferStrategy {}

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
    text.matches(['.', '!', '?']).count().max(1)
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

fn annotate_blocks(blocks: &mut [Block], rules: &[Box<dyn InferRule>], config: &InferConfig) {
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
                if conf >= config.min_confidence
                    && best.as_ref().is_none_or(|b| conf > b.1)
                {
                    best = Some((stype, conf, rule.name()));
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
// LLM-assisted semantic inference (feature-gated)
// ---------------------------------------------------------------------------

/// Extract plain text from a block for LLM classification.
#[allow(dead_code)]
fn block_text(block: &Block) -> String {
    match &block.kind {
        BlockKind::Paragraph { content } => inlines_to_text(content, TextMode::Plain),
        BlockKind::BlockQuote { content } => blocks_to_plain_text(content),
        BlockKind::Callout { content, .. } => inlines_to_text(content, TextMode::Plain),
        _ => String::new(),
    }
}

/// Build the LLM classification prompt for a batch of blocks.
pub fn build_classification_prompt(blocks: &[(usize, String)]) -> String {
    let mut prompt = String::from(
        "Classify each text block into one of these semantic types, or 'none' if it doesn't fit:\n\
         Types: Claim, Evidence, Definition, Theorem, Assumption, Result, Conclusion, Requirement, Recommendation\n\n\
         For each block, respond with one line: INDEX:TYPE:CONFIDENCE\n\
         where CONFIDENCE is 0.0-1.0 and TYPE is one of the above or 'none'.\n\n",
    );
    for (idx, text) in blocks {
        let truncated = if text.len() > 200 {
            &text[..200]
        } else {
            text.as_str()
        };
        prompt.push_str(&format!("Block {}:\n{}\n\n", idx, truncated));
    }
    prompt
}

/// Parse the LLM classification response (INDEX:TYPE:CONFIDENCE lines).
pub fn parse_classification_response(
    body: &str,
    _count: usize,
) -> Vec<(usize, Option<SemanticBlockType>, f64)> {
    // Try to extract text content from Anthropic API JSON response
    let text = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
        parsed["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string()
    } else {
        // Treat as raw text (useful for testing)
        body.to_string()
    };

    let mut results = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            let idx = match parts[0].trim().parse::<usize>() {
                Ok(i) => i,
                Err(_) => continue,
            };
            let stype = match parts[1].trim().to_lowercase().as_str() {
                "claim" => Some(SemanticBlockType::Claim),
                "evidence" => Some(SemanticBlockType::Evidence),
                "definition" => Some(SemanticBlockType::Definition),
                "theorem" => Some(SemanticBlockType::Theorem),
                "assumption" => Some(SemanticBlockType::Assumption),
                "result" => Some(SemanticBlockType::Result),
                "conclusion" => Some(SemanticBlockType::Conclusion),
                "requirement" => Some(SemanticBlockType::Requirement),
                "recommendation" => Some(SemanticBlockType::Recommendation),
                _ => None,
            };
            let conf = parts[2].trim().parse::<f64>().unwrap_or(0.0);
            results.push((idx, stype, conf));
        }
    }
    results
}

/// Collect blocks that were not already upgraded by pattern rules.
/// Returns (block_index, text_content) pairs for top-level blocks only.
#[allow(dead_code)]
fn collect_unmatched_blocks(doc: &Document) -> Vec<(usize, String)> {
    let mut unmatched = Vec::new();
    for (i, block) in doc.blocks.iter().enumerate() {
        match &block.kind {
            BlockKind::Paragraph { .. }
            | BlockKind::BlockQuote { .. }
            | BlockKind::Callout { .. } => {
                let text = block_text(block);
                if text.len() >= 10 {
                    unmatched.push((i, text));
                }
            }
            _ => {}
        }
    }
    unmatched
}

/// Apply LLM classifications back into the document.
#[allow(dead_code)]
fn apply_llm_classifications(
    doc: &mut Document,
    classifications: &[(usize, Option<SemanticBlockType>, f64)],
    min_confidence: f64,
) {
    for &(idx, ref stype, conf) in classifications {
        if idx >= doc.blocks.len() {
            continue;
        }
        let stype = match stype {
            Some(t) => t.clone(),
            None => continue,
        };
        if conf < min_confidence {
            continue;
        }
        // Only upgrade if block is still untyped (not already a SemanticBlock)
        let block = &doc.blocks[idx];
        match &block.kind {
            BlockKind::Paragraph { .. }
            | BlockKind::BlockQuote { .. }
            | BlockKind::Callout { .. } => {}
            _ => continue,
        }

        let content = extract_content(&doc.blocks[idx].kind);
        let mut pairs = BTreeMap::new();
        pairs.insert("_aif_inferred".to_string(), "true".to_string());
        pairs.insert("_aif_confidence".to_string(), format!("{:.2}", conf));
        pairs.insert("_aif_infer_rule".to_string(), "llm".to_string());

        doc.blocks[idx].kind = BlockKind::SemanticBlock {
            block_type: stype,
            attrs: Attrs { id: None, pairs },
            title: None,
            content,
        };
    }
}

/// Run pattern rules first, then batch remaining unmatched blocks for LLM classification.
///
/// Requires the `llm` feature. Without it, this function is not available;
/// use `annotate_semantics` (pattern-only) instead.
#[cfg(feature = "llm")]
pub async fn annotate_semantics_with_llm(doc: &mut Document, config: &InferConfig) {
    // Step 1: Run pattern rules (cheap, fast)
    let pattern_config = InferConfig {
        min_confidence: config.min_confidence,
        strategy: InferStrategy::Pattern,
    };
    annotate_semantics(doc, &pattern_config);

    // Step 2: Check if we're in LLM mode
    let llm_config = match &config.strategy {
        InferStrategy::Llm(c) => c,
        _ => return, // Not LLM mode, pattern-only already done
    };

    // Step 3: Collect unmatched blocks
    let unmatched = collect_unmatched_blocks(doc);
    if unmatched.is_empty() {
        return;
    }

    // Step 4: Call LLM for classification
    let classifications = classify_blocks_with_llm(llm_config, &unmatched).await;

    // Step 5: Apply results
    apply_llm_classifications(doc, &classifications, config.min_confidence);
}

#[cfg(feature = "llm")]
async fn classify_blocks_with_llm(
    config: &crate::config::LlmConfig,
    blocks: &[(usize, String)],
) -> Vec<(usize, Option<SemanticBlockType>, f64)> {
    let prompt = build_classification_prompt(blocks);

    let api_key = match &config.api_key {
        Some(key) => key,
        None => {
            eprintln!("Warning: no API key configured for LLM inference, skipping");
            return vec![];
        }
    };

    let model = config.resolved_model();

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap_or_default();
            parse_classification_response(&body, blocks.len())
        }
        Err(e) => {
            eprintln!("LLM classification request failed: {}", e);
            vec![]
        }
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

    // -----------------------------------------------------------------------
    // LLM inference tests (no network required)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_classification_response_raw_text() {
        let text = "0:Claim:0.85\n1:Evidence:0.72\n2:none:0.30\n";
        let results = super::parse_classification_response(text, 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], (0, Some(SemanticBlockType::Claim), 0.85));
        assert_eq!(results[1], (1, Some(SemanticBlockType::Evidence), 0.72));
        assert_eq!(results[2], (2, None, 0.30));
    }

    #[test]
    fn parse_classification_response_from_api_json() {
        let json = r#"{"content":[{"type":"text","text":"0:Definition:0.90\n1:Theorem:0.80"}]}"#;
        let results = super::parse_classification_response(json, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (0, Some(SemanticBlockType::Definition), 0.90));
        assert_eq!(results[1], (1, Some(SemanticBlockType::Theorem), 0.80));
    }

    #[test]
    fn parse_classification_response_handles_garbage() {
        let text = "not a valid line\nfoo:bar:baz\n";
        let results = super::parse_classification_response(text, 0);
        assert!(results.is_empty());
    }

    #[test]
    fn build_prompt_includes_all_blocks() {
        let blocks = vec![
            (0, "This is a claim about the world.".to_string()),
            (3, "Evidence suggests that X is true.".to_string()),
        ];
        let prompt = super::build_classification_prompt(&blocks);
        assert!(prompt.contains("Block 0:"));
        assert!(prompt.contains("Block 3:"));
        assert!(prompt.contains("Claim, Evidence, Definition"));
        assert!(prompt.contains("INDEX:TYPE:CONFIDENCE"));
    }

    #[test]
    fn build_prompt_truncates_long_text() {
        let long_text = "A".repeat(500);
        let blocks = vec![(0, long_text)];
        let prompt = super::build_classification_prompt(&blocks);
        // Should contain truncated version (200 chars), not full 500
        assert!(prompt.len() < 500 + 200);
    }

    #[test]
    fn collect_unmatched_skips_short_text() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![
                para(vec![text("Hi.")]), // < 10 chars, should be skipped
                para(vec![text("This paragraph has enough text to be classified.")]),
            ],
        };
        let unmatched = super::collect_unmatched_blocks(&doc);
        assert_eq!(unmatched.len(), 1);
        assert_eq!(unmatched[0].0, 1);
    }

    #[test]
    fn apply_llm_classifications_upgrades_blocks() {
        let mut doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![
                para(vec![text("This is a claim about something important.")]),
                para(vec![text("Some normal text here.")]),
            ],
        };
        let classifications = vec![
            (0, Some(SemanticBlockType::Claim), 0.85),
            (1, Some(SemanticBlockType::Evidence), 0.30), // below threshold
        ];
        super::apply_llm_classifications(&mut doc, &classifications, 0.5);

        // Block 0 should be upgraded
        match &doc.blocks[0].kind {
            BlockKind::SemanticBlock {
                block_type, attrs, ..
            } => {
                assert_eq!(*block_type, SemanticBlockType::Claim);
                assert_eq!(attrs.pairs.get("_aif_infer_rule").unwrap(), "llm");
            }
            other => panic!("expected SemanticBlock, got {:?}", other),
        }

        // Block 1 should remain a Paragraph (confidence too low)
        match &doc.blocks[1].kind {
            BlockKind::Paragraph { .. } => {}
            other => panic!("expected Paragraph to remain, got {:?}", other),
        }
    }

    #[test]
    fn infer_strategy_eq() {
        assert_eq!(InferStrategy::Pattern, InferStrategy::Pattern);
        let llm1 = InferStrategy::Llm(crate::config::LlmConfig::default());
        let llm2 = InferStrategy::Llm(crate::config::LlmConfig::default());
        assert_eq!(llm1, llm2);
        assert_ne!(InferStrategy::Pattern, llm1);
    }
}
