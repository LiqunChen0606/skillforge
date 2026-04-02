//! Document-level semantic linting.
//!
//! Checks structural quality of any AIF document (not just skills).
//! Covers: broken references, orphaned figures, claims without evidence,
//! duplicate IDs, empty sections, and missing metadata.

use crate::ast::*;
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

    results
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
}
