use aif_core::ast::*;
use aif_core::chunk::*;
use aif_core::span::Span;

fn make_paragraph(text: &str) -> Block {
    Block {
        kind: BlockKind::Paragraph {
            content: vec![Inline::Text {
                text: text.to_string(),
            }],
        },
        span: Span::new(0, 0),
    }
}

fn make_section(title: &str, children: Vec<Block>) -> Block {
    Block {
        kind: BlockKind::Section {
            attrs: Attrs::new(),
            title: vec![Inline::Text {
                text: title.to_string(),
            }],
            children,
        },
        span: Span::new(0, 0),
    }
}

fn make_semantic_block(text: &str) -> Block {
    Block {
        kind: BlockKind::SemanticBlock {
            block_type: SemanticBlockType::Claim,
            attrs: Attrs::new(),
            title: None,
            content: vec![Inline::Text {
                text: text.to_string(),
            }],
        },
        span: Span::new(0, 0),
    }
}

fn make_doc(blocks: Vec<Block>) -> Document {
    Document {
        metadata: std::collections::BTreeMap::new(),
        blocks,
    }
}

#[test]
fn chunk_graph_cross_document_links() {
    let doc_a = make_doc(vec![
        make_section("Claim", vec![make_paragraph("We assert X.")]),
    ]);
    let doc_b = make_doc(vec![
        make_section("Evidence", vec![make_paragraph("Data shows X is true.")]),
    ]);

    let chunks_a =
        aif_pdf::chunk::chunk_document(&doc_a, "doc_a.aif", ChunkStrategy::Section).unwrap();
    let chunks_b =
        aif_pdf::chunk::chunk_document(&doc_b, "doc_b.aif", ChunkStrategy::Section).unwrap();

    let mut graph = ChunkGraph::new();
    for chunk in chunks_a {
        graph.add_chunk(chunk);
    }
    for chunk in chunks_b {
        graph.add_chunk(chunk);
    }

    // Add a cross-document evidence link
    let source_id = graph.chunks.keys().next().unwrap().clone();
    let target_id = graph.chunks.keys().last().unwrap().clone();
    graph.add_link(ChunkLink {
        source: source_id.clone(),
        target: target_id.clone(),
        link_type: LinkType::Evidence,
        label: Some("supporting data".to_string()),
    });

    assert_eq!(graph.chunks.len(), 2);
    assert_eq!(graph.links.len(), 1);
    assert_eq!(graph.outgoing_links(&source_id).len(), 1);
    assert_eq!(graph.incoming_links(&target_id).len(), 1);
}

#[test]
fn chunk_graph_serialization_roundtrip() {
    let doc = make_doc(vec![
        make_section("Intro", vec![]),
        make_section("Body", vec![]),
    ]);

    let chunks =
        aif_pdf::chunk::chunk_document(&doc, "test.aif", ChunkStrategy::Section).unwrap();

    let mut graph = ChunkGraph::new();
    for chunk in chunks {
        graph.add_chunk(chunk);
    }

    let json = serde_json::to_string(&graph).unwrap();
    let restored: ChunkGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.chunks.len(), graph.chunks.len());
}

#[test]
fn semantic_chunking_splits_on_semantic_blocks() {
    let doc = make_doc(vec![
        make_paragraph("Preamble text."),
        make_semantic_block("Claim: X is true."),
        make_paragraph("Supporting paragraph."),
        make_semantic_block("Claim: Y is also true."),
    ]);

    let chunks =
        aif_pdf::chunk::chunk_document(&doc, "test.aif", ChunkStrategy::Semantic).unwrap();
    // Semantic: splits before each SemanticBlock
    // Chunk 1: Preamble, Chunk 2: Claim X + Supporting, Chunk 3: Claim Y
    assert_eq!(chunks.len(), 3);
}

#[test]
fn chunk_continuation_links() {
    let doc = make_doc(vec![
        make_paragraph("Part 1"),
        make_paragraph("Part 2"),
        make_paragraph("Part 3"),
    ]);

    let chunks = aif_pdf::chunk::chunk_document(
        &doc,
        "test.aif",
        ChunkStrategy::FixedBlocks { blocks_per_chunk: 1 },
    )
    .unwrap();

    // Create continuation links between sequential chunks
    let mut graph = ChunkGraph::new();
    for chunk in &chunks {
        graph.add_chunk(chunk.clone());
    }
    for i in 0..chunks.len() - 1 {
        graph.add_link(ChunkLink {
            source: chunks[i].id.clone(),
            target: chunks[i + 1].id.clone(),
            link_type: LinkType::Continuation,
            label: None,
        });
    }

    assert_eq!(graph.links.len(), 2); // 3 chunks, 2 continuation links
}
