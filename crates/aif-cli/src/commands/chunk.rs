use std::fs;

use crate::args::ChunkAction;
use crate::util::{parse_aif, read_source};

pub fn handle_chunk(action: ChunkAction) {
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
                eprintln!(
                    "Wrote {} chunk files to {}",
                    chunks.len(),
                    output_dir.display()
                );
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
        ChunkAction::Lint { input, format } => {
            let source = read_source(&input);
            let graph: aif_core::chunk::ChunkGraph =
                serde_json::from_str(&source).unwrap_or_else(|e| {
                    eprintln!("Error parsing chunk graph JSON: {}", e);
                    std::process::exit(1);
                });
            let results = aif_core::lint::lint_chunk_graph(&graph);
            let (total, passed, failed) = aif_core::lint::lint_summary(&results);

            if format == "json" {
                let json_results: Vec<_> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "check": format!("{:?}", r.check),
                            "passed": r.passed,
                            "severity": format!("{:?}", r.severity),
                            "message": r.message,
                            "block_id": r.block_id,
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "file": input.display().to_string(),
                        "total": total,
                        "passed": passed,
                        "failed": failed,
                        "results": json_results,
                    }))
                    .unwrap()
                );
            } else {
                println!("Chunk Graph Lint: {}", input.display());
                println!("{}", "=".repeat(60));
                for r in &results {
                    if r.passed {
                        println!("  [+] {:?}", r.check);
                    } else {
                        let sev = match r.severity {
                            aif_core::lint::DocLintSeverity::Error => "ERROR",
                            aif_core::lint::DocLintSeverity::Warning => "WARN",
                        };
                        let loc = r
                            .block_id
                            .as_ref()
                            .map(|id| format!(" ({})", id))
                            .unwrap_or_default();
                        println!(
                            "  [x] {:?} [{}]{}: {}",
                            r.check, sev, loc, r.message
                        );
                    }
                }
                println!("{}", "-".repeat(60));
                println!("{} checks: {} passed, {} failed", total, passed, failed);
                if failed > 0 {
                    std::process::exit(1);
                }
            }
        }
    }
}
