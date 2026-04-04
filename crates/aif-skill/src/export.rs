use aif_core::ast::*;

fn inline_to_text(out: &mut String, inline: &Inline) {
    match inline {
        Inline::Text { text } => out.push_str(text),
        Inline::Emphasis { content } | Inline::Strong { content } | Inline::Footnote { content } => {
            for i in content { inline_to_text(out, i); }
        }
        Inline::InlineCode { code } => {
            out.push('`');
            out.push_str(code);
            out.push('`');
        }
        Inline::Link { text, url } => {
            out.push('[');
            for i in text { inline_to_text(out, i); }
            out.push_str("](");
            out.push_str(url);
            out.push(')');
        }
        Inline::Image { alt, src } => {
            out.push_str("![");
            out.push_str(alt);
            out.push_str("](");
            out.push_str(src);
            out.push(')');
        }
        Inline::Reference { target } => out.push_str(target),
        Inline::SoftBreak => out.push(' '),
        Inline::HardBreak => out.push('\n'),
    }
}

fn inlines_to_string(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        inline_to_text(&mut out, i);
    }
    out
}

fn skill_type_to_heading(st: &SkillBlockType) -> &'static str {
    match st {
        SkillBlockType::Skill => "Skill",
        SkillBlockType::Step => "Steps",
        SkillBlockType::Verify => "Verification",
        SkillBlockType::Precondition => "Prerequisites",
        SkillBlockType::OutputContract => "Expected Output",
        SkillBlockType::Decision => "Options",
        SkillBlockType::Tool => "Commands",
        SkillBlockType::Fallback => "Fallback",
        SkillBlockType::RedFlag => "Anti-patterns",
        SkillBlockType::Example => "Examples",
        SkillBlockType::Scenario => "Scenario",
        SkillBlockType::ArtifactSkill => "Artifact Skill",
        SkillBlockType::InputSchema => "Input Schema",
        SkillBlockType::Template => "Template",
        SkillBlockType::Binding => "Binding",
        SkillBlockType::Generate => "Generate",
        SkillBlockType::Export => "Export",
        SkillBlockType::ArtifactSkill => "Artifact Skill",
        SkillBlockType::InputSchema => "Input Schema",
        SkillBlockType::Template => "Template",
        SkillBlockType::Binding => "Binding",
        SkillBlockType::Generate => "Generate",
        SkillBlockType::Export => "Export",
    }
}

pub fn export_skill_md(block: &Block) -> String {
    let mut out = String::new();

    if let BlockKind::SkillBlock { skill_type: SkillBlockType::Skill, attrs, content, children, .. } = &block.kind {
        let name = attrs.get("name").unwrap_or("unnamed");
        let description = attrs.get("description").unwrap_or("");
        let version = attrs.get("version").unwrap_or("1");
        let hash = crate::hash::compute_skill_hash(block);

        // YAML frontmatter
        out.push_str("---\n");
        out.push_str(&format!("name: {}\n", name));
        if !description.is_empty() {
            out.push_str(&format!("description: {}\n", description));
        }
        out.push_str(&format!("version: {}\n", version));
        out.push_str(&format!("hash: {}\n", hash));
        out.push_str("---\n\n");

        // H1 title
        out.push_str(&format!("# {}\n\n", name));

        // Intro content
        let intro = inlines_to_string(content);
        if !intro.is_empty() {
            out.push_str(&intro);
            out.push_str("\n\n");
        }

        // Emit children, grouping steps under one "## Steps" heading.
        let mut steps_heading_emitted = false;
        let mut step_counter = 0u32;

        for child in children {
            if let BlockKind::SkillBlock { skill_type, attrs: child_attrs, content: child_content, .. } = &child.kind {
                if *skill_type == SkillBlockType::Step {
                    if !steps_heading_emitted {
                        out.push_str("## Steps\n\n");
                        steps_heading_emitted = true;
                    }
                    step_counter += 1;
                    let order = child_attrs.get("order")
                        .and_then(|o| o.parse::<u32>().ok())
                        .unwrap_or(step_counter);
                    let text = inlines_to_string(child_content);
                    out.push_str(&format!("{}. {}\n", order, text));
                } else {
                    let heading = skill_type_to_heading(skill_type);
                    out.push_str(&format!("## {}\n\n", heading));
                    let text = inlines_to_string(child_content);
                    if !text.is_empty() {
                        out.push_str(&text);
                        out.push_str("\n\n");
                    }
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::span::Span;

    fn make_step(order: u32, text: &str) -> Block {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("order".into(), order.to_string());
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Step,
                attrs,
                title: None,
                content: vec![Inline::Text { text: text.into() }],
                children: vec![],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn export_minimal_skill() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "test-skill".into());
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![Inline::Text { text: "A test skill.".into() }],
                children: vec![],
            },
            span: Span::empty(),
        };
        let md = export_skill_md(&skill);
        assert!(md.contains("---"));
        assert!(md.contains("name: test-skill"));
        assert!(md.contains("# test-skill"));
        assert!(md.contains("A test skill."));
    }

    #[test]
    fn export_skill_with_steps() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "debugging".into());
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children: vec![
                    make_step(1, "Reproduce the bug"),
                    make_step(2, "Find root cause"),
                ],
            },
            span: Span::empty(),
        };
        let md = export_skill_md(&skill);
        assert!(md.contains("## Steps"));
        assert!(md.contains("1. Reproduce the bug"));
        assert!(md.contains("2. Find root cause"));
    }

    #[test]
    fn export_skill_with_precondition_and_verify() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "test".into());
        let skill = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children: vec![
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Precondition,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text { text: "Have a bug report.".into() }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                    Block {
                        kind: BlockKind::SkillBlock {
                            skill_type: SkillBlockType::Verify,
                            attrs: Attrs::new(),
                            title: None,
                            content: vec![Inline::Text { text: "All tests pass.".into() }],
                            children: vec![],
                        },
                        span: Span::empty(),
                    },
                ],
            },
            span: Span::empty(),
        };
        let md = export_skill_md(&skill);
        assert!(md.contains("## Prerequisites"));
        assert!(md.contains("Have a bug report."));
        assert!(md.contains("## Verification"));
        assert!(md.contains("All tests pass."));
    }
}
