use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io;

mod commands;
mod scheduling;
mod skip;
mod taskwarrior;

#[derive(Parser)]
#[command(
    name = "taskgun",
    version,
    about = "A rusty gun for our taskwarrior",
    long_about = "Extend Taskwarrior with bulk operations and smart scheduling.\n\n\
                  QUICK SEARCH:\n  \
                  taskgun <keyword>              # Quick search (projects + descriptions, case-insensitive)\n  \
                  taskgun <keyword> -r           # Quick regex search (case-sensitive)\n  \
                  taskgun <keyword> -s urg       # Sort by urgency (default)\n  \
                  taskgun <keyword> -s due       # Sort by due date\n\n\
                  QUICK DATE FILTERS:\n  \
                  taskgun d1                     # Tasks due today\n  \
                  taskgun 1d                     # Today + overdue (includes backlog)\n  \
                  taskgun tod                    # Alias for d1 (today)\n  \
                  taskgun tom                    # Alias for d2 (tomorrow)\n  \
                  taskgun week                   # Alias for w1 (this week)\n  \
                  taskgun 7d                     # Next 7 days + overdue\n\n\
                  Examples:\n  \
                  taskgun create \"Deep Learning\" -p 5\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 5d --interval 7d\n  \
                  taskgun create \"Deep Learning\" -p 2,3,1 --offset 5d --interval 7d --skip weekend\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 2h --interval 30m\n  \
                  taskgun learning               # Quick search\n  \
                  taskgun 'lec.*[0-9]+' -r       # Quick regex search",
    after_help = "QUICK SEARCH:\n  \
                  taskgun <keyword>              Quick search (projects + descriptions, case-insensitive)\n  \
                  taskgun <keyword> -r           Quick regex search (case-sensitive)\n  \
                  taskgun <keyword> -s urg       Sort by urgency (default)\n  \
                  taskgun <keyword> -s due       Sort by due date\n\n\
                  QUICK DATE FILTERS:\n  \
                  taskgun d1 / tod               Tasks due today\n  \
                  taskgun 1d                     Today + overdue (backlog)\n  \
                  taskgun tom                    Tomorrow\n  \
                  taskgun 7d                     Next 7 days + overdue\n  \
                  taskgun week                   This week\n\n\
                  Examples:\n  \
                  taskgun learning\n  \
                  taskgun 1d -s due              Today + overdue, sorted by due date\n  \
                  taskgun 'video [12]' -r",
    override_usage = "taskgun <COMMAND>\n       \
                      taskgun <keyword> [-r] [-s <SORT>]"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(allow_external_subcommands = true)]
enum Commands {
    /// Bulk task generation with smart scheduling
    Create(commands::create::CreateArgs),

    /// Search tasks by keyword (case-insensitive by default)
    Search(commands::search::SearchArgs),

    /// Filter tasks by due date (d1=today, d2=tomorrow, 2d=next 2 days, w1=this week, 2w=next 2 weeks, m1=this month, 2m=next 2 months)
    Due(commands::due::DueArgs),

