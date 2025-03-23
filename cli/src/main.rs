mod cli;
use clap::Parser;
use cli::{Cli, Commands, DeployType};
use version::{Version, VersionType};
use git::repository::Repository;
use git::repository::RealGitRepository;
use std::path::Path;
use std::process;

fn deploy(deploy_type: DeployType, version_type: VersionType, force: bool) {
    let repo = RealGitRepository::open();
    
    // 1. Validate status to check if the repository is clean
    if !force {
        match repo.validate_status() {
            Ok(is_clean) => {
                if !is_clean {
                    eprintln!("Error: Git repository is not clean. Please commit or stash your changes before deploying.");
                    eprintln!("Hint: Use --force to bypass this check during development.");
                    process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Error validating git status: {}", e);
                process::exit(1);
            }
        }
    } else {
        println!("Warning: Force flag enabled. Skipping clean repository check.");
    }
    
    // 2. Checkout to appropriate branch based on deploy type
    let target_branch = match deploy_type {
        DeployType::Release => match repo.get_default_branch() {
            Ok(branch) => branch,
            Err(e) => {
                eprintln!("Error getting default branch: {}", e);
                process::exit(1);
            }
        },
        DeployType::Hotfix => String::from("main"),  // For hotfixes, we use main/master
    };
    
    println!("Checking out {}", target_branch);
    if let Err(e) = repo.checkout_branch(&target_branch) {
        eprintln!("Error checking out to {}: {}", target_branch, e);
        process::exit(1);
    }
    
    // 3. Pull latest changes
    println!("Pulling latest changes from remote...");
    if let Err(e) = repo.pull() {
        eprintln!("Error pulling changes: {}", e);
        process::exit(1);
    }
    
    // 4. Read ecosystem version and increment it
    let current_path = Path::new(".");
    let new_version = match Version::update_in_project(current_path, version_type) {
        Ok(version) => version,
        Err(e) => {
            eprintln!("Error incrementing version: {}", e);
            process::exit(1);
        }
    };
    
    // 5. Create branch with pattern "release/x.x.x" or "hotfix/x.x.x"
    let branch_prefix = match deploy_type {
        DeployType::Release => "release",
        DeployType::Hotfix => "hotfix",
    };
    
    let new_branch = format!("{}/{}", branch_prefix, new_version);
    println!("Creating new branch: {}", new_branch);
    if let Err(e) = repo.create_branch(&new_branch) {
        eprintln!("Error creating branch {}: {}", new_branch, e);
        process::exit(1);
    }
    
    // 6. Checkout to the new branch
    println!("Checking out to {}", new_branch);
    if let Err(e) = repo.checkout_branch(&new_branch) {
        eprintln!("Error checking out to {}: {}", new_branch, e);
        process::exit(1);
    }
    
    // 7. The version was already updated in step 4
    println!("Successfully deployed {:?} version {}", deploy_type, new_version);
    println!("Branch {} has been created and checked out", new_branch);
    println!("You can now commit the version changes and push the branch to remote");
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Deploy {
            deploy_type,
            version_type,
            force,
        } => {
            deploy(deploy_type, version_type, force);
        }
    }
}
