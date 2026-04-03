//! Document-level semantic linting.
//!
//! Checks structural quality of any AIF document (not just skills).
//! Covers: broken references, orphaned figures, claims without evidence,
//! duplicate IDs, empty sections, and missing metadata.

use crate::ast::*;
use crate::chunk::*;
use std::collections::{BTreeMap, BTreeSet};

/// Categories of document-level lint checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocLintCheck {
    /// Every `@claim` should have a corresponding `@evidence` block.
    ClaimsWithoutEvidence,
    /// Every `Reference { target }` inline must point to an existing block ID.
    BrokenReferences,
    /// `refs` attribute values (e.g. `refs=e1,e2`) must point to existing block IDs.
    BrokenEvidenceLinks,
    /// Figure/Audio/Video blocks should have non-empty `src`.
    OrphanedMedia,
    /// No two blocks in the document should share the same `id`.
    DuplicateIds,
    /// Sections should contain at least one child block.
    EmptySections,
    /// Document should have `title` metadata.
    MissingMetadata,
    /// Footnotes should not be empty.
    EmptyFootnotes,
    /// Tables should have at least one header and one data row.
    MalformedTables,
    /// Terms used in @claim blocks that are not defined in any @definition block.
    UndefinedTerms,
    /// Chunk has no incoming or outgoing links (isolated node).
    OrphanedChunks,
    /// Sequential chunks from the same document lack a Continuation link.
    MissingContinuation,
    /// Circular Dependency or ParentContext links detected.
    DependencyCycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocLintSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct DocLintResult {
    pub check: DocLintCheck,
    pub passed: bool,
    pub severity: DocLintSeverity,
    pub message: String,
    /// Optional block ID where the issue was found.
    pub block_id: Option<String>,
}

impl DocLintResult {
    fn pass(check: DocLintCheck) -> Self {
        Self {
            check,
            passed: true,
            severity: DocLintSeverity::Warning,
            message: String::new(),
            block_id: None,
        }
    }

    fn fail(
        check: DocLintCheck,
        severity: DocLintSeverity,
        message: impl Into<String>,
        block_id: Option<String>,
    ) -> Self {
        Self {
            check,
            passed: false,
            severity,
            message: message.into(),
            block_id,
        }
    }
}

