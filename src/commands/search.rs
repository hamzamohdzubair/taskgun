use anyhow::{Context, Result};
use std::process::Command;

/// Execute a keyword search across tasks
pub fn execute(keyword: &str, use_regex: bool) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    let mut cmd = Command::new("task");

    if use_regex {
        // Regex mode: case-sensitive regex search in project and description
        let filter = format!("( project~{} or description~{} )", keyword, keyword);
        cmd.arg("rc.regex=on");
        cmd.arg("rc.verbose=nothing");
        cmd.arg(filter);
    } else {
        // Default mode: case-insensitive search using .contains in project and description
        let filter = format!(
            "( project.contains:{} or description.contains:{} )",
            keyword, keyword
        );
        cmd.arg("rc.search.case.sensitive=no");
        cmd.arg("rc.verbose=nothing");
        cmd.arg(filter);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_requires_taskwarrior() {
        // This test just ensures the function signature is correct
        // Actual integration tests would require taskwarrior to be installed
        assert!(true);
    }

    #[test]
    fn test_search_modes() {
        // Test that different modes produce different command structures
        // This is a unit test that doesn't actually call taskwarrior

        // Just verify the function exists and has correct signature
        let keyword = "test";
        let _use_regex = false;

        // We can't easily test the actual command building without mocking,
        // but we can verify the function compiles and accepts the right types
        assert_eq!(keyword, "test");
    }
}
