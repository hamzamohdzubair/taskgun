use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::process::Command;

#[derive(Args)]
pub struct DueArgs {
    /// Date pattern (d1, d2, 2d, 7d, w1, w2, 2w, m1, m2, 2m)
    pub pattern: String,

    /// Sort order (default: urgency descending)
    #[arg(short = 's', long, value_enum, default_value = "urg")]
    pub sort: SortOrder,
}

#[derive(Clone, ValueEnum)]
pub enum SortOrder {
    /// Sort by urgency (descending, default)
    #[value(name = "urg")]
    Urgency,

    /// Sort by task ID (ascending)
    #[value(name = "id")]
    Id,

    /// Sort by due date (ascending)
    #[value(name = "due")]
    Due,
}

/// Execute a date-based filter on tasks
pub fn execute(pattern: &str, sort: &SortOrder) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    // Parse pattern and build filter
    let filter = parse_pattern(pattern)?;

    let mut cmd = Command::new("task");

    // Enable colors for better readability
    cmd.arg("rc.color=on");
    cmd.arg("rc._forcecolor=on");

    // Use actual terminal width for flexible column layout
    if let Some((width, _)) = term_size::dimensions() {
        cmd.arg(format!("rc.defaultwidth={}", width));
    }

    // Add date filter
    cmd.arg("rc.verbose=nothing");
    cmd.arg(filter);

    // Apply sort order
    let sort_param = match sort {
        SortOrder::Urgency => "rc.report.list.sort=urgency-,id+",
        SortOrder::Id => "rc.report.list.sort=id+",
        SortOrder::Due => "rc.report.list.sort=due+,id+",
    };
    cmd.arg(sort_param);

    cmd.arg("list");

    let output = cmd.output()
        .context("Failed to execute task command")?;

    // Check stderr for messages first
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        // "No matches" is a normal result, not an error
        if stderr.contains("No matches") {
            println!("No matches found");
            return Ok(());
        } else {
            eprint!("{}", stderr);
        }
    }

    // Process stdout with line breaks for non-sequential IDs
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print_with_sequence_breaks(&stdout);
    } else if stderr.is_empty() && !output.status.success() {
        println!("No matches found");
    }

    Ok(())
}

/// Parse date pattern into taskwarrior filter
fn parse_pattern(pattern: &str) -> Result<String> {
    // Pattern examples:
    // d1/1d -> today
    // d2 -> tomorrow
    // d3 -> day after tomorrow
    // 2d -> next 2 days
    // 7d -> next 7 days
    // w1/1w -> this week
    // w2 -> next week only
    // 2w -> next 2 weeks
    // m1/1m -> this month
    // m2 -> next month only
    // 2m -> next 2 months

    let pattern = pattern.trim().to_lowercase();

    // Match letter-first (d1, w2, m3) or number-first (1d, 2w, 3m)
    // letter_first distinguishes d2 (tomorrow) from 2d (next 2 days)
    let (unit, num, letter_first) = if let Some(cap) = pattern.strip_prefix('d') {
        ('d', cap.parse::<u32>().context("Invalid day number")?, true)
    } else if let Some(cap) = pattern.strip_prefix('w') {
        ('w', cap.parse::<u32>().context("Invalid week number")?, true)
    } else if let Some(cap) = pattern.strip_prefix('m') {
        ('m', cap.parse::<u32>().context("Invalid month number")?, true)
    } else if let Some(cap) = pattern.strip_suffix('d') {
        ('d', cap.parse::<u32>().context("Invalid day number")?, false)
    } else if let Some(cap) = pattern.strip_suffix('w') {
        ('w', cap.parse::<u32>().context("Invalid week number")?, false)
    } else if let Some(cap) = pattern.strip_suffix('m') {
        ('m', cap.parse::<u32>().context("Invalid month number")?, false)
    } else {
        anyhow::bail!("Invalid pattern '{}'. Use d1, d2, 2d, 7d, w1, w2, 2w, m1, m2, 2m", pattern);
    };

    match unit {
        'd' => build_day_filter(num, letter_first),
        'w' => build_week_filter(num, letter_first),
        'm' => build_month_filter(num, letter_first),
        _ => unreachable!(),
    }
}

