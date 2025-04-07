use crate::cli::DeployType;
use crate::error::{CliError, Result};
use crate::progress::ProgressTracker;
use crate::ui;
use changelog::ChangelogFormat;
use dialoguer::{Select, theme::ColorfulTheme};
use git::{config::Config, repository::Repository};
use std::path::{Path, PathBuf};
use version::{SemverVersion, Version, VersionType};

pub fn validate_repository_status(repo: &impl Repository, _verbose: bool) -> Result<()> {
    repo.validate_status()
        .map_err(|e| CliError::Git(e).with_context("Failed to validate git repository status"))?
        .then_some(())
        .ok_or_else(|| {
            CliError::Other("Git repository is not clean".to_string())
                .with_context("Please commit or stash your changes before deploying")
        })?;

    ui::success_message("Repository is clean");
    Ok(())
}

pub fn fix_changelog_for_release(repo: &impl Repository, verbose: bool) -> Result<()> {
    let diff = repo
        .get_diff_from_main()
        .map_err(|e| CliError::Git(e).with_context("Failed to get diff from main branch"))?;

    let changelog_path = PathBuf::from("CHANGELOG.md");
    if !changelog_path.exists() {
        ui::warning_message("CHANGELOG.md not found, skipping changelog fix");
        return Ok(());
    }

    // Read the content from the file
    let content = std::fs::read_to_string(&changelog_path).map_err(|e| {
        CliError::Other(e.to_string()).with_context("Failed to read changelog file")
    })?;

    let config = changelog::ChangelogConfig {
        verbose,
        ignore_duplicates: true,
        ..changelog::ChangelogConfig::default()
    };

    let changelog =
        changelog::Changelog::new(content, config, changelog::ChangelogFormat::Standard).map_err(
            |e| CliError::Other(e.to_string()).with_context("Failed to parse changelog"),
        )?;

    let (new_content, changes_made, entries_moved) = changelog
        .fix_with_diff(&diff)
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to fix changelog"))?;

    if changes_made {
        // Write the changes back to the file
        std::fs::write(&changelog_path, new_content).map_err(|e| {
            CliError::Other(e.to_string()).with_context("Failed to write changelog")
        })?;

        ui::success_message(&format!(
            "Fixed changelog: moved {entries_moved} entries to unreleased section"
        ));
    } else {
        ui::info_message("No changelog entries needed to be moved");
    }

    Ok(())
}

pub fn get_target_branch(
    repo: &impl Repository,
    deploy_type: &DeployType,
    verbose: bool,
) -> Result<String> {
    match deploy_type {
        DeployType::Release => {
            ui::status_message("Determining default branch");
            let branch = repo
                .get_default_branch()
                .map_err(|e| CliError::Git(e).with_context("Failed to determine default branch"))?;
            ui::success_message(&format!("Default branch is '{branch}'"));
            Ok(branch)
        }
        DeployType::Hotfix => {
            let branch = repo
                .get_main_branch()
                .map_err(|e| CliError::Git(e).with_context("Failed to determine main branch"))?;
            if verbose {
                println!("Using '{branch}' branch for hotfix deployment");
            }
            Ok(branch)
        }
    }
}

pub fn calculate_new_version(
    version_type: &VersionType,
    verbose: bool,
) -> Result<(SemverVersion, SemverVersion)> {
    let current_path = Path::new(".");

    if verbose {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        println!("Current working directory: {}", current_dir.display());
    }

    ui::status_message(&format!("Calculating {version_type:?} version"));

    let current_version = Version::read_from_project(current_path).map_err(|e| {
        CliError::Version(e).with_context("Failed to read current version from project")
    })?;

    if verbose {
        println!("Current version: {current_version}");
    }

    let new_version = Version::increment(&current_version, version_type).map_err(|e| {
        CliError::Version(e).with_context(format!("Failed to increment {version_type:?} version"))
    })?;

    ui::success_message(&format!(
        "Version will be updated from {current_version} to {new_version}"
    ));

    Ok((current_version, new_version))
}

