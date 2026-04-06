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
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn test_link_command_help() {
    let mut cmd = Command::cargo_bin("taskgun").expect("Failed to find taskgun binary");
    cmd.arg("link").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create a new task with clipboard link"))
        .stdout(predicate::str::contains("--id"))
        .stdout(predicate::str::contains("-i"));
}

#[test]
fn test_link_command_no_args() {
    let env = TestEnv::new();

    env.taskgun()
        .arg("link")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Provide either a task description or -i <id>"));
}

#[test]
fn test_link_command_mutually_exclusive() {
    let env = TestEnv::new();

    env.taskgun()
        .arg("link")
        .arg("-i")
        .arg("5")
        .arg("some task")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cannot use -i/--id together with a task description"));
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

    env.taskgun()
        .arg("open")
        .arg("999")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task 999 not found"));
}
