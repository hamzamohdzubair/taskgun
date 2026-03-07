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
}
