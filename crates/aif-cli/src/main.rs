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
        /// Output format: html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, json, binary-wire, binary-token
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import a Markdown file to AIF IR (JSON)
    Import {
        /// Input Markdown file
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
}

#[derive(Subcommand)]
enum SkillAction {
    /// Import a SKILL.md file to AIF IR (JSON)
    Import {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format: json, html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, binary-wire, binary-token
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
                _ => {
                    eprintln!(
                        "Unknown format: {}. Supported: json, html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, binary-wire, binary-token",
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

            let result = match format.as_str() {
                "html" => aif_html::render_html(&doc),
                "markdown" | "md" => aif_markdown::render_markdown(&doc),
                "lml" => aif_lml::render_lml(&doc),
                "lml-compact" => aif_lml::render_lml_skill_compact(&doc),
                "lml-conservative" => aif_lml::render_lml_conservative(&doc),
                "lml-moderate" => aif_lml::render_lml_moderate(&doc),
                "lml-aggressive" => aif_lml::render_lml_aggressive(&doc),
                "json" => serde_json::to_string_pretty(&doc).unwrap(),
                _ => {
                    eprintln!(
                        "Unknown format: {}. Supported: html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, json, binary-wire, binary-token",
                        format
                    );
                    std::process::exit(1);
                }
            };

            write_output(&result, output.as_ref());
        }
        Commands::Import { input, output } => {
            let source = read_source(&input);
            let doc = aif_markdown::import_markdown(&source);
            let json = serde_json::to_string_pretty(&doc).unwrap();
            write_output(&json, output.as_ref());
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
    }
}
