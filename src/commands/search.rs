use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::process::Command;

#[derive(Args)]
pub struct SearchArgs {
    /// Search keyword or pattern
    pub keyword: String,

    /// Use regex mode (case-sensitive)
    #[arg(short, long)]
    pub regex: bool,

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

/// Execute a keyword search across tasks
pub fn execute(keyword: &str, use_regex: bool, sort: &SortOrder) -> Result<()> {
    // Validate Taskwarrior is installed
    crate::taskwarrior::check_taskwarrior()
        .context("Taskwarrior must be installed to use taskgun")?;

    let mut cmd = Command::new("task");

    // Enable colors for better readability (alternating row backgrounds)
    cmd.arg("rc.color=on");
    cmd.arg("rc._forcecolor=on");
    cmd.arg("rc.verbose=label"); // Ensure column headings are shown

    // Use actual terminal width for flexible column layout
    if let Some((width, _)) = term_size::dimensions() {
        cmd.arg(format!("rc.defaultwidth={}", width));
    }

    // Disable height limit to show all tasks
    cmd.arg("rc.defaultheight=0");

    if use_regex {
        // Regex mode: case-sensitive regex search in project and description
        let filter = format!("( project~{} or description~{} )", keyword, keyword);
        cmd.arg("rc.regex=on");
        cmd.arg(filter);
    } else {
        // Default mode: case-insensitive search using .contains in project and description
        let filter = format!(
            "( project.contains:{} or description.contains:{} )",
            keyword, keyword
        );
        cmd.arg("rc.search.case.sensitive=no");
        cmd.arg(filter);
    }

    // Apply sort order
    let sort_param = match sort {
        SortOrder::Urgency => "rc.report.next.sort=urgency-,id+",
        SortOrder::Id => "rc.report.next.sort=id+",
        SortOrder::Due => "rc.report.next.sort=due+,id+",
    };
    cmd.arg(sort_param);

    cmd.arg("next");

    let output = cmd.output()
        .context("Failed to execute task command")?;

    // Check stderr for messages first (filter out configuration override warnings)
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        // "No matches" is a normal result, not an error - print it normally
        if stderr.contains("No matches") {
            println!("No matches found");
            return Ok(());
        } else {
            // Filter out configuration override warnings
            let filtered_stderr: String = stderr
                .lines()
                .filter(|line| !line.starts_with("Configuration override"))
                .collect::<Vec<_>>()
                .join("\n");

            // Only print if there are actual errors after filtering
            if !filtered_stderr.trim().is_empty() {
                eprint!("{}", filtered_stderr);
            }
        }
    }

    // Process stdout - only add sequence breaks when sorting by ID
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        // Print blank line before table
        println!();

        let task_count = if matches!(sort, SortOrder::Id) {
            print_with_sequence_breaks(&stdout)
        } else {
            print!("{}", stdout);
            count_tasks(&stdout)
        };

        // Print blank line after table and summary
        println!();
        print_summary(task_count, keyword, use_regex);
    } else if stderr.is_empty() && !output.status.success() {
        // If both stdout and stderr are empty but command failed, assume no matches
        println!("No matches found");
    }

    Ok(())
}

/// Print task list with line breaks between non-sequential ID groups
/// Returns the count of tasks found
fn print_with_sequence_breaks(output: &str) -> usize {
    let lines: Vec<&str> = output.lines().collect();

    if lines.is_empty() {
        return 0;
    }

    // Extract task IDs from each line and track line index
    let tasks: Vec<(usize, Option<u32>)> = lines.iter()
        .enumerate()
        .map(|(idx, line)| {
            let id = extract_task_id(line);
            (idx, id)
        })
        .collect();

    // Count tasks and print lines with breaks where ID sequence is broken
    let mut task_count = 0;
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

        // Update task count and previous ID for next iteration
        if id_opt.is_some() {
            task_count += 1;
            prev_id = id_opt;
        }
    }

    task_count
}

/// Count the number of tasks in taskwarrior output
fn count_tasks(output: &str) -> usize {
    output.lines()
        .filter(|line| extract_task_id(line).is_some())
        .count()
}

