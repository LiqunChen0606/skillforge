use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

use aif_core::ast::{Block, BlockKind, SkillBlockType};

#[derive(Parser)]
#[command(name = "aif")]
#[command(about = "AIF: AI-native Interchange Format compiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an AIF document to an output format
    Compile {
        /// Input .aif file
        input: PathBuf,
        /// Output format: html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, lml-hybrid, json, binary-wire, binary-token, pdf
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import a Markdown or PDF file to AIF IR (JSON)
    Import {
        /// Input file (Markdown or PDF)
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Dump the parsed IR as JSON
    DumpIr {
        /// Input .aif file
        input: PathBuf,
    },
    /// Skill-related operations
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Chunk a document into addressable sub-document units
    Chunk {
        #[command(subcommand)]
        action: ChunkAction,
    },
    /// Print JSON Schema for the AIF Document type
    Schema {},
}

#[derive(Subcommand)]
enum SkillAction {
    /// Import a SKILL.md file to AIF IR (JSON)
    Import {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format: json, html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, lml-hybrid, binary-wire, binary-token
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Export an AIF skill to SKILL.md format
    Export {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify integrity hash of a skill
    Verify {
        input: PathBuf,
    },
    /// Recompute and update hash for a skill
    Rehash {
        input: PathBuf,
    },
    /// Show skill metadata
    Inspect {
        input: PathBuf,
    },
    /// Compare two skill versions and show changes
    Diff {
        /// Old version .aif file
        old: PathBuf,
        /// New version .aif file
        new: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Auto-bump version based on semantic changes
    Bump {
        input: PathBuf,
        /// Show what would change without modifying
        #[arg(long)]
        dry_run: bool,
    },
    /// Show dependency tree of a skill
    Deps {
        input: PathBuf,
    },
    /// Resolve and display execution chain for a skill
    Chain {
        input: PathBuf,
    },
    /// Compose a dependency chain into a single document
    Compose {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Search remote registry for skills
    Search {
        query: String,
        #[arg(long)]
        tags: Option<String>,
    },
    /// Publish a skill to the remote registry
    Publish {
        input: PathBuf,
    },
    /// Install a skill from the remote registry
    Install {
        name: String,
        #[arg(long)]
        version: Option<String>,
    },
    /// Show remote skill metadata
    Info {
        name: String,
        #[arg(long)]
        version: Option<String>,
    },
}

#[derive(Subcommand)]
enum ChunkAction {
    /// Chunk a document with a given strategy
    Split {
        /// Input .aif file
        input: PathBuf,
        /// Chunking strategy: section, token-budget, semantic, fixed-blocks
        #[arg(long, default_value = "token-budget")]
        strategy: String,
        /// Max tokens per chunk (for token-budget strategy)
        #[arg(long, default_value = "2048")]
        max_tokens: usize,
        /// Blocks per chunk (for fixed-blocks strategy)
        #[arg(long, default_value = "5")]
        blocks_per_chunk: usize,
        /// Output directory for individual chunk files
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Build a chunk graph from multiple documents
    Graph {
        /// Input .aif files
        inputs: Vec<PathBuf>,
        /// Output JSON file for the graph
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn find_skill_block(blocks: &[Block]) -> Option<&Block> {
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

fn write_output(content: &str, output: Option<&PathBuf>) {
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

fn read_source(input: &PathBuf) -> String {
    fs::read_to_string(input).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", input.display(), e);
        std::process::exit(1);
    })
}

fn parse_aif(source: &str) -> aif_core::ast::Document {
    aif_parser::parse(source).unwrap_or_else(|errors| {
        for e in &errors {
            eprintln!("{}", e);
        }
        std::process::exit(1);
    })
}

/// Load the local skill registry from the default path (~/.aif/registry.json).
fn load_local_registry() -> aif_skill::registry::Registry {
    let registry_path = dirs_or_default().join("registry.json");
    if registry_path.exists() {
        aif_skill::registry::Registry::load(&registry_path).unwrap_or_else(|e| {
            eprintln!("Warning: failed to load registry at {}: {}", registry_path.display(), e);
            aif_skill::registry::Registry::new(registry_path)
        })
    } else {
        aif_skill::registry::Registry::new(registry_path)
    }
}

fn dirs_or_default() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".aif"))
        .unwrap_or_else(|| PathBuf::from("/tmp/aif"))
}

fn handle_skill(action: SkillAction) {
    match action {
        SkillAction::Import { input, output, format } => {
            let source = read_source(&input);
            let result = aif_skill::import::import_skill_md(&source);

            // Print mappings to stderr
            for mapping in &result.mappings {
                eprintln!(
                    "  {} -> {:?} ({:?})",
                    mapping.heading, mapping.mapped_to, mapping.confidence
                );
            }

            // Wrap the skill block in a Document
            let doc = aif_core::ast::Document {
                metadata: std::collections::BTreeMap::new(),
                blocks: vec![result.block],
            };

            // Binary formats need raw byte output, not text
            match format.as_str() {
                "binary-wire" | "binary-token" => {
                    let bytes = if format == "binary-wire" {
                        aif_binary::render_wire(&doc)
                    } else {
                        aif_binary::render_token_optimized(&doc)
                    };
                    if let Some(output_path) = output.as_ref() {
                        std::fs::write(output_path, &bytes).unwrap_or_else(|e| {
                            eprintln!("Error writing {}: {}", output_path.display(), e);
                            std::process::exit(1);
                        });
                        eprintln!("Wrote {} ({} bytes)", output_path.display(), bytes.len());
                    } else {
                        use std::io::Write;
                        std::io::stdout().write_all(&bytes).unwrap();
                    }
                    return;
                }
                _ => {}
            }

            let output_text = match format.as_str() {
                "json" => serde_json::to_string_pretty(&doc).unwrap(),
                "html" => aif_html::render_html(&doc),
                "markdown" | "md" => aif_markdown::render_markdown(&doc),
                "lml" => aif_lml::render_lml(&doc),
                "lml-compact" => aif_lml::render_lml_skill_compact(&doc),
                "lml-conservative" => aif_lml::render_lml_conservative(&doc),
                "lml-moderate" => aif_lml::render_lml_moderate(&doc),
                "lml-aggressive" => aif_lml::render_lml_aggressive(&doc),
                "lml-hybrid" => aif_lml::render_lml_hybrid(&doc),
                _ => {
                    eprintln!(
                        "Unknown format: {}. Supported: json, html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, lml-hybrid, binary-wire, binary-token",
                        format
                    );
                    std::process::exit(1);
                }
            };
            write_output(&output_text, output.as_ref());
        }
        SkillAction::Export { input, output } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            let md = aif_skill::export::export_skill_md(skill_block);
            write_output(&md, output.as_ref());
        }
        SkillAction::Verify { input } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            match aif_skill::hash::verify_skill_hash(skill_block) {
                aif_skill::hash::HashVerifyResult::Valid => {
                    println!("Valid: hash matches content.");
                }
                aif_skill::hash::HashVerifyResult::Mismatch { expected, actual } => {
                    println!("Mismatch: expected {}, computed {}", expected, actual);
                    std::process::exit(1);
                }
                aif_skill::hash::HashVerifyResult::NoHash => {
                    println!("No hash attribute found on skill block.");
                }
                aif_skill::hash::HashVerifyResult::NotASkill => {
                    eprintln!("Block is not a skill block.");
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Rehash { input } => {
            let source = read_source(&input);
            let mut doc = parse_aif(&source);

            let skill_block = doc.blocks.iter_mut().find(|b| {
                matches!(
                    &b.kind,
                    BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Skill,
                        ..
                    }
                )
            });

            if let Some(block) = skill_block {
                let hash = aif_skill::hash::compute_skill_hash(block);
                if let BlockKind::SkillBlock { ref mut attrs, .. } = block.kind {
                    attrs.pairs.insert("hash".to_string(), hash.clone());
                }
                // Write back as JSON (the canonical serialization)
                let json = serde_json::to_string_pretty(&doc).unwrap();
                fs::write(&input, &json).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", input.display(), e);
                    std::process::exit(1);
                });
                println!("Updated hash: {}", hash);
            } else {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            }
        }
        SkillAction::Diff { old, new, format: _format } => {
            let old_source = read_source(&old);
            let old_doc = parse_aif(&old_source);
            let new_source = read_source(&new);
            let new_doc = parse_aif(&new_source);

            let old_block = find_skill_block(&old_doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", old.display());
                std::process::exit(1);
            });
            let new_block = find_skill_block(&new_doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", new.display());
                std::process::exit(1);
            });

            let changes = aif_skill::diff::diff_skills(old_block, new_block);
            if changes.is_empty() {
                println!("No changes detected.");
                return;
            }

            let bump = aif_skill::classify::highest_bump(&changes);
            for change in &changes {
                let class = aif_skill::classify::classify_change(change);
                println!("  [{:?}] {:?}: {}", class, change.kind, change.description);
            }
            println!("\nRecommended bump: {:?}", bump);
        }
        SkillAction::Bump { input, dry_run } => {
            let source = read_source(&input);
            let mut doc = parse_aif(&source);

            let skill_block = doc.blocks.iter_mut().find(|b| {
                matches!(
                    &b.kind,
                    BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Skill,
                        ..
                    }
                )
            });

            if let Some(block) = skill_block {
                let current = if let BlockKind::SkillBlock { ref attrs, .. } = block.kind {
                    attrs
                        .get("version")
                        .and_then(aif_skill::version::Semver::parse)
                        .unwrap_or_default()
                } else {
                    aif_skill::version::Semver::default()
                };

                // For bump without a diff target, just do patch bump
                let new_version = current.bump(aif_skill::version::BumpLevel::Patch);

                if dry_run {
                    println!("Current: {}", current);
                    println!("Would bump to: {}", new_version);
                } else {
                    if let BlockKind::SkillBlock { ref mut attrs, .. } = block.kind {
                        attrs.pairs.insert("version".to_string(), new_version.to_string());
                    }
                    let hash = aif_skill::hash::compute_skill_hash(block);
                    if let BlockKind::SkillBlock { ref mut attrs, .. } = block.kind {
                        attrs.pairs.insert("hash".to_string(), hash.clone());
                    }
                    let json = serde_json::to_string_pretty(&doc).unwrap();
                    fs::write(&input, &json).unwrap_or_else(|e| {
                        eprintln!("Error writing {}: {}", input.display(), e);
                        std::process::exit(1);
                    });
                    println!("Bumped {} -> {} (hash: {})", current, new_version, hash);
                }
            } else {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            }
        }
        SkillAction::Inspect { input } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            if let BlockKind::SkillBlock {
                attrs, children, ..
            } = &skill_block.kind
            {
                println!("Skill Metadata:");
                if let Some(name) = attrs.get("name") {
                    println!("  name: {}", name);
                }
                if let Some(version) = attrs.get("version") {
                    println!("  version: {}", version);
                }
                if let Some(tags) = attrs.get("tags") {
                    println!("  tags: {}", tags);
                }
                if let Some(priority) = attrs.get("priority") {
                    println!("  priority: {}", priority);
                }
                if let Some(hash) = attrs.get("hash") {
                    println!("  hash: {}", hash);
                }
                // Print remaining attrs not already printed
                for (key, value) in &attrs.pairs {
                    match key.as_str() {
                        "name" | "version" | "tags" | "priority" | "hash" => {}
                        _ => println!("  {}: {}", key, value),
                    }
                }
                println!("  children: {}", children.len());
                for child in children {
                    if let BlockKind::SkillBlock {
                        skill_type,
                        attrs: child_attrs,
                        ..
                    } = &child.kind
                    {
                        let order_info = child_attrs
                            .get("order")
                            .map(|o| format!(" (order={})", o))
                            .unwrap_or_default();
                        println!("    - {:?}{}", skill_type, order_info);
                    }
                }
            }
        }
        SkillAction::Deps { input } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            if let BlockKind::SkillBlock { attrs, .. } = &skill_block.kind {
                let name = attrs.get("name").unwrap_or("(unnamed)");
                let deps = aif_skill::chain::parse_requires(attrs);
                println!("Skill: {}", name);
                if deps.is_empty() {
                    println!("  No dependencies.");
                } else {
                    println!("  Dependencies:");
                    for dep in &deps {
                        println!("    - {}: {}", dep.name, dep.constraint);
                    }
                }
            }
        }
        SkillAction::Chain { input } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            let (name, deps) = aif_skill::chain::extract_skill_info(skill_block)
                .unwrap_or_else(|| {
                    eprintln!("Invalid skill block (missing name)");
                    std::process::exit(1);
                });

            if deps.is_empty() {
                println!("Skill '{}' has no dependencies. Execution order: [{}]", name, name);
            } else {
                let registry = load_local_registry();
                match aif_skill::chain::resolve_chain(&name, &registry) {
                    Ok(result) => {
                        println!("Execution order for '{}':", name);
                        for (i, skill_name) in result.order.iter().enumerate() {
                            let version = result.resolved.get(skill_name)
                                .map(|v| format!(" v{}", v))
                                .unwrap_or_default();
                            println!("  {}. {}{}", i + 1, skill_name, version);
                        }
                    }
                    Err(e) => {
                        eprintln!("Chain resolution failed: {}", e);
                        eprintln!("\nDirect dependencies:");
                        for dep in &deps {
                            println!("  - {}: {}", dep.name, dep.constraint);
                        }
                        eprintln!("\nEnsure all dependencies are registered with `aif skill register`.");
                        std::process::exit(1);
                    }
                }
            }
        }
        SkillAction::Compose { input, output } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            let (name, _deps) = aif_skill::chain::extract_skill_info(skill_block)
                .unwrap_or_else(|| {
                    eprintln!("Invalid skill block (missing name)");
                    std::process::exit(1);
                });

            let registry = load_local_registry();
            match aif_skill::chain::resolve_chain(&name, &registry) {
                Ok(result) => {
                    match aif_skill::chain::compose_chain(&result.order, &registry) {
                        Ok(composed) => {
                            let json = serde_json::to_string_pretty(&composed).unwrap();
                            write_output(&json, output.as_ref());
                        }
                        Err(e) => {
                            eprintln!("Composition failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Chain resolution failed: {}", e);
                    eprintln!("Ensure all dependencies are registered with `aif skill register`.");
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Search { query, tags } => {
            let config = aif_skill::remote::RemoteConfig::from_env();
            let remote = aif_skill::remote::RemoteRegistry::new(config);
            let tag_list: Vec<&str> = tags
                .as_deref()
                .map(|t| t.split(',').collect())
                .unwrap_or_default();

            match remote.search(&query, &tag_list) {
                Ok(response) => {
                    println!("Found {} results (page {}):", response.total, response.page);
                    for entry in &response.results {
                        println!(
                            "  {} v{} — {}",
                            entry.name,
                            entry.version,
                            entry.description.as_deref().unwrap_or("(no description)")
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Search failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Publish { input } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            let (name, _deps) = aif_skill::chain::extract_skill_info(skill_block)
                .unwrap_or_else(|| {
                    eprintln!("Invalid skill block (missing name)");
                    std::process::exit(1);
                });

            let version = if let BlockKind::SkillBlock { attrs, .. } = &skill_block.kind {
                attrs.get("version").unwrap_or("0.1.0").to_string()
            } else {
                "0.1.0".to_string()
            };

            let config = aif_skill::remote::RemoteConfig::from_env();
            let remote = aif_skill::remote::RemoteRegistry::new(config);
            let data = fs::read(&input).unwrap_or_else(|e| {
                eprintln!("Error reading {}: {}", input.display(), e);
                std::process::exit(1);
            });

            match remote.publish(&name, &version, &data) {
                Ok(()) => println!("Published {} v{}", name, version),
                Err(e) => {
                    eprintln!("Publish failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Install { name, version } => {
            let config = aif_skill::remote::RemoteConfig::from_env();
            let remote = aif_skill::remote::RemoteRegistry::new(config);
            let ver = version.as_deref().unwrap_or("latest");

            match remote.download(&name, ver) {
                Ok(data) => {
                    let cache_dir = std::env::var("HOME")
                        .map(|h| std::path::PathBuf::from(h).join(".aif/cache/skills").join(&name))
                        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/aif/cache/skills").join(&name));

                    fs::create_dir_all(&cache_dir).unwrap_or_else(|e| {
                        eprintln!("Error creating cache dir: {}", e);
                        std::process::exit(1);
                    });
                    let file_path = cache_dir.join(format!("{}.aif", ver));
                    fs::write(&file_path, &data).unwrap_or_else(|e| {
                        eprintln!("Error writing cache: {}", e);
                        std::process::exit(1);
                    });
                    println!("Installed {} v{} to {}", name, ver, file_path.display());
                }
                Err(e) => {
                    eprintln!("Install failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Info { name, version } => {
            let config = aif_skill::remote::RemoteConfig::from_env();
            let remote = aif_skill::remote::RemoteRegistry::new(config);

            match remote.fetch_metadata(&name, version.as_deref()) {
                Ok(entry) => {
                    println!("Name: {}", entry.name);
                    println!("Version: {}", entry.version);
                    println!("Hash: {}", entry.hash);
                    if let Some(desc) = &entry.description {
                        println!("Description: {}", desc);
                    }
                    if !entry.tags.is_empty() {
                        println!("Tags: {}", entry.tags.join(", "));
                    }
                    if !entry.requires.is_empty() {
                        println!("Requires: {}", entry.requires.join(", "));
                    }
                    if let Some(author) = &entry.author {
                        println!("Author: {}", author);
                    }
                    if let Some(ts) = &entry.published_at {
                        println!("Published: {}", ts);
                    }
                }
                Err(e) => {
                    eprintln!("Info failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            format,
            output,
        } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            // Binary and PDF formats need raw byte output, not text
            match format.as_str() {
                "binary-wire" | "binary-token" | "pdf" => {
                    let bytes = match format.as_str() {
                        "binary-wire" => aif_binary::render_wire(&doc),
                        "binary-token" => aif_binary::render_token_optimized(&doc),
                        "pdf" => aif_pdf::export::export_pdf(&doc).unwrap_or_else(|e| {
                            eprintln!("PDF export error: {}", e);
                            std::process::exit(1);
                        }),
                        _ => unreachable!(),
                    };
                    if let Some(output_path) = output.as_ref() {
                        std::fs::write(output_path, &bytes).unwrap_or_else(|e| {
                            eprintln!("Error writing {}: {}", output_path.display(), e);
                            std::process::exit(1);
                        });
                        eprintln!("Wrote {} ({} bytes)", output_path.display(), bytes.len());
                    } else {
                        use std::io::Write;
                        std::io::stdout().write_all(&bytes).unwrap();
                    }
                    return;
                }
                _ => {}
            }

            let result = match format.as_str() {
                "html" => aif_html::render_html(&doc),
                "markdown" | "md" => aif_markdown::render_markdown(&doc),
                "lml" => aif_lml::render_lml(&doc),
                "lml-compact" => aif_lml::render_lml_skill_compact(&doc),
                "lml-conservative" => aif_lml::render_lml_conservative(&doc),
                "lml-moderate" => aif_lml::render_lml_moderate(&doc),
                "lml-aggressive" => aif_lml::render_lml_aggressive(&doc),
                "lml-hybrid" => aif_lml::render_lml_hybrid(&doc),
                "json" => serde_json::to_string_pretty(&doc).unwrap(),
                _ => {
                    eprintln!(
                        "Unknown format: {}. Supported: html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, lml-hybrid, json, binary-wire, binary-token, pdf",
                        format
                    );
                    std::process::exit(1);
                }
            };

            write_output(&result, output.as_ref());
        }
        Commands::Import { input, output } => {
            let is_pdf = input
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                .unwrap_or(false);

            if is_pdf {
                let pdf_bytes = fs::read(&input).unwrap_or_else(|e| {
                    eprintln!("Error reading {}: {}", input.display(), e);
                    std::process::exit(1);
                });
                let result = aif_pdf::import::import_pdf(&pdf_bytes).unwrap_or_else(|e| {
                    eprintln!("PDF import error: {}", e);
                    std::process::exit(1);
                });
                eprintln!(
                    "Imported {} pages, {} blocks, avg confidence: {:.2}",
                    result.page_count,
                    result.document.blocks.len(),
                    result.avg_confidence
                );
                for diag in &result.diagnostics {
                    eprintln!(
                        "  [page {}] {:?}: {}",
                        diag.page, diag.kind, diag.message
                    );
                }
                let json = serde_json::to_string_pretty(&result.document).unwrap();
                write_output(&json, output.as_ref());
            } else {
                let source = read_source(&input);
                let doc = aif_markdown::import_markdown(&source);
                let json = serde_json::to_string_pretty(&doc).unwrap();
                write_output(&json, output.as_ref());
            }
        }
        Commands::DumpIr { input } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);
            let json = serde_json::to_string_pretty(&doc).unwrap();
            println!("{}", json);
        }
        Commands::Skill { action } => {
            handle_skill(action);
        }
        Commands::Chunk { action } => {
            handle_chunk(action);
        }
        Commands::Schema {} => {
            println!("{}", aif_core::schema::generate_schema());
        }
    }
}

fn handle_chunk(action: ChunkAction) {
    match action {
        ChunkAction::Split {
            input,
            strategy,
            max_tokens,
            blocks_per_chunk,
            output,
        } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let chunk_strategy = match strategy.as_str() {
                "section" => aif_core::chunk::ChunkStrategy::Section,
                "token-budget" => aif_core::chunk::ChunkStrategy::TokenBudget { max_tokens },
                "semantic" => aif_core::chunk::ChunkStrategy::Semantic,
                "fixed-blocks" => {
                    aif_core::chunk::ChunkStrategy::FixedBlocks { blocks_per_chunk }
                }
                _ => {
                    eprintln!(
                        "Unknown strategy: {}. Supported: section, token-budget, semantic, fixed-blocks",
                        strategy
                    );
                    std::process::exit(1);
                }
            };

            let chunks = aif_pdf::chunk::chunk_document(
                &doc,
                input.to_str().unwrap_or("input.aif"),
                chunk_strategy,
            )
            .unwrap_or_else(|e| {
                eprintln!("Chunking error: {}", e);
                std::process::exit(1);
            });

            eprintln!("Produced {} chunks", chunks.len());

            if let Some(output_dir) = output {
                fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
                    eprintln!("Error creating output dir: {}", e);
                    std::process::exit(1);
                });
                for chunk in &chunks {
                    let filename = format!("chunk_{}.json", chunk.metadata.sequence);
                    let path = output_dir.join(&filename);
                    let json = serde_json::to_string_pretty(chunk).unwrap();
                    fs::write(&path, &json).unwrap_or_else(|e| {
                        eprintln!("Error writing {}: {}", path.display(), e);
                        std::process::exit(1);
                    });
                }
                eprintln!("Wrote {} chunk files to {}", chunks.len(), output_dir.display());
            } else {
                // Print summary to stdout
                for chunk in &chunks {
                    println!(
                        "{} | blocks: {} | tokens: ~{} | title: {}",
                        chunk.id,
                        chunk.blocks.len(),
                        chunk.metadata.estimated_tokens,
                        chunk.metadata.title.as_deref().unwrap_or("(none)")
                    );
                }
            }
        }
        ChunkAction::Graph { inputs, output } => {
            let mut graph = aif_core::chunk::ChunkGraph::new();

            for input in &inputs {
                let source = read_source(input);
                let doc = parse_aif(&source);
                let doc_path = input.to_str().unwrap_or("unknown");

                let chunks = aif_pdf::chunk::chunk_document(
                    &doc,
                    doc_path,
                    aif_core::chunk::ChunkStrategy::Section,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Chunking error for {}: {}", input.display(), e);
                    std::process::exit(1);
                });

                let doc_hash = aif_pdf::chunk::compute_doc_hash(
                    &serde_json::to_string(&doc).unwrap_or_default(),
                );
                graph.documents.insert(
                    doc_path.to_string(),
                    aif_core::chunk::DocumentEntry {
                        path: doc_path.to_string(),
                        content_hash: doc_hash,
                        chunk_count: chunks.len(),
                        title: doc.metadata.get("title").cloned(),
                    },
                );

                // Add continuation links between sequential chunks
                for i in 0..chunks.len() {
                    if i > 0 {
                        graph.add_link(aif_core::chunk::ChunkLink {
                            source: chunks[i - 1].id.clone(),
                            target: chunks[i].id.clone(),
                            link_type: aif_core::chunk::LinkType::Continuation,
                            label: None,
                        });
                    }
                    graph.add_chunk(chunks[i].clone());
                }
            }

            let json = serde_json::to_string_pretty(&graph).unwrap();
            if let Some(output_path) = output {
                fs::write(&output_path, &json).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", output_path.display(), e);
                    std::process::exit(1);
                });
                eprintln!(
                    "Wrote graph ({} chunks, {} links, {} documents) to {}",
                    graph.chunks.len(),
                    graph.links.len(),
                    graph.documents.len(),
                    output_path.display()
                );
            } else {
                println!("{}", json);
            }
        }
    }
}
