use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "fnc")]
#[command(
    author,
    version,
    about = "Finance CLI tool that automate repetitive tasks"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Automate deploy flow creating branch, incrementing version and updating changelog
    Deploy {
        /// Run in interactive mode
        #[clap(short = 'i', long = "interactive")]
        interactive: bool,

        /// Type of deployment
        #[clap(value_enum, required_unless_present = "interactive")]
        deploy_type: Option<DeployType>,
        
        /// Version increment type
        #[clap(value_enum)]
        version: Option<Version>,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DeployType {
    Hotfix,
    Release,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Version {
    Patch,
    Minor,
    Major,
}
