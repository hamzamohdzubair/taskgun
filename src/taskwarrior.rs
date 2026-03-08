use anyhow::{Context, Result};
use std::process::Command;

/// Check if Taskwarrior is installed and return its version
pub fn check_taskwarrior() -> Result<String> {
    let output = Command::new("task")
        .arg("--version")
        .output()
        .context("Failed to execute 'task --version'. Is Taskwarrior installed?")?;

    if !output.status.success() {
        anyhow::bail!("Taskwarrior returned non-zero exit code");
    }

    let version =
        String::from_utf8(output.stdout).context("Failed to parse Taskwarrior version output")?;

    Ok(version.trim().to_string())
}

/// Add a task to Taskwarrior
///
/// # Arguments
/// * `description` - Task description (e.g., "Video 1.2")
/// * `project` - Project name
/// * `due` - Optional due date in Taskwarrior format (e.g., "2025-01-20T14:30" or "today+5d")
pub fn add_task(description: &str, project: &str, due: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("task");
    cmd.arg("add")
        .arg(description)
        .arg(format!("project:{}", project));

    if let Some(due_date) = due {
        cmd.arg(format!("due:{}", due_date));
    }

    let output = cmd
        .output()
        .context("Failed to execute 'task add' command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to add task '{}': {}", description, stderr.trim());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

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

    #[test]
    fn test_check_taskwarrior() {
        // This test will fail if Taskwarrior is not installed
        // In CI, you might want to skip this or mock it
        match check_taskwarrior() {
            Ok(version) => {
                assert!(!version.is_empty());
                println!("Taskwarrior version: {}", version);
            }
            Err(e) => {
                println!("Taskwarrior not found: {}", e);
                // Don't fail the test if Taskwarrior is not installed
                // This allows the code to compile without Taskwarrior present
            }
        }
    }

    #[test]
    fn test_add_task_simple() {
        if check_taskwarrior().is_err() {
            println!("Skipping test: taskwarrior not available");
            return;
        }

        let env = TestEnv::new();

        // Set environment for this test
        std::env::set_var("TASKDATA", env.data_dir.to_str().unwrap());
        std::env::set_var("TASKRC", env.rc_file.to_str().unwrap());

        let result = add_task("Test Task 1", "TestProject", None);
        assert!(result.is_ok(), "Should add task successfully");

        // Clean up env vars
        std::env::remove_var("TASKDATA");
        std::env::remove_var("TASKRC");
    }

    #[test]
    fn test_add_task_with_due_date() {
        if check_taskwarrior().is_err() {
            println!("Skipping test: taskwarrior not available");
            return;
        }

        let env = TestEnv::new();

        std::env::set_var("TASKDATA", env.data_dir.to_str().unwrap());
        std::env::set_var("TASKRC", env.rc_file.to_str().unwrap());

        let result = add_task("Test Task 2", "TestProject", Some("today+5d"));
        assert!(result.is_ok(), "Should add task with due date");

        std::env::remove_var("TASKDATA");
        std::env::remove_var("TASKRC");
    }

    #[test]
    fn test_add_task_with_timestamp() {
        if check_taskwarrior().is_err() {
            println!("Skipping test: taskwarrior not available");
            return;
        }

        let env = TestEnv::new();

        std::env::set_var("TASKDATA", env.data_dir.to_str().unwrap());
        std::env::set_var("TASKRC", env.rc_file.to_str().unwrap());

        let result = add_task("Test Task 3", "TestProject", Some("2026-12-31T15:30"));
        assert!(result.is_ok(), "Should add task with timestamp");

        std::env::remove_var("TASKDATA");
        std::env::remove_var("TASKRC");
    }
}
