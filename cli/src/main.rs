mod cli;
use clap::Parser;
use cli::{Cli, Commands, DeployType};
use version::{Version, VersionType};
use git::repository::Repository;
use git::repository::RealGitRepository;
use std::path::Path;
use std::process;
use anyhow::{Context, Result};


fn deploy(deploy_type: DeployType, version_type: VersionType, force: bool) -> Result<()> {
    let repo = RealGitRepository::open()
        .context("Failed to open git repository")?;
    
    // 1. Validate status to check if the repository is clean
    if !force {
        let is_clean = repo.validate_status()
            .context("Failed to validate git repository status")?;
        if !is_clean {
            eprintln!("Error: Git repository is not clean. Please commit or stash your changes before deploying.");
            eprintln!("Hint: Use --force to bypass this check during development.");
            process::exit(1);
        }
    } else {
        println!("Warning: Force flag enabled. Skipping clean repository check.");
    }
    
    // 2. Checkout to appropriate branch based on deploy type
    let target_branch = match deploy_type {
        DeployType::Release => repo.get_default_branch()
            .context("Failed to determine default branch")?,
        DeployType::Hotfix => String::from("main"),  // For hotfixes, we use main/master
    };
    
    println!("Checking out {}", target_branch);
    repo.checkout_branch(&target_branch)
        .with_context(|| format!("Failed to checkout branch '{}'", target_branch))?;
    
    // 3. Pull latest changes
    println!("Pulling latest changes from remote...");
    repo.pull()
        .context("Failed to pull latest changes from remote")?;
    
    // 4. Read ecosystem version and increment it
    let current_path = Path::new(".");
    let new_version = Version::update_in_project(current_path, &version_type)
        .with_context(|| format!("Failed to update {:?} version in project", version_type))?;
    
    // 5. Create branch with pattern "release/x.x.x" or "hotfix/x.x.x"
    let branch_prefix = match deploy_type {
        DeployType::Release => "release",
        DeployType::Hotfix => "hotfix",
    };
    
    let new_branch = format!("{}/{}", branch_prefix, new_version);
    println!("Creating new branch: {}", new_branch);
    repo.create_branch(&new_branch)
        .with_context(|| format!("Failed to create branch '{}'", new_branch))?;
    
    // 6. Checkout to the new branch
    println!("Checking out to {}", new_branch);
    repo.checkout_branch(&new_branch)
        .with_context(|| format!("Failed to checkout branch '{}'", new_branch))?;
    
    // 7. The version was already updated in step 4
    println!("Successfully deployed {:?} version {}", deploy_type, new_version);
    println!("Branch {} has been created and checked out", new_branch);
    println!("You can now commit the version changes and push the branch to remote");
    
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Deploy {
            deploy_type,
            version_type,
            force,
        } => {
            deploy(deploy_type, version_type, force)
        }
    };
    
    if let Err(err) = result {
        eprintln!("Error: {:#}", err);  // {:#} displays the full error chain
        process::exit(1);
    }
}
