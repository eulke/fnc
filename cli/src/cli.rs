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
    Deploy {
        #[clap(value_enum)]
        deploy_type: DeployType,
        #[clap(value_enum, default_value_t=Version::Patch)]
        version: Version,
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
