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
    about = "A gun to shoot tasks for Taskwarrior",
    long_about = "Extend Taskwarrior with bulk operations and smart scheduling.\n\n\
                  Examples:\n  \
                  taskgun create \"Deep Learning\" -p 5\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 5d --interval 7d\n  \
                  taskgun create \"Deep Learning\" -p 2,3,1 --offset 5d --interval 7d --skip weekend\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 2h --interval 6h --skip bedtime\n  \
                  taskgun create \"Deep Learning\" -p 5 --offset 1d --interval 1d --skip weekend --skip bedtime\n  \
                  taskgun create \"Deep Learning\" -p 30 --offset 2h --interval 2h --skip bedtime"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Bulk task generation with smart scheduling
    Create(commands::create::CreateArgs),

    /// Generate shell completions
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
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
    }

    Ok(())
}
