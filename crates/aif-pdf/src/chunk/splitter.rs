use aif_core::ast::{Block, BlockKind, Document, Inline};
use aif_core::chunk::{Chunk, ChunkId, ChunkMetadata, ChunkStrategy};
use std::fmt;

use super::ids::compute_doc_hash;

#[derive(Debug)]
pub enum ChunkError {
    EmptyDocument,
    InvalidStrategy(String),
    Other(String),
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkError::EmptyDocument => write!(f, "document has no blocks"),
            ChunkError::InvalidStrategy(msg) => write!(f, "invalid strategy: {}", msg),
            ChunkError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ChunkError {}

/// Chunk a document into pieces according to the given strategy.
pub fn chunk_document(
    doc: &Document,
    doc_path: &str,
    strategy: ChunkStrategy,
) -> Result<Vec<Chunk>, ChunkError> {
    if doc.blocks.is_empty() {
        return Err(ChunkError::EmptyDocument);
    }

    let doc_content = serde_json::to_string(doc)
        .map_err(|e| ChunkError::Other(format!("Failed to serialize document for hashing: {}", e)))?;
    let doc_hash = compute_doc_hash(&doc_content);

    let chunks = match strategy {
        ChunkStrategy::Section => chunk_by_section(doc, doc_path, &doc_hash),
        ChunkStrategy::TokenBudget { max_tokens } => {
            chunk_by_token_budget(doc, doc_path, &doc_hash, max_tokens)
        }
        ChunkStrategy::FixedBlocks { blocks_per_chunk } => {
            chunk_by_fixed_blocks(doc, doc_path, &doc_hash, blocks_per_chunk)
        }
        ChunkStrategy::Semantic => chunk_by_semantic(doc, doc_path, &doc_hash),
    };

    // Fill in total_chunks for each chunk
    let total = chunks.len();
    let chunks: Vec<Chunk> = chunks
        .into_iter()
        .map(|mut c| {
            c.metadata.total_chunks = total;
            c
        })
        .collect();

    Ok(chunks)
}

fn chunk_by_section(doc: &Document, doc_path: &str, doc_hash: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut sequence = 0;

    for (i, block) in doc.blocks.iter().enumerate() {
        if matches!(&block.kind, BlockKind::Section { .. }) {
            let title = extract_block_title(block);
            let block_types = vec![block_type_name(&block.kind)];
            let tokens = estimate_block_tokens(block);

            chunks.push(Chunk {
                id: ChunkId::new(doc_hash, &[i]),
                source_doc: doc_path.to_string(),
                block_path: vec![i],
                blocks: vec![block.clone()],
                metadata: ChunkMetadata {
                    title,
                    block_types,
                    estimated_tokens: tokens,
                    depth: 0,
                    sequence,
                    total_chunks: 0, // filled in later
                },
            });
            sequence += 1;
        } else {
            // Non-section blocks: group consecutive ones into a chunk
            if chunks.is_empty()
                || matches!(
                    &doc.blocks[chunks.last().unwrap().block_path[0]].kind,
                    BlockKind::Section { .. }
                )
            {
                let block_types = vec![block_type_name(&block.kind)];
                let tokens = estimate_block_tokens(block);
                chunks.push(Chunk {
                    id: ChunkId::new(doc_hash, &[i]),
                    source_doc: doc_path.to_string(),
                    block_path: vec![i],
                    blocks: vec![block.clone()],
                    metadata: ChunkMetadata {
                        title: None,
                        block_types,
                        estimated_tokens: tokens,
                        depth: 0,
                        sequence,
                        total_chunks: 0,
                    },
                });
                sequence += 1;
            } else {
                // Append to the last non-section chunk
                let last = chunks.last_mut().unwrap();
                last.blocks.push(block.clone());
                last.metadata
                    .block_types
                    .push(block_type_name(&block.kind));
                last.metadata.estimated_tokens += estimate_block_tokens(block);
            }
        }
    }

    chunks
}

fn chunk_by_token_budget(
    doc: &Document,
    doc_path: &str,
    doc_hash: &str,
    max_tokens: usize,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current_blocks: Vec<Block> = Vec::new();
    let mut current_tokens = 0;
    let mut current_start_idx = 0;
    let mut sequence = 0;

    for (i, block) in doc.blocks.iter().enumerate() {
        let block_tokens = estimate_block_tokens(block);

        if !current_blocks.is_empty() && current_tokens + block_tokens > max_tokens {
            // Flush current chunk
            chunks.push(make_chunk(
                doc_hash,
                doc_path,
                &current_blocks,
                current_start_idx,
                sequence,
            ));
            sequence += 1;
            current_blocks.clear();
            current_tokens = 0;
            current_start_idx = i;
        }

        current_blocks.push(block.clone());
        current_tokens += block_tokens;
    }

    if !current_blocks.is_empty() {
        chunks.push(make_chunk(
            doc_hash,
            doc_path,
            &current_blocks,
            current_start_idx,
            sequence,
        ));
    }

    chunks
}

fn chunk_by_fixed_blocks(
    doc: &Document,
    doc_path: &str,
    doc_hash: &str,
    blocks_per_chunk: usize,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    for (chunk_idx, block_group) in doc.blocks.chunks(blocks_per_chunk).enumerate() {
        let start_idx = chunk_idx * blocks_per_chunk;
        chunks.push(make_chunk(
            doc_hash,
            doc_path,
            block_group,
            start_idx,
            chunk_idx,
        ));
    }

    chunks
}

fn chunk_by_semantic(doc: &Document, doc_path: &str, doc_hash: &str) -> Vec<Chunk> {
    // Semantic chunking: split at semantic block boundaries (claims, evidence, etc.)
    let mut chunks = Vec::new();
    let mut current_blocks: Vec<Block> = Vec::new();
    let mut current_start_idx = 0;
    let mut sequence = 0;

    for (i, block) in doc.blocks.iter().enumerate() {
        let is_semantic_boundary = matches!(
            &block.kind,
            BlockKind::SemanticBlock { .. }
                | BlockKind::Section { .. }
                | BlockKind::SkillBlock { .. }
        );

        if is_semantic_boundary && !current_blocks.is_empty() {
            chunks.push(make_chunk(
                doc_hash,
                doc_path,
                &current_blocks,
                current_start_idx,
                sequence,
            ));
            sequence += 1;
            current_blocks.clear();
            current_start_idx = i;
        }

        current_blocks.push(block.clone());
    }

    if !current_blocks.is_empty() {
        chunks.push(make_chunk(
            doc_hash,
            doc_path,
            &current_blocks,
            current_start_idx,
            sequence,
        ));
    }

    chunks
}

fn make_chunk(
    doc_hash: &str,
    doc_path: &str,
    blocks: &[Block],
    start_idx: usize,
    sequence: usize,
) -> Chunk {
    let block_types: Vec<String> = blocks.iter().map(|b| block_type_name(&b.kind)).collect();
    let tokens: usize = blocks.iter().map(estimate_block_tokens).sum();
    let title = blocks.first().and_then(extract_block_title);

    Chunk {
        id: ChunkId::new(doc_hash, &[start_idx]),
        source_doc: doc_path.to_string(),
        block_path: vec![start_idx],
        blocks: blocks.to_vec(),
        metadata: ChunkMetadata {
            title,
            block_types,
            estimated_tokens: tokens,
            depth: 0,
            sequence,
            total_chunks: 0,
        },
    }
}

fn block_type_name(kind: &BlockKind) -> String {
    match kind {
        BlockKind::Section { .. } => "Section".to_string(),
        BlockKind::Paragraph { .. } => "Paragraph".to_string(),
        BlockKind::SemanticBlock { block_type, .. } => format!("Semantic:{:?}", block_type),
        BlockKind::Callout { callout_type, .. } => format!("Callout:{:?}", callout_type),
        BlockKind::Table { .. } => "Table".to_string(),
        BlockKind::Figure { .. } => "Figure".to_string(),
        BlockKind::CodeBlock { .. } => "CodeBlock".to_string(),
        BlockKind::BlockQuote { .. } => "BlockQuote".to_string(),
        BlockKind::List { ordered, .. } => {
            if *ordered {
                "OrderedList".to_string()
            } else {
                "UnorderedList".to_string()
            }
        }
        BlockKind::SkillBlock { skill_type, .. } => format!("Skill:{:?}", skill_type),
        BlockKind::ThematicBreak => "ThematicBreak".to_string(),
    }
}

fn extract_block_title(block: &Block) -> Option<String> {
    match &block.kind {
        BlockKind::Section { title, .. } => {
            let text = inlines_to_text(title);
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

fn inlines_to_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Emphasis { content } | Inline::Strong { content } => {
                out.push_str(&inlines_to_text(content))
            }
            Inline::InlineCode { code } => out.push_str(code),
            Inline::Link { text, .. } => out.push_str(&inlines_to_text(text)),
            Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out
}

/// Average ratio of BPE tokens to whitespace-delimited words for English text.
/// Based on empirical measurement across cl100k_base (GPT-4/Claude) tokenizers.
/// English prose averages ~1.3 tokens/word; code and technical text may be higher.
const BPE_TOKENS_PER_WORD: f64 = 1.3;

/// Estimate token count using a fixed BPE approximation (word_count * 1.3).
fn estimate_block_tokens(block: &Block) -> usize {
    let text = collect_all_text(block);
    let words = text.split_whitespace().count();
    (words as f64 * BPE_TOKENS_PER_WORD).ceil() as usize
}

fn collect_all_text(block: &Block) -> String {
    match &block.kind {
        BlockKind::Section {
            title, children, ..
        } => {
            let mut text = inlines_to_text(title);
            for child in children {
                text.push(' ');
                text.push_str(&collect_all_text(child));
            }
            text
        }
        BlockKind::Paragraph { content } => inlines_to_text(content),
        BlockKind::CodeBlock { code, .. } => code.clone(),
        BlockKind::BlockQuote { content } => content
            .iter()
            .map(collect_all_text)
            .collect::<Vec<_>>()
            .join(" "),
        BlockKind::List { items, .. } => items
            .iter()
            .map(|item| {
                let mut text = inlines_to_text(&item.content);
                for child in &item.children {
                    text.push(' ');
                    text.push_str(&collect_all_text(child));
                }
                text
            })
            .collect::<Vec<_>>()
            .join(" "),
        BlockKind::Table {
            headers, rows, ..
        } => {
            let mut text = String::new();
            for h in headers {
                text.push_str(&inlines_to_text(h));
                text.push(' ');
            }
            for row in rows {
                for cell in row {
                    text.push_str(&inlines_to_text(cell));
                    text.push(' ');
                }
            }
            text
        }
        BlockKind::SemanticBlock { content, .. } => inlines_to_text(content),
        BlockKind::Callout { content, .. } => inlines_to_text(content),
        BlockKind::SkillBlock {
            content, children, ..
        } => {
            let mut text = inlines_to_text(content);
            for child in children {
                text.push(' ');
                text.push_str(&collect_all_text(child));
            }
            text
        }
        BlockKind::Figure { .. } | BlockKind::ThematicBreak => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                attrs: aif_core::ast::Attrs::new(),
                title: vec![Inline::Text {
                    text: title.to_string(),
                }],
                children,
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
    fn chunk_by_section_strategy() {
        let doc = make_doc(vec![
            make_section("Intro", vec![make_paragraph("First paragraph.")]),
            make_section("Body", vec![make_paragraph("Second paragraph.")]),
            make_section("Conclusion", vec![]),
        ]);

        let chunks = chunk_document(&doc, "test.aif", ChunkStrategy::Section).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].metadata.title.as_deref(), Some("Intro"));
        assert_eq!(chunks[1].metadata.title.as_deref(), Some("Body"));
        assert_eq!(chunks[2].metadata.title.as_deref(), Some("Conclusion"));
    }

    #[test]
    fn chunk_by_fixed_blocks_strategy() {
        let doc = make_doc(vec![
            make_paragraph("A"),
            make_paragraph("B"),
            make_paragraph("C"),
            make_paragraph("D"),
            make_paragraph("E"),
        ]);

        let chunks =
            chunk_document(&doc, "test.aif", ChunkStrategy::FixedBlocks { blocks_per_chunk: 2 })
                .unwrap();
        assert_eq!(chunks.len(), 3); // 2+2+1
        assert_eq!(chunks[0].blocks.len(), 2);
        assert_eq!(chunks[1].blocks.len(), 2);
        assert_eq!(chunks[2].blocks.len(), 1);
    }

    #[test]
    fn chunk_by_token_budget_strategy() {
        // Each paragraph is ~2 tokens ("A" = 1 word * 1.3 ≈ 2)
        let doc = make_doc(vec![
            make_paragraph("one two three four five"),
            make_paragraph("six seven eight nine ten"),
            make_paragraph("eleven twelve thirteen"),
        ]);

        // Budget of 10 tokens should split into multiple chunks
        let chunks =
            chunk_document(&doc, "test.aif", ChunkStrategy::TokenBudget { max_tokens: 10 })
                .unwrap();
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn chunk_empty_doc_errors() {
        let doc = make_doc(vec![]);
        let result = chunk_document(&doc, "test.aif", ChunkStrategy::Section);
        assert!(result.is_err());
    }

    #[test]
    fn chunk_ids_are_deterministic() {
        let doc = make_doc(vec![
            make_paragraph("Hello world"),
            make_paragraph("Goodbye world"),
        ]);

        let chunks1 = chunk_document(&doc, "test.aif", ChunkStrategy::Section).unwrap();
        let chunks2 = chunk_document(&doc, "test.aif", ChunkStrategy::Section).unwrap();

        assert_eq!(chunks1[0].id, chunks2[0].id);
    }

    #[test]
    fn chunk_total_chunks_filled() {
        let doc = make_doc(vec![
            make_section("A", vec![]),
            make_section("B", vec![]),
        ]);

        let chunks = chunk_document(&doc, "test.aif", ChunkStrategy::Section).unwrap();
        for chunk in &chunks {
            assert_eq!(chunk.metadata.total_chunks, 2);
        }
    }

    #[test]
    fn estimate_tokens_basic() {
        let block = make_paragraph("one two three four five");
        let tokens = estimate_block_tokens(&block);
        // 5 words * 1.3 ≈ 7
        assert_eq!(tokens, 7);
    }
}
