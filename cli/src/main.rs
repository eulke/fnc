mod cli;
mod ui;
mod deploy;
mod progress;
mod package_version;
mod error;
mod sync_versions;
mod upgrade;
mod changelog;

use clap::Parser;
use cli::{Cli, Commands, FixType};
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
        Commands::Fix { fix_type } => {
            match fix_type {
                FixType::PackageVersions { dir, verbose } => {
                    package_version::execute(dir, verbose)
                }
                FixType::Changelog { verbose } => {
                    changelog::execute(verbose)
                }
            }
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