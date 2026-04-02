use aif_migrate::chunk::{ChunkStrategy, chunk_source_files, estimate_tokens};
use std::path::PathBuf;
use std::collections::HashMap;

fn make_files(entries: &[(&str, &str)]) -> HashMap<PathBuf, String> {
    entries.iter().map(|(p, c)| (PathBuf::from(p), c.to_string())).collect()
}

#[test]
fn file_per_chunk_creates_one_chunk_per_file() {
    let files = make_files(&[
        ("src/a.rs", "fn a() {}"),
        ("src/b.rs", "fn b() {}"),
        ("src/c.rs", "fn c() {}"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::FilePerChunk);
    assert_eq!(chunks.len(), 3);
    for chunk in &chunks {
        assert_eq!(chunk.files.len(), 1);
    }
}

#[test]
fn directory_chunk_groups_by_parent_dir() {
    let files = make_files(&[
        ("src/components/a.rs", "fn a() {}"),
        ("src/components/b.rs", "fn b() {}"),
        ("src/utils/c.rs", "fn c() {}"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::DirectoryChunk);
    assert_eq!(chunks.len(), 2);
    let comp_chunk = chunks.iter().find(|c| c.chunk_id.contains("components")).unwrap();
    assert_eq!(comp_chunk.files.len(), 2);
}

#[test]
fn token_budget_respects_limit() {
    let files = make_files(&[
        ("a.rs", "fn a() { let x = 1; }"),
        ("b.rs", "fn b() { let y = 2; }"),
        ("c.rs", "fn c() { let z = 3; }"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::TokenBudget { max_tokens: 25 });
    assert!(chunks.len() >= 2, "Should split into multiple chunks, got {}", chunks.len());
    for chunk in &chunks {
        let total_tokens: usize = chunk.files.iter()
            .map(|(_, content)| estimate_tokens(content))
            .sum();
        assert!(total_tokens <= 25, "Chunk exceeds token budget: {}", total_tokens);
    }
}

#[test]
fn empty_files_returns_empty_chunks() {
    let files: HashMap<PathBuf, String> = HashMap::new();
    let chunks = chunk_source_files(&files, ChunkStrategy::FilePerChunk);
    assert!(chunks.is_empty());
}

#[test]
fn oversized_file_produces_warning() {
    // A single file with ~50K tokens in a 100-token budget should produce a warning
    let big_content: String = (0..40000).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ");
    let files = make_files(&[("big.rs", &big_content)]);
    let chunks = chunk_source_files(&files, ChunkStrategy::TokenBudget { max_tokens: 100 });
    assert_eq!(chunks.len(), 1);
    assert!(!chunks[0].warnings.is_empty(), "Oversized chunk should have a warning");
    assert!(chunks[0].warnings[0].contains("exceeding budget"), "Warning should mention exceeding budget");
}

#[test]
fn chunk_ids_are_unique() {
    let files = make_files(&[
        ("a.rs", "fn a() {}"),
        ("b.rs", "fn b() {}"),
    ]);
    let chunks = chunk_source_files(&files, ChunkStrategy::FilePerChunk);
    let ids: Vec<&str> = chunks.iter().map(|c| c.chunk_id.as_str()).collect();
    let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), unique.len(), "Chunk IDs must be unique");
}
