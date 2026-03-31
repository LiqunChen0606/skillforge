use aif_core::schema::generate_schema;

#[test]
fn schema_contains_document_definition() {
    let schema = generate_schema();
    let json: serde_json::Value = serde_json::from_str(&schema).unwrap();
    assert!(json.is_object());
    let schema_str = schema.to_lowercase();
    assert!(schema_str.contains("document"));
}

#[test]
fn schema_is_valid_json_schema() {
    let schema = generate_schema();
    let json: serde_json::Value = serde_json::from_str(&schema).unwrap();
    assert!(json.is_object());
    // Should have properties or definitions
    assert!(json.get("type").is_some() || json.get("$ref").is_some() || json.get("definitions").is_some());
}

#[test]
fn schema_contains_block_types() {
    let schema = generate_schema();
    assert!(schema.contains("Paragraph"));
    assert!(schema.contains("Section"));
    assert!(schema.contains("SkillBlock"));
    assert!(schema.contains("SemanticBlock"));
}
