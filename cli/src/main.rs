mod cli;
mod ui;
mod deploy;
mod progress;

use clap::Parser;
use cli::{Cli, Commands};
use anyhow::Result;
fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Deploy {
            deploy_type,
            version_type,
            force,
            verbose,
        } => {
            deploy::execute(deploy_type, version_type, force, verbose)?;
            Ok(())
        }
    }
}
