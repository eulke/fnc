use crate::ui;
use crate::error::{Result, CliError};
use crate::progress::ProgressTracker;
use git::repository::Repository;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

type ChangelogSections = HashMap<String, HashMap<String, Vec<String>>>;

struct ChangelogEntry {
    content: String,
    category: String,
}

fn create_moved_items_regex(entries_to_move: &[ChangelogEntry]) -> Result<Regex> {
    if entries_to_move.is_empty() {
        Regex::new(r"^$").map_err(|e| CliError::Other(e.to_string()).with_context("Failed to create empty regex pattern"))
    } else {
        let pattern = entries_to_move.iter()
            .map(|entry| regex::escape(&entry.content))
            .collect::<Vec<_>>()
            .join("|");
        Regex::new(&format!(r"- ({})", pattern))
            .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to create regex pattern"))
    }
}

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
    
    let content = fs::read_to_string(&changelog_path)
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to read CHANGELOG.md"))?;
    progress.complete_step();
    
    progress.start_step();
    let sections = parse_changelog(&content)?;
    
    let unreleased_section = sections.get("unreleased")
        .cloned()
        .unwrap_or_else(HashMap::new);
    
    let version_sections: ChangelogSections = sections.iter()
        .filter(|(k, _)| *k != "unreleased")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    
    if verbose {
        println!("Found {} version sections in changelog", version_sections.len());
        println!("Unreleased section has {} categories", unreleased_section.len());
    }
    progress.complete_step();
    
    progress.start_step();
    let diff = repo.get_diff_from_main()
        .map_err(|e| CliError::Git(e).with_context("Failed to get diff from main branch"))?;
    
    if verbose {
        println!("Got diff from main branch ({} bytes)", diff.len());
    }
    progress.complete_step();
    
    progress.start_step();
    let entries_to_move = identify_entries_in_diff(&diff, &version_sections, verbose)?;
    
    if entries_to_move.is_empty() {
        ui::info_message("No changelog entries need to be moved to unreleased section");
        progress.skip_step("No changes found");
    } else {
        if verbose {
            println!("Found {} changelog entries to move to unreleased section", entries_to_move.len());
        }
        progress.complete_step();
    }
    
    progress.start_step();
    if entries_to_move.is_empty() {
        progress.skip_step("No reorganization needed");
    } else {
        let new_content = reorganize_changelog(&content, &unreleased_section, &entries_to_move)?;
        progress.complete_step();
        
        progress.start_step();
        fs::write(&changelog_path, new_content)
            .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to write updated CHANGELOG.md"))?;
        progress.complete_step();
    }
    
    progress.complete();
    
    ui::success_message("Changelog has been fixed.");
    if !entries_to_move.is_empty() {
        ui::info_message(&format!("Moved {} entries to the unreleased section", entries_to_move.len()));
    }
    
    Ok(())
}

fn parse_changelog(content: &str) -> Result<ChangelogSections> {
    let mut sections = HashMap::new();
    let mut current_version: Option<String> = None;
    let mut current_category: Option<String> = None;
    
    // Make version regex case-insensitive and flexible with spacing
    let version_pattern = Regex::new(r"(?i)##\s*\[\s*((?:un|un-)?released|\d+\.\d+\.\d+)\s*\]")
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to compile version regex"))?;
    
    let category_pattern = Regex::new(r"### (.+)")
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to compile category regex"))?;
    
    let item_pattern = Regex::new(r"- (.+)")
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to compile item regex"))?;
    
    for line in content.lines() {
        let line = line.trim();
        
        if let Some(captures) = version_pattern.captures(line) {
            if let Some(version_match) = captures.get(1) {
                let version = version_match.as_str().to_lowercase();
                current_version = Some(version.clone());
                current_category = None;
                sections.entry(version).or_insert_with(HashMap::new);
            }
        } else if let Some(captures) = category_pattern.captures(line) {
            if let (Some(version), Some(category_match)) = (&current_version, captures.get(1)) {
                let category = category_match.as_str().to_string();
                current_category = Some(category.clone());
                if let Some(version_map) = sections.get_mut(version) {
                    version_map.entry(category).or_insert_with(Vec::new);
                }
            }
        } else if let Some(captures) = item_pattern.captures(line) {
            if let (Some(version), Some(category), Some(item_match)) = (&current_version, &current_category, captures.get(1)) {
                let item = item_match.as_str().to_string();
                if let Some(categories) = sections.get_mut(version) {
                    if let Some(items) = categories.get_mut(category) {
                        items.push(item);
                    }
                }
            }
        }
    }
    
    Ok(sections)
}

