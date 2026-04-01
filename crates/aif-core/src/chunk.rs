use crate::ast::Block;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unique identifier for a chunk, derived from content + position.
/// Format: "{doc_hash_prefix_8chars}:{block_path_dot_separated}"
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChunkId(pub String);

impl ChunkId {
    pub fn new(doc_hash_prefix: &str, block_path: &[usize]) -> Self {
        let path_str = block_path
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(".");
        Self(format!("{}:{}", doc_hash_prefix, path_str))
    }
}

impl std::fmt::Display for ChunkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A chunk is a contiguous slice of blocks from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub source_doc: String,
    pub block_path: Vec<usize>,
    pub blocks: Vec<Block>,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub title: Option<String>,
    pub block_types: Vec<String>,
    pub estimated_tokens: usize,
    pub depth: usize,
    pub sequence: usize,
    pub total_chunks: usize,
}

/// A directed link between chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLink {
    pub source: ChunkId,
    pub target: ChunkId,
    pub link_type: LinkType,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LinkType {
    Evidence,
    Dependency,
    Continuation,
    CrossReference,
    Refutation,
}

/// The chunk graph: nodes are chunks, edges are links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkGraph {
    pub chunks: BTreeMap<ChunkId, Chunk>,
    pub links: Vec<ChunkLink>,
    pub documents: BTreeMap<String, DocumentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEntry {
    pub path: String,
    pub content_hash: String,
    pub chunk_count: usize,
    pub title: Option<String>,
}

impl ChunkGraph {
    pub fn new() -> Self {
        Self {
            chunks: BTreeMap::new(),
            links: Vec::new(),
            documents: BTreeMap::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.id.clone(), chunk);
    }

    pub fn add_link(&mut self, link: ChunkLink) {
        self.links.push(link);
    }

    pub fn get_chunk(&self, id: &ChunkId) -> Option<&Chunk> {
        self.chunks.get(id)
    }

    /// Get all links originating from a chunk.
    pub fn outgoing_links(&self, id: &ChunkId) -> Vec<&ChunkLink> {
        self.links.iter().filter(|l| &l.source == id).collect()
    }

    /// Get all links pointing to a chunk.
    pub fn incoming_links(&self, id: &ChunkId) -> Vec<&ChunkLink> {
        self.links.iter().filter(|l| &l.target == id).collect()
    }
}

impl Default for ChunkGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunking strategy for splitting documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkStrategy {
    /// Split at section boundaries.
    Section,
    /// Split at a target token count, respecting block boundaries.
    TokenBudget { max_tokens: usize },
    /// Split at semantic block boundaries.
    Semantic,
    /// Fixed number of top-level blocks per chunk.
    FixedBlocks { blocks_per_chunk: usize },
}

impl Default for ChunkStrategy {
    fn default() -> Self {
        Self::TokenBudget { max_tokens: 2048 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_id_generation() {
        let id = ChunkId::new("a1b2c3d4", &[2, 3, 1]);
        assert_eq!(id.0, "a1b2c3d4:2.3.1");
    }

    #[test]
    fn chunk_id_root_block() {
        let id = ChunkId::new("a1b2c3d4", &[0]);
        assert_eq!(id.0, "a1b2c3d4:0");
    }

    #[test]
    fn chunk_graph_add_and_get() {
        let mut graph = ChunkGraph::new();
        let chunk = Chunk {
            id: ChunkId::new("abcd1234", &[0]),
            source_doc: "test.aif".to_string(),
            block_path: vec![0],
            blocks: vec![],
            metadata: ChunkMetadata {
                title: Some("Intro".to_string()),
                block_types: vec!["Paragraph".to_string()],
                estimated_tokens: 100,
                depth: 0,
                sequence: 0,
                total_chunks: 1,
            },
        };
        let id = chunk.id.clone();
        graph.add_chunk(chunk);
        assert!(graph.get_chunk(&id).is_some());
    }

    #[test]
    fn chunk_graph_links() {
        let mut graph = ChunkGraph::new();
        let src = ChunkId::new("aaaa1111", &[0]);
        let tgt = ChunkId::new("bbbb2222", &[1]);
        graph.add_link(ChunkLink {
            source: src.clone(),
            target: tgt.clone(),
            link_type: LinkType::Evidence,
            label: Some("supports claim".to_string()),
        });
        assert_eq!(graph.outgoing_links(&src).len(), 1);
        assert_eq!(graph.incoming_links(&tgt).len(), 1);
        assert_eq!(graph.outgoing_links(&tgt).len(), 0);
    }

    #[test]
    fn chunk_graph_serializes() {
        let graph = ChunkGraph::new();
        let json = serde_json::to_string(&graph).unwrap();
        assert!(json.contains("\"chunks\""));
        assert!(json.contains("\"links\""));
    }

    #[test]
    fn default_strategy_is_token_budget() {
        let strategy = ChunkStrategy::default();
        match strategy {
            ChunkStrategy::TokenBudget { max_tokens } => assert_eq!(max_tokens, 2048),
            _ => panic!("expected TokenBudget"),
        }
    }
}