/// Print summary statistics
fn print_summary(task_count: usize, keyword: &str, _use_regex: bool) {
    println!("searching \"{}\": /{}/: {}", keyword, keyword, task_count);
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
/// Task list lines typically start with the ID in the first column
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
        // Test stripping ANSI color codes
        let line = "\x1b[48;5;234m  4\x1b[0m test";
        assert_eq!(strip_ansi(line), "  4 test");

        // Test line with no ANSI codes
        let plain = "5 Deep Learning";
        assert_eq!(strip_ansi(plain), "5 Deep Learning");

        // Test multiple ANSI codes
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
        // Test with colored background (common in taskwarrior output)
        let line = "\x1b[48;5;234m  4\x1b[0m\x1b[48;5;234m \x1b[0m\x1b[48;5;234m4d\x1b[0m test";
        assert_eq!(extract_task_id(line), Some(4));

        // Test with foreground color
        let line2 = "\x1b[38;5;1m 12\x1b[0m\x1b[38;5;1m \x1b[0m task";
        assert_eq!(extract_task_id(line2), Some(12));
    }

    #[test]
    fn test_extract_task_id_with_whitespace() {
        let line = "  42   Deep Learning   Video 2.1  ";
        assert_eq!(extract_task_id(line), Some(42));
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
    fn test_extract_task_id_whitespace_only() {
        let line = "   ";
        assert_eq!(extract_task_id(line), None);
    }

    #[test]
    fn test_extract_task_id_non_numeric() {
        let line = "ABC Project Description";
        assert_eq!(extract_task_id(line), None);
    }

    #[test]
    fn test_print_with_sequence_breaks_sequential_ids() {
        // Test output with sequential IDs (no breaks expected)
        let output = "ID Description\n5 Task 5\n6 Task 6\n7 Task 7\n";

        let count = print_with_sequence_breaks(output);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_print_with_sequence_breaks_non_sequential_ids() {
        // Test output with non-sequential IDs (breaks expected)
        let output = "ID Description\n5 Task 5\n6 Task 6\n7 Task 7\n9 Task 9\n10 Task 10\n";

        let count = print_with_sequence_breaks(output);
        assert_eq!(count, 5);
    }

    #[test]
    fn test_print_with_sequence_breaks_empty_output() {
        // Test with empty string
        let output = "";
        let count = print_with_sequence_breaks(output);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_print_with_sequence_breaks_single_task() {
        // Test with single task
        let output = "ID Description\n5 Task 5\n";
        let count = print_with_sequence_breaks(output);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_print_with_sequence_breaks_large_gap() {
        // Test with large gap in IDs
        let output = "ID Description\n5 Task 5\n100 Task 100\n";
        let count = print_with_sequence_breaks(output);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_tasks() {
        let output = "ID Description\n5 Task 5\n6 Task 6\n7 Task 7\n";
        assert_eq!(count_tasks(output), 3);

        let empty = "";
        assert_eq!(count_tasks(empty), 0);

        let with_header_only = "ID Description\n";
        assert_eq!(count_tasks(with_header_only), 0);
    }

    #[test]
    fn test_sort_order_enum_values() {
        // Verify enum values exist and are distinct
        let id_sort = SortOrder::Id;
        let due_sort = SortOrder::Due;

        // These should compile and be different variants
        assert!(matches!(id_sort, SortOrder::Id));
        assert!(matches!(due_sort, SortOrder::Due));
    }

    #[test]
    fn test_search_args_defaults() {
        // Test that SearchArgs can be constructed with expected defaults
        let args = SearchArgs {
            keyword: "test".to_string(),
            regex: false,
            sort: SortOrder::Id,
        };

        assert_eq!(args.keyword, "test");
        assert_eq!(args.regex, false);
        assert!(matches!(args.sort, SortOrder::Id));
    }

    #[test]
    fn test_search_args_with_regex() {
        // Test SearchArgs with regex enabled
        let args = SearchArgs {
            keyword: "test.*pattern".to_string(),
            regex: true,
            sort: SortOrder::Due,
        };

        assert_eq!(args.keyword, "test.*pattern");
        assert_eq!(args.regex, true);
        assert!(matches!(args.sort, SortOrder::Due));
    }
}
