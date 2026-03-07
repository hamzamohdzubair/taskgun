use anyhow::{Context, Result};
use std::process::Command;

/// Execute a keyword search across tasks
pub fn execute(keyword: &str, use_regex: bool) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    let mut cmd = Command::new("task");

    if use_regex {
        // Regex mode: case-sensitive regex search
        cmd.arg("rc.regex=on");
        cmd.arg("rc.verbose=nothing");
        cmd.arg(keyword);
    } else {
        // Default mode: case-insensitive plain text search
        cmd.arg("rc.search.case.sensitive=no");
        cmd.arg("rc.verbose=nothing");
        cmd.arg(keyword);
    }

    cmd.arg("list");

    let output = cmd.output()
        .context("Failed to execute task command")?;

    // Print stdout (task list)
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }

    // Check stderr for messages
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        // "No matches" is a normal result, not an error - print it normally
        if stderr.contains("No matches") {
            println!("No matches.");
        } else {
            // Other errors should be printed to stderr
            eprint!("{}", stderr);
        }
    }

    Ok(())
}
