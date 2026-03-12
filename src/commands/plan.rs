use anyhow::{Context, Result};
use clap::Args;
use std::process::{Command, Stdio};
use std::io::Write;

#[derive(Args)]
pub struct PlanArgs {
    /// Command: comma-separated task IDs to add (e.g., "1,5,9,4"), "clear"/"clean" to remove all, or "rm" to remove a specific task
    /// If not provided, displays current plan
    pub command: Option<String>,

    /// Task ID (required when command is "rm")
    pub id: Option<u32>,
}

/// Execute plan command - either set plan on tasks, clear plan, remove task, or display current plan
pub fn execute(command_opt: Option<&str>, id_opt: Option<u32>) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    match (command_opt, id_opt) {
        (Some("clear") | Some("clean"), None) => clear_plan(),
        (Some("rm"), Some(id)) => remove_from_plan(id),
        (Some("rm"), None) => anyhow::bail!("Task ID required for 'rm' command. Usage: taskgun plan rm <id>"),
        (Some(ids), None) => set_plan(ids),
        (Some(_), Some(_)) => anyhow::bail!("Unexpected arguments. Usage: taskgun plan [<ids>|clear|rm <id>]"),
        (None, None) => display_plan(),
        (None, Some(_)) => anyhow::bail!("Task ID provided without 'rm' command"),
    }
}

/// Set plan values on specified tasks in sequence, continuing from existing plan
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

    // Get the maximum existing plan value
    let max_plan = get_max_plan_value()?;
    let start_plan = max_plan + 1;

    // Assign plan values sequentially, starting after the highest existing plan value
    for (index, task_id) in task_ids.iter().enumerate() {
        let plan_value = start_plan + index as u32;

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

    println!("Plan set for {} tasks (starting from plan {})", task_ids.len(), start_plan);

    // Display the plan after setting it
    println!();
    display_plan()
}

/// Get the maximum plan value from all existing pending tasks
fn get_max_plan_value() -> Result<u32> {
    let mut cmd = Command::new("task");

    // Define the plan UDA
    cmd.arg("rc.uda.plan.type=numeric");
    cmd.arg("rc.uda.plan.label=Plan");

    // Export only pending tasks with plan values
    cmd.arg("status:pending");
    cmd.arg("plan.any:");
    cmd.arg("export");

    let output = cmd.output()
        .context("Failed to query existing plan values")?;

    if !output.status.success() {
        // If no tasks with plan exist, return 0
        return Ok(0);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output to find max plan value
    let tasks: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .context("Failed to parse task export JSON")?;

    let max = tasks.iter()
        .filter_map(|task| task.get("plan"))
        .filter_map(|plan| plan.as_u64())
        .max()
        .unwrap_or(0) as u32;

    Ok(max)
}

/// Clear all plan values from all pending tasks
fn clear_plan() -> Result<()> {
    let mut cmd = Command::new("task");

    // Confirm without prompting (must be first)
    cmd.arg("rc.confirmation=off");

    // Define the plan UDA
    cmd.arg("rc.uda.plan.type=numeric");
    cmd.arg("rc.uda.plan.label=Plan");

    // Modify only pending tasks with plan values to remove plan
    cmd.arg("status:pending");
    cmd.arg("plan.any:");
    cmd.arg("modify");
    cmd.arg("plan:");

    // Set up stdin to provide "all" answer for bulk modification prompt
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .context("Failed to spawn task command")?;

    // Write "all" to stdin to answer the confirmation prompt
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"all\n")
            .context("Failed to write to stdin")?;
    }

    let output = child.wait_with_output()
        .context("Failed to wait for task command")?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Filter out configuration override warnings
    let filtered_stderr: String = stderr
        .lines()
        .filter(|line| !line.starts_with("Configuration override"))
        .collect::<Vec<_>>()
        .join("\n");

    if !output.status.success() {
        if stderr.contains("No matches") {
            println!("No plan to clear");
            return Ok(());
        }
        if !filtered_stderr.trim().is_empty() {
            anyhow::bail!("Failed to clear plan: {}", filtered_stderr);
        }
    }

    println!("Plan cleared");
    Ok(())
}