/// Run all document-level lint checks. Returns one or more results per check category.
pub fn lint_document(doc: &Document) -> Vec<DocLintResult> {
    let mut results = Vec::new();

    let mut all_ids: Vec<String> = Vec::new();
    let mut claim_ids: Vec<String> = Vec::new();
    let mut evidence_ids: Vec<String> = Vec::new();
    let mut reference_targets: Vec<String> = Vec::new();
    let mut media_issues: Vec<(String, Option<String>)> = Vec::new();
    let mut empty_sections: Vec<Option<String>> = Vec::new();
    let mut empty_footnotes: usize = 0;
    let mut malformed_tables: Vec<Option<String>> = Vec::new();
    // Evidence linkage: (source_block_id, target_ref) from `refs` attrs
    let mut evidence_links: Vec<(Option<String>, String)> = Vec::new();

    collect_block_info(
        &doc.blocks,
        &mut all_ids,
        &mut claim_ids,
        &mut evidence_ids,
        &mut reference_targets,
        &mut media_issues,
        &mut empty_sections,
        &mut empty_footnotes,
        &mut malformed_tables,
        &mut evidence_links,
        0,
    );

    // 1. Claims without evidence
    let evidence_set: BTreeSet<&str> = evidence_ids.iter().map(|s| s.as_str()).collect();
    if claim_ids.is_empty() {
        results.push(DocLintResult::pass(DocLintCheck::ClaimsWithoutEvidence));
    } else if evidence_set.is_empty() {
        for claim_id in &claim_ids {
            results.push(DocLintResult::fail(
                DocLintCheck::ClaimsWithoutEvidence,
                DocLintSeverity::Warning,
                format!(
                    "Claim '{}' has no corresponding @evidence block in document",
                    claim_id
                ),
                Some(claim_id.clone()),
            ));
        }
    } else {
        results.push(DocLintResult::pass(DocLintCheck::ClaimsWithoutEvidence));
    }

    // 2. Broken references
    let id_set: BTreeSet<&str> = all_ids.iter().map(|s| s.as_str()).collect();
    let mut broken_found = false;
    for target in &reference_targets {
        if !id_set.contains(target.as_str()) {
            results.push(DocLintResult::fail(
                DocLintCheck::BrokenReferences,
                DocLintSeverity::Error,
                format!(
                    "Reference to '{}' but no block with that ID exists",
                    target
                ),
                None,
            ));
            broken_found = true;
        }
    }
    if !broken_found {
        results.push(DocLintResult::pass(DocLintCheck::BrokenReferences));
    }

    // 3. Broken evidence links (refs attribute)
    let mut broken_links = false;
    for (source_id, target) in &evidence_links {
        if !id_set.contains(target.as_str()) {
            results.push(DocLintResult::fail(
                DocLintCheck::BrokenEvidenceLinks,
                DocLintSeverity::Error,
                format!(
                    "refs attribute points to '{}' but no block with that ID exists",
                    target
                ),
                source_id.clone(),
            ));
            broken_links = true;
        }
    }
    if !broken_links {
        results.push(DocLintResult::pass(DocLintCheck::BrokenEvidenceLinks));
    }

    // 4. Orphaned media (empty src)
    if media_issues.is_empty() {
        results.push(DocLintResult::pass(DocLintCheck::OrphanedMedia));
    } else {
        for (kind, id) in &media_issues {
            results.push(DocLintResult::fail(
                DocLintCheck::OrphanedMedia,
                DocLintSeverity::Warning,
                format!("{} block has empty src attribute", kind),
                id.clone(),
            ));
        }
    }

    // 4. Duplicate IDs
    let mut id_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for id in &all_ids {
        *id_counts.entry(id.as_str()).or_insert(0) += 1;
    }
    let mut dup_found = false;
    for (id, count) in &id_counts {
        if *count > 1 {
            results.push(DocLintResult::fail(
                DocLintCheck::DuplicateIds,
                DocLintSeverity::Error,
                format!("ID '{}' appears {} times", id, count),
                Some(id.to_string()),
            ));
            dup_found = true;
        }
    }
    if !dup_found {
        results.push(DocLintResult::pass(DocLintCheck::DuplicateIds));
    }

    // 5. Empty sections
    if empty_sections.is_empty() {
        results.push(DocLintResult::pass(DocLintCheck::EmptySections));
    } else {
        for id in &empty_sections {
            results.push(DocLintResult::fail(
                DocLintCheck::EmptySections,
                DocLintSeverity::Warning,
                "Section has no child blocks".to_string(),
                id.clone(),
            ));
        }
    }

    // 6. Missing metadata
    if doc.metadata.contains_key("title") {
        results.push(DocLintResult::pass(DocLintCheck::MissingMetadata));
    } else {
        results.push(DocLintResult::fail(
            DocLintCheck::MissingMetadata,
            DocLintSeverity::Warning,
            "Document has no 'title' metadata",
            None,
        ));
    }

    // 7. Empty footnotes
    if empty_footnotes == 0 {
        results.push(DocLintResult::pass(DocLintCheck::EmptyFootnotes));
    } else {
        results.push(DocLintResult::fail(
            DocLintCheck::EmptyFootnotes,
            DocLintSeverity::Warning,
            format!("{} empty footnote(s) found", empty_footnotes),
            None,
        ));
    }

    // 8. Malformed tables
    if malformed_tables.is_empty() {
        results.push(DocLintResult::pass(DocLintCheck::MalformedTables));
    } else {
        for id in &malformed_tables {
            results.push(DocLintResult::fail(
                DocLintCheck::MalformedTables,
                DocLintSeverity::Warning,
                "Table has no headers or no data rows".to_string(),
                id.clone(),
            ));
        }
    }

    // 9. Undefined terms — collect defined terms from @definition blocks, check @claim blocks
    let mut defined_terms: BTreeSet<String> = BTreeSet::new();
    let mut claim_blocks: Vec<(Option<String>, Vec<Inline>)> = Vec::new();
    collect_definitions_and_claims(&doc.blocks, &mut defined_terms, &mut claim_blocks);

    if claim_blocks.is_empty() || defined_terms.is_empty() {
        // No claims or no definitions — nothing to check
        results.push(DocLintResult::pass(DocLintCheck::UndefinedTerms));
    } else {
        let mut found_undefined = false;
        for (block_id, content) in &claim_blocks {
            let undefined = find_undefined_inline_terms(content, &defined_terms);
            for undef in &undefined {
                results.push(DocLintResult::fail(
                    DocLintCheck::UndefinedTerms,
                    DocLintSeverity::Warning,
                    format!(
                        "Claim references term '{}' which is not defined in any @definition block",
                        undef
                    ),
                    block_id.clone(),
                ));
                found_undefined = true;
            }
        }
        if !found_undefined {
            results.push(DocLintResult::pass(DocLintCheck::UndefinedTerms));
        }
    }

    results
}

