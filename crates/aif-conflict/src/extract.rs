use aif_core::ast::{Block, BlockKind, Document, SkillBlockType};
use aif_core::text::{inlines_to_text, TextMode};

use crate::types::{Directive, DirectiveType, Polarity, Strength};

/// Stopwords to exclude from keyword extraction.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one",
    "our", "out", "has", "have", "been", "from", "this", "that", "with", "they", "will", "each",
    "make", "like", "long", "look", "many", "some", "than", "them", "then", "very", "when",
    "what", "your", "into", "also", "more", "most", "only", "over", "such", "take", "does",
    "just", "should", "must", "could", "would", "about", "other", "which", "their", "there",
    "these", "those", "being", "before", "after", "first", "every", "using", "ensure", "while",
];

/// Negation words that indicate negative polarity.
const NEGATION_WORDS: &[&str] = &[
    "never", "don't", "dont", "avoid", "not", "no", "without", "cannot", "can't", "cant",
    "shouldn't", "shouldnt", "mustn't", "mustnt", "forbidden", "prohibit", "refuse",
    "skip", "do not", "does not",
];

/// Words indicating Must strength.
const MUST_WORDS: &[&str] = &[
    "must", "always", "never", "required", "mandatory", "shall", "critical",
    "essential", "absolutely",
];

/// Words indicating Should strength.
const SHOULD_WORDS: &[&str] = &[
    "should", "recommend", "strongly", "important", "prefer", "ideally",
    "best practice", "encouraged",
];

/// Words indicating May strength.
const MAY_WORDS: &[&str] = &[
    "may", "can", "could", "optional", "consider", "might", "sometimes",
    "possibly", "if needed",
];

/// Extract all directives from a document's skill blocks.
pub fn extract_directives(doc: &Document) -> Vec<Directive> {
    let mut directives = Vec::new();
    for block in &doc.blocks {
        extract_from_block(block, &None, &mut directives);
    }
    directives
}

fn extract_from_block(
    block: &Block,
    current_skill_name: &Option<String>,
    directives: &mut Vec<Directive>,
) {
    match &block.kind {
        BlockKind::SkillBlock {
            skill_type: SkillBlockType::Skill,
            attrs,
            children,
            ..
        } => {
            let skill_name = attrs
                .get("name")
                .map(|s| s.trim_matches('"').to_string())
                .unwrap_or_else(|| "(unnamed)".to_string());
            for child in children {
                extract_from_block(child, &Some(skill_name.clone()), directives);
            }
        }
        BlockKind::SkillBlock {
            skill_type,
            attrs,
            content,
            children,
            ..
        } => {
            let directive_type = match skill_type {
                SkillBlockType::Step => Some(DirectiveType::Step),
                SkillBlockType::Precondition => Some(DirectiveType::Precondition),
                SkillBlockType::RedFlag => Some(DirectiveType::RedFlag),
                SkillBlockType::Verify => Some(DirectiveType::Verify),
                SkillBlockType::OutputContract => Some(DirectiveType::OutputContract),
                _ => None,
            };

            if let Some(dt) = directive_type {
                let text = inlines_to_text(content, TextMode::Plain);

                // Also collect text from children (paragraphs inside the block)
                let mut full_text = text.clone();
                for child in children {
                    if let BlockKind::Paragraph { content } = &child.kind {
                        let child_text = inlines_to_text(content, TextMode::Plain);
                        if !child_text.is_empty() {
                            full_text.push(' ');
                            full_text.push_str(&child_text);
                        }
                    }
                }

                let order = attrs
                    .get("order")
                    .and_then(|s| s.parse::<u32>().ok());

                let keywords = extract_keywords(&full_text);
                let polarity = detect_polarity(&full_text);
                let strength = detect_strength(&full_text);

                let skill_name = current_skill_name
                    .clone()
                    .unwrap_or_else(|| "(unknown)".to_string());

                directives.push(Directive {
                    source_skill: skill_name,
                    block_type: dt,
                    order,
                    text: full_text,
                    keywords,
                    polarity,
                    strength,
                });
            }
        }
        BlockKind::Section { children, .. } => {
            for child in children {
                extract_from_block(child, current_skill_name, directives);
            }
        }
        _ => {}
    }
}

/// Simple suffix stripping for better keyword matching.
/// Removes common English suffixes (ing, ed, tion, ly, s) to normalize word forms.
fn normalize_word(word: &str) -> String {
    let w = word.to_lowercase();
    // Simple suffix stripping for common English word forms.
    // Only strip suffixes where the remaining stem is still >= 4 chars.
    for suffix in &["ting", "ning", "ding", "sing", "ring", "ling", "ping", "king",
                     "ment", "ness", "able", "ible", "ated", "ized",
                     "ing", "ied", "ies",
                     "ed", "ly"] {
        if w.len() > suffix.len() + 3 {
            if let Some(stem) = w.strip_suffix(suffix) {
                if stem.len() >= 4 {
                    return stem.to_string();
                }
            }
        }
    }
    // Strip trailing 's' for plurals (but keep stem >= 4 chars)
    if w.len() > 4 && w.ends_with('s') && !w.ends_with("ss") {
        return w[..w.len() - 1].to_string();
    }
    w
}

