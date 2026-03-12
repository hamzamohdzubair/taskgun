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

        // Create data directory
        fs::create_dir(&data_dir).expect("Failed to create data dir");

        // Create minimal taskrc file with UDA definition
        fs::write(
            &rc_file,
            format!(
                "# Minimal taskrc for testing\ndata.location={}\nuda.plan.type=numeric\nuda.plan.label=Plan\n",
                data_dir.to_str().unwrap()
            ),
        )
        .expect("Failed to write taskrc");

        Self {
            temp_dir,
            data_dir,
            rc_file,
        }
    }

    // Get environment variables for task commands
    fn task_env(&self) -> Vec<(String, String)> {
        vec![
            ("TASKDATA".to_string(), self.data_dir.to_str().unwrap().to_string()),
            ("TASKRC".to_string(), self.rc_file.to_str().unwrap().to_string()),
        ]
    }

    // Initialize empty task database
    fn init(&self) {
        let mut cmd = StdCommand::new("task");
        for (key, val) in &self.task_env() {
            cmd.env(key, val);
        }
        let _ = cmd.arg("version").output();
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // TempDir automatically deletes on drop
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

// Helper to check if taskwarrior is available
fn taskwarrior_available() -> bool {
    StdCommand::new("task")
        .arg("--version")
        .output()
        .is_ok()
}

// Helper to create test task
fn create_test_task(env: &TestEnv, description: &str) -> String {
    let mut cmd = StdCommand::new("task");
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("add")
        .arg(description)
        .output()
        .expect("Failed to create test task");

    // Extract task ID from output (e.g., "Created task 1.")
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .nth(2)
        .and_then(|s| s.trim_end_matches('.').parse::<String>().ok())
        .expect("Failed to extract task ID")
}

#[test]
fn test_plan_display_no_plan() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    // Create some tasks without plan
    create_test_task(&env, "Task 1");
    create_test_task(&env, "Task 2");

    // Run taskgun plan (display)
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    cmd.arg("plan");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should display "No plan" when no tasks have plan UDA
    assert!(stdout.contains("No plan"));
}

#[test]
fn test_plan_set_and_display() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    // Create test tasks
    let id1 = create_test_task(&env, "Task 1");
    let id2 = create_test_task(&env, "Task 2");
    let id3 = create_test_task(&env, "Task 3");
    let id4 = create_test_task(&env, "Task 4");

    // Set plan in custom order (4, 1, 3, 2)
    let ids = format!("{},{},{},{}", id4, id1, id3, id2);
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    cmd.arg("plan").arg(&ids);

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should confirm plan was set
    assert!(stdout.contains("Plan set for 4 tasks"));

    // Display plan - should show tasks sorted by plan value
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    cmd.arg("plan");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Debug output if test fails
    if !output.status.success() {
        eprintln!("stdout: {}", stdout);
        eprintln!("stderr: {}", stderr);
    }

    // Should show plan summary
    assert!(stdout.contains("plan:"));
}

#[test]
fn test_plan_invalid_ids() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    // Try to set plan with invalid IDs
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    cmd.arg("plan").arg("abc,def");

    let output = cmd.output().unwrap();

    // Should fail with error message
    assert!(!output.status.success());
}

#[test]
fn test_plan_empty_ids() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    // Try to set plan with empty string
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    cmd.arg("plan").arg("");

    let output = cmd.output().unwrap();

    // Should fail with error message
    assert!(!output.status.success());
}
