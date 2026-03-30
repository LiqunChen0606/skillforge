mod emitter;

use aif_core::ast::Document;

pub fn render_lml(doc: &Document) -> String {
    emitter::emit_lml(doc)
}