/// Extract the defined term from a @definition block's content.
/// Heuristics:
/// 1. First Strong or Emphasis inline → that's the term
/// 2. Text before "is", "means", or ":" → that's the term
fn extract_defined_term(content: &[Inline]) -> Option<String> {
    // Try first Strong/Emphasis inline
    for inline in content {
        match inline {
            Inline::Strong { content } | Inline::Emphasis { content } => {
                let text = inlines_to_plain_text(content);
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_lowercase());
                }
            }
            _ => {}
        }
    }
    // Fallback: text before "is"/"means"/":"
    let full_text = inlines_to_plain_text(content);
    for separator in &[" is ", " means ", ": "] {
        if let Some(pos) = full_text.find(separator) {
            let term = full_text[..pos].trim();
            if !term.is_empty() && term.len() < 100 {
                return Some(term.to_lowercase());
            }
        }
    }
    None
}

/// Collect defined terms and claim block content from blocks recursively.
fn collect_definitions_and_claims(
    blocks: &[Block],
    defined_terms: &mut BTreeSet<String>,
    claim_blocks: &mut Vec<(Option<String>, Vec<Inline>)>,
) {
    for block in blocks {
        match &block.kind {
            BlockKind::SemanticBlock {
                block_type: SemanticBlockType::Definition,
                content,
                ..
            } => {
                if let Some(term) = extract_defined_term(content) {
                    defined_terms.insert(term);
                }
            }
            BlockKind::SemanticBlock {
                block_type: SemanticBlockType::Claim,
                attrs,
                content,
                ..
            } => {
                claim_blocks.push((attrs.id.clone(), content.clone()));
            }
            BlockKind::Section { children, .. } => {
                collect_definitions_and_claims(children, defined_terms, claim_blocks);
            }
            BlockKind::SkillBlock { children, .. } => {
                collect_definitions_and_claims(children, defined_terms, claim_blocks);
            }
            BlockKind::BlockQuote { content } => {
                collect_definitions_and_claims(content, defined_terms, claim_blocks);
            }
            _ => {}
        }
    }
}

/// Simple plain text extraction from inlines.
fn inlines_to_plain_text(inlines: &[Inline]) -> String {
    let mut result = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => result.push_str(text),
            Inline::Strong { content } | Inline::Emphasis { content } => {
                result.push_str(&inlines_to_plain_text(content));
            }
            Inline::InlineCode { code } => result.push_str(code),
            Inline::Link { text, .. } => {
                result.push_str(&inlines_to_plain_text(text));
            }
            Inline::Footnote { content } => {
                result.push_str(&inlines_to_plain_text(content));
            }
            Inline::SoftBreak | Inline::HardBreak => result.push(' '),
            _ => {}
        }
    }
    result
}

/// Check claim inlines for Strong/Emphasis terms that aren't defined.
fn find_undefined_inline_terms(
    content: &[Inline],
    defined_terms: &BTreeSet<String>,
) -> Vec<String> {
    let mut undefined = Vec::new();
    for inline in content {
        match inline {
            Inline::Strong { content } | Inline::Emphasis { content } => {
                let term = inlines_to_plain_text(content).trim().to_lowercase();
                if !term.is_empty() && !defined_terms.contains(&term) {
                    undefined.push(inlines_to_plain_text(content).trim().to_string());
                }
            }
            _ => {}
        }
    }
    undefined
}

/// Recursively collect IDs, claims, evidence, references, and issues from blocks.
/// Extract ref targets from a `refs` attribute value (comma-separated IDs).
fn collect_refs_from_attrs(
    attrs: &Attrs,
    evidence_links: &mut Vec<(Option<String>, String)>,
) {
    if let Some(refs_val) = attrs.pairs.get("refs") {
        for target in refs_val.split(',') {
            let target = target.trim();
            if !target.is_empty() {
                evidence_links.push((attrs.id.clone(), target.to_string()));
            }
        }
    }
}

