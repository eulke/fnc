mod cli;
use clap::Parser;
use cli::{Cli, Commands, DeployType};
use version::{Version, VersionType};
use git::repository::Repository;
use git::repository::RealGitRepository;
use std::path::Path;
use std::process;
use std::io::{self, Write};
use anyhow::{Context, Result};

/// Print a status message with a spinner-like indicator
fn status_message(message: &str) {
    println!("⏳ {} ... ", message);
    io::stdout().flush().unwrap();
}

/// Print a success message
fn success_message(message: &str) {
    println!("✅ {}", message);
}

/// Print a warning message
fn warning_message(message: &str) {
    println!("⚠️  {}", message);
}

/// Print an error message
fn error_message(message: &str) {
    eprintln!("❌ Error: {}", message);
}

fn deploy(deploy_type: DeployType, version_type: VersionType, force: bool, verbose: bool) -> Result<()> {
    println!("Starting {:?} deployment with {:?} version update", deploy_type, version_type);
    
    // 1. Open git repository
    status_message("Opening git repository");
    let repo = RealGitRepository::open()
        .context("Failed to open git repository")?;
    success_message("Git repository opened successfully");
    
    // 2. Validate status to check if the repository is clean
    if !force {
        status_message("Validating git repository status");
        let is_clean = repo.validate_status()
            .context("Failed to validate git repository status")?;
        if !is_clean {
            error_message("Git repository is not clean");
            println!("Please commit or stash your changes before deploying.");
            println!("Hint: Use --force to bypass this check during development.");
            process::exit(1);
        }
        success_message("Repository is clean");
    } else {
        warning_message("Force flag enabled. Skipping clean repository check");
    }
    
    // 3. Checkout to appropriate branch based on deploy type
    let target_branch = match deploy_type {
        DeployType::Release => {
            status_message("Determining default branch");
            let branch = repo.get_default_branch()
                .context("Failed to determine default branch")?;
            success_message(&format!("Default branch is '{}'", branch));
            branch
        },
        DeployType::Hotfix => {
            let branch = String::from("main");  // For hotfixes, we use main/master
            if verbose {
                println!("Using 'main' branch for hotfix deployment");
            }
            branch
        },
    };
    
    status_message(&format!("Checking out {}", target_branch));
    repo.checkout_branch(&target_branch)
        .with_context(|| format!("Failed to checkout branch '{}'", target_branch))?;
    success_message(&format!("Checked out {}", target_branch));
    
    // 4. Pull latest changes
    status_message("Pulling latest changes from remote");
    repo.pull()
        .context("Failed to pull latest changes from remote")?;
    success_message("Latest changes pulled from remote");
    
    // 5. Read ecosystem version and increment it
    let current_path = Path::new(".");
    
    if verbose {
        println!("Current working directory: {}", std::env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf())
            .display());
    }
    
    status_message(&format!("Updating {:?} version in project", version_type));
    let current_version = Version::read_from_project(current_path)
        .with_context(|| "Failed to read current version from project")?;
    
    if verbose {
        println!("Current version: {}", current_version);
    }
    
    let new_version = Version::update_in_project(current_path, &version_type)
        .with_context(|| format!("Failed to update {:?} version in project", version_type))?;
    success_message(&format!("Version updated from {} to {}", current_version, new_version));
    
    // 6. Create branch with pattern "release/x.x.x" or "hotfix/x.x.x"
    let branch_prefix = match deploy_type {
        DeployType::Release => "release",
        DeployType::Hotfix => "hotfix",
    };
    
    let new_branch = format!("{}/{}", branch_prefix, new_version);
    status_message(&format!("Creating new branch: {}", new_branch));
    repo.create_branch(&new_branch)
        .with_context(|| format!("Failed to create branch '{}'", new_branch))?;
    success_message(&format!("Created new branch: {}", new_branch));
    
    // 7. Checkout to the new branch
    status_message(&format!("Checking out to {}", new_branch));
    repo.checkout_branch(&new_branch)
        .with_context(|| format!("Failed to checkout branch '{}'", new_branch))?;
    success_message(&format!("Checked out {}", new_branch));
    
    // 8. Final success message
    println!("\n🎉 Successfully deployed {:?} version {}", deploy_type, new_version);
    println!("Branch {} has been created and checked out", new_branch);
    println!("\nNext steps:");
    println!("  1. Commit the version changes: git commit -am \"Bump version to {}\"", new_version);
    println!("  2. Push the branch to remote: git push -u origin {}", new_branch);
    
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    
    let result = match cli.command {
        Commands::Deploy {
            deploy_type,
            version_type,
            force,
            verbose,
        } => {
            deploy(deploy_type, version_type, force, verbose)
        }
    };
    
    if let Err(err) = result {
        error_message(&format!("{:#}", err));  // {:#} displays the full error chain
        process::exit(1);
    }
}
