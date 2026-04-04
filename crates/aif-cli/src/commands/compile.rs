use std::path::PathBuf;

use crate::util::{parse_aif, read_source, run_inference, write_output};

pub fn handle_compile(
    input: PathBuf,
    format: String,
    output: Option<PathBuf>,
    input_format: String,
    view: Option<String>,
) {
    let doc = match input_format.as_str() {
        "json" => {
            let source = read_source(&input);
            serde_json::from_str::<aif_core::ast::Document>(&source).unwrap_or_else(|e| {
                eprintln!("Error parsing JSON IR: {}", e);
                std::process::exit(1);
            })
        }
        _ => {
            let source = read_source(&input);
            parse_aif(&source)
        }
    };

    // Apply view filter if specified
    let doc = if let Some(view_name) = &view {
        match aif_core::view::ViewMode::from_str(view_name) {
            Some(mode) => aif_core::view::filter_for_view(&doc, mode),
            None => {
                eprintln!(
                    "Unknown view mode: {}. Supported: author, llm, api",
                    view_name
                );
                std::process::exit(1);
            }
        }
    } else {
        doc
    };

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

pub fn handle_import(
    input: PathBuf,
    output: Option<PathBuf>,
    strip_chrome: bool,
    infer_semantics: bool,
    infer_llm: bool,
) {
    let ext = input.extension().map(|e| e.to_ascii_lowercase());
    let is_pdf = ext.as_ref().map(|e| e == "pdf").unwrap_or(false);
    let is_html = ext
        .as_ref()
        .map(|e| e == "html" || e == "htm")
        .unwrap_or(false);

    let source_file = input.display().to_string();

    if is_pdf {
        let pdf_bytes = std::fs::read(&input).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", input.display(), e);
            std::process::exit(1);
        });
        let mut result = aif_pdf::import::import_pdf(&pdf_bytes).unwrap_or_else(|e| {
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
        // Provenance
        result
            .document
            .metadata
            .insert("_aif_source_format".into(), "pdf".into());
        result
            .document
            .metadata
            .insert("_aif_source_file".into(), source_file);
        result.document.metadata.insert(
            "_aif_import_confidence".into(),
            format!("{:.2}", result.avg_confidence),
        );
        run_inference(&mut result.document, infer_semantics, infer_llm);
        let json = serde_json::to_string_pretty(&result.document).unwrap();
        write_output(&json, output.as_ref());
    } else if is_html {
        let source = read_source(&input);
        let mut result = aif_html::import_html(&source, strip_chrome);
        eprintln!(
            "Imported HTML ({} mode), {} blocks",
            match result.mode {
                aif_html::ImportMode::AifRoundtrip => "AIF roundtrip",
                aif_html::ImportMode::Generic => "generic",
            },
            result.document.blocks.len()
        );
        // Provenance (source_format and import_mode already set by importer)
        result
            .document
            .metadata
            .insert("_aif_source_file".into(), source_file);
        run_inference(&mut result.document, infer_semantics, infer_llm);
        let json = serde_json::to_string_pretty(&result.document).unwrap();
        write_output(&json, output.as_ref());
    } else {
        let source = read_source(&input);
        let mut doc = aif_markdown::import_markdown(&source);
        // Provenance (source_format already set by importer)
        doc.metadata
            .insert("_aif_source_file".into(), source_file);
        run_inference(&mut doc, infer_semantics, infer_llm);
        let json = serde_json::to_string_pretty(&doc).unwrap();
        write_output(&json, output.as_ref());
    }
}

pub fn handle_dump_ir(input: PathBuf) {
    let source = read_source(&input);
    let doc = parse_aif(&source);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    println!("{}", json);
}

pub fn handle_schema() {
    println!("{}", aif_core::schema::generate_schema());
}
