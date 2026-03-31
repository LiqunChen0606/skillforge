use crate::ast::Document;
use schemars::schema_for;

/// Generate a JSON Schema string for the AIF Document type.
pub fn generate_schema() -> String {
    let schema = schema_for!(Document);
    serde_json::to_string_pretty(&schema).expect("schema serialization failed")
}
