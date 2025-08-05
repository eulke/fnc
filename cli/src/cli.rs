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
        deploy_type: Option<DeployType>,

        /// Type of version increment to make (major, minor, patch)
        #[clap(value_enum, default_value_t=VersionType::Patch)]
        version_type: VersionType,

        /// Force deployment even if the repository is not clean (development only)
        #[clap(long, default_value_t = false)]
        force: bool,

        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t = false)]
        verbose: bool,

        /// Use interactive mode with dialog prompts
        #[clap(short, long, default_value_t = false)]
        interactive: bool,
    },

    /// Fix various issues in your projects
    Fix {
        /// Type of fix to perform
        #[clap(subcommand)]
        fix_type: FixType,
    },

    /// Synchronize versions across multiple projects (including across ecosystems)
    SyncVersions {
        /// Primary project directory whose version will be used as the source
        #[clap(short, long)]
        source: String,

        /// Comma-separated list of target directories to update with the source version
        #[clap(short, long)]
        targets: Vec<String>,

        /// Enable auto-discovery of projects in subdirectories
        #[clap(short, long, default_value_t = false)]
        discover: bool,

        /// Max depth for auto-discovery (only used with --discover)
        #[clap(long, default_value_t = 3)]
        max_depth: usize,

        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t = false)]
        verbose: bool,
    },

    /// Upgrade FNC CLI to the latest version
    Upgrade {
        /// Force upgrade even if running from a development environment
        #[clap(long, default_value_t = false)]
        force: bool,

        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t = false)]
        verbose: bool,
    },

    /// Execute HTTP requests across multiple environments and compare responses
    HttpDiff {
        /// Comma-separated list of environments to test (defaults to all configured environments)
        #[clap(short, long)]
        environments: Option<String>,

        /// Comma-separated list of route names to test (defaults to all configured routes)
        #[clap(short, long)]
        routes: Option<String>,

        /// Include headers in response comparison (disabled by default)
        #[clap(short = 'H', long, default_value_t = false)]
        include_headers: bool,

        /// Include error analysis in output (disabled by default)
        #[clap(short = 'E', long, default_value_t = false)]
        include_errors: bool,

        /// Diff view style for response comparison
        #[clap(short = 'd', long, value_enum, default_value_t = DiffViewType::Unified)]
        diff_view: DiffViewType,

        /// Configuration file path (defaults to http-diff.toml)
        #[clap(short, long, default_value = "http-diff.toml")]
        config: String,

        /// CSV file with test user data (defaults to users.csv)
        #[clap(long, default_value = "users.csv")]
        users_file: String,

        /// Generate configuration files if they don't exist
        #[clap(long, default_value_t = false)]
        init: bool,

        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t = false)]
        verbose: bool,

        /// Save curl commands to file for debugging
        #[clap(long, default_value = "curl_commands.txt")]
        output_file: String,

        /// Generate executive summary report (format auto-detected from file extension)
        #[clap(long)]
        report: Option<String>,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum FixType {
    /// Fix package versions in a JavaScript monorepo
    #[clap(name = "package-versions")]
    PackageVersions {
        /// Directory to start searching from (defaults to current directory)
        #[clap(short, long)]
        dir: Option<String>,

        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t = false)]
        verbose: bool,
    },

    /// Fix changelog entries by moving changes from released versions to unreleased section
    #[clap(name = "changelog")]
    Changelog {
        /// Enable verbose output with additional information
        #[clap(short, long, default_value_t = false)]
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

#[derive(ValueEnum, Clone, Debug)]
pub enum DiffViewType {
    /// Traditional unified diff (up/down view) - default
    Unified,

    /// Side-by-side diff view for easier comparison
    SideBySide,
}
