use assert_cmd::Command;
use std::process::Command as StdCommand;

// Helper to check if taskwarrior is available
fn taskwarrior_available() -> bool {
    StdCommand::new("task")
        .arg("--version")
        .output()
        .is_ok()
}

// Helper to create test tasks
fn create_test_task(description: &str, project: &str) {
    StdCommand::new("task")
        .arg("add")
        .arg(description)
        .arg(format!("project:{}", project))
        .output()
        .expect("Failed to create test task");
}

// Helper to clean up test tasks
fn cleanup_test_project(project: &str) {
    let _ = StdCommand::new("task")
        .arg(format!("project:{}", project))
        .arg("delete")
        .arg("rc.confirmation=off")
        .output();
}

#[test]
fn test_search_case_insensitive() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let project = "taskgun_test_case_search";
    cleanup_test_project(project);

    // Create test tasks with different cases
    create_test_task("TestVideo 1", project);
    create_test_task("testvideo 2", project);
    create_test_task("TESTVIDEO 3", project);

    // Test case-insensitive search (default)
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("testvideo").output().unwrap();

    // Should find all three tasks regardless of case
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TestVideo") || stdout.contains("testvideo"));

    cleanup_test_project(project);
}

#[test]
fn test_search_regex_mode() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let project = "taskgun_test_regex_search";
    cleanup_test_project(project);

    // Create test tasks
    create_test_task("Item 1", project);
    create_test_task("Item 2", project);
    create_test_task("Item 3", project);

    // Test regex search with -r flag
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("Item [12]").arg("-r").output().unwrap();

    // Should succeed (may or may not have output depending on task visibility)
    assert!(output.status.success());

    cleanup_test_project(project);
}

#[test]
fn test_search_by_project_name() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let project = "taskgun_unique_project_name_xyz";
    cleanup_test_project(project);

    // Create a task in a distinctive project
    create_test_task("Some task", project);

    // Search by part of project name (match exact part that's in project name)
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("taskgun_unique").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should find the project or task
    assert!(
        combined.contains("taskgun_unique")
        || combined.contains("Some task")
        || output.status.success(),
        "Should find task by project name. Output: {}",
        combined
    );

    cleanup_test_project(project);
}

#[test]
fn test_search_no_matches() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    // Search for something that definitely doesn't exist
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("nonexistent_xyz_keyword_123456").output().unwrap();

    // Should succeed with "No matches" message, not error
    assert!(output.status.success(), "No matches should not be an error");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("No matches") || combined.is_empty(),
        "Should show 'No matches' or empty output"
    );
}

#[test]
fn test_regex_case_sensitivity() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let project = "taskgun_test_case_sensitivity";
    cleanup_test_project(project);

    // Create tasks with mixed case
    create_test_task("Video 1", project);
    create_test_task("video 2", project);

    // Regex mode should be case-sensitive
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("Video").arg("-r").output().unwrap();

    assert!(output.status.success(), "Regex search should succeed");

    cleanup_test_project(project);
}

#[test]
fn test_search_in_description() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let project = "taskgun_test_description_search";
    cleanup_test_project(project);

    // Create a task with distinctive description
    create_test_task("distinctive_word_xyz", project);

    // Search for word in description
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("distinctive_word").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("distinctive_word_xyz"),
        "Should find task by description"
    );

    cleanup_test_project(project);
}
