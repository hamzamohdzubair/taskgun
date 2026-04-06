use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::Args;
use std::process::Command;
use std::str;

#[derive(Args)]
pub struct LinkArgs {
    /// Existing task ID to attach link to (mutually exclusive with task description)
    #[arg(short = 'i', long = "id")]
    pub id: Option<u32>,

    /// Task description and optional taskwarrior arguments (e.g., 'Buy milk' due:tomorrow +shopping)
    pub task_parts: Vec<String>,
}

/// Execute link command - create a task with clipboard link, or attach link to existing task
pub fn execute(args: LinkArgs) -> Result<()> {
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    // Validate mutual exclusion
    if args.id.is_some() && !args.task_parts.is_empty() {
        anyhow::bail!("Cannot use -i/--id together with a task description");
    }

    // Require at least one of id or task_parts
    if args.id.is_none() && args.task_parts.is_empty() {
        anyhow::bail!("Provide either a task description or -i <id>");
    }

    // Read clipboard content (always required)
    let mut clipboard = Clipboard::new()
        .context("Failed to access clipboard")?;
    let link = clipboard.get_text()
        .context("Failed to read from clipboard")?;
    let link = link.trim();

    if link.is_empty() {
        anyhow::bail!("Clipboard is empty");
    }

    let mut cmd = Command::new("task");
    cmd.arg("rc.confirmation=off");
    cmd.arg("rc.uda.link.type=string");
    cmd.arg("rc.uda.link.label=Link");

    if let Some(id) = args.id {
        // Fetch current description to prepend emoji if needed
        let export = Command::new("task")
            .arg(id.to_string())
            .arg("export")
            .output()
            .context("Failed to run task export")?;
        let json = str::from_utf8(&export.stdout).context("Invalid UTF-8 from task export")?;
        let desc = parse_description(json, id)?;
        let new_desc = if desc.starts_with("🔗") {
            desc
        } else {
            format!("🔗 {}", desc)
        };

        // Modify existing task
        cmd.arg(id.to_string());
        cmd.arg("modify");
        cmd.arg(format!("description:{}", new_desc));
        cmd.arg(format!("link:{}", link));
    } else {
        // Create new task with 🔗 prefix
        cmd.arg("add");
        for (i, part) in args.task_parts.iter().enumerate() {
            if i == 0 {
                cmd.arg(format!("🔗 {}", part));
            } else {
                cmd.arg(part);
            }
        }
        cmd.arg(format!("link:{}", link));
    }

    let output = cmd.output()
        .context("Failed to run task command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let filtered: String = stderr
            .lines()
            .filter(|line| !line.starts_with("Configuration override"))
            .collect::<Vec<_>>()
            .join("\n");
        if !filtered.trim().is_empty() {
            anyhow::bail!("{}", filtered);
        }
    }

    print!("{}", String::from_utf8_lossy(&output.stdout));
    println!("Added link: {}", link);

    Ok(())
}

fn parse_description(json: &str, id: u32) -> Result<String> {
    // task export returns a JSON array; find "description" field simply
    for line in json.lines() {
        if let Some(pos) = line.find("\"description\":") {
            let rest = &line[pos + 14..].trim_start();
            if let Some(rest) = rest.strip_prefix('"') {
                let end = rest.find('"').unwrap_or(rest.len());
                return Ok(rest[..end].to_string());
            }
        }
    }
    anyhow::bail!("Could not find description for task {}", id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_args_id_only() {
        let args = LinkArgs {
            id: Some(5),
            task_parts: vec![],
        };
        assert_eq!(args.id, Some(5));
        assert!(args.task_parts.is_empty());
    }

    #[test]
    fn test_link_args_task_parts_only() {
        let args = LinkArgs {
            id: None,
            task_parts: vec!["Buy".to_string(), "milk".to_string()],
        };
        assert!(args.id.is_none());
        assert_eq!(args.task_parts.len(), 2);
    }
}
