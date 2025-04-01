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
use error::Result;

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
    }
}