pub fn write_version_to_files(new_version: &SemverVersion, verbose: bool) -> Result<()> {
    let current_path = Path::new(".");
    ui::status_message("Writing new version to project files");

    if verbose {
        println!(
            "Writing version {new_version} to files in {current_path:?}",
            new_version = new_version,
            current_path = current_path.display()
        );
    }

    Version::write_to_project(current_path, new_version).map_err(|e| {
        CliError::Version(e).with_context(format!(
            "Failed to write new version {new_version} to project files"
        ))
    })?;

    ui::success_message(&format!("Version {new_version} written to project files"));
    Ok(())
}

pub fn create_deployment_branch(
    repo: &impl Repository,
    deploy_type: &DeployType,
    new_version: &SemverVersion,
) -> Result<String> {
    let branch_prefix = if matches!(deploy_type, DeployType::Release) {
        "release"
    } else {
        "hotfix"
    };

    let new_branch = format!("{branch_prefix}/{new_version}");

    ui::status_message(&format!("Creating new branch: {new_branch}"));
    repo.create_branch(&new_branch).map_err(|e| {
        CliError::Git(e).with_context(format!("Failed to create branch '{new_branch}'"))
    })?;
    ui::success_message(&format!("Created new branch: {new_branch}"));

    ui::status_message(&format!("Checking out to {new_branch}"));
    repo.checkout_branch(&new_branch).map_err(|e| {
        CliError::Git(e).with_context(format!("Failed to checkout branch '{new_branch}'"))
    })?;
    ui::success_message(&format!("Checked out {new_branch}"));

    Ok(new_branch)
}

pub fn update_changelog(new_version: &SemverVersion, verbose: bool) -> Result<()> {
    ui::status_message("Updating CHANGELOG.md");

    let author = git::config::RealGitConfig::read_config()
        .map_err(|e| CliError::Git(e).with_context("Failed to get user from git config"))?;

    let author = format!("{} ({})", author.name, author.email);
    if verbose {
        println!("Using author info: {author}");
    }

    let config = changelog::ChangelogConfig {
        verbose,
        ignore_duplicates: true,
        ..changelog::ChangelogConfig::default()
    };

    let version_str = new_version.to_string();
    let changelog_path = Path::new("CHANGELOG.md");

    // Read content or create empty file if it doesn't exist
    let content = if changelog_path.exists() {
        std::fs::read_to_string(changelog_path)
            .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to read changelog"))?
    } else {
        // Create an empty changelog if it doesn't exist
        "# Changelog\n\n".to_string()
    };

    let changelog = changelog::Changelog::new(content, config, ChangelogFormat::Standard)
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to parse changelog"))?;

    // Get the updated content
    let new_content = changelog
        .update_with_version(&version_str, &author)
        .map_err(|e| {
            CliError::Other(e.to_string()).with_context("Failed to update CHANGELOG.md")
        })?;

    // Write the updated content back to the file
    std::fs::write(changelog_path, new_content)
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to write changelog"))?;

    ui::success_message("Updated CHANGELOG.md with new version");
    Ok(())
}

pub fn display_deployment_success(
    deploy_type: &DeployType,
    new_version: &SemverVersion,
    new_branch: &str,
) {
    println!();
    ui::success_message(&format!(
        "Successfully deployed {deploy_type:?} version {new_version}",
    ));
    ui::info_message(&format!(
        "Branch {new_branch} has been created and checked out",
    ));

    ui::section_header("Next Steps");
    ui::step_message(
        1,
        &format!("Commit the version changes: git commit -am \"Bump version to {new_version}\"",),
    );
    ui::step_message(
        2,
        &format!("Push the branch to remote: git push -u origin {new_branch}"),
    );
}

