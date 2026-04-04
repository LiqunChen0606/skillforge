use std::process::Command;

fn aif_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aif"))
}

#[test]
fn migrate_validate_valid_skill() {
    let skill = "\
@skill[name=\"test-migration\", version=\"1.0\", profile=migration]
  @precondition
    Source files exist.

  @step[order=1]
    Replace foo with bar.

  @verify
    Pattern `bar` should be present.

  @output_contract
    All foo replaced with bar.
@/skill
";
    let tmp = std::env::temp_dir().join("test_migrate_validate.aif");
    std::fs::write(&tmp, skill).unwrap();

    let output = aif_cli()
        .args(["migrate", "validate", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run aif-cli");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {}", stderr);
    assert!(stderr.contains("passed"), "Expected 'passed' in stderr: {}", stderr);

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn migrate_run_no_llm_key_placeholder() {
    // Create a valid migration skill
    let skill = "\
@skill[name=\"test-migration\", version=\"1.0\", profile=migration]
  @precondition
    Source files exist.

  @step[order=1]
    Replace foo with bar.

  @verify
    Pattern `bar` should be present.

  @output_contract
    All foo replaced with bar.
@/skill
";
    let skill_path = std::env::temp_dir().join("test_migrate_run_skill.aif");
    std::fs::write(&skill_path, skill).unwrap();

    // Create a source directory with a file
    let source_dir = std::env::temp_dir().join("test_migrate_run_source");
    let _ = std::fs::create_dir_all(&source_dir);
    std::fs::write(source_dir.join("test.txt"), "foo content here").unwrap();

    // Create output directory
    let output_dir = std::env::temp_dir().join("test_migrate_run_output");

    // Run without LLM key — should still work with placeholder apply_fn
    let output = aif_cli()
        .env_remove("AIF_LLM_API_KEY")
        .args([
            "migrate", "run",
            "--skill", skill_path.to_str().unwrap(),
            "--source", source_dir.to_str().unwrap(),
            "-o", output_dir.to_str().unwrap(),
            "--strategy", "file",
        ])
        .output()
        .expect("failed to run aif-cli");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // Should run successfully (exit 0) with the placeholder apply_fn
    assert!(output.status.success(), "Command failed. stderr: {}\nstdout: {}", stderr, stdout);
    // Should mention LLM integration or report
    assert!(
        combined.contains("Migration Report") || combined.contains("LLM") || combined.contains("report") || combined.contains("test-migration"),
        "Expected migration output, got: {}", combined
    );

    // Cleanup
    std::fs::remove_file(&skill_path).ok();
    let _ = std::fs::remove_dir_all(&source_dir);
    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn migrate_run_text_report_format() {
    let skill = "\
@skill[name=\"text-report-test\", version=\"1.0\", profile=migration]
  @precondition
    Source files exist.

  @step[order=1]
    Transform the code.

  @verify
    Code is transformed.

  @output_contract
    Transformation complete.
@/skill
";
    let skill_path = std::env::temp_dir().join("test_migrate_report_skill.aif");
    std::fs::write(&skill_path, skill).unwrap();

    let source_dir = std::env::temp_dir().join("test_migrate_report_source");
    let _ = std::fs::create_dir_all(&source_dir);
    std::fs::write(source_dir.join("a.txt"), "some code").unwrap();

    let output_dir = std::env::temp_dir().join("test_migrate_report_output");

    let output = aif_cli()
        .env_remove("AIF_LLM_API_KEY")
        .args([
            "migrate", "run",
            "--skill", skill_path.to_str().unwrap(),
            "--source", source_dir.to_str().unwrap(),
            "-o", output_dir.to_str().unwrap(),
            "--report", "text",
        ])
        .output()
        .expect("failed to run aif-cli");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stderr: {}\nstdout: {}", stderr, stdout);

    // Cleanup
    std::fs::remove_file(&skill_path).ok();
    let _ = std::fs::remove_dir_all(&source_dir);
    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn migrate_run_json_report_format() {
    let skill = "\
@skill[name=\"json-report-test\", version=\"1.0\", profile=migration]
  @precondition
    Source files exist.

  @step[order=1]
    Transform the code.

  @verify
    Code is transformed.

  @output_contract
    Transformation complete.
@/skill
";
    let skill_path = std::env::temp_dir().join("test_migrate_json_report_skill.aif");
    std::fs::write(&skill_path, skill).unwrap();

    let source_dir = std::env::temp_dir().join("test_migrate_json_report_source");
    let _ = std::fs::create_dir_all(&source_dir);
    std::fs::write(source_dir.join("b.txt"), "some code").unwrap();

    let output_dir = std::env::temp_dir().join("test_migrate_json_report_output");

    let output = aif_cli()
        .env_remove("AIF_LLM_API_KEY")
        .args([
            "migrate", "run",
            "--skill", skill_path.to_str().unwrap(),
            "--source", source_dir.to_str().unwrap(),
            "-o", output_dir.to_str().unwrap(),
            "--report", "json",
        ])
        .output()
        .expect("failed to run aif-cli");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stderr: {}\nstdout: {}", stderr, stdout);
    // JSON report should be valid JSON with expected fields
    let json_output = stdout.trim();
    assert!(!json_output.is_empty(), "Expected JSON output on stdout");
    let parsed: serde_json::Value = serde_json::from_str(json_output)
        .unwrap_or_else(|e| panic!("Invalid JSON: {}. Output: {}", e, json_output));
    assert!(parsed.get("skill_name").is_some(), "Expected skill_name in JSON");
    assert!(parsed.get("chunks").is_some(), "Expected chunks in JSON");

    // Cleanup
    std::fs::remove_file(&skill_path).ok();
    let _ = std::fs::remove_dir_all(&source_dir);
    let _ = std::fs::remove_dir_all(&output_dir);
}
