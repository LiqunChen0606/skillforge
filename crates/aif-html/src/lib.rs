mod emitter;

use aif_core::ast::Document;

pub fn render_html(doc: &Document) -> String {
    emitter::emit_html(doc)
}
