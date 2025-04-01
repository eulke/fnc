use crate::ui;
use crate::progress::ProgressTracker;
use crate::error::{Result, CliError};
use dialoguer::{Select, theme::ColorfulTheme};
use git::{config::Config, repository::Repository};
use std::path::Path;
use version::{SemverVersion, Version, VersionType};
use crate::cli::DeployType;

pub fn validate_repository_status(repo: &impl Repository, _verbose: bool) -> Result<()> {
    let is_clean = repo.validate_status()
        .map_err(|e| CliError::Git(e).with_context("Failed to validate git repository status"))?;
    if !is_clean {
        return Err(CliError::Other("Git repository is not clean".to_string())
            .with_context("Please commit or stash your changes before deploying"));
    }
    ui::success_message("Repository is clean");
    Ok(())
}

pub fn get_target_branch(repo: &impl Repository, deploy_type: &DeployType, verbose: bool) -> Result<String> {
    match deploy_type {
        DeployType::Release => {
            ui::status_message("Determining default branch");
            let branch = repo.get_default_branch()
                .map_err(|e| CliError::Git(e).with_context("Failed to determine default branch"))?;
            ui::success_message(&format!("Default branch is '{}'", branch));
            Ok(branch)
        },
        DeployType::Hotfix => {
            let branch = String::from("main");
            if verbose {
                println!("Using 'main' branch for hotfix deployment");
            }
            Ok(branch)
        },
    }
}

pub fn calculate_new_version(version_type: &VersionType, verbose: bool) -> Result<(SemverVersion, SemverVersion)> {
    let current_path = Path::new(".");
    
    if verbose {
        println!("Current working directory: {}", std::env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf())
            .display());
    }
    
    ui::status_message(&format!("Calculating {:?} version", version_type));
    let current_version = Version::read_from_project(current_path)
        .map_err(|e| CliError::Version(e).with_context("Failed to read current version from project"))?;
    
    if verbose {
        println!("Current version: {}", current_version);
    }
    
    // Just increment the version without writing to files
    let new_version = Version::increment(&current_version, version_type)
        .map_err(|e| CliError::Version(e)
            .with_context(format!("Failed to increment {:?} version", version_type)))?;
    
    ui::success_message(&format!("Version will be updated from {} to {}", current_version, new_version));
    
    Ok((current_version, new_version))
}

pub fn write_version_to_files(new_version: &SemverVersion, verbose: bool) -> Result<()> {
    let current_path = Path::new(".");
    
    ui::status_message("Writing new version to project files");
    
    if verbose {
        println!("Writing version {} to files in {}", new_version, current_path.display());
    }
    
    Version::write_to_project(current_path, new_version)
        .map_err(|e| CliError::Version(e)
            .with_context(format!("Failed to write new version {} to project files", new_version)))?;
    
    ui::success_message(&format!("Version {} written to project files", new_version));
    
    Ok(())
}

pub fn create_deployment_branch(repo: &impl Repository, deploy_type: &DeployType, new_version: &SemverVersion) -> Result<String> {
    let branch_prefix = match deploy_type {
        DeployType::Release => "release",
        DeployType::Hotfix => "hotfix",
    };
    
    let new_branch = format!("{}/{}", branch_prefix, new_version);
    ui::status_message(&format!("Creating new branch: {}", new_branch));
    repo.create_branch(&new_branch)
        .map_err(|e| CliError::Git(e).with_context(format!("Failed to create branch '{}'", new_branch)))?;
    ui::success_message(&format!("Created new branch: {}", new_branch));
    
    ui::status_message(&format!("Checking out to {}", new_branch));
    repo.checkout_branch(&new_branch)
        .map_err(|e| CliError::Git(e).with_context(format!("Failed to checkout branch '{}'", new_branch)))?;
    ui::success_message(&format!("Checked out {}", new_branch));
    
    Ok(new_branch)
}

pub fn update_changelog(new_version: &SemverVersion, verbose: bool) -> Result<()> {
    ui::status_message("Updating CHANGELOG.md");
    
    let author = git::config::RealGitConfig::read_config()
        .map_err(|e| CliError::Git(e).with_context("Failed to get user from git config"))?;
    
    let author = format!("{} ({})", author.name, author.email);
    
    if verbose {
        println!("Using author info: {}", author);
    }
    
    let changelog_path = Path::new("CHANGELOG.md");
    
    changelog::ensure_changelog_exists(changelog_path, &new_version.to_string(), &author)
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to ensure CHANGELOG.md exists"))?;
    
    changelog::update_changelog(changelog_path, &new_version.to_string(), &author)
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to update CHANGELOG.md"))?;
    
    ui::success_message("Updated CHANGELOG.md with new version");
    
    Ok(())
}

pub fn display_deployment_success(deploy_type: &DeployType, new_version: &SemverVersion, new_branch: &str) {
    println!();
    ui::success_message(&format!("Successfully deployed {:?} version {}", deploy_type, new_version));
    ui::info_message(&format!("Branch {} has been created and checked out", new_branch));
    
    ui::section_header("Next Steps");
    ui::step_message(1, &format!("Commit the version changes: git commit -am \"Bump version to {}\"", new_version));
    ui::step_message(2, &format!("Push the branch to remote: git push -u origin {}", new_branch));
}

