mod cli;
use clap::Parser;
use cli::{Cli, Commands, DeployType};
use version::{Version, VersionType};

fn deploy(deploy_type: DeployType, version_type: VersionType) {
    let current_version = "0.1.0";
    
    match Version::increment(&current_version, version_type) {
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
