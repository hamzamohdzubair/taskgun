use anyhow::{Context, Result};
use chrono::Local;
use clap::Args;

use crate::scheduling::{format_for_taskwarrior, format_relative_days, ScheduleConfig};
use crate::skip::{SkipPresets, SkipRule};
use crate::taskwarrior;

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Project name
    #[arg(required = true)]
    project: String,

    /// Number of tasks or subsection structure (e.g., "10" or "2,3,1" for hierarchical)
    #[arg(short = 'p', long, required = true)]
    parts: String,

    /// Task name prefix (default: "Video")
    #[arg(short = 'u', long, default_value = "Video")]
    unit: String,

    /// Time until first task is due (e.g., "5d" for 5 days, "2h" for 2 hours)
    #[arg(short = 'o', long, requires = "interval")]
    offset: Option<String>,

    /// Time between each task (e.g., "7d" for 7 days, "6h" for 6 hours)
    #[arg(short = 'i', long, requires = "offset")]
    interval: Option<String>,

    /// Skip windows - can be used multiple times. Use presets (bedtime, weekend), time ranges (2100-0600), or day names (fri,sat,sun)
    #[arg(long = "skip")]
    skip: Vec<String>,
}

/// Duration with a value and unit (days or hours)
#[derive(Debug, Clone, Copy)]
struct Duration {
    value: u32,
    unit: DurationUnit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DurationUnit {
    Days,
    Hours,
}

impl Duration {
    /// Parse a duration string like "5d" or "2h"
    fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            anyhow::bail!("Duration string is empty");
        }

        // Split into value and unit
        let (value_str, unit_str) = if s.ends_with('d') || s.ends_with('D') {
            (&s[..s.len() - 1], "d")
        } else if s.ends_with('h') || s.ends_with('H') {
            (&s[..s.len() - 1], "h")
        } else {
            anyhow::bail!(
                "Duration must end with 'd' (days) or 'h' (hours). Got: '{}'",
                s
            );
        };

        let value: u32 = value_str
            .trim()
            .parse()
            .context(format!("Invalid duration value: '{}'", value_str))?;

        let unit = match unit_str {
            "d" => DurationUnit::Days,
            "h" => DurationUnit::Hours,
            _ => unreachable!(),
        };

        Ok(Duration { value, unit })
    }
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
    let parts_str = args.parts.trim();

    // Determine if parts is a simple number or subsections (contains comma)
    if parts_str.contains(',') {
        // Hierarchical: "Video 1.1", "Video 1.2", "Video 2.1", ...
        let subsections = parse_subsections(parts_str)?;

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
        let count: usize = parts_str
            .parse()
            .context(format!("Invalid parts value '{}'. Must be a number (e.g., '10') or subsections (e.g., '2,3,1')", parts_str))?;

        if count == 0 {
            anyhow::bail!("Parts count cannot be zero");
        }

        for chapter in 1..=count {
            names.push(TaskName {
                chapter,
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
    let due_dates = if let (Some(offset_str), Some(interval_str)) =
        (args.offset.as_ref(), args.interval.as_ref())
    {
        // Parse durations
        let offset_duration = Duration::parse(offset_str)
            .context(format!("Invalid offset: '{}'", offset_str))?;
        let interval_duration = Duration::parse(interval_str)
            .context(format!("Invalid interval: '{}'", interval_str))?;

        // Both must have the same unit
        if offset_duration.unit != interval_duration.unit {
            anyhow::bail!(
                "Offset and interval must use the same unit. Got offset: '{}', interval: '{}'",
                offset_str,
                interval_str
            );
        }

        let use_hours = offset_duration.unit == DurationUnit::Hours;

        // Parse skip rules
        let mut presets = SkipPresets::with_defaults();
        if let Err(e) = presets.load_from_taskrc() {
            eprintln!("Warning: Could not load .taskrc: {}", e);
        }

        let mut skip_rules: Vec<SkipRule> = Vec::new();
        for skip_arg in &args.skip {
            let rules = presets
                .parse_skip_arg(skip_arg)
                .context(format!("Invalid skip argument: '{}'", skip_arg))?;
            skip_rules.extend(rules);
        }

        let config = ScheduleConfig {
            base_time: Local::now(),
            offset: offset_duration.value,
            interval: interval_duration.value,
            use_hours,
            skip_rules,
        };

        let scheduled = config.schedule_tasks(total_tasks)?;

        // Convert to Taskwarrior format
        let dates: Vec<String> = if use_hours {
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

    if let (Some(offset_str), Some(interval_str)) =
        (args.offset.as_ref(), args.interval.as_ref())
    {
        println!(
            "  Due dates: now+{} for first, then every {}",
            offset_str, interval_str
        );

        if !args.skip.is_empty() {
            println!("  Skip rules: {}", args.skip.join(", "));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_parse() {
        let dur = Duration::parse("5d").unwrap();
        assert_eq!(dur.value, 5);
        assert_eq!(dur.unit, DurationUnit::Days);

        let dur = Duration::parse("2h").unwrap();
        assert_eq!(dur.value, 2);
        assert_eq!(dur.unit, DurationUnit::Hours);

        let dur = Duration::parse("10D").unwrap();
        assert_eq!(dur.value, 10);
        assert_eq!(dur.unit, DurationUnit::Days);

        let dur = Duration::parse("24H").unwrap();
        assert_eq!(dur.value, 24);
        assert_eq!(dur.unit, DurationUnit::Hours);
    }

    #[test]
    fn test_duration_parse_invalid() {
        assert!(Duration::parse("").is_err());
        assert!(Duration::parse("5").is_err());
        assert!(Duration::parse("d5").is_err());
        assert!(Duration::parse("5x").is_err());
        assert!(Duration::parse("abc").is_err());
        assert!(Duration::parse("-5d").is_err());
    }

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
            parts: "3".to_string(),
            unit: "Video".to_string(),
            offset: None,
            interval: None,
            skip: vec![],
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
            parts: "2,3,1".to_string(),
            unit: "Video".to_string(),
            offset: None,
            interval: None,
            skip: vec![],
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

    #[test]
    fn test_generate_task_names_invalid() {
        let args = CreateArgs {
            project: "Test".to_string(),
            parts: "0".to_string(),
            unit: "Video".to_string(),
            offset: None,
            interval: None,
            skip: vec![],
        };
        assert!(generate_task_names(&args).is_err());

        let args = CreateArgs {
            project: "Test".to_string(),
            parts: "abc".to_string(),
            unit: "Video".to_string(),
            offset: None,
            interval: None,
            skip: vec![],
        };
        assert!(generate_task_names(&args).is_err());
    }
}
