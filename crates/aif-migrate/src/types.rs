use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for a migration run.
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub skill_path: PathBuf,
    pub source_dir: PathBuf,
    pub output_dir: PathBuf,
    pub max_repair_iterations: u32,
    pub file_patterns: Vec<String>,
}

/// Result for a single chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResult {
    pub chunk_id: String,
    pub files: Vec<PathBuf>,
    pub status: ChunkStatus,
    pub confidence: f64,
    pub verification: VerificationResult,
    pub repair_iterations: u32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkStatus {
    Success,
    PartialSuccess,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub static_checks: Vec<StaticCheck>,
    pub semantic_checks: Vec<SemanticCheck>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCheck {
    pub criterion: String,
    pub passed: bool,
    pub reasoning: String,
    pub confidence: f64,
}

/// Full migration report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub skill_name: String,
    pub source_dir: PathBuf,
    pub chunks: Vec<ChunkResult>,
    pub overall_confidence: f64,
    pub unresolved: Vec<String>,
    pub manual_review: Vec<String>,
    #[serde(with = "duration_serde")]
    pub duration: Duration,
}

impl MigrationReport {
    pub fn all_passed(&self) -> bool {
        self.chunks.iter().all(|c| c.status == ChunkStatus::Success)
    }

    pub fn success_rate(&self) -> f64 {
        if self.chunks.is_empty() {
            return 0.0;
        }
        let successes = self.chunks.iter()
            .filter(|c| matches!(c.status, ChunkStatus::Success | ChunkStatus::PartialSuccess))
            .count();
        successes as f64 / self.chunks.len() as f64
    }
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_millis().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}
