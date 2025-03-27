use anyhow::{Context, Result};
use chrono::Local;
use regex::Regex;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChangelogError {
    #[error("Failed to read changelog file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to parse changelog: {0}")]
    ParseError(String),
}

/// Updates the CHANGELOG.md file by replacing the unreleased section with a new version entry.
/// 
/// # Arguments
/// 
/// * `changelog_path` - Path to the CHANGELOG.md file
/// * `version` - The new version to add to the changelog
/// * `author` - The author's name and email
/// 
/// # Returns
/// 
/// Result indicating success or failure
pub fn update_changelog(changelog_path: &Path, version: &str, author: &str) -> Result<()> {
    // Read the changelog content
    let content = fs::read_to_string(changelog_path)
        .with_context(|| format!("Failed to read changelog file at {}", changelog_path.display()))?;
    
    let today = Local::now().format("%Y-%m-%d").to_string();
    let version_header = format!("## [{0}] {1} _{2}_", version, today, author);
    
    // Create regex pattern to match unreleased section headers (case insensitive)
    let unreleased_pattern = Regex::new(r"(?i)## \[(un|un-)?released\]").unwrap();
    
    let new_content = if unreleased_pattern.is_match(&content) {
        // Replace unreleased header with the new version header
        unreleased_pattern.replace(&content, &version_header).to_string()
    } else {
        // No unreleased section found, try to find the first version section
        let version_pattern = Regex::new(r"## \[\d+\.\d+\.\d+\]").unwrap();
        
        if let Some(first_match) = version_pattern.find(&content) {
            // Insert the new version section before the first version section
            let (before, after) = content.split_at(first_match.start());
            format!("{before}{version_header}\n\n{after}")
        } else {
            // No versions found, create a new section at the beginning
            format!("{version_header}\n\n{content}")
        }
    };
    
    // Write the updated content back to the file
    fs::write(changelog_path, new_content)
        .with_context(|| format!("Failed to write updated changelog to {}", changelog_path.display()))?;
    
    Ok(())
}

/// Checks if a CHANGELOG.md file exists at the specified path.
/// If it doesn't exist, creates a new one with a basic structure.
///
/// # Arguments
///
/// * `changelog_path` - Path to the CHANGELOG.md file
/// * `version` - The version to add to the changelog
/// * `author` - The author's name and email
///
/// # Returns
///
/// Result indicating success or failure
pub fn ensure_changelog_exists(changelog_path: &Path, version: &str, author: &str) -> Result<()> {
    if !changelog_path.exists() {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let initial_content = format!(
            "# Changelog\n\n## [{0}] {1} _{2}_\n### Added\n- Initial release\n",
            version, today, author
        );
        
        fs::write(changelog_path, initial_content)
            .with_context(|| format!("Failed to create new changelog at {}", changelog_path.display()))?;
    }
    
    Ok(())
}