pub fn execute(
    deploy_type: Option<DeployType>,
    version_type: VersionType,
    force: bool,
    verbose: bool,
    interactive: bool,
) -> Result<()> {
    if !interactive && deploy_type.is_none() {
        return Err(CliError::Other(
            "deploy_type is required when not using interactive mode (-i)".to_string(),
        )
        .with_context("Run with -i flag or specify a deployment type (release or hotfix)"));
    }

    let (deploy_type, version_type) = if interactive || deploy_type.is_none() {
        ui::section_header("Interactive Deployment Setup");

        let current_version = Version::read_from_project(Path::new(".")).map_err(|e| {
            CliError::Version(e).with_context("Failed to read current version from project")
        })?;

        let deploy_type = interactive_deploy_type_selection()?;
        println!();

        let version_type = interactive_version_type_selection(&current_version)?;
        println!();

        (deploy_type, version_type)
    } else if let Some(dt) = deploy_type {
        (dt, version_type)
    } else {
        // This branch shouldn't be reachable due to earlier validation
        return Err(CliError::Other("Missing deploy type".to_string()));
    };

    let mut steps = vec![
        "Opening git repository".to_string(),
        "Validating repository status".to_string(),
    ];

    if matches!(deploy_type, DeployType::Release) {
        steps.push("Fixing changelog entries".to_string());
    }

    steps.extend(
        [
            "Getting target branch",
            "Checking out target branch",
            "Pulling latest changes",
            "Calculating new version",
            "Creating deployment branch",
            "Writing version to files",
            "Updating CHANGELOG.md",
        ]
        .iter()
        .map(ToString::to_string),
    );

    let mut progress =
        ProgressTracker::new(&format!("{deploy_type:?} Deployment")).with_steps(steps);

    ui::info_message(&format!("Starting with {version_type:?} version update"));

    progress.start_step();
    let repo = git::repository::RealGitRepository::open()
        .map_err(|e| CliError::Git(e).with_context("Failed to open git repository"))?;
    progress.complete_step();

    progress.start_step();
    if force {
        progress.skip_step("Force flag enabled");
        ui::warning_message("Force flag enabled. Skipping clean repository check");
    } else {
        validate_repository_status(&repo, verbose)?;
        progress.complete_step();
    }

    if matches!(deploy_type, DeployType::Release) {
        progress.start_step();
        fix_changelog_for_release(&repo, verbose)?;
        progress.complete_step();
    }

    progress.start_step();
    let target_branch = get_target_branch(&repo, &deploy_type, verbose)?;
    progress.complete_step();

    progress.start_step();
    repo.checkout_branch(&target_branch).map_err(|e| {
        CliError::Git(e).with_context(format!("Failed to checkout branch '{target_branch}'"))
    })?;
    progress.complete_step();

    progress.start_step();
    repo.pull()
        .map_err(|e| CliError::Git(e).with_context("Failed to pull latest changes from remote"))?;
    progress.complete_step();

    progress.start_step();
    let (_, new_version) = calculate_new_version(&version_type, verbose)?;
    progress.complete_step();

    progress.start_step();
    let new_branch = create_deployment_branch(&repo, &deploy_type, &new_version)?;
    progress.complete_step();

    progress.start_step();
    write_version_to_files(&new_version, verbose)?;
    progress.complete_step();

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
    let deploy_types = [DeployType::Release, DeployType::Hotfix];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select deployment type")
        .default(0)
        .items(
            &deploy_types
                .iter()
                .map(|dt| format!("{dt:?}"))
                .collect::<Vec<_>>(),
        )
        .interact()
        .map_err(|e| CliError::Other(format!("Failed to get deployment type selection: {e}")))?;

    Ok(deploy_types[selection].clone())
}

pub fn interactive_version_type_selection(current_version: &SemverVersion) -> Result<VersionType> {
    let version_types = [VersionType::Major, VersionType::Minor, VersionType::Patch];

    let new_versions = version_types
        .iter()
        .map(|vt| {
            Version::increment(current_version, vt).map_err(|e| {
                CliError::Version(e).with_context(format!("Failed to calculate {vt:?} version"))
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let items: Vec<_> = version_types
        .iter()
        .enumerate()
        .map(|(i, vt)| {
            format!(
                "{:?} (current: {} → new: {})",
                vt, current_version, new_versions[i]
            )
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select version increment type")
        .default(2) // Default to patch
        .items(&items)
        .interact()
        .map_err(|e| CliError::Other(format!("Failed to get version type selection: {e}")))?;

    Ok(version_types[selection].clone())
}
