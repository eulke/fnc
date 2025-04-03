use crate::ui;
use crate::error::{Result, CliError};
use crate::progress::ProgressTracker;
use git::repository::Repository;
use std::path::PathBuf;
use changelog::ChangelogConfig;

pub fn execute(verbose: bool) -> Result<()> {
    let mut progress = ProgressTracker::new("Changelog Fix")
        .with_steps(vec![
            "Opening git repository".to_string(),
            "Finding main branch".to_string(),
            "Reading CHANGELOG.md".to_string(),
            "Parsing changelog sections".to_string(),
            "Getting diff from main branch".to_string(),
            "Analyzing changelog entries".to_string(),
            "Reorganizing changelog entries".to_string(),
            "Writing updated changelog".to_string(),
        ]);
    
    progress.start_step();
    let repo = git::repository::RealGitRepository::open()
        .map_err(|e| CliError::Git(e).with_context("Failed to open git repository"))?;
    progress.complete_step();
    
    progress.start_step();
    let main_branch = repo.get_main_branch()
        .map_err(|e| CliError::Git(e).with_context("Failed to determine main branch"))?;
    
    let current_branch = repo.get_current_branch()
        .map_err(|e| CliError::Git(e).with_context("Failed to determine current branch"))?;
    
    if verbose {
        println!("Using '{}' as the main branch", main_branch);
        println!("Current branch: '{}'", current_branch);
    }
    progress.complete_step();
    
    progress.start_step();
    let changelog_path = PathBuf::from("CHANGELOG.md");
    
    if !changelog_path.exists() {
        return Err(CliError::Other("CHANGELOG.md not found".to_string())
            .with_context("Create a CHANGELOG.md file in the root of your project first"));
    }
    progress.complete_step();
    
    progress.start_step();
    progress.complete_step();
    
    progress.start_step();
    let diff = repo.get_diff_from_main()
        .map_err(|e| CliError::Git(e).with_context("Failed to get diff from main branch"))?;
    
    if verbose {
        println!("Got diff from main branch ({} bytes)", diff.len());
    }
    progress.complete_step();
    
    progress.start_step();
    progress.complete_step();
    
    progress.start_step();
    // Create CLI-specific configuration
    let config = ChangelogConfig {
        ignore_duplicates: true, // CLI always ignores duplicates for better UX
        verbose,
        ..ChangelogConfig::default()
    };
    
    // Create a Changelog instance with our configuration and fix it
    let mut changelog = changelog::Changelog::with_config(changelog_path, config, changelog::ChangelogFormat::default())
        .map_err(|e| CliError::Other(e.to_string()))?;
        
    let result = changelog.fix_with_diff(&diff)
        .map_err(|e| CliError::Other(e.to_string()))?;
    progress.complete_step();
    
    progress.complete();
    
    ui::success_message("Changelog has been fixed.");
    if result.0 {
        ui::info_message(&format!("Moved {} entries to the unreleased section", result.1));
    } else {
        ui::info_message("No changelog entries need to be moved to unreleased section");
    }
    
    Ok(())
}