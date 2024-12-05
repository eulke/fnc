mod cli;
mod error;
mod git;
mod interactive;
mod language;
mod progress;
mod semver;
mod ports;

use clap::Parser;
use cli::{Cli, Commands};
use error::DeployError;
use crate::error::Result;
use crate::language::Language;
use ports::{PackageOperations, VCSOperations};
use crate::cli::{DeployType, Version};
use crate::progress::DeployProgress;

fn handle_deploy<T: VCSOperations>(
    vcs: &T,
    language: &dyn PackageOperations,
    deploy_type: DeployType,
    version: Version,
) -> Result<()> {
    let progress = DeployProgress::new();

    // Validate VCS status
    progress.status_check();
    if !vcs.validate_status().map_err(|e| DeployError::VCSStatusError(e.to_string()))? {
        progress.finish(false);
        return Err(DeployError::VCSStatusError("There are uncommitted changes".to_string()));
    }

    // Handle branch checkout based on deploy type
    let target_branch = match deploy_type {
        DeployType::Release => {
            let branch = vcs.get_default_branch()
                .map_err(|e| DeployError::BranchError(format!("Failed to get default branch: {}", e)))?;
            progress.branch_checkout(&branch);
            branch
        }
        DeployType::Hotfix => {
            // Try master first, then main
            match vcs.checkout_branch("master") {
                Ok(_) => {
                    progress.branch_checkout("master");
                    String::from("master")
                }
                Err(_) => {
                    let result = vcs.checkout_branch("main")
                        .map(|_| String::from("main"))
                        .map_err(|e| DeployError::BranchError(format!("Failed to checkout master/main: {}", e)))?;
                    progress.branch_checkout("main");
                    result
                }
            }
        }
    };

    vcs.checkout_branch(&target_branch)
        .map_err(|e| DeployError::BranchError(format!("Failed to checkout {}: {}", target_branch, e)))?;

    progress.pulling();
    vcs.pull()
        .map_err(|e| DeployError::RemoteError(format!("Failed to pull from remote: {}", e)))?;

    let current_semver = language.current_pkg_version();
    let incremented_semver = semver::increment(&current_semver, &version);
    progress.version_increment(&current_semver, &incremented_semver);

    let branch_name = match deploy_type {
        DeployType::Release => format!("release/{}", &incremented_semver),
        DeployType::Hotfix => format!("hotfix/{}", &incremented_semver),
    };

    progress.branch_creation(&branch_name);
    vcs.create_branch(&branch_name)
        .map_err(|e| DeployError::BranchError(format!("Failed to create branch {}: {}", branch_name, e)))?;

    progress.branch_switch(&branch_name);
    vcs.checkout_branch(&branch_name)
        .map_err(|e| DeployError::BranchError(format!("Failed to checkout branch {}: {}", branch_name, e)))?;

    let author = vcs.read_config()
        .map_err(|e| DeployError::ConfigError(format!("Failed to get author info: {}", e)))?;

    progress.updating_version();
    language.increment_pkg_version(&incremented_semver, &author)
        .map_err(|e| DeployError::VersionError(format!("Failed to increment version: {}", e)))?;

    progress.finish(true);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy {
            deploy_type,
            version,
            interactive,
        } => {
            let (deploy_type, version) = if interactive {
                let options = interactive::DeployOptions::prompt();
                (options.deploy_type, options.version)
            } else {
                (deploy_type, version)
            };

            let language = Language::detect().expect("Unable to detect language");
            let git = git::Adapter::new();
            
            handle_deploy(&git, &*language, deploy_type, version)?;
            Ok(())
        }
    }
}