    /// Generate shell completions
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// External subcommand (fallback for shorthand search syntax)
    #[command(external_subcommand)]
    External(Vec<String>),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create(args) => commands::create::execute(args)?,
        Commands::Search(args) => {
            commands::search::execute(&args.keyword, args.regex, &args.sort)?;
        }
        Commands::Due(args) => {
            commands::due::execute(&args.pattern, &args.sort)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
        }
        Commands::External(args) => {
            // External subcommand - treat as keyword search (shorthand syntax)
            if args.is_empty() {
                anyhow::bail!("Search keyword required");
            }

            // Find the keyword (first non-flag argument)
            let keyword = args.iter()
                .find(|a| !a.starts_with('-'))
                .ok_or_else(|| anyhow::anyhow!("Search keyword required"))?;

            // Check if this is a date shorthand keyword or date pattern
            let date_pattern = match keyword.as_str() {
                "tod" => Some("d1"),   // today
                "tom" => Some("d2"),   // tomorrow
                "week" => Some("w1"),  // this week
                kw if is_date_pattern(kw) => Some(kw),  // d1, 1d, 2d, w1, 1w, m1, 1m, etc.
                _ => None,
            };

            if let Some(pattern) = date_pattern {
                // Route to due command with the mapped pattern
                // Parse sort option (default to urgency if not specified)
                let sort = if args.iter().any(|a| a == "-s" || a == "--sort") {
                    // Find the value after -s or --sort
                    let sort_idx = args.iter()
                        .position(|a| a == "-s" || a == "--sort")
                        .unwrap();

                    if sort_idx + 1 < args.len() {
                        let sort_value = &args[sort_idx + 1];
                        match sort_value.as_str() {
                            "urg" => commands::due::SortOrder::Urgency,
                            "due" => commands::due::SortOrder::Due,
                            "id" => commands::due::SortOrder::Id,
                            _ => commands::due::SortOrder::Urgency,
                        }
                    } else {
                        commands::due::SortOrder::Urgency
                    }
                } else {
                    commands::due::SortOrder::Urgency
                };

                commands::due::execute(pattern, &sort)?;
            } else {
                // Regular search keyword
                // Check for -r or --regex flag
                let use_regex = args.iter().any(|a| a == "-r" || a == "--regex");

                // Parse sort option (default to urgency if not specified)
                let sort = if args.iter().any(|a| a == "-s" || a == "--sort") {
                    // Find the value after -s or --sort
                    let sort_idx = args.iter()
                        .position(|a| a == "-s" || a == "--sort")
                        .unwrap();

                    if sort_idx + 1 < args.len() {
                        let sort_value = &args[sort_idx + 1];
                        match sort_value.as_str() {
                            "urg" => commands::search::SortOrder::Urgency,
                            "due" => commands::search::SortOrder::Due,
                            "id" => commands::search::SortOrder::Id,
                            _ => commands::search::SortOrder::Urgency,
                        }
                    } else {
                        commands::search::SortOrder::Urgency
                    }
                } else {
                    commands::search::SortOrder::Urgency
                };

                commands::search::execute(keyword, use_regex, &sort)?;
            }
        }
    }

    Ok(())
}

/// Check if a string matches a valid date pattern (d1, 1d, 2d, w1, 1w, m1, 1m, etc.)
fn is_date_pattern(s: &str) -> bool {
    let s = s.trim().to_lowercase();

    // Check letter-first patterns: d1-99, w1-52, m1-12
    if let Some(cap) = s.strip_prefix('d') {
        return cap.parse::<u32>().is_ok();
    }
    if let Some(cap) = s.strip_prefix('w') {
        return cap.parse::<u32>().is_ok();
    }
    if let Some(cap) = s.strip_prefix('m') {
        return cap.parse::<u32>().is_ok();
    }

    // Check number-first patterns: 1d-99d, 1w-52w, 1m-12m
    if let Some(cap) = s.strip_suffix('d') {
        return cap.parse::<u32>().is_ok();
    }
    if let Some(cap) = s.strip_suffix('w') {
        return cap.parse::<u32>().is_ok();
    }
    if let Some(cap) = s.strip_suffix('m') {
        return cap.parse::<u32>().is_ok();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_date_pattern() {
        // Valid letter-first patterns
        assert!(is_date_pattern("d1"));
        assert!(is_date_pattern("d2"));
        assert!(is_date_pattern("d99"));
        assert!(is_date_pattern("w1"));
        assert!(is_date_pattern("w52"));
        assert!(is_date_pattern("m1"));
        assert!(is_date_pattern("m12"));

        // Valid number-first patterns
        assert!(is_date_pattern("1d"));
        assert!(is_date_pattern("2d"));
        assert!(is_date_pattern("7d"));
        assert!(is_date_pattern("99d"));
        assert!(is_date_pattern("1w"));
        assert!(is_date_pattern("2w"));
        assert!(is_date_pattern("52w"));
        assert!(is_date_pattern("1m"));
        assert!(is_date_pattern("2m"));
        assert!(is_date_pattern("12m"));

        // Case insensitive
        assert!(is_date_pattern("D1"));
        assert!(is_date_pattern("W2"));
        assert!(is_date_pattern("2D"));
        assert!(is_date_pattern("3W"));

        // Invalid patterns
        assert!(!is_date_pattern("tod"));
        assert!(!is_date_pattern("tom"));
        assert!(!is_date_pattern("week"));
        assert!(!is_date_pattern("learning"));
        assert!(!is_date_pattern("video"));
        assert!(!is_date_pattern("d"));
        assert!(!is_date_pattern("1"));
        assert!(!is_date_pattern("abc"));
        assert!(!is_date_pattern("d1d"));
        assert!(!is_date_pattern("1d1"));
    }
}
