#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

/// Parse an AIF document and return JSON IR.
#[pyfunction]
fn parse(source: &str) -> PyResult<String> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    serde_json::to_string_pretty(&doc)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Detect AIF syntax version of a source string. Returns "v1" or "v2".
#[pyfunction]
fn detect_syntax(source: &str) -> PyResult<String> {
    match aif_parser::detect_syntax_version(source) {
        Ok(aif_parser::SyntaxVersion::V1) => Ok("v1".to_string()),
        Ok(aif_parser::SyntaxVersion::V2) => Ok("v2".to_string()),
        Err(e) => Err(PyValueError::new_err(e)),
    }
}

/// Migrate AIF v1 syntax (@end) to v2 (@/name). Idempotent on v2 input.
#[pyfunction]
fn migrate_syntax(source: &str) -> PyResult<String> {
    Ok(aif_parser::migrate::migrate_v1_to_v2(source))
}

/// Import a Markdown string into AIF JSON IR.
#[pyfunction]
fn import_markdown(source: &str) -> PyResult<String> {
    let doc = aif_markdown::import_markdown(source);
    serde_json::to_string_pretty(&doc)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Import HTML into AIF JSON IR.
#[pyfunction]
#[pyo3(signature = (source, strip_chrome=false))]
fn import_html(source: &str, strip_chrome: bool) -> PyResult<String> {
    let result = aif_html::import_html(source, strip_chrome);
    serde_json::to_string_pretty(&result.document)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Compile an AIF document to a target format.
/// Formats: "html", "markdown", "lml", "lml-aggressive", "lml-compact",
///          "lml-conservative", "lml-moderate", "json"
#[pyfunction]
fn compile(source: &str, format: &str) -> PyResult<String> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    let result = match format {
        "html" => aif_html::render_html(&doc),
        "markdown" | "md" => aif_markdown::render_markdown(&doc),
        "lml" => aif_lml::render_lml(&doc),
        "lml-aggressive" => aif_lml::render_lml_aggressive(&doc),
        "lml-compact" => aif_lml::render_lml_skill_compact(&doc),
        "lml-conservative" => aif_lml::render_lml_conservative(&doc),
        "lml-moderate" => aif_lml::render_lml_moderate(&doc),
        "json" => return serde_json::to_string_pretty(&doc)
            .map_err(|e| PyValueError::new_err(e.to_string())),
        _ => return Err(PyValueError::new_err(format!("Unknown format: {}", format))),
    };
    Ok(result)
}

/// Lint an AIF document (10 structural checks). Returns JSON array of lint results.
#[pyfunction]
fn lint(source: &str) -> PyResult<String> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    let results = aif_core::lint::lint_document(&doc);
    let json_results: Vec<serde_json::Value> = results.iter().map(|r| {
        serde_json::json!({
            "check": format!("{:?}", r.check),
            "passed": r.passed,
            "severity": format!("{:?}", r.severity),
            "message": r.message,
            "block_id": r.block_id,
        })
    }).collect();
    serde_json::to_string_pretty(&json_results)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Security scan an AIF document (OWASP AST10 aligned). Returns JSON array of findings.
#[pyfunction]
fn scan(source: &str) -> PyResult<String> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    let findings = aif_core::scan::scan_document(&doc);
    let json_findings: Vec<serde_json::Value> = findings.iter().map(|f| {
        serde_json::json!({
            "rule": f.rule,
            "severity": format!("{:?}", f.severity),
            "message": f.message,
            "block_id": f.block_id,
            "owasp_ref": f.owasp_ref,
        })
    }).collect();
    serde_json::to_string_pretty(&json_findings)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Run semantic inference on an AIF document (pattern-based).
/// Returns the enriched JSON IR with inferred semantic types.
#[pyfunction]
#[pyo3(signature = (source, min_confidence=0.5))]
fn infer(source: &str, min_confidence: f64) -> PyResult<String> {
    let mut doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    let config = aif_core::infer::InferConfig {
        min_confidence,
        strategy: aif_core::infer::InferStrategy::Pattern,
    };
    aif_core::infer::annotate_semantics(&mut doc, &config);
    serde_json::to_string_pretty(&doc)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Clean HTML by stripping chrome (nav, header, footer, scripts, styles)
/// and importing to AIF JSON IR. Equivalent to `aif import --strip-chrome`.
#[pyfunction]
fn clean_html(source: &str) -> PyResult<String> {
    let result = aif_html::import_html(source, true);
    serde_json::to_string_pretty(&result.document)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Import a SKILL.md file and return the skill block as JSON IR.
#[pyfunction]
fn import_skill_md(source: &str) -> PyResult<String> {
    let result = aif_skill::import::import_skill_md(source);
    serde_json::to_string_pretty(&result.block)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Export an AIF skill to SKILL.md Markdown format.
#[pyfunction]
fn export_skill_md(source: &str) -> PyResult<String> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    // Find first skill block
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(&b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    });
    match skill_block {
        Some(block) => Ok(aif_skill::export::export_skill_md(block)),
        None => Err(PyValueError::new_err("No @skill block found".to_string())),
    }
}

/// Compute the SHA-256 content hash of a skill.
#[pyfunction]
fn hash_skill(source: &str) -> PyResult<String> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(&b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    });
    match skill_block {
        Some(block) => Ok(aif_skill::hash::compute_skill_hash(block)),
        None => Err(PyValueError::new_err("No @skill block found".to_string())),
    }
}