fn collect_block_info(
    blocks: &[Block],
    all_ids: &mut Vec<String>,
    claim_ids: &mut Vec<String>,
    evidence_ids: &mut Vec<String>,
    reference_targets: &mut Vec<String>,
    media_issues: &mut Vec<(String, Option<String>)>,
    empty_sections: &mut Vec<Option<String>>,
    empty_footnotes: &mut usize,
    malformed_tables: &mut Vec<Option<String>>,
    evidence_links: &mut Vec<(Option<String>, String)>,
    depth: usize,
) {
    for block in blocks {
        // Helper: collect refs from any block with attrs
        let maybe_collect_refs = |attrs: &Attrs, links: &mut Vec<(Option<String>, String)>| {
            collect_refs_from_attrs(attrs, links);
        };

        match &block.kind {
            BlockKind::Section {
                attrs,
                children,
                title: _,
            } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
                maybe_collect_refs(attrs, evidence_links);
                if children.is_empty() && depth > 0 {
                    empty_sections.push(attrs.id.clone());
                }
                collect_block_info(
                    children, all_ids, claim_ids, evidence_ids, reference_targets,
                    media_issues, empty_sections, empty_footnotes, malformed_tables,
                    evidence_links, depth + 1,
                );
            }
            BlockKind::SemanticBlock {
                block_type, attrs, content, ..
            } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                    match block_type {
                        SemanticBlockType::Claim => claim_ids.push(id.clone()),
                        SemanticBlockType::Evidence => evidence_ids.push(id.clone()),
                        _ => {}
                    }
                } else {
                    match block_type {
                        SemanticBlockType::Claim => {
                            claim_ids.push(format!("__anon_claim_{}", claim_ids.len()));
                        }
                        SemanticBlockType::Evidence => {
                            evidence_ids.push(format!("__anon_evidence_{}", evidence_ids.len()));
                        }
                        _ => {}
                    }
                }
                maybe_collect_refs(attrs, evidence_links);
                collect_inline_refs(content, reference_targets, empty_footnotes);
            }
            BlockKind::Callout { attrs, content, .. } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
                maybe_collect_refs(attrs, evidence_links);
                collect_inline_refs(content, reference_targets, empty_footnotes);
            }
            BlockKind::Paragraph { content } => {
                collect_inline_refs(content, reference_targets, empty_footnotes);
            }
            BlockKind::Figure { attrs, src, .. } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
                maybe_collect_refs(attrs, evidence_links);
                if src.is_empty() {
                    media_issues.push(("Figure".to_string(), attrs.id.clone()));
                }
            }
            BlockKind::Audio { attrs, src, .. } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
                maybe_collect_refs(attrs, evidence_links);
                if src.is_empty() {
                    media_issues.push(("Audio".to_string(), attrs.id.clone()));
                }
            }
            BlockKind::Video { attrs, src, .. } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
                maybe_collect_refs(attrs, evidence_links);
                if src.is_empty() {
                    media_issues.push(("Video".to_string(), attrs.id.clone()));
                }
            }
            BlockKind::Table { attrs, headers, rows, .. } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
                maybe_collect_refs(attrs, evidence_links);
                if headers.is_empty() || rows.is_empty() {
                    malformed_tables.push(attrs.id.clone());
                }
            }
            BlockKind::CodeBlock { attrs, .. } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
            }
            BlockKind::SkillBlock { attrs, content, children, .. } => {
                if let Some(id) = &attrs.id {
                    all_ids.push(id.clone());
                }
                maybe_collect_refs(attrs, evidence_links);
                collect_inline_refs(content, reference_targets, empty_footnotes);
                collect_block_info(
                    children, all_ids, claim_ids, evidence_ids, reference_targets,
                    media_issues, empty_sections, empty_footnotes, malformed_tables,
                    evidence_links, depth,
                );
            }
            BlockKind::BlockQuote { content } => {
                collect_block_info(
                    content, all_ids, claim_ids, evidence_ids, reference_targets,
                    media_issues, empty_sections, empty_footnotes, malformed_tables,
                    evidence_links, depth,
                );
            }
            BlockKind::List { items, .. } => {
                for item in items {
                    collect_inline_refs(&item.content, reference_targets, empty_footnotes);
                    collect_block_info(
                        &item.children, all_ids, claim_ids, evidence_ids, reference_targets,
                        media_issues, empty_sections, empty_footnotes, malformed_tables,
                        evidence_links, depth,
                    );
                }
            }
            BlockKind::ThematicBreak => {}
        }
    }
}

/// Collect Reference targets and count empty footnotes from inlines.
fn collect_inline_refs(
    inlines: &[Inline],
    reference_targets: &mut Vec<String>,
    empty_footnotes: &mut usize,
) {
    for inline in inlines {
        match inline {
            Inline::Reference { target } => {
                reference_targets.push(target.clone());
            }
            Inline::Footnote { content } => {
                if content.is_empty() {
                    *empty_footnotes += 1;
                }
                collect_inline_refs(content, reference_targets, empty_footnotes);
            }
            Inline::Emphasis { content } | Inline::Strong { content } => {
                collect_inline_refs(content, reference_targets, empty_footnotes);
            }
            Inline::Link { text, .. } => {
                collect_inline_refs(text, reference_targets, empty_footnotes);
            }
            _ => {}
        }
    }
}

/// Summary of lint results for display.
pub fn lint_summary(results: &[DocLintResult]) -> (usize, usize, usize) {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    (total, passed, failed)
}

