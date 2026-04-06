use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::Args;
use std::process::Command;

#[derive(Args)]
pub struct AddArgs {
    /// Attach clipboard content as link/url UDA to the task
    #[arg(short = 'l', long = "link")]
    pub link: bool,

    /// Task description and optional taskwarrior arguments (e.g., 'Buy milk' due:tomorrow +shopping)
    pub task_parts: Vec<String>,
}

/// Execute add command - create a task with optional clipboard link
pub fn execute(args: AddArgs) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    if args.task_parts.is_empty() {
        anyhow::bail!("Task description required");
    }

    // Get clipboard content if -l flag is set
    let clipboard_content = if args.link {
        let mut clipboard = Clipboard::new()
            .context("Failed to access clipboard")?;

        let content = clipboard.get_text()
            .context("Failed to read from clipboard")?;

        if content.trim().is_empty() {
            anyhow::bail!("Clipboard is empty");
        }

        Some(content.trim().to_string())
    } else {
        None
    };

    // Build the task add command
    let mut cmd = Command::new("task");

    // Disable confirmation
    cmd.arg("rc.confirmation=off");

    // Define the link UDA (we'll use 'link' as the UDA name)
    cmd.arg("rc.uda.link.type=string");
    cmd.arg("rc.uda.link.label=Link");

    cmd.arg("add");

    // Add task parts (description and any taskwarrior arguments)
    // If link flag is used, prepend 🔗 emoji to the description
    for (i, part) in args.task_parts.iter().enumerate() {
        if i == 0 && args.link {
            // Prepend link emoji to the description (first part)
            cmd.arg(format!("🔗 {}", part));
        } else {
            cmd.arg(part);
        }
    }

    // Add link from clipboard if present
    if let Some(link) = &clipboard_content {
        cmd.arg(format!("link:{}", link));
    }

    let output = cmd.output()
        .context("Failed to add task")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Filter out configuration override warnings
        let filtered_stderr: String = stderr
            .lines()
            .filter(|line| !line.starts_with("Configuration override"))
            .collect::<Vec<_>>()
            .join("\n");

        if !filtered_stderr.trim().is_empty() {
            anyhow::bail!("Failed to add task: {}", filtered_stderr);
        }
    }

    // Print success message
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{}", stdout);

    if let Some(link) = clipboard_content {
        println!("Added link: {}", link);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_args() {
        let args = AddArgs {
            link: true,
            task_parts: vec!["Buy".to_string(), "milk".to_string()],
        };
        assert!(args.link);
        assert_eq!(args.task_parts.len(), 2);
    }
}
