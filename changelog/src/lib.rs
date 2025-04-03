use chrono::Local;
use regex::Regex;
use std::fs;
use std::path::Path;
use thiserror::Error;
use std::collections::HashMap;

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
    
    #[error("Git operation failed: {0}")]
    Git(String),
    
    #[error("{0}")]
    Other(String),
}

impl ChangelogError {
    pub fn with_context(self, context: &str) -> Self {
        match self {
            ChangelogError::Other(msg) => ChangelogError::Other(format!("{}: {}", context, msg)),
            error => error,
        }
    }
}

/// Type alias for Result with ChangelogError
pub type Result<T> = std::result::Result<T, ChangelogError>;

/// Map of sections in changelog, organized by version and category
pub type ChangelogSections = HashMap<String, HashMap<String, Vec<String>>>;

/// An entry in the changelog to be moved
pub struct ChangelogEntry {
    pub content: String,
    pub category: String,
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

/// Parses a changelog file into sections by version and category
pub fn parse_changelog(content: &str) -> Result<ChangelogSections> {
    let mut sections = HashMap::new();
    let mut current_version: Option<String> = None;
    let mut current_category: Option<String> = None;
    
    let version_pattern = Regex::new(r"(?i)##\s*\[\s*((?:un|un-)?released|\d+\.\d+\.\d+)\s*\]")
        .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
    
    let category_pattern = Regex::new(r"### (.+)")
        .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
    
    let item_pattern = Regex::new(r"- (.+)")
        .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
    
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

fn create_moved_items_regex(entries_to_move: &[ChangelogEntry]) -> Result<Regex> {
    if entries_to_move.is_empty() {
        Regex::new(r"^$").map_err(|e| ChangelogError::ParseError(e.to_string()))
    } else {
        let pattern = entries_to_move.iter()
            .map(|entry| regex::escape(&entry.content))
            .collect::<Vec<_>>()
            .join("|");
        Regex::new(&format!(r"- ({})", pattern))
            .map_err(|e| ChangelogError::ParseError(e.to_string()))
    }
}

/// Identifies changelog entries that appear in the git diff and should be moved
pub fn identify_entries_in_diff(
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
                    .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
                
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

/// Reorganizes a changelog by moving entries to the unreleased section
pub fn reorganize_changelog(
    content: &str,
    unreleased_section: &HashMap<String, Vec<String>>,
    entries_to_move: &[ChangelogEntry],
) -> Result<String> {
    let mut new_unreleased = unreleased_section.clone();
    for entry in entries_to_move {
        new_unreleased
            .entry(entry.category.to_owned())
            .or_insert_with(Vec::new)
            .push(entry.content.to_owned());
    }
    
    let mut new_content = String::new();
    let lines = content.lines().collect::<Vec<_>>();
    
    let unreleased_pattern = Regex::new(r"(?i)## \[(un|un-)?released\]")
        .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
    let version_pattern = Regex::new(r"## \[\d+\.\d+\.\d+\]")
        .map_err(|e| ChangelogError::ParseError(e.to_string()))?;
    
    let mut formatted_unreleased = String::new();
    let actual_categories: Vec<_> = new_unreleased.keys().cloned().collect();

    for category in actual_categories {
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

    if let Some(idx) = lines.iter().position(|&line| unreleased_pattern.is_match(line)) {
        for i in 0..=idx {
            new_content.push_str(lines[i]);
            new_content.push('\n');
        }
        
        new_content.push_str(&formatted_unreleased);
        
        let next_version_idx = lines.iter()
            .skip(idx + 1)
            .position(|&line| version_pattern.is_match(line))
            .map(|pos| pos + idx + 1)
            .unwrap_or(lines.len());

        let moved_items_regex = create_moved_items_regex(entries_to_move)?;
        
        for i in next_version_idx..lines.len() {
            let line = lines[i];
            if !moved_items_regex.is_match(line) {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }
    } else {
        let title_idx = lines.iter().position(|&line| line.starts_with("# ")).unwrap_or(0);

        for i in 0..=title_idx {
            new_content.push_str(lines[i]);
            new_content.push('\n');
        }
        
        new_content.push_str("\n## [Unreleased]\n\n");
        new_content.push_str(&formatted_unreleased);
        
        let moved_items_regex = create_moved_items_regex(entries_to_move)?;
        
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

/// Fixes the changelog by moving entries that appear in the git diff from released sections to the unreleased section
pub fn fix_changelog(changelog_path: &Path, diff: &str, verbose: bool) -> Result<(bool, usize)> {
    if !changelog_path.exists() {
        return Err(ChangelogError::Other("CHANGELOG.md not found".to_string()));
    }
    
    let content = fs::read_to_string(changelog_path)
        .map_err(|e| ChangelogError::ReadError(e))?;
    
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
    
    let entries_to_move = identify_entries_in_diff(&diff, &version_sections, verbose)?;
    
    if entries_to_move.is_empty() {
        if verbose {
            println!("No changelog entries need to be moved to unreleased section");
        }
        return Ok((false, 0));
    }
    
    if verbose {
        println!("Found {} changelog entries to move to unreleased section", entries_to_move.len());
    }
    
    let new_content = reorganize_changelog(&content, &unreleased_section, &entries_to_move)?;
    
    fs::write(changelog_path, new_content)
        .map_err(|e| ChangelogError::ReadError(e))?;
    
    Ok((true, entries_to_move.len()))
}
