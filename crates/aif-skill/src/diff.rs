use aif_core::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub kind: ChangeKind,
    pub block_type: SkillBlockType,
    pub description: String,
}

/// Compare two skill blocks and return a list of changes.
pub fn diff_skills(old: &Block, new: &Block) -> Vec<Change> {
    let old_children = skill_children(old);
    let new_children = skill_children(new);

    let mut changes = Vec::new();

    let old_indexed = index_children(&old_children);
    let new_indexed = index_children(&new_children);

    // Find removed and modified
    for (key, old_block) in &old_indexed {
        match new_indexed.get(key) {
            None => {
                changes.push(Change {
                    kind: ChangeKind::Removed,
                    block_type: child_skill_type(old_block),
                    description: format!("Removed {:?} {}", child_skill_type(old_block), key),
                });
            }
            Some(new_block) => {
                if !blocks_equal(old_block, new_block) {
                    changes.push(Change {
                        kind: ChangeKind::Modified,
                        block_type: child_skill_type(old_block),
                        description: format!("Modified {:?} {}", child_skill_type(old_block), key),
                    });
                }
            }
        }
    }

    // Find added
    for (key, new_block) in &new_indexed {
        if !old_indexed.contains_key(key) {
            changes.push(Change {
                kind: ChangeKind::Added,
                block_type: child_skill_type(new_block),
                description: format!("Added {:?} {}", child_skill_type(new_block), key),
            });
        }
    }

    changes
}

fn skill_children(block: &Block) -> Vec<&Block> {
    match &block.kind {
        BlockKind::SkillBlock { children, .. } => children.iter().collect(),
        _ => vec![],
    }
}

fn child_skill_type(block: &Block) -> SkillBlockType {
    match &block.kind {
        BlockKind::SkillBlock { skill_type, .. } => skill_type.clone(),
        _ => SkillBlockType::Step, // fallback
    }
}

fn index_children<'a>(children: &[&'a Block]) -> std::collections::BTreeMap<String, &'a Block> {
    let mut map = std::collections::BTreeMap::new();
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for block in children {
        let (type_name, order) = match &block.kind {
            BlockKind::SkillBlock {
                skill_type, attrs, ..
            } => {
                let name = format!("{:?}", skill_type);
                let order = attrs.get("order").map(|s| s.to_string());
                (name, order)
            }
            _ => continue,
        };

        let key = if let Some(ord) = order {
            format!("{}/{}", type_name, ord)
        } else {
            let count = type_counts.entry(type_name.clone()).or_insert(0);
            *count += 1;
            format!("{}/{}", type_name, count)
        };

        map.insert(key, *block);
    }
    map
}

fn blocks_equal(a: &Block, b: &Block) -> bool {
    match (&a.kind, &b.kind) {
        (
            BlockKind::SkillBlock {
                skill_type: st1,
                attrs: a1,
                content: c1,
                children: ch1,
                title: t1,
            },
            BlockKind::SkillBlock {
                skill_type: st2,
                attrs: a2,
                content: c2,
                children: ch2,
                title: t2,
            },
        ) => {
            st1 == st2
                && a1 == a2
                && c1 == c2
                && t1 == t2
                && ch1.len() == ch2.len()
                && ch1
                    .iter()
                    .zip(ch2.iter())
                    .all(|(x, y)| blocks_equal(x, y))
        }
        _ => a.kind == b.kind,
    }
}
