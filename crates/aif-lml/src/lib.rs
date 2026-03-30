mod emitter;

use aif_core::ast::Document;

pub fn render_lml(doc: &Document) -> String {
    emitter::emit_lml(doc)
}

pub fn render_lml_skill_compact(doc: &Document) -> String {
    emitter::emit_lml_skill_compact(doc)
}
