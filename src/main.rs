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
                  Examples:\n  \
                  taskgun create \"Deep Learning\" -p 5\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 5d --interval 7d\n  \
                  taskgun create \"Deep Learning\" -p 2,3,1 --offset 5d --interval 7d --skip weekend\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 2h --interval 30m\n  \
                  taskgun learning               # Quick search\n  \
                  taskgun due d1                 # Tasks due today\n  \
                  taskgun due 7d                 # Tasks due in next 7 days\n  \
                  taskgun 'lec.*[0-9]+' -r       # Quick regex search",
    after_help = "QUICK SEARCH:\n  \
                  taskgun <keyword>              Quick search (projects + descriptions, case-insensitive)\n  \
                  taskgun <keyword> -r           Quick regex search (case-sensitive)\n  \
                  taskgun <keyword> -s urg       Sort by urgency (default)\n  \
                  taskgun <keyword> -s due       Sort by due date\n\n\
                  Examples:\n  \
                  taskgun learning\n  \
                  taskgun due d1                 Tasks due today\n  \
                  taskgun due 7d -s due          Tasks due in next 7 days, sorted by due date\n  \
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

            // Find the keyword (first non-flag argument)
            let keyword = args.iter()
                .find(|a| !a.starts_with('-'))
                .ok_or_else(|| anyhow::anyhow!("Search keyword required"))?;

            commands::search::execute(keyword, use_regex, &sort)?;
        }
    }

    Ok(())
}
