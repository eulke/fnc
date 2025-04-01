mod cli;
mod ui;
mod deploy;
mod progress;
mod package_version;
mod error;
mod sync_versions;
mod upgrade;

use clap::Parser;
use cli::{Cli, Commands};
use colored::Colorize;
use std::process;

fn main() {
    let cli = Cli::parse();
    
    let result = match cli.command {
        Commands::Deploy {
            deploy_type,
            version_type,
            force,
            verbose,
            interactive,
        } => {
            deploy::execute(deploy_type, version_type, force, verbose, interactive)
        }
        Commands::FixPackageVersion {
            dir,
            verbose,
        } => {
            package_version::execute(dir, verbose)
        }
        Commands::SyncVersions {
            source,
            targets,
            discover,
            max_depth,
            verbose,
        } => {
            sync_versions::execute(source, targets, discover, max_depth, verbose)
        }
        Commands::Upgrade {
            force,
            verbose,
        } => {
            upgrade::execute(force, verbose)
        }
    };

    if let Err(err) = result {
        eprintln!("{} {}", "Error:".bold().red(), err.user_message());
        process::exit(1);
    }
}