pub fn execute(deploy_type: Option<DeployType>, version_type: VersionType, force: bool, verbose: bool, interactive: bool) -> Result<()> {
    // Validate that deploy_type is provided when not in interactive mode
    if !interactive && deploy_type.is_none() {
        return Err(CliError::Other("deploy_type is required when not using interactive mode (-i)".to_string())
            .with_context("Run with -i flag or specify a deployment type (release or hotfix)"));
    }

    // First, if in interactive mode or deploy_type is None, determine deployment and version type
    let (deploy_type, version_type) = if interactive || deploy_type.is_none() {
        ui::section_header("Interactive Deployment Setup");

        // Get current version for showing in the interactive version selection
        let current_path = Path::new(".");
        let current_version = Version::read_from_project(current_path)
            .map_err(|e| CliError::Version(e).with_context("Failed to read current version from project"))?;
        
        // Interactive deployment type selection
        let deploy_type = interactive_deploy_type_selection()?;
        println!(); // Add some spacing
        
        // Interactive version type selection
        let version_type = interactive_version_type_selection(&current_version)?;
        println!(); // Add some spacing
        
        (deploy_type, version_type)
    } else {
        (deploy_type.unwrap(), version_type)
    };
    
    let mut progress = ProgressTracker::new(&format!("{:?} Deployment", deploy_type))
        .with_steps(vec![
            "Opening git repository".to_string(),
            "Validating repository status".to_string(),
            "Getting target branch".to_string(),
            "Checking out target branch".to_string(),
            "Pulling latest changes".to_string(),
            "Calculating new version".to_string(),
            "Creating deployment branch".to_string(),
            "Writing version to files".to_string(),
            "Updating CHANGELOG.md".to_string(),
        ]);
    
    ui::info_message(&format!("Starting with {:?} version update", version_type));
    
    // 1. Open git repository
    progress.start_step();
    let repo = git::repository::RealGitRepository::open()
        .map_err(|e| CliError::Git(e).with_context("Failed to open git repository"))?;
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
        .map_err(|e| CliError::Git(e).with_context(format!("Failed to checkout branch '{}'", target_branch)))?;
    progress.complete_step();
    
    // 5. Pull latest changes
    progress.start_step();
    repo.pull()
        .map_err(|e| CliError::Git(e).with_context("Failed to pull latest changes from remote"))?;
    progress.complete_step();
    
    // 6. Calculate new version (without writing to files)
    progress.start_step();
    let (_, new_version) = calculate_new_version(&version_type, verbose)?;
    progress.complete_step();
    
    // 7. Create deployment branch
    progress.start_step();
    let new_branch = create_deployment_branch(&repo, &deploy_type, &new_version)?;
    progress.complete_step();

    // 8. Write version to files
    progress.start_step();
    write_version_to_files(&new_version, verbose)?;
    progress.complete_step();

    // 9. Update CHANGELOG.md
    progress.start_step();
    update_changelog(&new_version, verbose)?;
    progress.complete_step();
    
    // Complete the progress tracking
    progress.complete();
    
    // Display next steps
    display_deployment_success(&deploy_type, &new_version, &new_branch);
    
    Ok(())
}

pub fn interactive_deploy_type_selection() -> Result<DeployType> {
    let items = vec!["Release", "Hotfix"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select deployment type")
        .default(0)
        .items(&items)
        .interact()
        .map_err(|e| CliError::Other(format!("Failed to get deployment type selection: {}", e)))?;
    
    match selection {
        0 => Ok(DeployType::Release),
        1 => Ok(DeployType::Hotfix),
        _ => Err(CliError::Other("Invalid deployment type selection".to_string()).into()),
    }
}

pub fn interactive_version_type_selection(current_version: &SemverVersion) -> Result<VersionType> {
    // Calculate what the new versions would be for each version type
    let major_version = Version::increment(current_version, &VersionType::Major)
        .map_err(|e| CliError::Version(e).with_context("Failed to calculate major version"))?;
    
    let minor_version = Version::increment(current_version, &VersionType::Minor)
        .map_err(|e| CliError::Version(e).with_context("Failed to calculate minor version"))?;
    
    let patch_version = Version::increment(current_version, &VersionType::Patch)
        .map_err(|e| CliError::Version(e).with_context("Failed to calculate patch version"))?;
    
    let items = vec![
        format!("Major (current: {} → new: {})", current_version, major_version),
        format!("Minor (current: {} → new: {})", current_version, minor_version),
        format!("Patch (current: {} → new: {})", current_version, patch_version),
    ];
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select version increment type")
        .default(2) // Default to patch
        .items(&items)
        .interact()
        .map_err(|e| CliError::Other(format!("Failed to get version type selection: {}", e)))?;
    
    match selection {
        0 => Ok(VersionType::Major),
        1 => Ok(VersionType::Minor),
        2 => Ok(VersionType::Patch),
        _ => Err(CliError::Other("Invalid version type selection".to_string()).into()),
    }
}