/// Run structural lint checks on a chunk graph.
pub fn lint_chunk_graph(graph: &ChunkGraph) -> Vec<DocLintResult> {
    let mut results = Vec::new();

    // 1. Orphaned chunks (skip single-chunk documents)
    let mut doc_chunk_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for chunk in graph.chunks.values() {
        *doc_chunk_counts.entry(&chunk.source_doc).or_insert(0) += 1;
    }
    let mut orphan_found = false;
    for (id, chunk) in &graph.chunks {
        let doc_count = doc_chunk_counts
            .get(chunk.source_doc.as_str())
            .copied()
            .unwrap_or(0);
        if doc_count <= 1 {
            continue;
        }
        let has_outgoing = graph.links.iter().any(|l| &l.source == id);
        let has_incoming = graph.links.iter().any(|l| &l.target == id);
        if !has_outgoing && !has_incoming {
            results.push(DocLintResult::fail(
                DocLintCheck::OrphanedChunks,
                DocLintSeverity::Warning,
                format!("Chunk {} has no links (isolated)", id),
                Some(id.0.clone()),
            ));
            orphan_found = true;
        }
    }
    if !orphan_found {
        results.push(DocLintResult::pass(DocLintCheck::OrphanedChunks));
    }

    // 2. Missing continuation links
    let mut missing_cont = false;
    for doc_path in doc_chunk_counts.keys() {
        let mut doc_chunks: Vec<_> = graph
            .chunks
            .values()
            .filter(|c| c.source_doc == *doc_path)
            .collect();
        doc_chunks.sort_by_key(|c| c.metadata.sequence);
        for window in doc_chunks.windows(2) {
            let a_id = &window[0].id;
            let b_id = &window[1].id;
            let has_cont = graph.links.iter().any(|l| {
                &l.source == a_id
                    && &l.target == b_id
                    && l.link_type == LinkType::Continuation
            });
            if !has_cont {
                results.push(DocLintResult::fail(
                    DocLintCheck::MissingContinuation,
                    DocLintSeverity::Warning,
                    format!("No Continuation link from {} to {}", a_id, b_id),
                    Some(a_id.0.clone()),
                ));
                missing_cont = true;
            }
        }
    }
    if !missing_cont {
        results.push(DocLintResult::pass(DocLintCheck::MissingContinuation));
    }

    // 3. Dependency cycles
    let cycles = detect_dependency_cycles(graph);
    if cycles.is_empty() {
        results.push(DocLintResult::pass(DocLintCheck::DependencyCycle));
    } else {
        for cycle in &cycles {
            let cycle_str = cycle
                .iter()
                .map(|id| id.0.as_str())
                .collect::<Vec<_>>()
                .join(" → ");
            results.push(DocLintResult::fail(
                DocLintCheck::DependencyCycle,
                DocLintSeverity::Error,
                format!("Dependency cycle: {}", cycle_str),
                cycle.first().map(|id| id.0.clone()),
            ));
        }
    }

    results
}

fn detect_dependency_cycles(graph: &ChunkGraph) -> Vec<Vec<ChunkId>> {
    let mut color: BTreeMap<&ChunkId, u8> = BTreeMap::new();
    let mut cycles = Vec::new();
    let mut path: Vec<ChunkId> = Vec::new();
    for id in graph.chunks.keys() {
        if color.get(id).copied().unwrap_or(0) == 0 {
            dfs_cycle(graph, id, &mut color, &mut path, &mut cycles);
        }
    }
    cycles
}

