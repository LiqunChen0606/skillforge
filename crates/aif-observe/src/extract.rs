use aif_core::ast::{Block, BlockKind, Document, SkillBlockType};
use aif_core::text::{inlines_to_text, TextMode};

use crate::types::ObservableBlock;

/// Find the first top-level `@skill` block in a document.
pub fn find_skill_block(doc: &Document) -> Option<&Block> {
    doc.blocks.iter().find(|b| {
        matches!(
            &b.kind,
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                ..
            }
        )
    })
}

/// Observable block types we care about for observability.
fn is_observable(skill_type: &SkillBlockType) -> bool {
    matches!(
        skill_type,
        SkillBlockType::Step
            | SkillBlockType::Verify
            | SkillBlockType::RedFlag
            | SkillBlockType::Precondition
            | SkillBlockType::OutputContract
    )
}

/// Extract all observable blocks from a skill block's children.
pub fn extract_observables(skill_block: &Block) -> Vec<ObservableBlock> {
    let mut result = Vec::new();

    if let BlockKind::SkillBlock { children, .. } = &skill_block.kind {
        for child in children {
            if let BlockKind::SkillBlock {
                skill_type,
                attrs,
                content,
                ..
            } = &child.kind
            {
                if !is_observable(skill_type) {
                    continue;
                }

                let full_content = inlines_to_text(content, TextMode::Plain);
                let content_snippet = if full_content.len() > 100 {
                    format!("{}...", &full_content[..100])
                } else {
                    full_content.clone()
                };

                let order = attrs
                    .get("order")
                    .and_then(|v| v.parse::<u32>().ok());

                result.push(ObservableBlock {
                    block_type: skill_type.clone(),
                    block_id: attrs.id.clone(),
                    order,
                    content_snippet,
                    full_content,
                });
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_skill_aif() {
        let source = r#"
#title: Test Skill

@skill[name="test-skill", version="1.0"]

  @precondition
    A codebase is available for review.
  @end

  @step[order=1]
    Read the code carefully and understand the intent.
  @end

  @step[order=2]
    Check for correctness, security, and performance issues.
  @end

  @verify
    All blocking issues have suggested fixes.
  @end

  @red_flag
    Approving without running tests.
  @end

  @output_contract
    A structured list of findings categorized as blocking or suggestion.
  @end

  @example
    This is an example block and should NOT be extracted.
  @end

@end
"#;
        let doc = aif_parser::parse(source).expect("parse failed");
        let skill = find_skill_block(&doc).expect("no skill block found");
        let observables = extract_observables(skill);

        // Should have: 1 precondition + 2 steps + 1 verify + 1 red_flag + 1 output_contract = 6
        assert_eq!(observables.len(), 6);

        // Check precondition
        assert_eq!(observables[0].block_type, SkillBlockType::Precondition);
        assert!(observables[0].full_content.contains("codebase"));

        // Check steps have order
        assert_eq!(observables[1].block_type, SkillBlockType::Step);
        assert_eq!(observables[1].order, Some(1));

        assert_eq!(observables[2].block_type, SkillBlockType::Step);
        assert_eq!(observables[2].order, Some(2));

        // Check verify
        assert_eq!(observables[3].block_type, SkillBlockType::Verify);

        // Check red_flag
        assert_eq!(observables[4].block_type, SkillBlockType::RedFlag);
        assert!(observables[4].full_content.contains("tests"));

        // Check output_contract
        assert_eq!(observables[5].block_type, SkillBlockType::OutputContract);

        // Example should NOT be in the list
        assert!(!observables.iter().any(|o| o.block_type == SkillBlockType::Example));
    }

    #[test]
    fn content_snippet_truncation() {
        let source = r#"
@skill[name="verbose", version="1.0"]
  @step[order=1]
    This is a very long step description that exceeds one hundred characters and should be truncated in the content snippet field but preserved in full content.
  @end
@end
"#;
        let doc = aif_parser::parse(source).expect("parse failed");
        let skill = find_skill_block(&doc).expect("no skill block");
        let observables = extract_observables(skill);

        assert_eq!(observables.len(), 1);
        assert!(observables[0].content_snippet.ends_with("..."));
        assert!(observables[0].content_snippet.len() <= 103); // 100 + "..."
        assert!(observables[0].full_content.len() > 100);
    }

    #[test]
    fn no_skill_block_returns_none() {
        let source = r#"
#title: Not a Skill
Just a regular paragraph.
"#;
        let doc = aif_parser::parse(source).expect("parse failed");
        assert!(find_skill_block(&doc).is_none());
    }
}
