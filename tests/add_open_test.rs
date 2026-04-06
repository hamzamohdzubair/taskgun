use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create an isolated task environment
struct TestEnv {
    _temp_dir: TempDir,
    task_data: String,
    task_rc: String,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let task_data = temp_dir.path().join("task_data");
        let task_rc = temp_dir.path().join("taskrc");

        fs::create_dir(&task_data).expect("Failed to create task_data dir");
        fs::write(
            &task_rc,
            format!("data.location={}\n", task_data.display()),
        )
        .expect("Failed to write taskrc");

        TestEnv {
            _temp_dir: temp_dir,
            task_data: task_data.to_str().unwrap().to_string(),
            task_rc: task_rc.to_str().unwrap().to_string(),
        }
    }

    fn taskgun(&self) -> Command {
        let mut cmd = Command::cargo_bin("taskgun").expect("Failed to find taskgun binary");
        cmd.env("TASKDATA", &self.task_data);
        cmd.env("TASKRC", &self.task_rc);
        cmd
    }

    fn task(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new("task");
        cmd.env("TASKDATA", &self.task_data);
        cmd.env("TASKRC", &self.task_rc);
        cmd
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn test_add_command_without_link() {
    let env = TestEnv::new();

    // Add a task without link
    env.taskgun()
        .arg("add")
        .arg("Buy")
        .arg("milk")
        .arg("due:tomorrow")
        .assert()
        .success();

    // Verify task was created
    let output = env
        .task()
        .arg("rc.uda.link.type=string")
        .arg("rc.uda.link.label=Link")
        .arg("export")
        .output()
        .expect("Failed to run task");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Buy milk"));
}

#[test]
fn test_add_command_help() {
    let mut cmd = Command::cargo_bin("taskgun").expect("Failed to find taskgun binary");
    cmd.arg("add").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Add a single task with optional link from clipboard"))
        .stdout(predicate::str::contains("--link"))
        .stdout(predicate::str::contains("-l"));
}

#[test]
fn test_open_command_help() {
    let mut cmd = Command::cargo_bin("taskgun").expect("Failed to find taskgun binary");
    cmd.arg("open").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Open link/url from a task in the browser"))
        .stdout(predicate::str::contains("Task ID to open link from"));
}

#[test]
fn test_open_command_task_not_found() {
    let env = TestEnv::new();

    // Try to open a non-existent task
    env.taskgun()
        .arg("open")
        .arg("999")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task 999 not found"));
}

#[test]
fn test_add_command_with_taskwarrior_args() {
    let env = TestEnv::new();

    // Add a task with various taskwarrior arguments
    env.taskgun()
        .arg("add")
        .arg("Fix")
        .arg("bug")
        .arg("in")
        .arg("login")
        .arg("project:webapp")
        .arg("due:today")
        .arg("priority:H")
        .arg("+urgent")
        .assert()
        .success();

    // Verify task was created with correct attributes
    let output = env
        .task()
        .arg("export")
        .output()
        .expect("Failed to run task");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fix bug in login"));
    assert!(stdout.contains("webapp"));
    assert!(stdout.contains("urgent"));
}

#[test]
fn test_add_command_empty_args() {
    let env = TestEnv::new();

    // Try to add a task without description
    env.taskgun()
        .arg("add")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task description required"));
}

#[test]
fn test_add_command_without_link_no_emoji() {
    let env = TestEnv::new();

    // Add a task without -l flag
    env.taskgun()
        .arg("add")
        .arg("Test")
        .arg("task")
        .assert()
        .success();

    // Verify task was created without emoji
    let output = env
        .task()
        .arg("export")
        .output()
        .expect("Failed to run task");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain "Test task" without emoji
    assert!(stdout.contains("Test task"));
    // Should not have emoji prepended
    assert!(!stdout.contains("🔗 Test task"));
}
