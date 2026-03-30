mod emitter;
mod importer;

use aif_core::ast::Document;

pub fn render_markdown(doc: &Document) -> String {
    emitter::emit_markdown(doc)
}

pub fn import_markdown(input: &str) -> Document {
    importer::import(input)
}