fn build_day_filter(num: u32, letter_first: bool) -> Result<String> {
    if num < 1 {
        anyhow::bail!("Day number must be >= 1");
    }

    let filter = if letter_first {
        // d1, d2, d3 -> specific days
        match num {
            1 => "due:today".to_string(),
            2 => "( due:tomorrow )".to_string(),
            3 => "( due:tomorrow+1day )".to_string(),
            n => format!("( due:today+{}days )", n - 1),
        }
    } else {
        // 1d, 2d, 7d -> next N days (range)
        match num {
            1 => "due:today".to_string(),
            n => format!("( due.before:today+{}days )", n),
        }
    };
    Ok(filter)
}

fn build_week_filter(num: u32, letter_first: bool) -> Result<String> {
    if num < 1 {
        anyhow::bail!("Week number must be >= 1");
    }

    let filter = if letter_first {
        // w1, w2 -> specific weeks
        match num {
            1 => "( due.after:sow-1s and due.before:eow+1s )".to_string(),
            2 => "( due.after:eow and due.before:eow+1w+1s )".to_string(),
            n => format!("( due.after:sow+{}w-1s and due.before:sow+{}w+7d+1s )", n - 1, n - 1),
        }
    } else {
        // 1w, 2w -> next N weeks (range)
        match num {
            1 => "( due.after:sow-1s and due.before:eow+1s )".to_string(),
            n => format!("( due.after:sow-1s and due.before:eow+{}w+1s )", n - 1),
        }
    };
    Ok(filter)
}

fn build_month_filter(num: u32, letter_first: bool) -> Result<String> {
    if num < 1 {
        anyhow::bail!("Month number must be >= 1");
    }

    let filter = if letter_first {
        // m1, m2 -> specific months
        match num {
            1 => "( due.after:som-1s and due.before:eom+1s )".to_string(),
            2 => "( due.after:eom and due.before:eom+1month+1s )".to_string(),
            n => format!("( due.after:som+{}month-1s and due.before:som+{}month+1month+1s )", n - 1, n - 1),
        }
    } else {
        // 1m, 2m -> next N months (range)
        match num {
            1 => "( due.after:som-1s and due.before:eom+1s )".to_string(),
            n => format!("( due.after:som-1s and due.before:eom+{}month+1s )", n - 1),
        }
    };
    Ok(filter)
}

