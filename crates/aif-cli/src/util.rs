use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use aif_core::ast::{Block, BlockKind, SkillBlockType};

pub fn find_skill_block(blocks: &[Block]) -> Option<&Block> {
    blocks.iter().find(|b| {
        matches!(
            &b.kind,
            BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                ..
            }
        )
    })
}

pub fn write_output(content: &str, output: Option<&PathBuf>) {
    if let Some(output_path) = output {
        fs::write(output_path, content).unwrap_or_else(|e| {
            eprintln!("Error writing {}: {}", output_path.display(), e);
            std::process::exit(1);
        });
        eprintln!("Wrote {}", output_path.display());
    } else {
        print!("{}", content);
    }
}

pub fn read_source(input: &PathBuf) -> String {
    fs::read_to_string(input).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", input.display(), e);
        std::process::exit(1);
    })
}

/// Recursively read all files from a directory into a HashMap<PathBuf, String>.
/// Keys are relative paths from the directory root.
pub fn read_source_directory(dir: &Path) -> HashMap<PathBuf, String> {
    let mut files = HashMap::new();
    if !dir.is_dir() {
        eprintln!("Error: {} is not a directory", dir.display());
        std::process::exit(1);
    }
    fn walk(base: &Path, current: &Path, files: &mut HashMap<PathBuf, String>) {
        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(base, &path, files);
                } else if path.is_file() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let relative = path.strip_prefix(base).unwrap_or(&path).to_path_buf();
                        files.insert(relative, content);
                    }
                }
            }
        }
    }
    walk(dir, dir, &mut files);
    files
}

pub fn parse_aif(source: &str) -> aif_core::ast::Document {
    aif_parser::parse(source).unwrap_or_else(|errors| {
        for e in &errors {
            eprintln!("{}", e);
        }
        std::process::exit(1);
    })
}

/// Load the local skill registry from the default path (~/.aif/registry.json).
pub fn load_local_registry() -> aif_skill::registry::Registry {
    let registry_path = dirs_or_default().join("registry.json");
    if registry_path.exists() {
        aif_skill::registry::Registry::load(&registry_path).unwrap_or_else(|e| {
            eprintln!(
                "Warning: failed to load registry at {}: {}",
                registry_path.display(),
                e
            );
            aif_skill::registry::Registry::new(registry_path)
        })
    } else {
        aif_skill::registry::Registry::new(registry_path)
    }
}

pub fn dirs_or_default() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".aif"))
        .unwrap_or_else(|| PathBuf::from("/tmp/aif"))
}

/// Run semantic inference on an imported document.
/// If `infer_llm` is true, uses LLM-assisted inference (pattern rules first, then LLM for unmatched).
/// If `infer_semantics` is true (but not `infer_llm`), uses pattern-only inference.
pub fn run_inference(doc: &mut aif_core::ast::Document, infer_semantics: bool, infer_llm: bool) {
    if !infer_semantics && !infer_llm {
        return;
    }

    if infer_llm {
        // Load LLM config
        let config_path = dirs_or_default().join("config.toml");
        let aif_config = aif_core::config::AifConfig::load_with_env(&config_path);

        if aif_config.llm.api_key.is_none() {
            eprintln!("Warning: --infer-llm requested but no API key configured.");
            eprintln!("  Set AIF_LLM_API_KEY or run: aif config set llm.api-key <key>");
            eprintln!("  Falling back to pattern-only inference.");
            aif_core::infer::annotate_semantics(doc, &aif_core::infer::InferConfig::default());
        } else {
            let infer_config = aif_core::infer::InferConfig {
                min_confidence: 0.5,
                strategy: aif_core::infer::InferStrategy::Llm(aif_config.llm),
            };
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(aif_core::infer::annotate_semantics_with_llm(
                doc,
                &infer_config,
            ));
        }
    } else {
        aif_core::infer::annotate_semantics(doc, &aif_core::infer::InferConfig::default());
    }

    let inferred_count = doc
        .blocks
        .iter()
        .filter(|b| {
            matches!(&b.kind, aif_core::ast::BlockKind::SemanticBlock { attrs, .. } if attrs.pairs.contains_key("_aif_inferred"))
        })
        .count();
    if inferred_count > 0 {
        eprintln!("Inferred {} semantic block(s)", inferred_count);
    }
}

pub fn truncate_text(text: &str, max_len: usize) -> String {
    let text = text.replace('\n', " ");
    if text.len() <= max_len {
        text
    } else {
        format!("{}...", &text[..max_len])
    }
}
