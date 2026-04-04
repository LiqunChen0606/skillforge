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

/// Lint an AIF document. Returns JSON array of lint results.
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

/// SkillForge Python module
#[pymodule]
fn skillforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(import_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(import_html, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)?;
    m.add_function(wrap_pyfunction!(infer, m)?)?;
    m.add_function(wrap_pyfunction!(clean_html, m)?)?;
    Ok(())
}
