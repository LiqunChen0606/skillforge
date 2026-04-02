use std::collections::HashMap;
use std::path::PathBuf;

const BPE_TOKENS_PER_WORD: f64 = 1.3;

#[derive(Debug, Clone)]
pub enum ChunkStrategy {
    FilePerChunk,
    DirectoryChunk,
    TokenBudget { max_tokens: usize },
}

#[derive(Debug, Clone)]
pub struct SourceChunk {
    pub chunk_id: String,
    pub files: Vec<(PathBuf, String)>,
}

pub fn estimate_tokens(text: &str) -> usize {
    (text.split_whitespace().count() as f64 * BPE_TOKENS_PER_WORD).ceil() as usize
}

pub fn chunk_source_files(
    files: &HashMap<PathBuf, String>,
    strategy: ChunkStrategy,
) -> Vec<SourceChunk> {
    if files.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<_> = files.iter().collect();
    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));

    match strategy {
        ChunkStrategy::FilePerChunk => {
            sorted.iter().enumerate().map(|(i, (path, content))| {
                SourceChunk {
                    chunk_id: format!("file-{:04}-{}", i, path.display()),
                    files: vec![((*path).clone(), content.to_string())],
                }
            }).collect()
        }
        ChunkStrategy::DirectoryChunk => {
            let mut by_dir: HashMap<String, Vec<(PathBuf, String)>> = HashMap::new();
            for (path, content) in &sorted {
                let dir = path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string());
                by_dir.entry(dir).or_default().push(((*path).clone(), content.to_string()));
            }
            let mut dirs: Vec<_> = by_dir.into_iter().collect();
            dirs.sort_by(|a, b| a.0.cmp(&b.0));
            dirs.into_iter().enumerate().map(|(i, (dir, files))| {
                SourceChunk {
                    chunk_id: format!("dir-{:04}-{}", i, dir),
                    files,
                }
            }).collect()
        }
        ChunkStrategy::TokenBudget { max_tokens } => {
            let mut chunks = Vec::new();
            let mut current_files = Vec::new();
            let mut current_tokens = 0usize;

            for (path, content) in &sorted {
                let file_tokens = estimate_tokens(content);
                if !current_files.is_empty() && current_tokens + file_tokens > max_tokens {
                    chunks.push(SourceChunk {
                        chunk_id: format!("budget-{:04}", chunks.len()),
                        files: std::mem::take(&mut current_files),
                    });
                    current_tokens = 0;
                }
                current_files.push(((*path).clone(), content.to_string()));
                current_tokens += file_tokens;
            }
            if !current_files.is_empty() {
                chunks.push(SourceChunk {
                    chunk_id: format!("budget-{:04}", chunks.len()),
                    files: current_files,
                });
            }
            chunks
        }
    }
}