/// Generate an Ed25519 keypair. Returns (private_key_base64, public_key_base64).
#[pyfunction]
fn generate_keypair() -> PyResult<(String, String)> {
    Ok(aif_skill::sign::generate_keypair())
}

/// Sign a skill with an Ed25519 private key. Returns base64 signature.
#[pyfunction]
fn sign_skill(source: &str, private_key_b64: &str) -> PyResult<String> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(&b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    });
    match skill_block {
        Some(block) => aif_skill::sign::sign_skill(block, private_key_b64)
            .map_err(PyValueError::new_err),
        None => Err(PyValueError::new_err("No @skill block found".to_string())),
    }
}

/// Verify a skill's Ed25519 signature. Returns True if valid.
#[pyfunction]
fn verify_skill(source: &str, signature_b64: &str, public_key_b64: &str) -> PyResult<bool> {
    let doc = aif_parser::parse(source)
        .map_err(|e| PyValueError::new_err(format!("Parse error: {:?}", e)))?;
    let skill_block = doc.blocks.iter().find(|b| {
        matches!(&b.kind, aif_core::ast::BlockKind::SkillBlock {
            skill_type: aif_core::ast::SkillBlockType::Skill, ..
        })
    });
    match skill_block {
        Some(block) => aif_skill::sign::verify_skill(block, signature_b64, public_key_b64)
            .map_err(PyValueError::new_err),
        None => Err(PyValueError::new_err("No @skill block found".to_string())),
    }
}

/// SkillForge Python module — quality layer for Agent Skills.
///
/// Functions:
///   parse(source)                   -> JSON IR
///   import_markdown(source)         -> JSON IR
///   import_html(source, strip=False)-> JSON IR
///   import_skill_md(source)         -> skill JSON
///   export_skill_md(source)         -> Markdown
///   compile(source, format)         -> formatted output
///   lint(source)                    -> JSON[] (10 checks)
///   scan(source)                    -> JSON[] (OWASP AST10 security checks)
///   infer(source, min_confidence)   -> enriched JSON IR
///   clean_html(source)              -> cleaned JSON IR
///   hash_skill(source)              -> SHA-256 hex
///   generate_keypair()              -> (private, public) base64
///   sign_skill(source, privkey)     -> signature base64
///   verify_skill(source, sig, pub)  -> bool
#[pymodule]
fn skillforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(detect_syntax, m)?)?;
    m.add_function(wrap_pyfunction!(migrate_syntax, m)?)?;
    m.add_function(wrap_pyfunction!(import_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(import_html, m)?)?;
    m.add_function(wrap_pyfunction!(import_skill_md, m)?)?;
    m.add_function(wrap_pyfunction!(export_skill_md, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)?;
    m.add_function(wrap_pyfunction!(scan, m)?)?;
    m.add_function(wrap_pyfunction!(infer, m)?)?;
    m.add_function(wrap_pyfunction!(clean_html, m)?)?;
    m.add_function(wrap_pyfunction!(hash_skill, m)?)?;
    m.add_function(wrap_pyfunction!(generate_keypair, m)?)?;
    m.add_function(wrap_pyfunction!(sign_skill, m)?)?;
    m.add_function(wrap_pyfunction!(verify_skill, m)?)?;
    Ok(())
}
