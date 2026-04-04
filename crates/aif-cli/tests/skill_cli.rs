use std::process::Command;

fn aif_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aif"))
}

#[test]
fn skill_import_produces_json() {
    let input = "# Test Skill\n\n## Steps\n\n1. First step\n2. Second step\n";
    let tmp = std::env::temp_dir().join("test_skill_import.md");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["skill", "import", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SkillBlock"));
    assert!(stdout.contains("Step"));

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn skill_verify_no_hash() {
    let input = "@skill[name=test]\nSome content.\n@end\n";
    let tmp = std::env::temp_dir().join("test_verify_nohash.aif");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["skill", "verify", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("no hash") || combined.contains("NoHash") || combined.contains("No hash"),
        "output: {}", combined);

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn skill_inspect_shows_metadata() {
    let input = "@skill[name=debugging version=1.0 tags=process,debug]\n@step[order=1]\n  Do it.\n@end\n@end\n";
    let tmp = std::env::temp_dir().join("test_inspect_meta.aif");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["skill", "inspect", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("debugging"));
    assert!(stdout.contains("1.0"));

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn check_format_json_produces_valid_json() {
    let input = "# Test Skill\n\n## Steps\n\n1. First step\n2. Second step\n\n## Verification\n\nCheck it works.\n";
    let tmp = std::env::temp_dir().join("test_check_json.md");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["check", tmp.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("failed to run aif-cli");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON output: {}\nstdout: {}", e, stdout));

    // Verify required fields exist
    assert!(parsed.get("file").is_some(), "missing 'file' field");
    assert!(parsed.get("skill_name").is_some(), "missing 'skill_name' field");
    assert!(parsed.get("lint_checks").is_some(), "missing 'lint_checks' field");
    assert!(parsed.get("hash_valid").is_some(), "missing 'hash_valid' field");
    assert!(parsed.get("overall").is_some(), "missing 'overall' field");

    // Verify lint_checks is an array with expected structure
    let checks = parsed["lint_checks"].as_array().expect("lint_checks should be array");
    assert!(!checks.is_empty(), "lint_checks should not be empty");
    for check in checks {
        assert!(check.get("name").is_some(), "lint check missing 'name'");
        assert!(check.get("passed").is_some(), "lint check missing 'passed'");
        assert!(check.get("severity").is_some(), "lint check missing 'severity'");
    }

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn check_format_text_is_default() {
    let input = "# Test Skill\n\n## Steps\n\n1. First step\n\n## Verification\n\nCheck it.\n";
    let tmp = std::env::temp_dir().join("test_check_text.md");
    std::fs::write(&tmp, input).unwrap();

    let output = aif_cli()
        .args(["check", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Text format should contain human-readable output, not JSON
    assert!(stdout.contains("SkillForge Quality Check"), "expected text output, got: {}", stdout);

    std::fs::remove_file(&tmp).ok();
}
