mod cli;
mod error;
mod loader;
mod models;

use cli::{Cli, Commands};
use clap::Parser;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { output } => {
            println!("generate not yet implemented (output: {output})");
        }
        Commands::Sync => {
            println!("sync not yet implemented");
        }
    }
    Ok(())
}
