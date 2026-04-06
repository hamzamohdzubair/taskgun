use anyhow::{Context, Result};
use clap::Args;
use std::process::Command;

#[derive(Args)]
pub struct OpenArgs {
    /// Task ID to open link from
    pub id: u32,
}

/// Execute open command - open the link/url from a task in the browser
pub fn execute(task_id: u32) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    // Get the task's link UDA
    let link = get_task_link(task_id)?;

    if link.is_empty() {
        anyhow::bail!("Task {} has no link attached", task_id);
    }

    // Open the link in the browser
    opener::open(&link)
        .context(format!("Failed to open link: {}", link))?;

    println!("Opening: {}", link);

    Ok(())
}

/// Get the link UDA from a task
fn get_task_link(task_id: u32) -> Result<String> {
    let mut cmd = Command::new("task");

    // Define the link UDA
    cmd.arg("rc.uda.link.type=string");
    cmd.arg("rc.uda.link.label=Link");

    // Export the specific task
    cmd.arg(task_id.to_string());
    cmd.arg("export");

    let output = cmd.output()
        .context(format!("Failed to query task {}", task_id))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Filter out configuration override warnings
        let filtered_stderr: String = stderr
            .lines()
            .filter(|line| !line.starts_with("Configuration override"))
            .collect::<Vec<_>>()
            .join("\n");

        if !filtered_stderr.trim().is_empty() {
            anyhow::bail!("Failed to query task {}: {}", task_id, filtered_stderr);
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output
    let tasks: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .context("Failed to parse task export JSON")?;

    if tasks.is_empty() {
        anyhow::bail!("Task {} not found", task_id);
    }

    // Get the link field from the first (and only) task
    let link = tasks[0]
        .get("link")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(link)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_args() {
        let args = OpenArgs { id: 45 };
        assert_eq!(args.id, 45);
    }
}
