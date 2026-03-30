use std::fs;

#[test]
fn roundtrip_debugging_skill() {
    let md_input = fs::read_to_string("../../tests/fixtures/skills/debugging.md").unwrap();

    // Import
    let import_result = aif_skill::import::import_skill_md(&md_input);
    let block = &import_result.block;

    // Verify structure
    if let aif_core::ast::BlockKind::SkillBlock { attrs, children, .. } = &block.kind {
        assert_eq!(attrs.get("name"), Some("debugging"));
        assert_eq!(attrs.get("description"), Some("Use when encountering any bug, test failure, or unexpected behavior"));
        assert!(children.len() >= 5, "expected at least 5 children, got {}", children.len());
    } else {
        panic!("expected SkillBlock");
    }

    // Check mappings
    assert!(!import_result.mappings.is_empty());

    // Export back to markdown
    let exported = aif_skill::export::export_skill_md(block);
    assert!(exported.contains("# debugging"));
    assert!(exported.contains("## Steps"));
    assert!(exported.contains("1. Read error messages carefully"));

    // Validate
    let errors = aif_skill::validate::validate_skill(block);
    assert!(errors.is_empty(), "validation errors: {:?}", errors);

    // Hash
    let hash = aif_skill::hash::compute_skill_hash(block);
    assert!(hash.starts_with("sha256:"));
}

#[test]
fn manifest_entry_from_imported_skill() {
    let md_input = fs::read_to_string("../../tests/fixtures/skills/debugging.md").unwrap();
    let import_result = aif_skill::import::import_skill_md(&md_input);
    let entry = aif_skill::manifest::skill_to_entry(&import_result.block, "skills/debugging.aif").unwrap();

    assert_eq!(entry.name, "debugging");
    assert!(entry.hash.starts_with("sha256:"));
    assert!(entry.blocks.contains(&"step".to_string()));
    assert!(entry.blocks.contains(&"verify".to_string()));
}
