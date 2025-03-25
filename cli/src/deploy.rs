use crate::ui;
use crate::progress::ProgressTracker;
use anyhow::{Context, Result};
use git::repository::Repository;
use std::path::Path;
use std::process;
use version::{Version, VersionType};
use crate::cli::DeployType;

/// Validate that the repository is clean
pub fn validate_repository_status(repo: &impl Repository, _verbose: bool) -> Result<()> {
    let is_clean = repo.validate_status()
        .context("Failed to validate git repository status")?;
    if !is_clean {
        ui::error_message("Git repository is not clean");
        println!("Please commit or stash your changes before deploying.");
        println!("Hint: Use --force to bypass this check during development.");
        process::exit(1);
    }
    ui::success_message("Repository is clean");
    Ok(())
}

/// Get the target branch based on deployment type
pub fn get_target_branch(repo: &impl Repository, deploy_type: &DeployType, verbose: bool) -> Result<String> {
    match deploy_type {
        DeployType::Release => {
            ui::status_message("Determining default branch");
            let branch = repo.get_default_branch()
                .context("Failed to determine default branch")?;
            ui::success_message(&format!("Default branch is '{}'", branch));
            Ok(branch)
        },
        DeployType::Hotfix => {
            let branch = String::from("main");  // For hotfixes, we use main/master
            if verbose {
                println!("Using 'main' branch for hotfix deployment");
            }
            Ok(branch)
        },
    }
}

/// Update version in the project
pub fn update_version(version_type: &VersionType, verbose: bool) -> Result<(String, String)> {
    let current_path = Path::new(".");
    
    if verbose {
        println!("Current working directory: {}", std::env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf())
            .display());
    }
    
    ui::status_message(&format!("Updating {:?} version in project", version_type));
    let current_version = Version::read_from_project(current_path)
        .with_context(|| "Failed to read current version from project")?;
    
    if verbose {
        println!("Current version: {}", current_version);
    }
    
    let new_version = Version::update_in_project(current_path, version_type)
        .with_context(|| format!("Failed to update {:?} version in project", version_type))?;
    ui::success_message(&format!("Version updated from {} to {}", current_version, new_version));
    
    Ok((current_version.to_string(), new_version.to_string()))
}

/// Create and checkout a new branch for the deployment
pub fn create_deployment_branch(repo: &impl Repository, deploy_type: &DeployType, new_version: &str) -> Result<String> {
    let branch_prefix = match deploy_type {
        DeployType::Release => "release",
        DeployType::Hotfix => "hotfix",
    };
    
    let new_branch = format!("{}/{}", branch_prefix, new_version);
    ui::status_message(&format!("Creating new branch: {}", new_branch));
    repo.create_branch(&new_branch)
        .with_context(|| format!("Failed to create branch '{}'", new_branch))?;
    ui::success_message(&format!("Created new branch: {}", new_branch));
    
    // Checkout to the new branch
    ui::status_message(&format!("Checking out to {}", new_branch));
    repo.checkout_branch(&new_branch)
        .with_context(|| format!("Failed to checkout branch '{}'", new_branch))?;
    ui::success_message(&format!("Checked out {}", new_branch));
    
    Ok(new_branch)
}

/// Display final success message and next steps
pub fn display_deployment_success(deploy_type: &DeployType, new_version: &str, new_branch: &str) {
    println!();
    ui::success_message(&format!("Successfully deployed {:?} version {}", deploy_type, new_version));
    ui::info_message(&format!("Branch {} has been created and checked out", new_branch));
    
    ui::section_header("Next Steps");
    ui::step_message(1, &format!("Commit the version changes: git commit -am \"Bump version to {}\"", new_version));
    ui::step_message(2, &format!("Push the branch to remote: git push -u origin {}", new_branch));
}

/// Execute the deployment process
pub fn execute(deploy_type: DeployType, version_type: VersionType, force: bool, verbose: bool) -> Result<()> {
    // Initialize progress tracker
    let mut progress = ProgressTracker::new(&format!("{:?} Deployment", deploy_type))
        .with_steps(vec![
            "Opening git repository".to_string(),
            "Validating repository status".to_string(),
            "Getting target branch".to_string(),
            "Checking out target branch".to_string(),
            "Pulling latest changes".to_string(),
            "Updating version".to_string(),
            "Creating deployment branch".to_string(),
        ]);
    
    ui::info_message(&format!("Starting with {:?} version update", version_type));
    
    // 1. Open git repository
    progress.start_step();
    let repo = git::repository::RealGitRepository::open()
        .context("Failed to open git repository")?;
    progress.complete_step();
    
    // 2. Validate repository status
    progress.start_step();
    if force {
        progress.skip_step("Force flag enabled");
        ui::warning_message("Force flag enabled. Skipping clean repository check");
    } else {
        validate_repository_status(&repo, verbose)?;
        progress.complete_step();
    }
    
    // 3. Get target branch
    progress.start_step();
    let target_branch = get_target_branch(&repo, &deploy_type, verbose)?;
    progress.complete_step();
    
    // 4. Checkout target branch
    progress.start_step();
    repo.checkout_branch(&target_branch)
        .with_context(|| format!("Failed to checkout branch '{}'", target_branch))?;
    progress.complete_step();
    
    // 5. Pull latest changes
    progress.start_step();
    repo.pull()
        .context("Failed to pull latest changes from remote")?;
    progress.complete_step();
    
    // 6. Update version
    progress.start_step();
    let (_, new_version) = update_version(&version_type, verbose)?;
    progress.complete_step();
    
    // 7. Create deployment branch
    progress.start_step();
    let new_branch = create_deployment_branch(&repo, &deploy_type, &new_version)?;
    progress.complete_step();
    
    // Complete the progress tracking
    progress.complete();
    
    // Display next steps
    display_deployment_success(&deploy_type, &new_version, &new_branch);
    
    Ok(())
}