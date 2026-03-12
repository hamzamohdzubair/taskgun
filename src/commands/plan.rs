use anyhow::{Context, Result};
use clap::Args;
use std::process::Command;

#[derive(Args)]
pub struct PlanArgs {
    /// Comma-separated task IDs to add to plan in sequence (e.g., "1,5,9,4")
    /// If not provided, displays current plan
    pub ids: Option<String>,
}

/// Execute plan command - either set plan on tasks or display current plan
pub fn execute(ids_opt: Option<&str>) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    match ids_opt {
        Some(ids) => set_plan(ids),
        None => display_plan(),
    }
}

/// Set plan values on specified tasks in sequence
fn set_plan(ids: &str) -> Result<()> {
    // Parse comma-separated task IDs
    let task_ids: Result<Vec<u32>, _> = ids
        .split(',')
        .map(|s| s.trim().parse::<u32>())
        .collect();

    let task_ids = task_ids.context("Invalid task ID format. Use comma-separated numbers (e.g., 1,5,9,4)")?;

    if task_ids.is_empty() {
        anyhow::bail!("No task IDs provided");
    }

    // Assign plan values sequentially
    for (index, task_id) in task_ids.iter().enumerate() {
        let plan_value = index + 1;

        let mut cmd = Command::new("task");

        // Define the plan UDA via rc overrides (no .taskrc modification needed)
        cmd.arg("rc.uda.plan.type=numeric");
        cmd.arg("rc.uda.plan.label=Plan");

        cmd.arg(task_id.to_string());
        cmd.arg("modify");
        cmd.arg(format!("plan:{}", plan_value));

        let output = cmd.output()
            .context(format!("Failed to set plan on task {}", task_id))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to set plan on task {}: {}", task_id, stderr);
        }
    }

    println!("Plan set for {} tasks", task_ids.len());

    // Display the plan after setting it
    println!();
    display_plan()
}

/// Display tasks with plan UDA sorted by plan value
fn display_plan() -> Result<()> {
    let mut cmd = Command::new("task");

    // Enable colors for better readability
    cmd.arg("rc.color=on");
    cmd.arg("rc._forcecolor=on");
    cmd.arg("rc.verbose=label");

    // Use actual terminal width for flexible column layout
    if let Some((width, _)) = term_size::dimensions() {
        cmd.arg(format!("rc.defaultwidth={}", width));
    }

    // Define the plan UDA if not already defined
    cmd.arg("rc.uda.plan.type=numeric");
    cmd.arg("rc.uda.plan.label=Plan");

    // Filter for tasks with plan UDA
    cmd.arg("plan.any:");

    // Sort by plan ascending
    cmd.arg("rc.report.next.sort=plan+,id+");

    // Use all default next report columns with plan inserted as second column
    // Default: id,start.age,entry.age,depends,priority,project,tags,recur,scheduled.countdown,due.relative,until.remaining,description,urgency
    cmd.arg("rc.report.next.columns=id,plan,start.age,entry.age,depends,priority,project,tags,recur,scheduled.countdown,due.relative,until.remaining,description,urgency");
    cmd.arg("rc.report.next.labels=ID,Plan,Active,Age,Deps,P,Project,Tag,Recur,S,Due,Until,Description,Urg");

    cmd.arg("next");

    let output = cmd.output()
        .context("Failed to execute task command")?;

    // Check stderr for messages
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        // "No matches" means no tasks with plan
        if stderr.contains("No matches") {
            println!("No plan");
            return Ok(());
        } else {
            // Filter out configuration override warnings
            let filtered_stderr: String = stderr
                .lines()
                .filter(|line| !line.starts_with("Configuration override"))
                .collect::<Vec<_>>()
                .join("\n");

            if !filtered_stderr.trim().is_empty() {
                eprint!("{}", filtered_stderr);
            }
        }
    }

    // Process stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        // Print blank line before table
        println!();

        // Print the output directly (no sequence breaks for plan view)
        print!("{}", stdout);
        let task_count = count_tasks(&stdout);

        // Print blank line after table and summary
        println!();
        print_summary(task_count);
    } else if stderr.is_empty() && !output.status.success() {
        println!("No plan");
    }

    Ok(())
}

/// Count the number of tasks in taskwarrior output
fn count_tasks(output: &str) -> usize {
    output.lines()
        .filter(|line| extract_task_id(line).is_some())
        .count()
}

/// Print summary statistics
fn print_summary(task_count: usize) {
    println!("plan: {}", task_count);
}

/// Strip ANSI escape sequences from a string
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ANSI escape sequence
            if chars.next() == Some('[') {
                // Skip until we find a letter (the command character)
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Extract task ID from a taskwarrior list output line
fn extract_task_id(line: &str) -> Option<u32> {
    // Strip ANSI color codes first
    let clean_line = strip_ansi(line);

    // Skip header lines and empty lines
    if clean_line.trim().is_empty() || clean_line.starts_with("ID") || clean_line.starts_with("--") {
        return None;
    }

    // Try to parse the first whitespace-separated token as a number
    clean_line.split_whitespace()
        .next()
        .and_then(|token| token.parse::<u32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        let line = "\x1b[48;5;234m  4\x1b[0m test";
        assert_eq!(strip_ansi(line), "  4 test");

        let plain = "5 Deep Learning";
        assert_eq!(strip_ansi(plain), "5 Deep Learning");

        let multi = "\x1b[38;5;1m  1\x1b[0m\x1b[38;5;1m \x1b[0m\x1b[38;5;1m5d\x1b[0m";
        assert_eq!(strip_ansi(multi), "  1 5d");
    }

    #[test]
    fn test_extract_task_id_valid_line() {
        let line = "5 Deep Learning Video 1";
        assert_eq!(extract_task_id(line), Some(5));
    }

    #[test]
    fn test_extract_task_id_with_ansi_codes() {
        let line = "\x1b[48;5;234m  4\x1b[0m\x1b[48;5;234m \x1b[0m\x1b[48;5;234m4d\x1b[0m test";
        assert_eq!(extract_task_id(line), Some(4));

        let line2 = "\x1b[38;5;1m 12\x1b[0m\x1b[38;5;1m \x1b[0m task";
        assert_eq!(extract_task_id(line2), Some(12));
    }

    #[test]
    fn test_extract_task_id_header_line() {
        let line = "ID Project Description";
        assert_eq!(extract_task_id(line), None);
    }

    #[test]
    fn test_extract_task_id_separator_line() {
        let line = "-- ------- -----------";
        assert_eq!(extract_task_id(line), None);
    }

    #[test]
    fn test_extract_task_id_empty_line() {
        let line = "";
        assert_eq!(extract_task_id(line), None);
    }

    #[test]
    fn test_count_tasks() {
        let output = "ID Plan Description\n5 1 Task 5\n6 2 Task 6\n7 3 Task 7\n";
        assert_eq!(count_tasks(output), 3);

        let empty = "";
        assert_eq!(count_tasks(empty), 0);

        let with_header_only = "ID Plan Description\n";
        assert_eq!(count_tasks(with_header_only), 0);
    }

    #[test]
    fn test_plan_args_with_ids() {
        let args = PlanArgs {
            ids: Some("1,2,3".to_string()),
        };
        assert_eq!(args.ids, Some("1,2,3".to_string()));
    }

    #[test]
    fn test_plan_args_without_ids() {
        let args = PlanArgs {
            ids: None,
        };
        assert_eq!(args.ids, None);
    }
}