/// Print task list with line breaks between non-sequential ID groups
fn print_with_sequence_breaks(output: &str) {
    let lines: Vec<&str> = output.lines().collect();

    if lines.is_empty() {
        return;
    }

    // Extract task IDs from each line and track line index
    let tasks: Vec<(usize, Option<u32>)> = lines.iter()
        .enumerate()
        .map(|(idx, line)| {
            let id = extract_task_id(line);
            (idx, id)
        })
        .collect();

    // Print lines with breaks where ID sequence is broken
    let mut prev_id: Option<u32> = None;

    for (line_idx, id_opt) in tasks {
        // Check if we should insert a line break
        if let (Some(prev), Some(current)) = (prev_id, id_opt) {
            // If IDs are not sequential (difference > 1), add line break
            if current > prev + 1 {
                println!(); // Insert blank line
            }
        }

        // Print the line
        println!("{}", lines[line_idx]);

        // Update previous ID for next iteration
        if id_opt.is_some() {
            prev_id = id_opt;
        }
    }
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
    fn test_parse_pattern_d1() {
        let filter = parse_pattern("d1").unwrap();
        assert_eq!(filter, "due:today");
    }

    #[test]
    fn test_parse_pattern_1d() {
        let filter = parse_pattern("1d").unwrap();
        assert_eq!(filter, "due:today");
    }

    #[test]
    fn test_parse_pattern_d2() {
        let filter = parse_pattern("d2").unwrap();
        assert_eq!(filter, "( due:tomorrow )");
    }

    #[test]
    fn test_parse_pattern_d3() {
        let filter = parse_pattern("d3").unwrap();
        assert_eq!(filter, "( due:tomorrow+1day )");
    }

    #[test]
    fn test_parse_pattern_2d() {
        let filter = parse_pattern("2d").unwrap();
        assert_eq!(filter, "( due.before:today+2days )");
    }

    #[test]
    fn test_parse_pattern_7d() {
        let filter = parse_pattern("7d").unwrap();
        assert_eq!(filter, "( due.before:today+7days )");
    }

    #[test]
    fn test_parse_pattern_w1() {
        let filter = parse_pattern("w1").unwrap();
        assert_eq!(filter, "( due.after:sow-1s and due.before:eow+1s )");
    }

    #[test]
    fn test_parse_pattern_1w() {
        let filter = parse_pattern("1w").unwrap();
        assert_eq!(filter, "( due.after:sow-1s and due.before:eow+1s )");
    }

    #[test]
    fn test_parse_pattern_w2() {
        let filter = parse_pattern("w2").unwrap();
        assert_eq!(filter, "( due.after:eow and due.before:eow+1w+1s )");
    }

    #[test]
    fn test_parse_pattern_2w() {
        let filter = parse_pattern("2w").unwrap();
        assert_eq!(filter, "( due.after:sow-1s and due.before:eow+1w+1s )");
    }

    #[test]
    fn test_parse_pattern_m1() {
        let filter = parse_pattern("m1").unwrap();
        assert_eq!(filter, "( due.after:som-1s and due.before:eom+1s )");
    }

    #[test]
    fn test_parse_pattern_1m() {
        let filter = parse_pattern("1m").unwrap();
        assert_eq!(filter, "( due.after:som-1s and due.before:eom+1s )");
    }

    #[test]
    fn test_parse_pattern_m2() {
        let filter = parse_pattern("m2").unwrap();
        assert_eq!(filter, "( due.after:eom and due.before:eom+1month+1s )");
    }

    #[test]
    fn test_parse_pattern_2m() {
        let filter = parse_pattern("2m").unwrap();
        assert_eq!(filter, "( due.after:som-1s and due.before:eom+1month+1s )");
    }

    #[test]
    fn test_parse_pattern_invalid() {
        assert!(parse_pattern("x1").is_err());
        assert!(parse_pattern("1x").is_err());
        assert!(parse_pattern("abc").is_err());
    }

    #[test]
    fn test_parse_pattern_case_insensitive() {
        let filter = parse_pattern("D1").unwrap();
        assert_eq!(filter, "due:today");

        let filter = parse_pattern("W2").unwrap();
        assert_eq!(filter, "( due.after:eow and due.before:eow+1w+1s )");
    }

    #[test]
    fn test_build_day_filter() {
        // Letter first (specific days: d1, d2, d3)
        assert_eq!(build_day_filter(1, true).unwrap(), "due:today");
        assert_eq!(build_day_filter(2, true).unwrap(), "( due:tomorrow )");
        assert_eq!(build_day_filter(3, true).unwrap(), "( due:tomorrow+1day )");

        // Number first (ranges: 1d, 2d, 7d)
        assert_eq!(build_day_filter(1, false).unwrap(), "due:today");
        assert_eq!(build_day_filter(2, false).unwrap(), "( due.before:today+2days )");
        assert_eq!(build_day_filter(7, false).unwrap(), "( due.before:today+7days )");

        assert!(build_day_filter(0, true).is_err());
        assert!(build_day_filter(0, false).is_err());
    }

    #[test]
    fn test_build_week_filter() {
        // Letter first (specific weeks: w1, w2)
        assert_eq!(build_week_filter(1, true).unwrap(), "( due.after:sow-1s and due.before:eow+1s )");
        assert_eq!(build_week_filter(2, true).unwrap(), "( due.after:eow and due.before:eow+1w+1s )");

        // Number first (ranges: 1w, 2w, 3w)
        assert_eq!(build_week_filter(1, false).unwrap(), "( due.after:sow-1s and due.before:eow+1s )");
        assert_eq!(build_week_filter(2, false).unwrap(), "( due.after:sow-1s and due.before:eow+1w+1s )");
        assert_eq!(build_week_filter(3, false).unwrap(), "( due.after:sow-1s and due.before:eow+2w+1s )");

        assert!(build_week_filter(0, true).is_err());
        assert!(build_week_filter(0, false).is_err());
    }

    #[test]
    fn test_build_month_filter() {
        // Letter first (specific months: m1, m2)
        assert_eq!(build_month_filter(1, true).unwrap(), "( due.after:som-1s and due.before:eom+1s )");
        assert_eq!(build_month_filter(2, true).unwrap(), "( due.after:eom and due.before:eom+1month+1s )");

        // Number first (ranges: 1m, 2m, 3m)
        assert_eq!(build_month_filter(1, false).unwrap(), "( due.after:som-1s and due.before:eom+1s )");
        assert_eq!(build_month_filter(2, false).unwrap(), "( due.after:som-1s and due.before:eom+1month+1s )");
        assert_eq!(build_month_filter(3, false).unwrap(), "( due.after:som-1s and due.before:eom+2month+1s )");

        assert!(build_month_filter(0, true).is_err());
        assert!(build_month_filter(0, false).is_err());
    }
}
