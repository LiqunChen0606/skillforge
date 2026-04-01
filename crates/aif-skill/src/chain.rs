use crate::registry::Registry;
use crate::version::{Semver, VersionConstraint};
use aif_core::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// A single dependency declaration parsed from the `requires` attribute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub constraint: VersionConstraint,
}

/// Result of dependency resolution — topologically sorted execution order
#[derive(Debug)]
pub struct ResolutionResult {
    /// Skill names in execution order (dependencies before dependents)
    pub order: Vec<String>,
    /// Resolved version for each skill
    pub resolved: BTreeMap<String, Semver>,
}

/// Errors during chain resolution
#[derive(Debug)]
pub enum ChainError {
    CyclicDependency(Vec<String>),
    MissingDependency { skill: String, requires: String },
    VersionConflict { skill: String, required: VersionConstraint, available: Semver },
    SkillNotFound(String),
    InvalidSkillBlock,
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainError::CyclicDependency(cycle) => {
                write!(f, "Cyclic dependency detected: {}", cycle.join(" -> "))
            }
            ChainError::MissingDependency { skill, requires } => {
                write!(f, "Skill '{}' requires '{}' which is not in the registry", skill, requires)
            }
            ChainError::VersionConflict { skill, required, available } => {
                write!(
                    f,
                    "Skill '{}' requires version {} but only {} is available",
                    skill, required, available
                )
            }
            ChainError::SkillNotFound(name) => write!(f, "Skill '{}' not found", name),
            ChainError::InvalidSkillBlock => write!(f, "Invalid skill block"),
        }
    }
}

impl std::error::Error for ChainError {}

/// Parse a dependency specifier like "tdd:>=1.0.0"
fn parse_dep_specifier(s: &str) -> Option<SkillDependency> {
    let s = s.trim();
    if let Some(colon_pos) = s.find(':') {
        let name = s[..colon_pos].trim().to_string();
        let constraint_str = &s[colon_pos + 1..];
        let constraint = VersionConstraint::parse(constraint_str)?;
        Some(SkillDependency { name, constraint })
    } else {
        // Just a name with no constraint means "any version"
        Some(SkillDependency {
            name: s.to_string(),
            constraint: VersionConstraint::Any,
        })
    }
}

/// Parse the `requires` attribute from a skill block into a list of dependencies
pub fn parse_requires(attrs: &Attrs) -> Vec<SkillDependency> {
    let Some(requires_str) = attrs.get("requires") else {
        return vec![];
    };

    requires_str
        .split(',')
        .filter_map(parse_dep_specifier)
        .collect()
}

/// Extract skill name and dependencies from a skill block
pub fn extract_skill_info(block: &Block) -> Option<(String, Vec<SkillDependency>)> {
    if let BlockKind::SkillBlock {
        skill_type: SkillBlockType::Skill,
        attrs,
        ..
    } = &block.kind
    {
        let name = attrs.get("name")?.to_string();
        let deps = parse_requires(attrs);
        Some((name, deps))
    } else {
        None
    }
}

