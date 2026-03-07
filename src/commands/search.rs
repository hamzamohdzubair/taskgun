use anyhow::{Context, Result};
use std::process::Command;

/// Execute a keyword search across tasks
pub fn execute(keyword: &str) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    // Build search filter for taskwarrior
    // Search in both project and description fields
    let filter = format!(
        "( project.contains:{} or description.contains:{} )",
        keyword, keyword
    );

    // Execute taskwarrior list command with filter
    let output = Command::new("task")
        .arg(&filter)
        .arg("list")
        .output()
        .context("Failed to execute task command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Taskwarrior command failed: {}", stderr);
    }

    // Print the output directly
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{}", stdout);

    Ok(())
}
