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
                  Examples:\n  \
                  taskgun create \"Deep Learning\" -p 5\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 5d --interval 7d\n  \
                  taskgun create \"Deep Learning\" -p 2,3,1 --offset 5d --interval 7d --skip weekend\n  \
                  taskgun learning          # Case-insensitive search\n  \
                  taskgun 'lec.*[0-9]+' -r  # Regex search (case-sensitive)"
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

    /// Generate shell completions
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// External subcommand (treated as keyword search)
    #[command(external_subcommand)]
    Search(Vec<String>),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create(args) => commands::create::execute(args)?,
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
        }
        Commands::Search(args) => {
            // External subcommand - treat as keyword search
            if args.is_empty() {
                anyhow::bail!("Search keyword required");
            }

            // Check for -r or --regex flag
            let use_regex = args.iter().any(|a| a == "-r" || a == "--regex");

            // Find the keyword (first non-flag argument)
            let keyword = args.iter()
                .find(|a| !a.starts_with('-'))
                .ok_or_else(|| anyhow::anyhow!("Search keyword required"))?;

            commands::search::execute(keyword, use_regex)?;
        }
    }

    Ok(())
}
