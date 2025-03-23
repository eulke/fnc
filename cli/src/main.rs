mod cli;
use clap::Parser;
use cli::{Cli, Commands, DeployType};
use version::{Version, VersionType};
use std::path::Path;

fn deploy(deploy_type: DeployType, version_type: VersionType) {
    match Version::update_in_project(&Path::new("./cli"), version_type) {
        Ok(new_version) => println!("Deploying {:?} version {}", deploy_type, new_version),
        Err(e) => eprintln!("Error incrementing version: {}", e),
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Deploy {
            deploy_type,
            version_type,
        } => {
            deploy(deploy_type, version_type);
        }
    }
}
