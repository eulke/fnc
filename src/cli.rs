use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "meli")]
#[command(
    author,
    version,
    about = "Meli CLI tool that automate repetitive tasks"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Automate deploy flow creating branch, incrementing version and updating changelog
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
