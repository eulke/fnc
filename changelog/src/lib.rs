use chrono::Local;
use regex::Regex;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when working with changelogs
#[derive(Error, Debug)]
pub enum ChangelogError {
    #[error("Failed to read changelog file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to parse changelog: {0}")]
    ParseError(String),
    
    #[error("Failed to find version section in changelog")]
    MissingVersionSection,
    
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),
}

/// Type alias for Result with ChangelogError
pub type Result<T> = std::result::Result<T, ChangelogError>;

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
        .map_err(ChangelogError::ReadError)?;
    
    let today = Local::now().format("%Y-%m-%d").to_string();
    let version_header = format!("## [{0}] {1} _{2}_", version, today, author);
    
    // Create regex pattern to match unreleased section headers (case insensitive)
    let unreleased_pattern = Regex::new(r"(?i)## \[(un|un-)?released\]")
        .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
    
    let new_content = if unreleased_pattern.is_match(&content) {
        // Replace unreleased header with the new version header
        unreleased_pattern.replace(&content, &version_header).to_string()
    } else {
        // No unreleased section found, try to find the first version section
        let version_pattern = Regex::new(r"## \[\d+\.\d+\.\d+\]")
            .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
        
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
        .map_err(ChangelogError::ReadError)?;
    
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
            .map_err(ChangelogError::ReadError)?;
    }
    
    Ok(())
}

/// Parses the changelog file to extract the list of changes since the last version.
/// This can be useful when generating release notes.
///
/// # Arguments
///
/// * `changelog_path` - Path to the CHANGELOG.md file
/// * `version` - Specific version to extract changes from, or None to extract the most recent
///
/// # Returns
///
/// The extracted changes as a string, or an error if the parsing fails
pub fn extract_changes(changelog_path: &Path, version: Option<&str>) -> Result<String> {
    let content = fs::read_to_string(changelog_path)
        .map_err(ChangelogError::ReadError)?;

    // If a specific version was requested, find that section
    // Otherwise, find the first (most recent) version section
    let version_pattern = if let Some(v) = version {
        Regex::new(&format!(r"## \[{}\]", regex::escape(v)))
            .map_err(|e| ChangelogError::ParseError(e.to_string()))?
    } else {
        Regex::new(r"## \[\d+\.\d+\.\d+\]")
            .map_err(|e| ChangelogError::ParseError(e.to_string()))?
    };
    
    // Find the start of the section for this version
    let section_start = match version_pattern.find(&content) {
        Some(m) => m.start(),
        None => return Err(ChangelogError::MissingVersionSection),
    };
    
    // Find the start of the next version section (or EOF if none)
    let next_section = version_pattern
        .find_at(&content, section_start + 1)
        .map(|m| m.start())
        .unwrap_or(content.len());
    
    // Extract the changes section
    let section = content[section_start..next_section].trim();
    Ok(section.to_string())
}