/// Remove a task from the plan and shift all tasks with higher plan values up
fn remove_from_plan(task_id: u32) -> Result<()> {
    // Get all tasks with plan values
    let tasks_with_plan = get_tasks_with_plan()?;

    // Find the task with the given ID
    let target_task = tasks_with_plan.iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| anyhow::anyhow!("Task {} not found in plan", task_id))?;

    let target_plan = target_task.plan;

    // Remove plan from target task
    remove_plan_from_task(task_id)?;

    // Decrement plan values for all tasks with plan > target_plan
    for task in tasks_with_plan.iter().filter(|t| t.plan > target_plan) {
        set_plan_on_task(task.id, task.plan - 1)?;
    }

    println!("Task {} removed from plan (shifted remaining tasks up)", task_id);

    // Display the plan after modification
    println!();
    display_plan()
}

/// Struct to hold task ID and plan value
struct TaskWithPlan {
    id: u32,
    plan: u32,
}

/// Get all tasks with plan values
fn get_tasks_with_plan() -> Result<Vec<TaskWithPlan>> {
    let mut cmd = Command::new("task");

    // Define the plan UDA
    cmd.arg("rc.uda.plan.type=numeric");
    cmd.arg("rc.uda.plan.label=Plan");

    // Export only pending tasks with plan values (status:pending to exclude completed/deleted)
    cmd.arg("status:pending");
    cmd.arg("plan.any:");
    cmd.arg("export");

    let output = cmd.output()
        .context("Failed to query tasks with plan values")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output
    let tasks: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .context("Failed to parse task export JSON")?;

    let result = tasks.iter()
        .filter_map(|task| {
            let id = task.get("id")?.as_u64()? as u32;
            let plan = task.get("plan")?.as_u64()? as u32;
            // Only include valid task IDs (> 0)
            if id > 0 {
                Some(TaskWithPlan { id, plan })
            } else {
                None
            }
        })
        .collect();

    Ok(result)
}

/// Remove plan value from a specific task
fn remove_plan_from_task(task_id: u32) -> Result<()> {
    let mut cmd = Command::new("task");

    // Confirm without prompting (must be first)
    cmd.arg("rc.confirmation=off");

    // Define the plan UDA
    cmd.arg("rc.uda.plan.type=numeric");
    cmd.arg("rc.uda.plan.label=Plan");

    cmd.arg(task_id.to_string());
    cmd.arg("modify");
    cmd.arg("plan:");

    let output = cmd.output()
        .context(format!("Failed to remove plan from task {}", task_id))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Filter out configuration override warnings
        let filtered_stderr: String = stderr
            .lines()
            .filter(|line| !line.starts_with("Configuration override"))
            .collect::<Vec<_>>()
            .join("\n");

        if !filtered_stderr.trim().is_empty() {
            anyhow::bail!("Failed to remove plan from task {}: {}", task_id, filtered_stderr);
        }
    }

    Ok(())
}

/// Set plan value on a specific task
fn set_plan_on_task(task_id: u32, plan_value: u32) -> Result<()> {
    let mut cmd = Command::new("task");

    // Confirm without prompting (must be first)
    cmd.arg("rc.confirmation=off");

    // Define the plan UDA
    cmd.arg("rc.uda.plan.type=numeric");
    cmd.arg("rc.uda.plan.label=Plan");

    cmd.arg(task_id.to_string());
    cmd.arg("modify");
    cmd.arg(format!("plan:{}", plan_value));

    let output = cmd.output()
        .context(format!("Failed to set plan on task {}", task_id))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Filter out configuration override warnings
        let filtered_stderr: String = stderr
            .lines()
            .filter(|line| !line.starts_with("Configuration override"))
            .collect::<Vec<_>>()
            .join("\n");

        if !filtered_stderr.trim().is_empty() {
            anyhow::bail!("Failed to set plan on task {}: {}", task_id, filtered_stderr);
        }
    }

    Ok(())
}

/// Display tasks with plan UDA sorted by plan value
pub fn display_plan() -> Result<()> {
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

    // Filter for pending tasks with plan UDA
    cmd.arg("status:pending");
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