fn dfs_cycle<'a>(
    graph: &'a ChunkGraph,
    node: &'a ChunkId,
    color: &mut BTreeMap<&'a ChunkId, u8>,
    path: &mut Vec<ChunkId>,
    cycles: &mut Vec<Vec<ChunkId>>,
) {
    color.insert(node, 1);
    path.push(node.clone());
    for link in &graph.links {
        if &link.source != node {
            continue;
        }
        if !matches!(
            link.link_type,
            LinkType::Dependency | LinkType::ParentContext
        ) {
            continue;
        }
        match color.get(&link.target).copied().unwrap_or(0) {
            0 => dfs_cycle(graph, &link.target, color, path, cycles),
            1 => {
                if let Some(pos) = path.iter().position(|p| p == &link.target) {
                    cycles.push(path[pos..].to_vec());
                }
            }
            _ => {}
        }
    }
    path.pop();
    color.insert(node, 2);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn clean_document_passes_all_checks() {
        let doc = Document {
            metadata: [("title".to_string(), "Test".to_string())].into(),
            blocks: vec![Block {
                kind: BlockKind::Section {
                    attrs: Attrs {
                        id: Some("s1".into()),
                        ..Default::default()
                    },
                    title: vec![Inline::Text {
                        text: "Intro".into(),
                    }],
                    children: vec![Block {
                        kind: BlockKind::Paragraph {
                            content: vec![Inline::Text {
                                text: "Hello".into(),
                            }],
                        },
                        span: span(),
                    }],
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        assert!(
            results.iter().all(|r| r.passed),
            "All checks should pass: {:?}",
            results
        );
    }

    #[test]
    fn missing_title_metadata() {
        let doc = Document {
            metadata: BTreeMap::new(),
            blocks: vec![],
        };
        let results = lint_document(&doc);
        let meta = results
            .iter()
            .find(|r| r.check == DocLintCheck::MissingMetadata)
            .unwrap();
        assert!(!meta.passed);
        assert!(meta.message.contains("title"));
    }

    #[test]
    fn duplicate_ids_detected() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::Section {
                        attrs: Attrs {
                            id: Some("dup".into()),
                            ..Default::default()
                        },
                        title: vec![],
                        children: vec![Block {
                            kind: BlockKind::Paragraph {
                                content: vec![Inline::Text { text: "a".into() }],
                            },
                            span: span(),
                        }],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::CodeBlock {
                        lang: None,
                        attrs: Attrs {
                            id: Some("dup".into()),
                            ..Default::default()
                        },
                        code: "x".into(),
                    },
                    span: span(),
                },
            ],
        };
        let results = lint_document(&doc);
        let dups: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::DuplicateIds && !r.passed)
            .collect();
        assert_eq!(dups.len(), 1);
        assert!(dups[0].message.contains("dup"));
    }

    #[test]
    fn claim_without_evidence_warns() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::SemanticBlock {
                    block_type: SemanticBlockType::Claim,
                    attrs: Attrs {
                        id: Some("c1".into()),
                        ..Default::default()
                    },
                    title: None,
                    content: vec![Inline::Text {
                        text: "claim".into(),
                    }],
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        let claims: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::ClaimsWithoutEvidence && !r.passed)
            .collect();
        assert_eq!(claims.len(), 1);
    }

    #[test]
    fn claim_with_evidence_passes() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Claim,
                        attrs: Attrs {
                            id: Some("c1".into()),
                            ..Default::default()
                        },
                        title: None,
                        content: vec![Inline::Text {
                            text: "claim".into(),
                        }],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Evidence,
                        attrs: Attrs {
                            id: Some("e1".into()),
                            ..Default::default()
                        },
                        title: None,
                        content: vec![Inline::Text {
                            text: "evidence".into(),
                        }],
                    },
                    span: span(),
                },
            ],
        };
        let results = lint_document(&doc);
        let claims = results
            .iter()
            .find(|r| r.check == DocLintCheck::ClaimsWithoutEvidence)
            .unwrap();
        assert!(claims.passed);
    }

    #[test]
    fn broken_reference_detected() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::Paragraph {
                    content: vec![Inline::Reference {
                        target: "nonexistent".into(),
                    }],
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        let refs: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::BrokenReferences && !r.passed)
            .collect();
        assert_eq!(refs.len(), 1);
        assert!(refs[0].message.contains("nonexistent"));
    }

    #[test]
    fn empty_nested_section_warned() {
        // Top-level flat sections (heading-style) are OK with no children.
        // Only nested sections (inside a parent with @end) are flagged.
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::Section {
                    attrs: Attrs {
                        id: Some("parent".into()),
                        ..Default::default()
                    },
                    title: vec![Inline::Text {
                        text: "Parent".into(),
                    }],
                    children: vec![Block {
                        kind: BlockKind::Section {
                            attrs: Attrs {
                                id: Some("empty-child".into()),
                                ..Default::default()
                            },
                            title: vec![Inline::Text {
                                text: "Empty Child".into(),
                            }],
                            children: vec![],
                        },
                        span: span(),
                    }],
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        let empty: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::EmptySections && !r.passed)
            .collect();
        assert_eq!(empty.len(), 1);
    }

    #[test]
    fn top_level_flat_section_not_warned() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::Section {
                        attrs: Attrs {
                            id: Some("heading".into()),
                            ..Default::default()
                        },
                        title: vec![Inline::Text {
                            text: "Heading".into(),
                        }],
                        children: vec![],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::Paragraph {
                        content: vec![Inline::Text {
                            text: "Content follows".into(),
                        }],
                    },
                    span: span(),
                },
            ],
        };
        let results = lint_document(&doc);
        let empty: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::EmptySections && !r.passed)
            .collect();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn orphaned_media_detected() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::Figure {
                    attrs: Attrs {
                        id: Some("fig1".into()),
                        ..Default::default()
                    },
                    caption: None,
                    src: "".into(),
                    meta: MediaMeta::default(),
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        let media: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::OrphanedMedia && !r.passed)
            .collect();
        assert_eq!(media.len(), 1);
    }

    #[test]
    fn malformed_table_detected() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::Table {
                    attrs: Attrs::default(),
                    caption: None,
                    headers: vec![],
                    rows: vec![],
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        let tables: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::MalformedTables && !r.passed)
            .collect();
        assert_eq!(tables.len(), 1);
    }

    #[test]
    fn valid_evidence_link_passes() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Claim,
                        attrs: {
                            let mut a = Attrs::new();
                            a.id = Some("c1".into());
                            a.pairs.insert("refs".into(), "e1".into());
                            a
                        },
                        title: None,
                        content: vec![Inline::Text { text: "claim".into() }],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Evidence,
                        attrs: Attrs { id: Some("e1".into()), ..Default::default() },
                        title: None,
                        content: vec![Inline::Text { text: "evidence".into() }],
                    },
                    span: span(),
                },
            ],
        };
        let results = lint_document(&doc);
        let links = results.iter().find(|r| r.check == DocLintCheck::BrokenEvidenceLinks).unwrap();
        assert!(links.passed);
    }

    #[test]
    fn broken_evidence_link_detected() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::SemanticBlock {
                    block_type: SemanticBlockType::Claim,
                    attrs: {
                        let mut a = Attrs::new();
                        a.id = Some("c1".into());
                        a.pairs.insert("refs".into(), "missing_evidence".into());
                        a
                    },
                    title: None,
                    content: vec![Inline::Text { text: "claim".into() }],
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        let broken: Vec<_> = results.iter()
            .filter(|r| r.check == DocLintCheck::BrokenEvidenceLinks && !r.passed)
            .collect();
        assert_eq!(broken.len(), 1);
        assert!(broken[0].message.contains("missing_evidence"));
    }

    #[test]
    fn multiple_refs_validated() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Claim,
                        attrs: {
                            let mut a = Attrs::new();
                            a.id = Some("c1".into());
                            a.pairs.insert("refs".into(), "e1, e2".into());
                            a
                        },
                        title: None,
                        content: vec![Inline::Text { text: "claim".into() }],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Evidence,
                        attrs: Attrs { id: Some("e1".into()), ..Default::default() },
                        title: None,
                        content: vec![Inline::Text { text: "ev1".into() }],
                    },
                    span: span(),
                },
                // e2 is missing — should fail
            ],
        };
        let results = lint_document(&doc);
        let broken: Vec<_> = results.iter()
            .filter(|r| r.check == DocLintCheck::BrokenEvidenceLinks && !r.passed)
            .collect();
        assert_eq!(broken.len(), 1);
        assert!(broken[0].message.contains("e2"));
    }

    #[test]
    fn lint_summary_counts() {
        let results = vec![
            DocLintResult::pass(DocLintCheck::MissingMetadata),
            DocLintResult::fail(
                DocLintCheck::DuplicateIds,
                DocLintSeverity::Error,
                "dup",
                Some("x".into()),
            ),
            DocLintResult::pass(DocLintCheck::EmptySections),
        ];
        let (total, passed, failed) = lint_summary(&results);
        assert_eq!(total, 3);
        assert_eq!(passed, 2);
        assert_eq!(failed, 1);
    }

    // --- Chunk graph lint tests ---

    fn make_chunk(id: ChunkId, doc: &str, seq: usize, total: usize) -> Chunk {
        Chunk {
            id,
            source_doc: doc.into(),
            block_path: vec![seq],
            blocks: vec![],
            metadata: ChunkMetadata {
                title: None,
                block_types: vec![],
                estimated_tokens: 100,
                depth: 0,
                sequence: seq,
                total_chunks: total,
                summary: None,
                requires_parent_context: false,
                semantic_types: vec![],
            },
        }
    }

    #[test]
    fn orphaned_chunk_detected() {
        let mut graph = ChunkGraph::new();
        graph.add_chunk(make_chunk(ChunkId::new("doc", &[0]), "doc.aif", 0, 2));
        graph.add_chunk(make_chunk(ChunkId::new("doc", &[1]), "doc.aif", 1, 2));
        let results = lint_chunk_graph(&graph);
        let orphaned: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::OrphanedChunks && !r.passed)
            .collect();
        assert_eq!(orphaned.len(), 2);
    }

    #[test]
    fn single_chunk_not_orphaned() {
        let mut graph = ChunkGraph::new();
        graph.add_chunk(make_chunk(ChunkId::new("doc", &[0]), "doc.aif", 0, 1));
        let results = lint_chunk_graph(&graph);
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn missing_continuation_detected() {
        let mut graph = ChunkGraph::new();
        graph.add_chunk(make_chunk(ChunkId::new("doc", &[0]), "doc.aif", 0, 2));
        graph.add_chunk(make_chunk(ChunkId::new("doc", &[1]), "doc.aif", 1, 2));
        let results = lint_chunk_graph(&graph);
        let missing: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::MissingContinuation && !r.passed)
            .collect();
        assert_eq!(missing.len(), 1);
    }

    // --- Undefined terms lint tests ---

    #[test]
    fn undefined_term_detected() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Definition,
                        attrs: Attrs::default(),
                        title: None,
                        content: vec![
                            Inline::Strong {
                                content: vec![Inline::Text { text: "Token Budget".into() }],
                            },
                            Inline::Text { text: " is the max tokens per chunk.".into() },
                        ],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Claim,
                        attrs: Attrs { id: Some("c1".into()), ..Default::default() },
                        title: None,
                        content: vec![
                            Inline::Text { text: "The ".into() },
                            Inline::Strong {
                                content: vec![Inline::Text { text: "Chunk Graph".into() }],
                            },
                            Inline::Text { text: " improves retrieval.".into() },
                        ],
                    },
                    span: span(),
                },
            ],
        };
        let results = lint_document(&doc);
        let undef: Vec<_> = results.iter()
            .filter(|r| r.check == DocLintCheck::UndefinedTerms && !r.passed)
            .collect();
        assert_eq!(undef.len(), 1);
        assert!(undef[0].message.contains("Chunk Graph"));
    }

    #[test]
    fn defined_term_passes() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Definition,
                        attrs: Attrs::default(),
                        title: None,
                        content: vec![
                            Inline::Strong {
                                content: vec![Inline::Text { text: "Token Budget".into() }],
                            },
                            Inline::Text { text: " is the max tokens.".into() },
                        ],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Claim,
                        attrs: Attrs { id: Some("c1".into()), ..Default::default() },
                        title: None,
                        content: vec![
                            Inline::Text { text: "The ".into() },
                            Inline::Strong {
                                content: vec![Inline::Text { text: "Token Budget".into() }],
                            },
                            Inline::Text { text: " controls chunking.".into() },
                        ],
                    },
                    span: span(),
                },
            ],
        };
        let results = lint_document(&doc);
        let undef = results.iter()
            .find(|r| r.check == DocLintCheck::UndefinedTerms)
            .unwrap();
        assert!(undef.passed);
    }

    #[test]
    fn no_definitions_skips_check() {
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![Block {
                kind: BlockKind::SemanticBlock {
                    block_type: SemanticBlockType::Claim,
                    attrs: Attrs { id: Some("c1".into()), ..Default::default() },
                    title: None,
                    content: vec![
                        Inline::Strong {
                            content: vec![Inline::Text { text: "Something".into() }],
                        },
                    ],
                },
                span: span(),
            }],
        };
        let results = lint_document(&doc);
        let undef = results.iter()
            .find(|r| r.check == DocLintCheck::UndefinedTerms)
            .unwrap();
        assert!(undef.passed); // No definitions → nothing to check
    }

    #[test]
    fn definition_term_from_text_separator() {
        // Test the fallback heuristic: "Term is ..."
        let doc = Document {
            metadata: [("title".into(), "T".into())].into(),
            blocks: vec![
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Definition,
                        attrs: Attrs::default(),
                        title: None,
                        content: vec![
                            Inline::Text { text: "Semantic block is a typed container.".into() },
                        ],
                    },
                    span: span(),
                },
                Block {
                    kind: BlockKind::SemanticBlock {
                        block_type: SemanticBlockType::Claim,
                        attrs: Attrs { id: Some("c1".into()), ..Default::default() },
                        title: None,
                        content: vec![
                            Inline::Strong {
                                content: vec![Inline::Text { text: "Semantic block".into() }],
                            },
                            Inline::Text { text: " improves structure.".into() },
                        ],
                    },
                    span: span(),
                },
            ],
        };
        let results = lint_document(&doc);
        let undef = results.iter()
            .find(|r| r.check == DocLintCheck::UndefinedTerms)
            .unwrap();
        assert!(undef.passed);
    }

    // --- Chunk graph lint tests ---

    #[test]
    fn dependency_cycle_detected() {
        let mut graph = ChunkGraph::new();
        let a = ChunkId::new("doc", &[0]);
        let b = ChunkId::new("doc", &[1]);
        graph.add_chunk(make_chunk(a.clone(), "doc.aif", 0, 2));
        graph.add_chunk(make_chunk(b.clone(), "doc.aif", 1, 2));
        graph.add_link(ChunkLink {
            source: a.clone(),
            target: b.clone(),
            link_type: LinkType::Dependency,
            label: None,
        });
        graph.add_link(ChunkLink {
            source: b.clone(),
            target: a.clone(),
            link_type: LinkType::Dependency,
            label: None,
        });
        let results = lint_chunk_graph(&graph);
        let cycles: Vec<_> = results
            .iter()
            .filter(|r| r.check == DocLintCheck::DependencyCycle && !r.passed)
            .collect();
        assert!(!cycles.is_empty());
    }
}
