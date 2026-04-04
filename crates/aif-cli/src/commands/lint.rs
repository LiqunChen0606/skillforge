use std::path::PathBuf;

use aif_core::ast::BlockKind;

use crate::util::{find_skill_block, parse_aif, read_source};

pub fn handle_lint(input: PathBuf, format: String) {
    let source = read_source(&input);
    let doc = parse_aif(&source);
    let results = aif_core::lint::lint_document(&doc);
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
        println!("Document Lint: {}", input.display());
        println!("{}", "=".repeat(60));
        for r in &results {
            let icon = if r.passed { "+" } else { "x" };
            let sev = match r.severity {
                aif_core::lint::DocLintSeverity::Error => "ERROR",
                aif_core::lint::DocLintSeverity::Warning => "WARN",
            };
            if r.passed {
                println!("  [{}] {:?}", icon, r.check);
            } else {
                let loc = r
                    .block_id
                    .as_ref()
                    .map(|id| format!(" ({})", id))
                    .unwrap_or_default();
                println!("  [{}] {:?} [{}]{}: {}", icon, r.check, sev, loc, r.message);
            }
        }
        println!("{}", "-".repeat(60));
        println!("{} checks: {} passed, {} failed", total, passed, failed);
        if failed > 0 {
            std::process::exit(1);
        }
    }
}

pub fn handle_check(input: PathBuf, format: String) {
    let ext = input.extension().map(|e| e.to_ascii_lowercase());
    let is_md = ext.as_ref().map(|e| e == "md").unwrap_or(false);
    let is_json_format = format == "json";

    if !is_json_format {
        println!("SkillForge Quality Check: {}", input.display());
        println!("{}", "=".repeat(60));
    }

    // Step 1: Import if SKILL.md, parse if .aif
    let (doc, source_desc) = if is_md {
        let source = read_source(&input);
        let result = aif_skill::import::import_skill_md(&source);
        let doc = aif_core::ast::Document {
            metadata: std::collections::BTreeMap::new(),
            blocks: vec![result.block],
        };
        if !is_json_format {
            println!("  [+] Imported SKILL.md (1 skill block)");
        }
        (doc, "imported from SKILL.md")
    } else {
        let source = read_source(&input);
        let doc = parse_aif(&source);
        if !is_json_format {
            println!("  [+] Parsed AIF ({} blocks)", doc.blocks.len());
        }
        (doc, "parsed from .aif")
    };

    // Step 2: Find skill block
    let skill_block = find_skill_block(&doc.blocks);
    if skill_block.is_none() {
        if is_json_format {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "file": input.display().to_string(),
                    "skill_name": serde_json::Value::Null,
                    "lint_checks": [],
                    "hash_valid": serde_json::Value::Null,
                    "overall": "error",
                    "error": "No @skill block found"
                }))
                .unwrap()
            );
        } else {
            println!("  [x] No @skill block found");
        }
        std::process::exit(1);
    }
    let skill_block = skill_block.unwrap();

    // Step 3: Skill metadata
    let skill_name = if let BlockKind::SkillBlock { attrs, .. } = &skill_block.kind {
        let name = attrs.get("name").unwrap_or("(unnamed)").to_string();
        let version = attrs.get("version").unwrap_or("(none)");
        if !is_json_format {
            println!("  [+] Skill: {} v{}", name, version);
        }
        name
    } else {
        "(unnamed)".to_string()
    };

    // Step 4: Structural lint
    let lint_results = aif_skill::lint::lint_skill(skill_block);
    let lint_passed = lint_results.iter().filter(|r| r.passed).count();
    let lint_failed = lint_results.iter().filter(|r| !r.passed).count();
    if !is_json_format {
        if lint_failed == 0 {
            println!(
                "  [+] Lint: {}/{} checks passed",
                lint_passed,
                lint_results.len()
            );
        } else {
            println!(
                "  [!] Lint: {}/{} checks passed, {} failed:",
                lint_passed,
                lint_results.len(),
                lint_failed
            );
            for r in &lint_results {
                if !r.passed {
                    println!("      - {:?}: {}", r.check, r.message);
                }
            }
        }
    }

    // Step 5: Hash verification
    let hash = aif_skill::hash::compute_skill_hash(skill_block);
    let hash_valid = if let BlockKind::SkillBlock { attrs, .. } = &skill_block.kind {
        match attrs.get("hash") {
            Some(stored) if stored == hash.as_str() => {
                if !is_json_format {
                    println!("  [+] Hash: verified ({})", &hash[..20]);
                }
                Some(true)
            }
            Some(stored) => {
                if !is_json_format {
                    println!("  [!] Hash: MISMATCH — content may be tampered");
                    println!("      stored:   {}", stored);
                    println!("      computed: {}", hash);
                }
                Some(false)
            }
            None => {
                if !is_json_format {
                    println!("  [~] Hash: not set (run `aif skill rehash` to add)");
                }
                None
            }
        }
    } else {
        None
    };

    // Step 6: Document lint
    let doc_results = aif_core::lint::lint_document(&doc);
    let doc_passed = doc_results.iter().filter(|r| r.passed).count();
    let doc_failed = doc_results.iter().filter(|r| !r.passed).count();
    if !is_json_format {
        if doc_failed == 0 {
            println!(
                "  [+] Document lint: {}/{} checks passed",
                doc_passed,
                doc_results.len()
            );
        } else {
            println!("  [!] Document lint: {} issues found", doc_failed);
        }
    }

    // Summary
    let total_issues = lint_failed + doc_failed;
    let overall = if total_issues == 0 { "pass" } else { "fail" };

    if is_json_format {
        let json_checks: Vec<_> = lint_results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": format!("{:?}", r.check),
                    "passed": r.passed,
                    "severity": format!("{:?}", r.severity),
                    "message": r.message,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "file": input.display().to_string(),
                "skill_name": skill_name,
                "lint_checks": json_checks,
                "hash_valid": hash_valid,
                "overall": overall,
            }))
            .unwrap()
        );
    } else {
        println!("{}", "-".repeat(60));
        if total_issues == 0 {
            println!("PASS — {} is clean ({})", input.display(), source_desc);
        } else {
            println!(
                "ISSUES — {} problem(s) found in {}",
                total_issues,
                input.display()
            );
        }
    }

    if total_issues > 0 {
        std::process::exit(1);
    }
}
