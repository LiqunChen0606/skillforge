mod splitter;
mod ids;

pub use splitter::{chunk_document, ChunkError};
pub use ids::compute_doc_hash;

// Re-export core chunk types for convenience
pub use aif_core::chunk::*;
