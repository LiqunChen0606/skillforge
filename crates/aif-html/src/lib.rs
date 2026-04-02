mod emitter;
pub mod importer;
pub mod readability;

use aif_core::ast::Document;

pub use importer::{import_html, HtmlImportResult, ImportMode};

pub fn render_html(doc: &Document) -> String {
    emitter::emit_html(doc)
}
