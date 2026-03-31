mod compress;
mod emitter;
mod hybrid;
mod parser;

use aif_core::ast::Document;

pub use emitter::LmlMode;

pub fn parse_lml(input: &str) -> Result<Document, String> {
    parser::parse_lml(input)
}

pub fn render_lml(doc: &Document) -> String {
    emitter::emit_lml(doc)
}

pub fn render_lml_skill_compact(doc: &Document) -> String {
    emitter::emit_lml_skill_compact(doc)
}

pub fn render_lml_conservative(doc: &Document) -> String {
    emitter::emit_lml_conservative(doc)
}

pub fn render_lml_moderate(doc: &Document) -> String {
    emitter::emit_lml_moderate(doc)
}

pub fn render_lml_aggressive(doc: &Document) -> String {
    emitter::emit_lml_aggressive(doc)
}

pub fn render_lml_with_mode(doc: &Document, mode: LmlMode) -> String {
    emitter::emit_lml_mode(doc, mode)
}

pub fn render_lml_hybrid(doc: &Document) -> String {
    hybrid::emit_lml_hybrid(doc)
}

pub fn render_lml_compressed(doc: &Document) -> String {
    compress::render_compressed(doc)
}
