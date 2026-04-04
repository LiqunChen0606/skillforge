use aif_core::ast::SkillBlockType;

use crate::types::{BlockObservation, ObservableBlock, ObservationStatus};

/// Common English stopwords to filter out when extracting keywords.
const STOPWORDS: &[&str] = &[
    "about", "after", "also", "been", "before", "being", "between", "both",
    "could", "does", "doing", "down", "each", "even", "every", "from",
    "have", "having", "here", "into", "just", "like", "make", "many",
    "more", "most", "much", "must", "only", "other", "over", "same",
    "should", "some", "such", "take", "than", "that", "their", "them",
    "then", "there", "these", "they", "this", "through", "very", "well",
    "were", "what", "when", "where", "which", "while", "will", "with",
    "within", "without", "would", "your",
];

/// Extract meaningful keywords (>4 chars, not stopwords) from text.
fn extract_keywords(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|w| w.len() > 4)
        .map(|w| w.to_lowercase())
        .filter(|w| !STOPWORDS.contains(&w.as_str()))
        .collect::<Vec<_>>()
}

/// Check if a keyword appears in a negation context in the output.
/// Looks for negation words within a small window before the keyword.
fn is_negated(output_lower: &str, keyword: &str) -> bool {
    let negation_markers = &["not ", "never ", "don't ", "doesn't ", "didn't ", "cannot ",
                             "can't ", "won't ", "shouldn't ", "avoid "];
    // Find all occurrences of the keyword
    for (idx, _) in output_lower.match_indices(keyword) {
        // Look at the 30 chars before the keyword
        let start = idx.saturating_sub(30);
        let window = &output_lower[start..idx];
        for marker in negation_markers {
            if window.contains(marker) {
                return true;
            }
        }
    }
    false
}

/// Match an observable block against LLM output text.
///
/// Strategy:
/// - Extract keywords from the block content
/// - Check keyword presence in the output
/// - Score: >=60% = Followed, 30-60% = Partial, <30% = Skipped
/// - For RedFlag blocks: if keywords found in negated context = ok (Followed),
///   if keywords found without negation = Violated
pub fn match_block(block: &ObservableBlock, llm_output: &str) -> BlockObservation {
    let keywords = extract_keywords(&block.full_content);

    // If no keywords extractable, mark NotApplicable
    if keywords.is_empty() {
        return BlockObservation {
            block: block.clone(),
            status: ObservationStatus::NotApplicable,
            match_score: 0.0,
            matched_keywords: vec![],
            missing_keywords: vec![],
        };
    }

    let output_lower = llm_output.to_lowercase();
    let mut matched = Vec::new();
    let mut missing = Vec::new();

    for kw in &keywords {
        if output_lower.contains(kw.as_str()) {
            matched.push(kw.clone());
        } else {
            missing.push(kw.clone());
        }
    }

    let score = matched.len() as f64 / keywords.len() as f64;

    // For red_flag blocks, the logic is inverted:
    // - If keywords appear WITHOUT negation, it means the flag was violated
    // - If keywords appear WITH negation or don't appear, it's followed (the flag was respected)
    let status = if block.block_type == SkillBlockType::RedFlag {
        if matched.is_empty() {
            // Red flag topics not mentioned at all = followed (avoided)
            ObservationStatus::Followed
        } else {
            // Check if matched keywords are in negation context
            let negated_count = matched.iter()
                .filter(|kw| is_negated(&output_lower, kw))
                .count();

            if negated_count == matched.len() {
                // All mentions are negated = the agent acknowledged and avoided
                ObservationStatus::Followed
            } else if negated_count > 0 {
                // Mixed = partial
                ObservationStatus::Partial
            } else {
                // Keywords present without negation = violation
                ObservationStatus::Violated
            }
        }
    } else {
        // For non-red-flag blocks: standard keyword coverage scoring
        if score >= 0.6 {
            ObservationStatus::Followed
        } else if score >= 0.3 {
            ObservationStatus::Partial
        } else {
            ObservationStatus::Skipped
        }
    };

    BlockObservation {
        block: block.clone(),
        status,
        match_score: score,
        matched_keywords: matched,
        missing_keywords: missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_step(content: &str) -> ObservableBlock {
        ObservableBlock {
            block_type: SkillBlockType::Step,
            block_id: None,
            order: Some(1),
            content_snippet: content.chars().take(100).collect(),
            full_content: content.to_string(),
        }
    }

    fn make_red_flag(content: &str) -> ObservableBlock {
        ObservableBlock {
            block_type: SkillBlockType::RedFlag,
            block_id: None,
            order: None,
            content_snippet: content.chars().take(100).collect(),
            full_content: content.to_string(),
        }
    }

    #[test]
    fn step_followed() {
        let block = make_step("Check correctness of the code and verify error handling");
        let output = "I checked the correctness of the code. The error handling looks solid \
                      and I verified all edge cases.";
        let obs = match_block(&block, output);
        assert_eq!(obs.status, ObservationStatus::Followed);
        assert!(obs.match_score >= 0.6);
    }

    #[test]
    fn step_skipped() {
        let block = make_step("Check correctness of the code and verify error handling");
        let output = "The weather is nice today. I went for a walk in the park.";
        let obs = match_block(&block, output);
        assert_eq!(obs.status, ObservationStatus::Skipped);
        assert!(obs.match_score < 0.3);
    }

    #[test]
    fn step_partial() {
        let block = make_step("Check correctness security performance and maintainability of the implementation");
        let output = "I reviewed the correctness of the code and found no issues with the implementation.";
        let obs = match_block(&block, output);
        assert_eq!(obs.status, ObservationStatus::Partial);
        assert!(obs.match_score >= 0.3);
        assert!(obs.match_score < 0.6);
    }

    #[test]
    fn red_flag_violated() {
        let block = make_red_flag("Approving without running tests because it looks fine");
        let output = "Everything looks fine so I'm approving without running tests.";
        let obs = match_block(&block, output);
        assert_eq!(obs.status, ObservationStatus::Violated);
    }

    #[test]
    fn red_flag_followed_negated() {
        let block = make_red_flag("Approving without running tests");
        let output = "I would never approve without running tests first. Let me run the suite.";
        let obs = match_block(&block, output);
        assert_eq!(obs.status, ObservationStatus::Followed);
    }

    #[test]
    fn red_flag_followed_not_mentioned() {
        let block = make_red_flag("Bikeshedding on style preferences while ignoring actual defects");
        let output = "I focused on correctness and found two bugs in the pagination handler.";
        let obs = match_block(&block, output);
        assert_eq!(obs.status, ObservationStatus::Followed);
    }

    #[test]
    fn empty_keywords_not_applicable() {
        let block = ObservableBlock {
            block_type: SkillBlockType::Step,
            block_id: None,
            order: None,
            content_snippet: "Do it.".into(),
            full_content: "Do it.".into(),
        };
        let obs = match_block(&block, "anything");
        assert_eq!(obs.status, ObservationStatus::NotApplicable);
    }

    #[test]
    fn extract_keywords_filters_short_and_stops() {
        let kws = extract_keywords("The quick brown fox jumps over lazy dogs and performs testing");
        assert!(kws.contains(&"quick".to_string()));
        assert!(kws.contains(&"brown".to_string()));
        assert!(kws.contains(&"jumps".to_string()));
        assert!(kws.contains(&"testing".to_string()));
        // "the", "over", "and", "dogs", "lazy", "fox" should be filtered (<=4 chars or stopword)
        assert!(!kws.contains(&"the".to_string()));
        assert!(!kws.contains(&"over".to_string()));
        assert!(!kws.contains(&"fox".to_string()));
    }
}
