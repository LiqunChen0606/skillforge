use std::fs;

use aif_core::ast::BlockKind;

use crate::args::SkillAction;
use crate::util::{
    dirs_or_default, find_skill_block, load_local_registry, parse_aif, read_source, write_output,
};

pub fn handle_skill(action: SkillAction) {
    match action {
        SkillAction::Import {
            input,
            output,
            format,
        } => {
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
                        skill_type: aif_core::ast::SkillBlockType::Skill,
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
        SkillAction::Diff {
            old,
            new,
            format: _format,
        } => {
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
                println!(
                    "  [{:?}] {:?}: {}",
                    class, change.kind, change.description
                );
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
                        skill_type: aif_core::ast::SkillBlockType::Skill,
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
                        attrs
                            .pairs
                            .insert("version".to_string(), new_version.to_string());
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
                println!(
                    "Skill '{}' has no dependencies. Execution order: [{}]",
                    name, name
                );
            } else {
                let registry = load_local_registry();
                match aif_skill::chain::resolve_chain(&name, &registry) {
                    Ok(result) => {
                        println!("Execution order for '{}':", name);
                        for (i, skill_name) in result.order.iter().enumerate() {
                            let version = result
                                .resolved
                                .get(skill_name)
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
                        eprintln!(
                            "\nEnsure all dependencies are registered with `aif skill register`."
                        );
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
                Ok(result) => match aif_skill::chain::compose_chain(&result.order, &registry) {
                    Ok(composed) => {
                        let json = serde_json::to_string_pretty(&composed).unwrap();
                        write_output(&json, output.as_ref());
                    }
                    Err(e) => {
                        eprintln!("Composition failed: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Chain resolution failed: {}", e);
                    eprintln!(
                        "Ensure all dependencies are registered with `aif skill register`."
                    );
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
                        .map(|h| {
                            std::path::PathBuf::from(h)
                                .join(".aif/cache/skills")
                                .join(&name)
                        })
                        .unwrap_or_else(|_| {
                            std::path::PathBuf::from("/tmp/aif/cache/skills").join(&name)
                        });

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
        SkillAction::Eval {
            input,
            stage,
            report,
        } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            let stage_filter = stage
                .and_then(aif_eval::pipeline::StageFilter::from_stage_number)
                .unwrap_or(aif_eval::pipeline::StageFilter::All);

            let llm_config = if matches!(
                stage_filter,
                aif_eval::pipeline::StageFilter::UpToCompliance
                    | aif_eval::pipeline::StageFilter::All
            ) {
                let config_path = dirs_or_default().join("config.toml");
                let config = aif_core::config::AifConfig::load_with_env(&config_path);
                Some(config.llm)
            } else {
                None
            };

            let pipeline_config = aif_eval::pipeline::PipelineConfig {
                stages: stage_filter,
                llm: llm_config,
                compliance_task: None,
            };

            let pipeline = aif_eval::pipeline::EvalPipeline::new(pipeline_config);

            let eval_report =
                if matches!(stage_filter, aif_eval::pipeline::StageFilter::LintOnly) {
                    pipeline.run_lint(skill_block)
                } else {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(pipeline.run(skill_block, &source))
                };

            match report.as_str() {
                "json" => {
                    print_eval_report_json(&eval_report);
                }
                _ => {
                    print_eval_report_text(&eval_report);
                }
            }

            if !eval_report.all_passed() {
                std::process::exit(1);
            }
        }
        SkillAction::Resolve { input, output } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No skill block found in {}", input.display());
                std::process::exit(1);
            });

            let registry = load_local_registry();
            match aif_skill::inherit::resolve_inheritance(skill_block, &registry) {
                Ok(resolved) => {
                    let resolved_doc = aif_core::ast::Document {
                        metadata: doc.metadata.clone(),
                        blocks: vec![resolved],
                    };
                    let json = serde_json::to_string_pretty(&resolved_doc).unwrap();
                    write_output(&json, output.as_ref());
                }
                Err(e) => {
                    eprintln!("Inheritance resolution failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Keygen {} => {
            let (private_key, public_key) = aif_skill::sign::generate_keypair();
            println!("Private key: {}", private_key);
            println!("Public key:  {}", public_key);
            eprintln!("Store the private key securely. Share only the public key.");
        }
        SkillAction::Sign { input, key } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);
            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No @skill block found");
                std::process::exit(1);
            });
            // Read key from file if it's a path, otherwise use as base64
            let key_str = if std::path::Path::new(&key).exists() {
                std::fs::read_to_string(&key)
                    .unwrap_or_else(|e| {
                        eprintln!("Error reading key file '{}': {}", key, e);
                        std::process::exit(1);
                    })
                    .trim()
                    .to_string()
            } else {
                key
            };
            match aif_skill::sign::sign_skill(skill_block, &key_str) {
                Ok(signature) => {
                    println!("{}", signature);
                    eprintln!("Signature generated. Verify with: aif skill verify-signature --signature <sig> --pubkey <key>");
                }
                Err(e) => {
                    eprintln!("Signing failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        SkillAction::VerifySignature {
            input,
            signature,
            pubkey,
        } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);
            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No @skill block found");
                std::process::exit(1);
            });
            let pubkey_str = if std::path::Path::new(&pubkey).exists() {
                std::fs::read_to_string(&pubkey)
                    .unwrap_or_else(|e| {
                        eprintln!("Error reading pubkey file '{}': {}", pubkey, e);
                        std::process::exit(1);
                    })
                    .trim()
                    .to_string()
            } else {
                pubkey
            };
            match aif_skill::sign::verify_skill(skill_block, &signature, &pubkey_str) {
                Ok(true) => {
                    println!("VALID — signature matches skill content");
                }
                Ok(false) => {
                    println!("INVALID — signature does not match (skill may be tampered)");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Verification error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        SkillAction::Test {
            input,
            format,
            baseline,
            save_baseline,
            output,
        } => {
            let source = read_source(&input);
            let doc = parse_aif(&source);

            let skill_block = find_skill_block(&doc.blocks).unwrap_or_else(|| {
                eprintln!("No @skill block found in {}", input.display());
                std::process::exit(1);
            });

            // Run CI (lint + scenarios with mock/no-LLM for now)
            let ci_result = aif_eval::ci_runner::run_ci(skill_block, |spec| {
                // Without LLM config, scenarios get a placeholder result
                let config_path = dirs_or_default().join("config.toml");
                let config = aif_core::config::AifConfig::load_with_env(&config_path);
                let llm = config.llm;

                if llm.api_key.is_none() || llm.api_key.as_deref() == Some("") {
                    return aif_skill::eval::ScenarioResult {
                        name: spec.name.clone(),
                        passed: false,
                        evidence: "No LLM configured — cannot evaluate scenario. Run `aif config set llm.api-key <key>`".into(),
                        scenario_type: aif_skill::eval::ScenarioType::Scenario,
                    };
                }

                let client = match aif_eval::anthropic::AnthropicClient::new(
                    llm.api_key.as_deref().unwrap_or(""),
                    llm.resolved_model(),
                    llm.base_url.as_deref(),
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        return aif_skill::eval::ScenarioResult {
                            name: spec.name.clone(),
                            passed: false,
                            evidence: format!("LLM client error: {}", e),
                            scenario_type: aif_skill::eval::ScenarioType::Scenario,
                        };
                    }
                };

                let runner = aif_eval::scenario::ScenarioRunner::new(2048);
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(runner.evaluate_one(&client, &source, spec)) {
                    Ok(r) => r,
                    Err(e) => aif_skill::eval::ScenarioResult {
                        name: spec.name.clone(),
                        passed: false,
                        evidence: format!("LLM API error: {}", e),
                        scenario_type: aif_skill::eval::ScenarioType::Scenario,
                    },
                }
            });

            // Extract scenario results for output formatting
            let (exit_code, scenario_results) = match &ci_result {
                aif_eval::ci_runner::CiResult::LintFailed(lint_results) => {
                    let text_output = format_lint_failed(lint_results);
                    write_output(&text_output, output.as_ref());
                    std::process::exit(1);
                }
                aif_eval::ci_runner::CiResult::Completed(results) => {
                    let code = if results.iter().all(|r| r.passed) {
                        0
                    } else {
                        1
                    };
                    (code, results.clone())
                }
            };

            // Compare against baseline if provided
            let mut regressions = Vec::new();
            let mut final_exit_code = exit_code;
            if let Some(baseline_path) = &baseline {
                match aif_eval::baseline::load_baseline(baseline_path) {
                    Ok(bl) => {
                        regressions =
                            aif_eval::baseline::detect_regressions(&bl, &scenario_results);
                        if !regressions.is_empty() {
                            final_exit_code = 2; // regressions detected
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: could not load baseline: {}", e);
                    }
                }
            }

            // Save baseline if requested
            if let Some(save_path) = &save_baseline {
                let skill_name = if let aif_core::ast::BlockKind::SkillBlock { attrs, .. } =
                    &skill_block.kind
                {
                    attrs.get("name").unwrap_or("(unnamed)").to_string()
                } else {
                    "(unnamed)".to_string()
                };

                let bl = aif_eval::baseline::Baseline {
                    skill_name,
                    model: "unknown".into(),
                    timestamp: "now".into(),
                    results: scenario_results.clone(),
                };
                if let Err(e) = aif_eval::baseline::save_baseline(&bl, save_path) {
                    eprintln!("Warning: could not save baseline: {}", e);
                } else {
                    eprintln!("Baseline saved to {}", save_path.display());
                }
            }

            // Format output
            let result_text = match format.as_str() {
                "junit" => {
                    let skill_name =
                        if let aif_core::ast::BlockKind::SkillBlock { attrs, .. } =
                            &skill_block.kind
                        {
                            attrs.get("name").unwrap_or("(unnamed)").to_string()
                        } else {
                            "(unnamed)".to_string()
                        };
                    aif_eval::junit::generate_junit_xml(&skill_name, &scenario_results)
                }
                "json" => {
                    let json = serde_json::json!({
                        "passed": exit_code == 0,
                        "scenarios": scenario_results,
                        "regressions": regressions.iter().map(|r| serde_json::json!({
                            "scenario_name": r.scenario_name,
                            "baseline_passed": r.baseline_passed,
                            "current_passed": r.current_passed,
                            "score_delta": r.score_delta,
                        })).collect::<Vec<_>>(),
                    });
                    serde_json::to_string_pretty(&json).unwrap()
                }
                _ => {
                    // text format
                    let mut out = String::new();
                    for r in &scenario_results {
                        let mark = if r.passed { "PASS" } else { "FAIL" };
                        out.push_str(&format!("[{}] {}: {}\n", mark, r.name, r.evidence));
                    }
                    if !regressions.is_empty() {
                        out.push_str("\nREGRESSIONS:\n");
                        for r in &regressions {
                            out.push_str(&format!(
                                "  {} — was {}, now {} (delta: {:.2})\n",
                                r.scenario_name,
                                if r.baseline_passed {
                                    "passing"
                                } else {
                                    "failing"
                                },
                                if r.current_passed {
                                    "passing"
                                } else {
                                    "failing"
                                },
                                r.score_delta,
                            ));
                        }
                    }
                    let passed = scenario_results.iter().filter(|r| r.passed).count();
                    out.push_str(&format!(
                        "\n{} of {} scenarios passed.",
                        passed,
                        scenario_results.len()
                    ));
                    out
                }
            };

            write_output(&result_text, output.as_ref());
            std::process::exit(final_exit_code);
        }
    }
}

fn format_lint_failed(results: &[aif_skill::lint::LintResult]) -> String {
    let mut out = String::from("LINT FAILED:\n");
    for r in results {
        if !r.passed {
            out.push_str(&format!("  x {:?}: {}\n", r.check, r.message));
        }
    }
    out
}

fn print_eval_report_text(report: &aif_skill::eval::EvalReport) {
    println!("Skill: {}\n", report.skill_name);
    for stage in &report.stages {
        let status = if matches!(stage.details, aif_skill::eval::StageDetails::Skipped(_)) {
            "SKIP"
        } else if stage.passed {
            "PASS"
        } else {
            "FAIL"
        };
        let stage_name = match stage.stage {
            aif_skill::eval::EvalStage::StructuralLint => "STAGE 1: STRUCTURAL LINT",
            aif_skill::eval::EvalStage::BehavioralCompliance => "STAGE 2: BEHAVIORAL COMPLIANCE",
            aif_skill::eval::EvalStage::EffectivenessEval => "STAGE 3: EFFECTIVENESS EVAL",
        };
        println!(
            "{} {} {} ({}ms)",
            stage_name,
            ".".repeat(40usize.saturating_sub(stage_name.len())),
            status,
            stage.duration_ms
        );

        match &stage.details {
            aif_skill::eval::StageDetails::Lint(results) => {
                for r in results {
                    if !r.passed {
                        println!("  x {:?}: {}", r.check, r.message);
                    }
                }
            }
            aif_skill::eval::StageDetails::Compliance(results) => {
                for r in results {
                    let mark = if r.passed { "+" } else { "x" };
                    println!("  {} {}: {}", mark, r.check_name, r.evidence);
                }
            }
            aif_skill::eval::StageDetails::Effectiveness(results) => {
                for r in results {
                    let mark = if r.passed { "+" } else { "x" };
                    println!(
                        "  {} {} ({:?}): {}",
                        mark, r.name, r.scenario_type, r.evidence
                    );
                }
            }
            aif_skill::eval::StageDetails::Skipped(reason) => {
                println!("  SKIPPED ({})", reason);
            }
        }
    }

    let passed = report.stages.iter().filter(|s| s.passed).count();
    let total = report.stages.len();
    println!("\n{} of {} stages passed.", passed, total);
}

fn print_eval_report_json(report: &aif_skill::eval::EvalReport) {
    let mut stages = Vec::new();
    for stage in &report.stages {
        let stage_json = serde_json::json!({
            "stage": format!("{:?}", stage.stage),
            "passed": stage.passed,
            "duration_ms": stage.duration_ms,
        });
        stages.push(stage_json);
    }
    let output = serde_json::json!({
        "skill_name": report.skill_name,
        "all_passed": report.all_passed(),
        "stages": stages,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