fn identify_entries_in_diff(
    diff: &str,
    version_sections: &ChangelogSections,
    verbose: bool,
) -> Result<Vec<ChangelogEntry>> {
    let mut entries_to_move = Vec::new();
    
    for (version, categories) in version_sections {
        if version.to_lowercase() == "unreleased" {
            continue;
        }
        
        for (category, items) in categories {
            for item in items {
                if item.to_lowercase().contains("initial release") {
                    continue;
                }
                
                let escaped_item = regex::escape(item);
                let item_pattern = Regex::new(&format!(r"(?m)^\+.*{}.*$", escaped_item))
                    .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to create regex pattern"))?;
                
                if item_pattern.is_match(diff) {
                    if verbose {
                        println!("Found '{}' in diff from main branch", item);
                    }
                    
                    entries_to_move.push(ChangelogEntry {
                        content: item.clone(),
                        category: category.clone(),
                    });
                }
            }
        }
    }
    
    Ok(entries_to_move)
}

fn reorganize_changelog(
    content: &str,
    unreleased_section: &HashMap<String, Vec<String>>,
    entries_to_move: &[ChangelogEntry],
) -> Result<String> {
    // Clone unreleased section and add entries to move
    let mut new_unreleased = unreleased_section.clone();
    for entry in entries_to_move {
        new_unreleased
            .entry(entry.category.to_owned())
            .or_insert_with(Vec::new)
            .push(entry.content.to_owned());
    }
    
    // Create fully formatted changelog content
    let mut new_content = String::new();
    let lines = content.lines().collect::<Vec<_>>();
    
    // Regex patterns for section identification
    let unreleased_pattern = Regex::new(r"(?i)## \[(un|un-)?released\]")
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to compile unreleased regex"))?;
    let version_pattern = Regex::new(r"## \[\d+\.\d+\.\d+\]")
        .map_err(|e| CliError::Other(e.to_string()).with_context("Failed to compile version regex"))?;
    
    // Format the new unreleased section content
    let mut formatted_unreleased = String::new();
    let actual_categories: Vec<_> = new_unreleased.keys().cloned().collect();

    for category in actual_categories { // Iterate over actual, sorted categories
        if let Some(items) = new_unreleased.get(&category) {
            if !items.is_empty() {
                formatted_unreleased.push_str(&format!("### {}\n", category));
                for item in items {
                    formatted_unreleased.push_str(&format!("- {}\n", item));
                }
                formatted_unreleased.push('\n');
            }
        }
    }

    // Find existing unreleased section or determine where to insert it
    if let Some(idx) = lines.iter().position(|&line| unreleased_pattern.is_match(line)) {
        // Copy content up to and including the unreleased header
        for i in 0..=idx {
            new_content.push_str(lines[i]);
            new_content.push('\n');
        }
        
        // Add new unreleased content
        new_content.push_str(&formatted_unreleased);
        
        // Find the next version section (or end of file)
        let next_version_idx = lines.iter()
            .skip(idx + 1)
            .position(|&line| version_pattern.is_match(line))
            .map(|pos| pos + idx + 1)
            .unwrap_or(lines.len());

        // Create regex for matching entries we're moving to skip them in released versions
        let moved_items_regex = create_moved_items_regex(entries_to_move)?;
        
        // Copy remaining content, skipping moved entries
        for i in next_version_idx..lines.len() {
            let line = lines[i];
            if !moved_items_regex.is_match(line) {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }
    } else {
        // No unreleased section exists - create one after title
        let title_idx = lines.iter().position(|&line| line.starts_with("# ")).unwrap_or(0);

        // Copy up to and including title
        for i in 0..=title_idx {
            new_content.push_str(lines[i]);
            new_content.push('\n');
        }
        
        // Add new unreleased section
        new_content.push_str("\n## [Unreleased]\n\n");
        new_content.push_str(&formatted_unreleased);
        
        // Create regex for matching entries we're moving to skip them in released versions
        let moved_items_regex = create_moved_items_regex(entries_to_move)?;
        
        // Copy remaining content, skipping moved entries
        for i in (title_idx + 1)..lines.len() {
            let line = lines[i];
            if !moved_items_regex.is_match(line) {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }
    }
    
    Ok(new_content)
}