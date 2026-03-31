mod emitter;

use aif_core::ast::Document;

pub use emitter::LmlMode;

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