/// Extract content keywords from text: words > 3 chars, lowercased, normalized, not stopwords.
fn extract_keywords(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut seen = std::collections::HashSet::new();
    let mut keywords = Vec::new();

    for word in lower.split(|c: char| !c.is_alphanumeric() && c != '\'') {
        let word = word.trim_matches('\'');
        if word.len() > 3 && !STOPWORDS.contains(&word) && !NEGATION_WORDS.contains(&word) {
            let normalized = normalize_word(word);
            if normalized.len() > 2 && seen.insert(normalized.clone()) {
                keywords.push(normalized);
            }
        }
    }
    keywords
}

/// Detect whether the text has negative polarity based on negation words.
fn detect_polarity(text: &str) -> Polarity {
    let lower = text.to_lowercase();
    for neg in NEGATION_WORDS {
        if lower.contains(neg) {
            return Polarity::Negative;
        }
    }
    // RedFlag-style phrases
    if lower.contains("warn") || lower.contains("danger") || lower.contains("risk") {
        return Polarity::Negative;
    }
    Polarity::Positive
}

/// Detect the strength of a directive based on modal verbs.
fn detect_strength(text: &str) -> Strength {
    let lower = text.to_lowercase();

    // Check Must first (strongest)
    for word in MUST_WORDS {
        if lower.contains(word) {
            return Strength::Must;
        }
    }
    // Then Should
    for word in SHOULD_WORDS {
        if lower.contains(word) {
            return Strength::Should;
        }
    }
    // Then May
    for word in MAY_WORDS {
        if lower.contains(word) {
            return Strength::May;
        }
    }
    // Default to Should if no modal detected
    Strength::Should
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let kw = extract_keywords("Write tests before writing code implementation");
        // "write" and "writing" both normalize to "writ" (or similar)
        assert!(kw.iter().any(|k| k.starts_with("writ")));
        // "tests" normalizes to "test"
        assert!(kw.contains(&"test".to_string()));
        assert!(kw.contains(&"code".to_string()));
        // "before" is a stopword
        assert!(!kw.contains(&"before".to_string()));
    }

    #[test]
    fn test_detect_polarity_positive() {
        assert_eq!(detect_polarity("Write tests first"), Polarity::Positive);
        assert_eq!(detect_polarity("Run the test suite"), Polarity::Positive);
    }

    #[test]
    fn test_detect_polarity_negative() {
        assert_eq!(
            detect_polarity("Never deploy without tests"),
            Polarity::Negative
        );
        assert_eq!(
            detect_polarity("Don't skip code review"),
            Polarity::Negative
        );
        assert_eq!(
            detect_polarity("Avoid using global state"),
            Polarity::Negative
        );
    }

    #[test]
    fn test_detect_strength_must() {
        assert_eq!(detect_strength("You must write tests"), Strength::Must);
        assert_eq!(detect_strength("Always run linting"), Strength::Must);
        assert_eq!(detect_strength("Never skip reviews"), Strength::Must);
    }

    #[test]
    fn test_detect_strength_should() {
        assert_eq!(
            detect_strength("You should write documentation"),
            Strength::Should
        );
        assert_eq!(
            detect_strength("We recommend using TypeScript"),
            Strength::Should
        );
    }

    #[test]
    fn test_detect_strength_may() {
        assert_eq!(
            detect_strength("You may use optional logging"),
            Strength::May
        );
        assert_eq!(
            detect_strength("Consider adding metrics"),
            Strength::May
        );
    }

    #[test]
    fn test_extract_directives_from_skill() {
        let source = r#"
@skill[name="test-driven-dev", version="1.0"]
  @precondition
    Code changes require test coverage.
  @end

  @step[order=1]
    Always write tests before writing implementation code.
  @end

  @step[order=2]
    Run the test suite to verify all tests pass.
  @end

  @red_flag
    Never deploy code without running the full test suite.
  @end

  @verify
    All tests must pass before merge.
  @end
@end
"#;
        let doc = aif_parser::parse(source).unwrap();
        let directives = extract_directives(&doc);

        assert_eq!(directives.len(), 5);

        // Check skill name propagation
        for d in &directives {
            assert_eq!(d.source_skill, "test-driven-dev");
        }

        // Check types
        assert_eq!(directives[0].block_type, DirectiveType::Precondition);
        assert_eq!(directives[1].block_type, DirectiveType::Step);
        assert_eq!(directives[2].block_type, DirectiveType::Step);
        assert_eq!(directives[3].block_type, DirectiveType::RedFlag);
        assert_eq!(directives[4].block_type, DirectiveType::Verify);

        // Check order
        assert_eq!(directives[1].order, Some(1));
        assert_eq!(directives[2].order, Some(2));

        // Check polarity
        assert_eq!(directives[1].polarity, Polarity::Positive); // "Always write..."
        assert_eq!(directives[3].polarity, Polarity::Negative); // "Never deploy..."

        // Check strength
        assert_eq!(directives[1].strength, Strength::Must); // "Always"
        assert_eq!(directives[4].strength, Strength::Must); // "must pass"
    }
}
