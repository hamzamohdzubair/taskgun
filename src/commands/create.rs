use anyhow::{Context, Result};
use chrono::Local;
use clap::Args;

use crate::scheduling::{format_for_taskwarrior, format_relative_days, ScheduleConfig};
use crate::taskwarrior;

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Project name (required)
    #[arg(short = 'p', long, required = true)]
    project: String,

    /// Number of chapters (default: 10, overridden by subsections if present)
    #[arg(short = 'n', long, default_value = "10")]
    count: u32,

    /// Task name prefix (default: "Video")
    #[arg(short = 'u', long, default_value = "Video")]
    unit: String,

    /// Days (or hours if --hours) until first task is due
    #[arg(short = 'o', long, requires = "interval")]
    offset: Option<u32>,

    /// Days (or hours if --hours) between each task
    #[arg(short = 'i', long, requires = "offset")]
    interval: Option<u32>,

    /// Treat --offset and --interval as hours, skipping 22:00-06:00 quiet window
    #[arg(long)]
    hours: bool,

    /// Comma-separated subsection counts per chapter (e.g., "2,3,1")
    #[arg(short = 's', long)]
    subsections: Option<String>,
}

/// A task name, either simple ("Video 1") or hierarchical ("Video 1.2")
#[derive(Debug, Clone)]
struct TaskName {
    chapter: usize,
    section: Option<usize>,
}

impl TaskName {
    fn format(&self, unit: &str) -> String {
        match self.section {
            Some(s) => format!("{} {}.{}", unit, self.chapter, s),
            None => format!("{} {}", unit, self.chapter),
        }
    }
}

/// Parse subsections string (e.g., "2,3,1") into chapter structure
/// Returns: Vec<(chapter_num, section_count)>
fn parse_subsections(s: &str) -> Result<Vec<(usize, usize)>> {
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();

    if parts.is_empty() {
        anyhow::bail!("Subsections string is empty");
    }

    let mut result = Vec::new();
    for (idx, part) in parts.iter().enumerate() {
        let count: usize = part.parse().context(format!(
            "Invalid subsection count '{}' at position {}",
            part,
            idx + 1
        ))?;

        if count == 0 {
            anyhow::bail!("Subsection count cannot be zero at position {}", idx + 1);
        }

        result.push((idx + 1, count));
    }

    Ok(result)
}

/// Generate task names based on arguments
fn generate_task_names(args: &CreateArgs) -> Result<Vec<TaskName>> {
    let mut names = Vec::new();

    if let Some(ref subsections_str) = args.subsections {
        // Hierarchical: "Video 1.1", "Video 1.2", "Video 2.1", ...
        let subsections = parse_subsections(subsections_str)?;

        for (chapter, section_count) in subsections {
            for section in 1..=section_count {
                names.push(TaskName {
                    chapter,
                    section: Some(section),
                });
            }
        }
    } else {
        // Simple: "Video 1", "Video 2", ...
        for chapter in 1..=args.count {
            names.push(TaskName {
                chapter: chapter as usize,
                section: None,
            });
        }
    }

    Ok(names)
}

pub fn execute(args: CreateArgs) -> Result<()> {
    // Validate Taskwarrior is installed
    taskwarrior::check_taskwarrior().context("Taskwarrior must be installed to use taskgun")?;

    // Generate task names
    let task_names = generate_task_names(&args)?;
    let total_tasks = task_names.len();

    // Calculate schedule if offset/interval provided
    let due_dates = if let (Some(offset), Some(interval)) = (args.offset, args.interval) {
        let config = ScheduleConfig {
            base_time: Local::now(),
            offset,
            interval,
            use_hours: args.hours,
        };

        let scheduled = config.schedule_tasks(total_tasks)?;

        // Convert to Taskwarrior format
        let dates: Vec<String> = if args.hours {
            scheduled
                .iter()
                .map(|st| format_for_taskwarrior(&st.resolved, true))
                .collect()
        } else {
            scheduled
                .iter()
                .map(|st| format_relative_days(config.base_time, st.resolved))
                .collect()
        };

        Some(dates)
    } else {
        None
    };

    // Create tasks
    for (idx, task_name) in task_names.iter().enumerate() {
        let description = task_name.format(&args.unit);
        let due = due_dates.as_ref().map(|dates| dates[idx].as_str());

        taskwarrior::add_task(&description, &args.project, due)
            .context(format!("Failed to create task '{}'", description))?;
    }

    // Print summary
    println!(
        "✓ Created {} tasks under project: '{}'",
        total_tasks, args.project
    );

    if let (Some(offset), Some(interval)) = (args.offset, args.interval) {
        if args.hours {
            println!(
                "  Due dates: now+{}h for first, then every {}h (quiet window 2200-0600 shifts schedule forward)",
                offset, interval
            );
        } else {
            println!(
                "  Due dates: today+{}d for first, then every {}d per section",
                offset, interval
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subsections() {
        let result = parse_subsections("2,3,1").unwrap();
        assert_eq!(result, vec![(1, 2), (2, 3), (3, 1)]);

        let result = parse_subsections("5").unwrap();
        assert_eq!(result, vec![(1, 5)]);

        let result = parse_subsections("1,1,1,1").unwrap();
        assert_eq!(result, vec![(1, 1), (2, 1), (3, 1), (4, 1)]);
    }

    #[test]
    fn test_parse_subsections_invalid() {
        assert!(parse_subsections("").is_err());
        assert!(parse_subsections("0,1,2").is_err());
        assert!(parse_subsections("1,abc,2").is_err());
        assert!(parse_subsections("1,-1,2").is_err());
    }

    #[test]
    fn test_task_name_format() {
        let simple = TaskName {
            chapter: 5,
            section: None,
        };
        assert_eq!(simple.format("Video"), "Video 5");

        let hierarchical = TaskName {
            chapter: 2,
            section: Some(3),
        };
        assert_eq!(hierarchical.format("Video"), "Video 2.3");

        let custom_unit = TaskName {
            chapter: 1,
            section: Some(1),
        };
        assert_eq!(custom_unit.format("Lecture"), "Lecture 1.1");
    }

    #[test]
    fn test_generate_task_names_simple() {
        let args = CreateArgs {
            project: "Test".to_string(),
            count: 3,
            unit: "Video".to_string(),
            offset: None,
            interval: None,
            hours: false,
            subsections: None,
        };

        let names = generate_task_names(&args).unwrap();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0].format("Video"), "Video 1");
        assert_eq!(names[1].format("Video"), "Video 2");
        assert_eq!(names[2].format("Video"), "Video 3");
    }

    #[test]
    fn test_generate_task_names_hierarchical() {
        let args = CreateArgs {
            project: "Test".to_string(),
            count: 10, // Should be ignored when subsections present
            unit: "Video".to_string(),
            offset: None,
            interval: None,
            hours: false,
            subsections: Some("2,3,1".to_string()),
        };

        let names = generate_task_names(&args).unwrap();
        assert_eq!(names.len(), 6); // 2+3+1
        assert_eq!(names[0].format("Video"), "Video 1.1");
        assert_eq!(names[1].format("Video"), "Video 1.2");
        assert_eq!(names[2].format("Video"), "Video 2.1");
        assert_eq!(names[3].format("Video"), "Video 2.2");
        assert_eq!(names[4].format("Video"), "Video 2.3");
        assert_eq!(names[5].format("Video"), "Video 3.1");
    }
}
