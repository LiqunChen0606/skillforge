use crate::args::MigrateAction;
use crate::util::{dirs_or_default, find_skill_block, parse_aif, read_source, read_source_directory};

pub fn handle_migrate(action: MigrateAction) {
    match action {
        MigrateAction::Validate { input } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No @skill block found in {}", input.display());
                std::process::exit(1);
            });

            let results = aif_migrate::validate::validate_migration_skill(skill_block);
            let all_passed = results.iter().all(|r| r.passed);

            for r in &results {
                let icon = if r.passed { "PASS" } else { "FAIL" };
                eprintln!("  [{}] {:?}: {}", icon, r.check, r.message);
            }

            if all_passed {
                eprintln!("\nMigration skill validation passed.");
            } else {
                eprintln!("\nMigration skill validation failed.");
                std::process::exit(1);
            }
        }
        MigrateAction::Run {
            skill,
            source,
            output,
            strategy,
            max_repairs,
            report,
        } => {
            // 1. Parse chunk strategy
            let chunk_strategy = match strategy.as_str() {
                "file" => aif_migrate::chunk::ChunkStrategy::FilePerChunk,
                "directory" => aif_migrate::chunk::ChunkStrategy::DirectoryChunk,
                "token-budget" => {
                    aif_migrate::chunk::ChunkStrategy::TokenBudget { max_tokens: 4000 }
                }
                other => {
                    eprintln!(
                        "Unknown strategy: {}. Use: file, directory, token-budget",
                        other
                    );
                    std::process::exit(1);
                }
            };

            // 2. Read and parse the skill file
            let skill_source = read_source(&skill);
            let skill_doc = parse_aif(&skill_source);
            let skill_block = find_skill_block(&skill_doc.blocks).unwrap_or_else(|| {
                eprintln!("No @skill block found in {}", skill.display());
                std::process::exit(1);
            });

            // 3. Read source files from the source directory
            let source_files = read_source_directory(&source);
            if source_files.is_empty() {
                eprintln!("No source files found in {}", source.display());
                std::process::exit(1);
            }
            eprintln!(
                "Found {} source file(s) in {}",
                source_files.len(),
                source.display()
            );

            // 4. Build migration config
            let config = aif_migrate::types::MigrationConfig {
                skill_path: skill.clone(),
                source_dir: source.clone(),
                output_dir: output.clone(),
                max_repair_iterations: max_repairs,
                file_patterns: Vec::new(),
                chunk_strategy,
                dry_run: false,
            };

            // 5. Determine apply_fn: use LLM if configured, otherwise placeholder
            let config_path = dirs_or_default().join("config.toml");
            let aif_config = aif_core::config::AifConfig::load_with_env(&config_path);

            let api_key = std::env::var("AIF_LLM_API_KEY")
                .ok()
                .or_else(|| aif_config.llm.api_key.clone());

            let apply_fn: aif_migrate::llm::ApplyFn = if let Some(key) = api_key {
                let model = aif_config
                    .llm
                    .model
                    .clone()
                    .unwrap_or_else(|| "claude-sonnet-4-5-20250514".to_string());
                eprintln!("Using LLM for migration (model: {})", model);
                aif_migrate::llm::make_llm_apply_fn(key, model)
            } else {
                eprintln!("Note: No LLM API key configured. Running with placeholder (returns original content unchanged).");
                eprintln!("For real migrations, set AIF_LLM_API_KEY or run: aif config set llm.api-key <key>");
                eprintln!();
                Box::new(
                    |_steps: &[String],
                     source_code: &str,
                     _repair_ctx: Option<&str>|
                     -> Option<String> { Some(source_code.to_string()) },
                )
            };

            // 6. Run the migration engine
            let engine = aif_migrate::engine::MigrationEngine::new(config);
            match engine.run(skill_block, &source_files, apply_fn) {
                Ok(migration_report) => {
                    match report.as_str() {
                        "json" => {
                            let json = serde_json::to_string_pretty(&migration_report)
                                .unwrap_or_else(|e| {
                                    eprintln!("Error serializing report: {}", e);
                                    std::process::exit(1);
                                });
                            println!("{}", json);
                        }
                        _ => {
                            // Text report: generate AIF report document and render as markdown
                            let report_doc =
                                aif_migrate::report::generate_report_document(&migration_report);
                            let md = aif_markdown::render_markdown(&report_doc);
                            println!("{}", md);
                        }
                    }
                    eprintln!(
                        "\nMigration complete. Success rate: {:.0}%",
                        migration_report.success_rate() * 100.0
                    );
                    if !migration_report.all_passed() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Migration failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
