use crate::chunk::ChunkStrategy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Unified configuration for a migration run.
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub skill_path: PathBuf,
    pub source_dir: PathBuf,
    pub output_dir: PathBuf,
    pub max_repair_iterations: u32,
    pub file_patterns: Vec<String>,
    pub chunk_strategy: ChunkStrategy,
    pub dry_run: bool,
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

    /// Count chunks by status.
    pub fn status_counts(&self) -> (usize, usize, usize, usize) {
        let mut success = 0;
        let mut partial = 0;
        let mut failed = 0;
        let mut skipped = 0;
        for c in &self.chunks {
            match c.status {
                ChunkStatus::Success => success += 1,
                ChunkStatus::PartialSuccess => partial += 1,
                ChunkStatus::Failed => failed += 1,
                ChunkStatus::Skipped => skipped += 1,
            }
        }
        (success, partial, failed, skipped)
    }

    /// Collect all failed static checks across all chunks.
    pub fn failed_static_checks(&self) -> Vec<(&str, &str)> {
        self.chunks.iter()
            .flat_map(|c| {
                c.verification.static_checks.iter()
                    .filter(|sc| !sc.passed)
                    .map(move |sc| (c.chunk_id.as_str(), sc.name.as_str()))
            })
            .collect()
    }

    /// Collect all failed semantic checks across all chunks.
    pub fn failed_semantic_checks(&self) -> Vec<(&str, &str, &str)> {
        self.chunks.iter()
            .flat_map(|c| {
                c.verification.semantic_checks.iter()
                    .filter(|sc| !sc.passed)
                    .map(move |sc| (c.chunk_id.as_str(), sc.criterion.as_str(), sc.reasoning.as_str()))
            })
            .collect()
    }

    /// Total repair iterations across all chunks.
    pub fn total_repair_iterations(&self) -> u32 {
        self.chunks.iter().map(|c| c.repair_iterations).sum()
    }

    /// Average confidence across all non-skipped chunks.
    pub fn average_confidence(&self) -> f64 {
        let active: Vec<_> = self.chunks.iter()
            .filter(|c| c.status != ChunkStatus::Skipped)
            .collect();
        if active.is_empty() {
            return 0.0;
        }
        active.iter().map(|c| c.confidence).sum::<f64>() / active.len() as f64
    }

    /// Confidence level label for display.
    pub fn confidence_label(confidence: f64) -> &'static str {
        if confidence >= 0.9 {
            "High"
        } else if confidence >= 0.7 {
            "Medium"
        } else if confidence >= 0.5 {
            "Low"
        } else {
            "Very Low"
        }
    }

    /// Risk assessment based on migration results.
    pub fn risk_level(&self) -> &'static str {
        let rate = self.success_rate();
        let conf = self.average_confidence();
        if rate >= 0.95 && conf >= 0.9 {
            "Low Risk"
        } else if rate >= 0.8 && conf >= 0.7 {
            "Medium Risk"
        } else if rate >= 0.5 {
            "High Risk"
        } else {
            "Critical Risk"
        }
    }
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        (d.as_millis() as u64).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}
