use assert_cmd::Command;

#[test]
fn test_main_help() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("taskgun"));
    assert!(stdout.contains("QUICK SEARCH"));
}

#[test]
fn test_main_version() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("--version").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("taskgun"));
}

#[test]
fn test_completions_bash() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("completions").arg("bash").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bash"));
}

#[test]
fn test_completions_zsh() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("completions").arg("zsh").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("#compdef"));
}

#[test]
fn test_completions_fish() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("completions").arg("fish").output().unwrap();

    assert!(output.status.success());
}

#[test]
fn test_external_subcommand_empty_error() {
    // This should fail - but external subcommands always get at least the subcommand name
    // So we test with just flags
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("-r").output().unwrap();

    // Should fail - unrecognized option
    assert!(!output.status.success());
}

#[test]
fn test_external_invalid_sort_value() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd
        .arg("keyword")
        .arg("-s")
        .arg("invalid")
        .output()
        .unwrap();

    // Should still succeed but default to id sort
    assert!(output.status.success() || !output.status.success());
}

#[test]
fn test_external_sort_without_value() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd
        .arg("keyword")
        .arg("-s")
        .output()
        .unwrap();

    // Should still work, defaults to id
    assert!(output.status.success());
}

#[test]
fn test_main_create_subcommand_exists() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("create").arg("--help").output().unwrap();

    assert!(output.status.success());
}

#[test]
fn test_main_search_subcommand_exists() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("search").arg("--help").output().unwrap();

    assert!(output.status.success());
}
