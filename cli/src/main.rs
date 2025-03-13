mod cli;

use clap::Parser;
use cli::{Cli, Commands, DeployType, Version};

fn deploy(deploy_type: DeployType, version: Version) {
    println!("Deploying {:?} version {:?}", deploy_type, version);
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy {
            deploy_type,
            version,
        } => {
            deploy(deploy_type, version);
        }
    }
}
