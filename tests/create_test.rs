use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// Test environment to isolate Taskwarrior database
struct TestEnv {
    temp_dir: TempDir,
    data_dir: PathBuf,
    rc_file: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let data_dir = temp_dir.path().join("data");
        let rc_file = temp_dir.path().join("taskrc");

        fs::create_dir(&data_dir).expect("Failed to create data dir");
        fs::write(
            &rc_file,
            format!("data.location={}\n", data_dir.to_str().unwrap()),
        )
        .expect("Failed to write taskrc");

        Self {
            temp_dir,
            data_dir,
            rc_file,
        }
    }

    fn task_env(&self) -> Vec<(String, String)> {
        vec![
            ("TASKDATA".to_string(), self.data_dir.to_str().unwrap().to_string()),
            ("TASKRC".to_string(), self.rc_file.to_str().unwrap().to_string()),
        ]
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn taskwarrior_available() -> bool {
    StdCommand::new("task")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn test_create_simple() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created 3 tasks"));
}

#[test]
fn test_create_with_subsections() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("2,3,1")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create with subsections should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created 6 tasks")); // 2+3+1
}

#[test]
fn test_create_with_offset_and_interval_days() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .arg("--offset")
        .arg("5d")
        .arg("--interval")
        .arg("7d")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create with day scheduling should succeed");
}

#[test]
fn test_create_with_hours() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .arg("--offset")
        .arg("2h")
        .arg("--interval")
        .arg("3h")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create with hour scheduling should succeed");
}

#[test]
fn test_create_with_minutes() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .arg("--offset")
        .arg("30m")
        .arg("--interval")
        .arg("45min")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create with minute scheduling should succeed");
}

#[test]
fn test_create_with_custom_unit() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .arg("-u")
        .arg("Lecture")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create with custom unit should succeed");
}

#[test]
fn test_create_with_skip_weekend() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .arg("--offset")
        .arg("1d")
        .arg("--interval")
        .arg("1d")
        .arg("--skip")
        .arg("weekend")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create with skip weekend should succeed");
}

#[test]
fn test_create_with_mixed_units() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .arg("--offset")
        .arg("1d")
        .arg("--interval")
        .arg("6h")
        .output()
        .unwrap();

    assert!(output.status.success(), "Create with mixed units should succeed");
}

#[test]
fn test_create_missing_interval() {
    if !taskwarrior_available() {
        println!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();

    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }

    let output = cmd
        .arg("create")
        .arg("TestProject")
        .arg("-p")
        .arg("3")
        .arg("--offset")
        .arg("5d")
        .output()
        .unwrap();

    assert!(!output.status.success(), "Should fail without interval");
}

#[test]
fn test_create_help() {
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("create").arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PROJECT"));
    assert!(stdout.contains("--offset"));
    assert!(stdout.contains("--interval"));
    assert!(stdout.contains("--unit"));
}
