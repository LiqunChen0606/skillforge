use aif_core::ast::*;

#[derive(Debug, PartialEq)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}

pub fn validate_skill(block: &Block) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let BlockKind::SkillBlock { skill_type, attrs, children, .. } = &block.kind {
        if !matches!(skill_type, SkillBlockType::Skill) {
            errors.push(ValidationError::new("Top-level block must be @skill"));
            return errors;
        }

        if attrs.get("name").is_none() {
            errors.push(ValidationError::new("@skill must have a 'name' attribute"));
        }

        let mut step_orders: Vec<u32> = Vec::new();
        for child in children {
            if let BlockKind::SkillBlock { skill_type: SkillBlockType::Step, attrs, .. } = &child.kind {
                if let Some(order_str) = attrs.get("order") {
                    if let Ok(order) = order_str.parse::<u32>() {
                        step_orders.push(order);
                    }
                }
            }
        }

        if !step_orders.is_empty() {
            let mut sorted = step_orders.clone();
            sorted.sort();
            let mut seen = std::collections::HashSet::new();
            for &o in &sorted {
                if !seen.insert(o) {
                    errors.push(ValidationError::new(format!("Duplicate step order: {}", o)));
                    break;
                }
            }

            if seen.len() == sorted.len() {
                for (i, &o) in sorted.iter().enumerate() {
                    if o != (i as u32) + 1 {
                        errors.push(ValidationError::new(
                            format!("Step order values must be contiguous starting from 1, found gap at {}", o),
                        ));
                        break;
                    }
                }
            }
        }
    } else {
        errors.push(ValidationError::new("Expected SkillBlock"));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    fn make_skill(name: Option<&str>, children: Vec<Block>) -> Block {
        let mut attrs = Attrs::new();
        if let Some(n) = name {
            attrs.pairs.insert("name".into(), n.into());
        }
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children,
            },
            span: Span::empty(),
        }
    }

    fn make_step(order: &str) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("order".into(), order.into());
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text { text: format!("Step {}", order) }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn valid_skill_passes() {
        let skill = make_skill(Some("debugging"), vec![make_step("1"), make_step("2")]);
        let errors = validate_skill(&skill);
        assert!(errors.is_empty());
    }

    #[test]
    fn skill_missing_name_fails() {
        let skill = make_skill(None, vec![]);
        let errors = validate_skill(&skill);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("name"));
    }

    #[test]
    fn duplicate_step_order_fails() {
        let skill = make_skill(Some("test"), vec![make_step("1"), make_step("1")]);
        let errors = validate_skill(&skill);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("order"));
    }

    #[test]
    fn non_contiguous_step_order_fails() {
        let skill = make_skill(Some("test"), vec![make_step("1"), make_step("3")]);
        let errors = validate_skill(&skill);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("contiguous"));
    }
}
