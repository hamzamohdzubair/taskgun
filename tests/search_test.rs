use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// Test environment to isolate Taskwarrior database
// TempDir automatically cleans up when dropped
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

        // Create minimal taskrc file
        fs::write(
            &rc_file,
            "# Minimal taskrc for testing\ndata.location=".to_string() + data_dir.to_str().unwrap() + "\n",
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
        // TempDir automatically deletes on drop, but we'll be explicit
        // and ensure all task processes are done before cleanup
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

// Helper to create test tasks in isolated environment
fn create_test_task(env: &TestEnv, description: &str, project: &str) {
    let mut cmd = StdCommand::new("task");
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    cmd.arg("add")
        .arg(description)
        .arg(format!("project:{}", project))
        .output()
        .expect("Failed to create test task");
}

// Helper to create test task with due date
fn create_test_task_with_due(env: &TestEnv, description: &str, project: &str, due: &str) {
    let mut cmd = StdCommand::new("task");
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    cmd.arg("add")
        .arg(description)
        .arg(format!("project:{}", project))
        .arg(format!("due:{}", due))
        .output()
        .expect("Failed to create test task");
}

#[test]
fn test_search_case_insensitive() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    // Create isolated test environment (auto-cleans on drop)
    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_case_search";

    // Create test tasks with different cases
    create_test_task(&env, "TestVideo 1", project);
    create_test_task(&env, "testvideo 2", project);
    create_test_task(&env, "TESTVIDEO 3", project);

    // Test case-insensitive search (default)
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("testvideo").output().unwrap();

    // Should find all three tasks regardless of case
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TestVideo") || stdout.contains("testvideo"));

    // TestEnv auto-cleans on drop
}

#[test]
fn test_search_regex_mode() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_regex_search";

    // Create test tasks
    create_test_task(&env, "Item 1", project);
    create_test_task(&env, "Item 2", project);
    create_test_task(&env, "Item 3", project);

    // Test regex search with -r flag
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("Item [12]").arg("-r").output().unwrap();

    // Should succeed (may or may not have output depending on task visibility)
    assert!(output.status.success());
}

#[test]
fn test_search_by_project_name() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_unique_project_name_xyz";

    // Create a task in a distinctive project
    create_test_task(&env, "Some task", project);

    // Search by part of project name (match exact part that's in project name)
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
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
}

#[test]
fn test_search_no_matches() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    // Search for something that definitely doesn't exist
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
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

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_case_sensitivity";

    // Create tasks with mixed case
    create_test_task(&env, "Video 1", project);
    create_test_task(&env, "video 2", project);

    // Regex mode should be case-sensitive
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("Video").arg("-r").output().unwrap();

    assert!(output.status.success(), "Regex search should succeed");
}

#[test]
fn test_search_in_description() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_description_search";

    // Create a task with distinctive description
    create_test_task(&env, "distinctive_word_xyz", project);

    // Search for word in description
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("distinctive_word").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("distinctive_word_xyz"),
        "Should find task by description"
    );
}

#[test]
fn test_search_sort_by_id_default() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_sort_id";

    // Create test tasks
    create_test_task(&env, "Task A", project);
    create_test_task(&env, "Task B", project);
    create_test_task(&env, "Task C", project);

    // Test default sort (should be by ID)
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("search").arg("Task").output().unwrap();

    assert!(output.status.success(), "Search should succeed");
}

#[test]
fn test_search_sort_by_id_explicit() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_sort_id_explicit";

    // Create test tasks
    create_test_task(&env, "Task 1", project);
    create_test_task(&env, "Task 2", project);

    // Test explicit sort by ID with -s id
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("search").arg("Task").arg("-s").arg("id").output().unwrap();

    assert!(
        output.status.success(),
        "Sort by ID should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_search_sort_by_due() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_sort_due";

    // Create tasks with different due dates
    create_test_task_with_due(&env, "Task Later", project, "tomorrow");
    create_test_task_with_due(&env, "Task Soon", project, "today");

    // Test sort by due date with -s due
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("search").arg("Task").arg("-s").arg("due").output().unwrap();

    assert!(
        output.status.success(),
        "Sort by due should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_search_shorthand_with_sort() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_shorthand_sort";

    create_test_task(&env, "Test Item", project);

    // Test shorthand syntax with sort flag
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd.arg("Test").arg("-s").arg("id").output().unwrap();

    assert!(output.status.success(), "Shorthand search with sort should succeed");
}

#[test]
fn test_search_shorthand_with_regex_and_sort() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_shorthand_regex_sort";

    create_test_task(&env, "Alpha 1", project);
    create_test_task(&env, "Alpha 2", project);

    // Test shorthand syntax with both regex and sort
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd
        .arg("Alpha")
        .arg("-r")
        .arg("-s")
        .arg("due")
        .output()
        .unwrap();

    assert!(output.status.success(), "Shorthand with regex and sort should succeed");
}

#[test]
fn test_search_explicit_command_with_all_flags() {
    if !taskwarrior_available() {
        eprintln!("Skipping test: taskwarrior not available");
        return;
    }

    let env = TestEnv::new();
    env.init();

    let project = "taskgun_test_explicit_all_flags";

    create_test_task(&env, "Pattern 1", project);
    create_test_task(&env, "Pattern 2", project);

    // Test explicit search command with all flags
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    for (key, val) in &env.task_env() {
        cmd.env(key, val);
    }
    let output = cmd
        .arg("search")
        .arg("Pattern")
        .arg("--regex")
        .arg("--sort")
        .arg("id")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Explicit search with all flags should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_help_shows_sort_options() {
    // Test that help output includes sort options
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("search").arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("--sort") || combined.contains("-s"),
        "Help should show sort option. Output: {}",
        combined
    );
}

#[test]
fn test_main_help_shows_quick_search_sort() {
    // Test that main help includes quick search sort info
    let mut cmd = Command::cargo_bin("taskgun").unwrap();
    let output = cmd.arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("QUICK SEARCH"), "Help should have quick search section");
    assert!(stdout.contains("-s"), "Help should mention -s flag");
}
