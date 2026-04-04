use std::path::PathBuf;
use crate::util::{read_source, parse_aif};

pub fn handle_scan(input: PathBuf, format: String) {
    let ext = input.extension().map(|e| e.to_ascii_lowercase());
    let is_md = ext.as_ref().map(|e| e == "md").unwrap_or(false);

    // Import or parse
    let doc = if is_md {
        let source = read_source(&input);
        let result = aif_skill::import::import_skill_md(&source);
        aif_core::ast::Document {
            metadata: std::collections::BTreeMap::new(),
            blocks: vec![result.block],
        }
    } else {
        let source = read_source(&input);
        parse_aif(&source)
    };

    let findings = aif_core::scan::scan_document(&doc);
    let (critical, high, medium, low, info) = aif_core::scan::scan_summary(&findings);
    let total = findings.len();

    if format == "json" {
        let json_findings: Vec<serde_json::Value> = findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "rule": f.rule,
                    "severity": format!("{:?}", f.severity),
                    "message": f.message,
                    "block_id": f.block_id,
                    "owasp_ref": f.owasp_ref,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "file": input.display().to_string(),
                "total": total,
                "critical": critical,
                "high": high,
                "medium": medium,
                "low": low,
                "info": info,
                "findings": json_findings,
            }))
            .unwrap()
        );
    } else {
        println!("Security Scan: {}", input.display());
        println!("{}", "=".repeat(60));

        if findings.is_empty() {
            println!("  No security issues found.");
        } else {
            for f in &findings {
                let sev = match f.severity {
                    aif_core::scan::Severity::Critical => "\x1b[91mCRITICAL\x1b[0m",
                    aif_core::scan::Severity::High => "\x1b[31mHIGH\x1b[0m",
                    aif_core::scan::Severity::Medium => "\x1b[33mMEDIUM\x1b[0m",
                    aif_core::scan::Severity::Low => "\x1b[36mLOW\x1b[0m",
                    aif_core::scan::Severity::Info => "INFO",
                };
                let loc = f
                    .block_id
                    .as_ref()
                    .map(|id| format!(" ({})", id))
                    .unwrap_or_default();
                let owasp = f
                    .owasp_ref
                    .map(|r| format!(" [{}]", r))
                    .unwrap_or_default();
                println!("  [{}]{}{}: {} ({})", sev, loc, owasp, f.message, f.rule);
            }
        }

        println!("{}", "-".repeat(60));
        if total == 0 {
            println!("CLEAN — no security issues in {}", input.display());
        } else {
            println!(
                "{} finding(s): {} critical, {} high, {} medium, {} low",
                total, critical, high, medium, low
            );
            if critical > 0 || high > 0 {
                std::process::exit(1);
            }
        }
    }
}
