use clap::{Parser, Subcommand, ValueEnum};
use version::VersionType;

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
    /// Deploy a new version by creating a release or hotfix branch
    Deploy {
        /// Type of deployment to perform
        #[clap(value_enum)]
        deploy_type: DeployType,
        
        /// Type of version increment to make (major, minor, patch)
        #[clap(value_enum, default_value_t=VersionType::Patch)]
        version_type: VersionType,
        
        /// Force deployment even if the repository is not clean (development only)
        #[clap(long, default_value_t=false)]
        force: bool,
        
        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t=false)]
        verbose: bool,
    },
    
    /// Fix package versions in a JavaScript monorepo
    FixPackageVersion {
        /// Directory to start searching from (defaults to current directory)
        #[clap(short, long)]
        dir: Option<String>,
        
        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t=false)]
        verbose: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DeployType {
    /// Create a hotfix branch from main/master
    Hotfix,
    
    /// Create a release branch from the default branch
    Release,
}