/// Build a dependency graph and resolve execution order using Kahn's algorithm.
///
/// Returns skills in topological order (dependencies first).
/// The `available_skills` map provides name -> (version, dependencies) for all known skills.
pub fn resolve_chain_from_graph(
    root: &str,
    available_skills: &BTreeMap<String, (Semver, Vec<SkillDependency>)>,
) -> Result<ResolutionResult, ChainError> {
    // Build the full dependency graph starting from root
    let mut graph: HashMap<String, Vec<String>> = HashMap::new(); // skill -> its dependencies
    let mut to_visit = VecDeque::new();
    let mut visited = std::collections::HashSet::new();

    to_visit.push_back(root.to_string());

    while let Some(skill_name) = to_visit.pop_front() {
        if !visited.insert(skill_name.clone()) {
            continue;
        }

        let (version, deps) = available_skills
            .get(&skill_name)
            .ok_or_else(|| ChainError::SkillNotFound(skill_name.clone()))?;

        let mut dep_names = Vec::new();
        for dep in deps {
            // Check that the dependency exists
            let (dep_version, _) = available_skills
                .get(&dep.name)
                .ok_or_else(|| ChainError::MissingDependency {
                    skill: skill_name.clone(),
                    requires: dep.name.clone(),
                })?;

            // Check version constraint
            if !dep.constraint.satisfies(dep_version) {
                return Err(ChainError::VersionConflict {
                    skill: skill_name.clone(),
                    required: dep.constraint.clone(),
                    available: *dep_version,
                });
            }

            dep_names.push(dep.name.clone());
            to_visit.push_back(dep.name.clone());
        }

        let _ = version; // used for version conflict check above
        graph.insert(skill_name, dep_names);
    }

    // Kahn's algorithm for topological sort
    // in_degree[node] = number of prerequisites that must come before node
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for name in graph.keys() {
        in_degree.entry(name.clone()).or_insert(0);
    }

    // For each skill, its dependencies are prerequisites. So skill's in_degree += number of deps.
    // And we need reverse adjacency: dep -> [skills that depend on dep]
    let mut reverse_graph: HashMap<String, Vec<String>> = HashMap::new();
    for (skill, deps) in &graph {
        in_degree.insert(skill.clone(), deps.len());
        for dep in deps {
            reverse_graph
                .entry(dep.clone())
                .or_default()
                .push(skill.clone());
        }
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    for (name, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(name.clone());
        }
    }

    let mut order = Vec::new();
    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        if let Some(dependents) = reverse_graph.get(&node) {
            for dependent in dependents {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
    }

    if order.len() != graph.len() {
        // Cycle detected — find the cycle
        let remaining: Vec<String> = graph
            .keys()
            .filter(|k| !order.contains(k))
            .cloned()
            .collect();
        return Err(ChainError::CyclicDependency(remaining));
    }

    // Build resolved versions
    let mut resolved = BTreeMap::new();
    for name in &order {
        if let Some((version, _)) = available_skills.get(name) {
            resolved.insert(name.clone(), *version);
        }
    }

    Ok(ResolutionResult { order, resolved })
}

/// Resolve a skill chain starting from a root skill, using the registry for lookups.
///
/// Loads each skill file from the registry to extract its `requires` attribute,
/// building the full dependency graph for transitive resolution.
pub fn resolve_chain(
    root_skill: &str,
    registry: &Registry,
) -> Result<ResolutionResult, ChainError> {
    let mut available_skills: BTreeMap<String, (Semver, Vec<SkillDependency>)> = BTreeMap::new();

    for entry in registry.list() {
        let version = Semver::parse(&entry.version).ok_or_else(|| {
            ChainError::SkillNotFound(format!(
                "{} (invalid version '{}' in registry)",
                entry.name, entry.version
            ))
        })?;

        // Load the skill file and extract dependencies
        let deps = if std::path::Path::new(&entry.path).exists() {
            match std::fs::read_to_string(&entry.path) {
                Ok(source) => {
                    if let Ok(doc) = serde_json::from_str::<Document>(&source) {
                        extract_deps_from_doc(&doc)
                    } else {
                        vec![]
                    }
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        available_skills.insert(entry.name.clone(), (version, deps));
    }

    if !available_skills.contains_key(root_skill) {
        return Err(ChainError::SkillNotFound(root_skill.to_string()));
    }

    resolve_chain_from_graph(root_skill, &available_skills)
}

/// Extract dependencies from all skill blocks in a document.
fn extract_deps_from_doc(doc: &Document) -> Vec<SkillDependency> {
    for block in &doc.blocks {
        if let Some((_, deps)) = extract_skill_info(block) {
            return deps;
        }
    }
    vec![]
}

/// Compose a resolved chain into a single document by loading skills from the registry.
pub fn compose_chain(
    order: &[String],
    registry: &Registry,
) -> Result<Document, ChainError> {
    let mut doc = Document::new();
    doc.metadata.insert("chain_root".to_string(), order.first().cloned().unwrap_or_default());
    doc.metadata.insert("chain_length".to_string(), order.len().to_string());

    for name in order {
        let entry = registry
            .lookup(name)
            .ok_or_else(|| ChainError::SkillNotFound(name.clone()))?;

        // Read and parse the skill file
        let source = std::fs::read_to_string(&entry.path).map_err(|_| {
            ChainError::SkillNotFound(format!("{} (file not found: {})", name, entry.path))
        })?;
        let skill_doc: Document = serde_json::from_str(&source).map_err(|e| {
            ChainError::SkillNotFound(format!(
                "{} (failed to parse {}: {})", name, entry.path, e
            ))
        })?;

        for block in skill_doc.blocks {
            doc.blocks.push(block);
        }
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_dependency() {
        let dep = parse_dep_specifier("tdd:>=1.0.0").unwrap();
        assert_eq!(dep.name, "tdd");
        assert_eq!(dep.constraint, VersionConstraint::MinVersion(Semver { major: 1, minor: 0, patch: 0 }));
    }

    #[test]
    fn parse_exact_version_dependency() {
        let dep = parse_dep_specifier("verification:=2.1.0").unwrap();
        assert_eq!(dep.name, "verification");
        assert_eq!(dep.constraint, VersionConstraint::Exact(Semver { major: 2, minor: 1, patch: 0 }));
    }

    #[test]
    fn parse_any_version_dependency() {
        let dep = parse_dep_specifier("logging:*").unwrap();
        assert_eq!(dep.name, "logging");
        assert_eq!(dep.constraint, VersionConstraint::Any);
    }

    #[test]
    fn parse_range_dependency() {
        let dep = parse_dep_specifier("tdd:>=1.0.0+<2.0.0").unwrap();
        assert_eq!(dep.name, "tdd");
        assert_eq!(
            dep.constraint,
            VersionConstraint::Range {
                min: Semver { major: 1, minor: 0, patch: 0 },
                max: Semver { major: 2, minor: 0, patch: 0 },
            }
        );
    }

    #[test]
    fn parse_name_only_dependency() {
        let dep = parse_dep_specifier("utils").unwrap();
        assert_eq!(dep.name, "utils");
        assert_eq!(dep.constraint, VersionConstraint::Any);
    }

    #[test]
    fn parse_requires_attribute() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("requires".into(), "tdd:>=1.0.0,verification:>=0.5.0".into());
        let deps = parse_requires(&attrs);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "tdd");
        assert_eq!(deps[1].name, "verification");
    }

    #[test]
    fn parse_requires_empty() {
        let attrs = Attrs::new();
        let deps = parse_requires(&attrs);
        assert!(deps.is_empty());
    }

    #[test]
    fn extract_skill_info_from_block() {
        let mut attrs = Attrs::new();
        attrs.pairs.insert("name".into(), "debugging".into());
        attrs.pairs.insert("version".into(), "1.2.0".into());
        attrs.pairs.insert("requires".into(), "tdd:>=1.0.0".into());

        let block = Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs,
                title: None,
                content: vec![],
                children: vec![],
            },
            span: aif_core::span::Span::empty(),
        };

        let (name, deps) = extract_skill_info(&block).unwrap();
        assert_eq!(name, "debugging");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "tdd");
    }

    #[test]
    fn topological_sort_simple_chain() {
        // A depends on B, B depends on C
        let mut skills = BTreeMap::new();
        let v1 = Semver { major: 1, minor: 0, patch: 0 };

        skills.insert("C".to_string(), (v1, vec![]));
        skills.insert("B".to_string(), (v1, vec![
            SkillDependency { name: "C".to_string(), constraint: VersionConstraint::Any },
        ]));
        skills.insert("A".to_string(), (v1, vec![
            SkillDependency { name: "B".to_string(), constraint: VersionConstraint::Any },
        ]));

        let result = resolve_chain_from_graph("A", &skills).unwrap();
        // C must come before B, B before A
        let c_pos = result.order.iter().position(|s| s == "C").unwrap();
        let b_pos = result.order.iter().position(|s| s == "B").unwrap();
        let a_pos = result.order.iter().position(|s| s == "A").unwrap();
        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
        assert_eq!(result.order.len(), 3);
    }

    #[test]
    fn topological_sort_diamond() {
        // A depends on B and C, both B and C depend on D
        let mut skills = BTreeMap::new();
        let v1 = Semver { major: 1, minor: 0, patch: 0 };

        skills.insert("D".to_string(), (v1, vec![]));
        skills.insert("B".to_string(), (v1, vec![
            SkillDependency { name: "D".to_string(), constraint: VersionConstraint::Any },
        ]));
        skills.insert("C".to_string(), (v1, vec![
            SkillDependency { name: "D".to_string(), constraint: VersionConstraint::Any },
        ]));
        skills.insert("A".to_string(), (v1, vec![
            SkillDependency { name: "B".to_string(), constraint: VersionConstraint::Any },
            SkillDependency { name: "C".to_string(), constraint: VersionConstraint::Any },
        ]));

        let result = resolve_chain_from_graph("A", &skills).unwrap();
        let d_pos = result.order.iter().position(|s| s == "D").unwrap();
        let b_pos = result.order.iter().position(|s| s == "B").unwrap();
        let c_pos = result.order.iter().position(|s| s == "C").unwrap();
        let a_pos = result.order.iter().position(|s| s == "A").unwrap();
        assert!(d_pos < b_pos);
        assert!(d_pos < c_pos);
        assert!(b_pos < a_pos);
        assert!(c_pos < a_pos);
    }

    #[test]
    fn cyclic_dependency_detected() {
        let mut skills = BTreeMap::new();
        let v1 = Semver { major: 1, minor: 0, patch: 0 };

        skills.insert("A".to_string(), (v1, vec![
            SkillDependency { name: "B".to_string(), constraint: VersionConstraint::Any },
        ]));
        skills.insert("B".to_string(), (v1, vec![
            SkillDependency { name: "A".to_string(), constraint: VersionConstraint::Any },
        ]));

        let result = resolve_chain_from_graph("A", &skills);
        assert!(matches!(result, Err(ChainError::CyclicDependency(_))));
    }

    #[test]
    fn missing_dependency_detected() {
        let mut skills = BTreeMap::new();
        let v1 = Semver { major: 1, minor: 0, patch: 0 };

        skills.insert("A".to_string(), (v1, vec![
            SkillDependency { name: "nonexistent".to_string(), constraint: VersionConstraint::Any },
        ]));

        let result = resolve_chain_from_graph("A", &skills);
        assert!(matches!(result, Err(ChainError::MissingDependency { .. })));
    }

    #[test]
    fn version_conflict_detected() {
        let mut skills = BTreeMap::new();
        let v1 = Semver { major: 0, minor: 5, patch: 0 };
        let v2 = Semver { major: 1, minor: 0, patch: 0 };

        skills.insert("dep".to_string(), (v1, vec![]));
        skills.insert("A".to_string(), (v2, vec![
            SkillDependency {
                name: "dep".to_string(),
                constraint: VersionConstraint::MinVersion(Semver { major: 1, minor: 0, patch: 0 }),
            },
        ]));

        let result = resolve_chain_from_graph("A", &skills);
        assert!(matches!(result, Err(ChainError::VersionConflict { .. })));
    }

    #[test]
    fn no_dependencies_resolves_single() {
        let mut skills = BTreeMap::new();
        let v1 = Semver { major: 1, minor: 0, patch: 0 };
        skills.insert("solo".to_string(), (v1, vec![]));

        let result = resolve_chain_from_graph("solo", &skills).unwrap();
        assert_eq!(result.order, vec!["solo"]);
    }

    #[test]
    fn version_constraint_satisfies() {
        let v1 = Semver { major: 1, minor: 0, patch: 0 };
        let v2 = Semver { major: 2, minor: 0, patch: 0 };
        let v05 = Semver { major: 0, minor: 5, patch: 0 };

        assert!(VersionConstraint::Any.satisfies(&v1));
        assert!(VersionConstraint::Exact(v1).satisfies(&v1));
        assert!(!VersionConstraint::Exact(v1).satisfies(&v2));
        assert!(VersionConstraint::MinVersion(v1).satisfies(&v1));
        assert!(VersionConstraint::MinVersion(v1).satisfies(&v2));
        assert!(!VersionConstraint::MinVersion(v1).satisfies(&v05));

        let range = VersionConstraint::Range { min: v1, max: v2 };
        assert!(range.satisfies(&v1));
        assert!(!range.satisfies(&v2));
        assert!(!range.satisfies(&v05));
    }

    #[test]
    fn inverted_version_range_rejected() {
        // >=2.0.0+<1.0.0 is an inverted range and should fail to parse
        let result = VersionConstraint::parse(">=2.0.0+<1.0.0");
        assert!(result.is_none(), "Inverted version range should be rejected");
    }

    #[test]
    fn equal_version_range_rejected() {
        // >=1.0.0+<1.0.0 is an empty range and should fail to parse
        let result = VersionConstraint::parse(">=1.0.0+<1.0.0");
        assert!(result.is_none(), "Equal min/max range should be rejected");
    }

    #[test]
    fn resolve_chain_with_registry_and_files() {
        use std::io::Write;

        // Create temp dir with skill files
        let dir = std::env::temp_dir().join("aif_chain_test");
        let _ = std::fs::create_dir_all(&dir);

        // Create a skill doc with requires attribute
        let skill_a = aif_core::ast::Document {
            metadata: std::collections::BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Skill,
                    attrs: {
                        let mut a = aif_core::ast::Attrs::new();
                        a.pairs.insert("name".into(), "skill-a".into());
                        a.pairs.insert("version".into(), "1.0.0".into());
                        a.pairs.insert("requires".into(), "skill-b".into());
                        a
                    },
                    title: None,
                    content: vec![],
                    children: vec![],
                },
                span: aif_core::span::Span::empty(),
            }],
        };

        let skill_b = aif_core::ast::Document {
            metadata: std::collections::BTreeMap::new(),
            blocks: vec![Block {
                kind: BlockKind::SkillBlock {
                    skill_type: SkillBlockType::Skill,
                    attrs: {
                        let mut a = aif_core::ast::Attrs::new();
                        a.pairs.insert("name".into(), "skill-b".into());
                        a.pairs.insert("version".into(), "1.0.0".into());
                        a
                    },
                    title: None,
                    content: vec![],
                    children: vec![],
                },
                span: aif_core::span::Span::empty(),
            }],
        };

        let path_a = dir.join("skill-a.json");
        let path_b = dir.join("skill-b.json");
        let mut f = std::fs::File::create(&path_a).unwrap();
        f.write_all(serde_json::to_string(&skill_a).unwrap().as_bytes()).unwrap();
        let mut f = std::fs::File::create(&path_b).unwrap();
        f.write_all(serde_json::to_string(&skill_b).unwrap().as_bytes()).unwrap();

        // Build registry
        let mut registry = Registry::new(dir.join("registry.json"));
        registry.register("skill-a", "1.0.0", "sha256:aaa", path_a.to_str().unwrap());
        registry.register("skill-b", "1.0.0", "sha256:bbb", path_b.to_str().unwrap());

        // Resolve chain — should find skill-b as dependency of skill-a
        let result = resolve_chain("skill-a", &registry).unwrap();
        assert_eq!(result.order.len(), 2);
        let b_pos = result.order.iter().position(|s| s == "skill-b").unwrap();
        let a_pos = result.order.iter().position(|s| s == "skill-a").unwrap();
        assert!(b_pos < a_pos, "skill-b should come before skill-a");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
