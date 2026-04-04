use std::process::Command;

#[test]
fn eval_help_works() {
    let output = Command::new("cargo")
        .args(["run", "-p", "aif-cli", "--", "skill", "eval", "--help"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("eval") || combined.contains("Eval"),
        "Expected eval help text, got: {}",
        combined
    );
}

#[test]
fn eval_lint_only_on_fixture() {
    let dir = std::env::temp_dir().join("aif-eval-test");
    std::fs::create_dir_all(&dir).unwrap();
    let skill_file = dir.join("test-skill.aif");
    std::fs::write(
        &skill_file,
        r#"@skill[name=test-skill, description="Use when testing"]
@step[order=1]
Do the thing.
@verify
Check it worked.
@/skill
"#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "aif-cli",
            "--",
            "skill",
            "eval",
            skill_file.to_str().unwrap(),
            "--stage",
            "1",
        ])
        .output()
        .expect("failed to run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("STRUCTURAL LINT")
            || combined.contains("PASS")
            || combined.contains("pass"),
        "Expected lint output, got: {}",
        combined
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_list_works() {
    let output = Command::new("cargo")
        .args(["run", "-p", "aif-cli", "--", "config", "list"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("provider") || combined.contains("llm"),
        "Expected config output, got: {}",
        combined
    );
}
