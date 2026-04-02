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
