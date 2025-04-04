use crate::error::{CliError, Result};
use crate::progress::ProgressTracker;
use crate::ui;
use changelog::{Changelog, ChangelogConfig, ChangelogFormat};
use git::repository::{RealGitRepository, Repository};
use std::convert::From;
use std::path::PathBuf;

struct FixResult {
    entries_moved: bool,
    entry_count: usize,
}

pub fn execute(verbose: bool) -> Result<()> {
    let mut progress = ProgressTracker::new("Changelog Fix").with_steps(vec![
        "Opening git repository".to_string(),
        "Finding main branch".to_string(),
        "Reading CHANGELOG.md".to_string(),
        "Getting diff from main branch".to_string(),
        "Fixing changelog entries".to_string(),
    ]);

    progress.start_step();
    let repo = RealGitRepository::open()
        .map_err(|e| CliError::Git(e).with_context("Failed to open git repository"))?;
    progress.complete_step();

    progress.start_step();
    let main_branch = repo
        .get_main_branch()
        .map_err(|e| CliError::Git(e).with_context("Failed to determine main branch"))?;

    let current_branch = repo
        .get_current_branch()
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
    let diff = repo
        .get_diff_from_main()
        .map_err(|e| CliError::Git(e).with_context("Failed to get diff from main branch"))?;

    if verbose {
        println!("Got diff from main branch ({} bytes)", diff.len());
    }
    progress.complete_step();

    progress.start_step();
    let config = ChangelogConfig {
        ignore_duplicates: false,
        verbose,
        ..ChangelogConfig::default()
    };

    let result = fix_changelog(&changelog_path, &diff, config)?;
    progress.complete_step();

    progress.complete();
    display_results(result);

    Ok(())
}

fn fix_changelog(path: &PathBuf, diff: &str, config: ChangelogConfig) -> Result<FixResult> {
    let mut changelog = Changelog::with_config(path, config, ChangelogFormat::default())
        .map_err(|e| CliError::Other(e.user_message()).with_context("Failed to load changelog"))?;

    let (entries_moved, entry_count) = changelog.fix_with_diff(diff).map_err(|e| {
        CliError::Other(e.user_message()).with_context("Failed to fix changelog entries")
    })?;

    Ok(FixResult {
        entries_moved,
        entry_count,
    })
}

fn display_results(result: FixResult) {
    ui::success_message("Changelog has been fixed.");

    if result.entries_moved {
        ui::info_message(&format!(
            "Moved {} entries to the unreleased section",
            result.entry_count
        ));
    } else {
        ui::info_message("No changelog entries need to be moved to unreleased section");
    }
}
