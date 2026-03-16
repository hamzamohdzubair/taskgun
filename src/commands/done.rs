use anyhow::{Context, Result};
use clap::Args;
use std::process::Command;

#[derive(Args)]
pub struct DoneArgs {
    /// Task ID to mark as complete
    pub id: u32,
}

/// Execute done command - mark task as complete and remove from plan (shifting remaining tasks)
pub fn execute(task_id: u32) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    // Get all tasks with plan values BEFORE marking the task as done
    // (so we can find the task's plan value while it's still pending)
    let tasks_with_plan = get_tasks_with_plan()?;

    // Find the task with the given ID
    let target_task = tasks_with_plan.iter()
        .find(|t| t.id == task_id);

    let target_plan = target_task.map(|t| t.plan);

    // Mark the task as complete
    mark_task_done(task_id)?;

    // If the task had a plan value, shift all tasks with higher plan values up
    if let Some(plan_value) = target_plan {
        for task in tasks_with_plan.iter().filter(|t| t.plan > plan_value && t.id != task_id) {
            set_plan_on_task(task.id, task.plan - 1)?;
        }
        println!("Task {} marked as done (shifted remaining tasks up in plan)", task_id);
    } else {
        println!("Task {} marked as done", task_id);
    }

    Ok(())
}

/// Mark a task as complete using `task <id> done`
fn mark_task_done(task_id: u32) -> Result<()> {
    let mut cmd = Command::new("task");

    // Confirm without prompting
    cmd.arg("rc.confirmation=off");

    cmd.arg(task_id.to_string());
    cmd.arg("done");

    let output = cmd.output()
        .context(format!("Failed to mark task {} as done", task_id))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Filter out configuration override warnings
        let filtered_stderr: String = stderr
            .lines()
            .filter(|line| !line.starts_with("Configuration override"))
            .collect::<Vec<_>>()
            .join("\n");

        if !filtered_stderr.trim().is_empty() {
            anyhow::bail!("Failed to mark task {} as done: {}", task_id, filtered_stderr);
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_done_args() {
        let args = DoneArgs { id: 5 };
        assert_eq!(args.id, 5);
    }
}
