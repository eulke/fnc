use chrono::Local;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use std::collections::HashMap;
use once_cell::sync::Lazy;

static SEMVER_VERSION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"## \[\d+\.\d+\.\d+\]").expect("Failed to compile semver regex")
});

static UNRELEASED_SECTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)## \[(un|un-)?released\]").expect("Failed to compile unreleased section regex")
});

static CHANGELOG_CATEGORY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"### (.+)").expect("Failed to compile category regex")
});

static CHANGELOG_ITEM_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"- (.+)").expect("Failed to compile item regex")
});

static VERSION_HEADER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)##\s*\[\s*((?:un|un-)?released|\d+\.\d+\.\d+)\s*\]").expect("Failed to compile version header regex")
});

/// Errors that can occur when working with changelogs
#[derive(Error, Debug)]
pub enum ChangelogError {
    #[error("Failed to read or write changelog file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to parse changelog: {0}")]
    ParseError(String),
    
    #[error("Failed to find version section in changelog")]
    MissingVersionSection,
    
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),
    
    #[error("Git operation failed: {0}")]
    Git(String),
    
    #[error("Invalid changelog format at line {0}: {1}")]
    InvalidFormat(usize, String),
    
    #[error("Duplicate category {0} in version {1}")]
    DuplicateCategory(String, String),
    
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("{0}")]
    Other(String),
    
    #[error("{0}: {1}")]
    WithContext(String, Box<ChangelogError>),
}

impl ChangelogError {
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        ChangelogError::WithContext(context.into(), Box::new(self))
    }
    
    pub fn user_message(&self) -> String {
        match self {
            ChangelogError::ReadError(e) => format!("File operation failed: {}", e),
            ChangelogError::ParseError(msg) => format!("Failed to parse changelog: {}", msg),
            ChangelogError::MissingVersionSection => "Failed to find version section in changelog".to_string(),
            ChangelogError::InvalidVersion(ver) => format!("Invalid version format: {}", ver),
            ChangelogError::Git(msg) => format!("Git operation failed: {}", msg),
            ChangelogError::InvalidFormat(line, msg) => format!("Invalid changelog format at line {}: {}", line, msg),
            ChangelogError::DuplicateCategory(cat, ver) => format!("Duplicate category {} in version {}", cat, ver),
            ChangelogError::RegexError(e) => format!("Regular expression error: {}", e),
            ChangelogError::Other(msg) => msg.clone(),
            ChangelogError::WithContext(ctx, err) => format!("{}: {}", ctx, err.user_message()),
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

/// Configuration options for changelog formatting and behavior
#[derive(Debug, Clone)]
pub struct ChangelogConfig {
    pub date_format: String,
    pub version_header_format: String,
    pub category_order: Vec<String>,
    pub default_categories: Vec<String>,
    pub ignore_duplicates: bool,
    pub verbose: bool,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            date_format: "%Y-%m-%d".to_string(),
            version_header_format: "## [{0}] {1} _{2}_".to_string(),
            category_order: vec![
                "Added".to_string(),
                "Changed".to_string(),
                "Fixed".to_string(),
                "Deprecated".to_string(),
                "Removed".to_string(),
                "Security".to_string(),
            ],
            default_categories: vec!["Added".to_string()],
            ignore_duplicates: false,
            verbose: false,
        }
    }
}

/// Defines the format of the changelog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangelogFormat {
    Standard,
    GitHub,
}

impl Default for ChangelogFormat {
    fn default() -> Self {
        Self::Standard
    }
}

/// Represents a changelog file with its contents and structured sections
pub struct Changelog {
    path: PathBuf,
    content: String,
    sections: ChangelogSections,
    config: ChangelogConfig,
    format: ChangelogFormat,
}

impl Changelog {
    /// Creates a new Changelog instance from a file path
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_config(path, ChangelogConfig::default(), ChangelogFormat::default())
    }
    
    /// Creates a new Changelog with custom configuration
    pub fn with_config(
        path: impl Into<PathBuf>,
        config: ChangelogConfig,
        format: ChangelogFormat,
    ) -> Result<Self> {
        let path = path.into();
        
        if !path.exists() {
            return Err(ChangelogError::Other(format!(
                "Changelog file not found at {:?}",
                path
            )));
        }
        
        let raw_content = fs::read_to_string(&path).map_err(ChangelogError::ReadError)?;
        
        // Parse the changelog using the config's ignore_duplicates setting
        let sections = parse_changelog(&raw_content, Some(&config))?;
        
        Ok(Self {
            path,
            content: raw_content,
            sections,
            config,
            format,
        })
    }
    
    /// Ensures a changelog file exists, creating it if necessary
    pub fn ensure_exists(
        path: impl Into<PathBuf>,
        version: &str,
        author: &str,
        config: Option<ChangelogConfig>,
        format: Option<ChangelogFormat>,
    ) -> Result<Self> {
        let path = path.into();
        let config = config.unwrap_or_default();
        let format = format.unwrap_or_default();
        
        if !path.exists() {
            let today = Local::now().format(&config.date_format).to_string();
            let initial_content = format!(
                "# Changelog\n\n{}\n### Added\n- Initial release\n",
                format_version_header(&config, version, &today, author, format)
            );
            
            fs::write(&path, initial_content).map_err(ChangelogError::ReadError)?;
        }
        
        Self::with_config(path, config, format)
    }
    
    /// Gets a reference to all changelog sections
    pub fn sections(&self) -> &ChangelogSections {
        &self.sections
    }
    
    /// Gets the path to the changelog file
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Gets the raw content of the changelog
    pub fn content(&self) -> &str {
        &self.content
    }
    
    /// Gets the unreleased section, or an empty map if none exists
    pub fn unreleased_section(&self) -> HashMap<String, Vec<String>> {
        self.sections
            .get("unreleased")
            .cloned()
            .unwrap_or_default()
    }
    
    /// Gets all version sections except unreleased
    pub fn version_sections(&self) -> ChangelogSections {
        self.sections
            .iter()
            .filter(|(k, _)| k.to_lowercase() != "unreleased")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    
    /// Updates the changelog by replacing the unreleased section with a new version entry
    /// 
    /// # Arguments
    /// 
    /// * `version` - The new version to add to the changelog
    /// * `author` - The author's name and email
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure
    pub fn update_with_version(&mut self, version: &str, author: &str) -> Result<()> {
        let today = Local::now().format(&self.config.date_format).to_string();
        let version_header = format_version_header(&self.config, version, &today, author, self.format);
        
        let new_content = if UNRELEASED_SECTION_PATTERN.is_match(&self.content) {
            UNRELEASED_SECTION_PATTERN.replace(&self.content, &version_header).to_string()
        } else if let Some(first_match) = SEMVER_VERSION_PATTERN.find(&self.content) {
            let (before, after) = self.content.split_at(first_match.start());
            format!("{before}{version_header}\n\n{after}")
        } else {
            format!("{version_header}\n\n{}", self.content)
        };
        
        fs::write(&self.path, &new_content).map_err(ChangelogError::ReadError)?;
        self.content = new_content;
        self.sections = parse_changelog(&self.content, Some(&self.config))?;
        
        Ok(())
    }
    
    /// Extracts changes for a specific version or the most recent one
    /// 
    /// # Arguments
    /// 
    /// * `version` - Specific version to extract changes from, or None to extract the most recent
    /// 
    /// # Returns
    /// 
    /// The extracted changes as a string, or an error if the parsing fails
    pub fn extract_changes(&self, version: Option<&str>) -> Result<String> {
        let version_regex = if let Some(v) = version {
            Regex::new(&format!(r"## \[{}\]", regex::escape(v)))
                .map_err(|e| ChangelogError::ParseError(e.to_string()))?
        } else {
            Regex::new(r"## \[\d+\.\d+\.\d+\]")
                .map_err(|e| ChangelogError::ParseError(e.to_string()))?
        };
        
        let section_start = match version_regex.find(&self.content) {
            Some(m) => m.start(),
            None => return Err(ChangelogError::MissingVersionSection),
        };
        
        let next_section = version_regex
            .find_at(&self.content, section_start + 1)
            .map(|m| m.start())
            .unwrap_or(self.content.len());
        
        let section = self.content[section_start..next_section].trim();
        Ok(section.to_string())
    }
    
    /// Fixes the changelog by moving entries that appear in the git diff to the unreleased section
    /// 
    /// # Arguments
    /// 
    /// * `diff` - Git diff content to analyze for changelog entries
    /// 
    /// # Returns
    /// 
    /// A tuple with (were_entries_moved, number_of_entries_moved)
    pub fn fix_with_diff(&mut self, diff: &str) -> Result<(bool, usize)> {
        let unreleased_section = self.unreleased_section();
        let version_sections = self.version_sections();
        let verbose = self.config.verbose;
        
        if verbose {
            println!("Found {} version sections in changelog", version_sections.len());
            println!("Unreleased section has {} categories", unreleased_section.len());
        }
        
        let entries_to_move = identify_entries_in_diff(diff, &version_sections, verbose)?;
        
        if entries_to_move.is_empty() {
            if verbose {
                println!("No changelog entries need to be moved to unreleased section");
            }
            return Ok((false, 0));
        }
        
        if verbose {
            println!(
                "Found {} changelog entries to move to unreleased section",
                entries_to_move.len()
            );
        }
        
        let new_content = reorganize_changelog(&self.content, &unreleased_section, &entries_to_move)?;
        
        fs::write(&self.path, &new_content).map_err(ChangelogError::ReadError)?;
        self.content = new_content;
        self.sections = parse_changelog(&self.content, Some(&self.config))?;
        
        Ok((true, entries_to_move.len()))
    }
    
    /// Returns an iterator over all changelog entries
    pub fn iter_entries(&self) -> impl Iterator<Item = (&str, &str, &str)> + '_ {
        self.sections
            .iter()
            .flat_map(|(version, categories)| {
                categories.iter().flat_map(move |(category, items)| {
                    items
                        .iter()
                        .map(move |item| (version.as_str(), category.as_str(), item.as_str()))
                })
            })
    }

    /// Gets the format of the changelog
    pub fn format(&self) -> ChangelogFormat {
        self.format
    }
    
    /// Sets the format of the changelog
    pub fn set_format(&mut self, format: ChangelogFormat) {
        self.format = format;
    }
    
    /// Gets a reference to the changelog's configuration
    pub fn config(&self) -> &ChangelogConfig {
        &self.config
    }
    
    /// Sets the changelog's configuration
    pub fn set_config(&mut self, config: ChangelogConfig) {
        self.config = config;
    }
}

fn format_version_header(
    config: &ChangelogConfig,
    version: &str,
    date: &str,
    author: &str,
    format: ChangelogFormat,
) -> String {
    match format {
        ChangelogFormat::Standard => {
            config
                .version_header_format
                .replace("{0}", version)
                .replace("{1}", date)
                .replace("{2}", author)
        },
        ChangelogFormat::GitHub => {
            format!("## [{0}] - {1}", version, date)
        }
    }
}

/// Parses a changelog file into sections by version and category
///
/// # Arguments
///
/// * `content` - The content of the changelog file
/// * `config` - Optional configuration that controls how parsing is done
///
/// # Returns
///
/// Parsed changelog sections, or an error if parsing fails 
pub fn parse_changelog(content: &str, config: Option<&ChangelogConfig>) -> Result<ChangelogSections> {
    let ignore_duplicates = config.is_some_and(|c| c.ignore_duplicates);
    
    let mut sections: ChangelogSections = HashMap::new();
    let mut current_version: Option<String> = None;
    let mut current_category: Option<String> = None;
    
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        
        if let Some(captures) = CHANGELOG_CATEGORY_PATTERN.captures(line) {
            if let (Some(version), Some(category_match)) = (&current_version, captures.get(1)) {
                let category = category_match.as_str().to_string();
                current_category = Some(category.clone());
                
                if let Some(version_map) = sections.get_mut(version) {
                    if version_map.contains_key(&category) && !ignore_duplicates {
                        return Err(ChangelogError::DuplicateCategory(
                            category,
                            version.clone(),
                        ));
                    }
                    version_map.entry(category).or_default();
                }
            }
        } else if let Some(captures) = CHANGELOG_ITEM_PATTERN.captures(line) {
            if let (Some(version), Some(category), Some(item_match)) = 
                (&current_version, &current_category, captures.get(1)) {
                let item = item_match.as_str().to_string();
                if let Some(categories) = sections.get_mut(version) {
                    if let Some(items) = categories.get_mut(category) {
                        // Check for duplicate entries if not ignoring duplicates
                        if !ignore_duplicates && items.contains(&item) {
                            // Skip duplicates but don't error
                        } else {
                            items.push(item);
                        }
                    }
                }
            }
        } else if let Some(captures) = VERSION_HEADER_PATTERN.captures(line) {
            if let Some(version_match) = captures.get(1) {
                let version = version_match.as_str().to_lowercase();
                current_version = Some(version.clone());
                current_category = None;
                sections.entry(version).or_default();
            }
        } else if !line.is_empty() && !line.starts_with('#') && 
                  current_version.is_some() && current_category.is_none() {
            return Err(ChangelogError::InvalidFormat(
                line_num + 1,
                format!("Expected category header but found: {}", line),
            ));
        }
    }
    
    Ok(sections)
}

fn create_moved_items_regex(entries_to_move: &[ChangelogEntry]) -> Result<Regex> {
    if entries_to_move.is_empty() {
        // Return a regex that won't match anything
        Ok(Regex::new(r"^$")?)
    } else {
        // Construct a pattern from all entries to detect moved items
        let pattern = entries_to_move.iter()
            .map(|entry| regex::escape(&entry.content))
            .collect::<Vec<_>>()
            .join("|");
        
        // Create the regex with error conversion through From trait
        Ok(Regex::new(&format!(r"- ({})", pattern))?)
    }
}

/// Identifies changelog entries that appear in the git diff and should be moved
fn identify_entries_in_diff(
    diff: &str,
    version_sections: &ChangelogSections,
    verbose: bool,
) -> Result<Vec<ChangelogEntry>> {
    let mut entries_to_move = Vec::with_capacity(16); // Pre-allocate to avoid frequent reallocations
    
    // Skip unreleased section and examine each version section
    for (version, categories) in version_sections {
        if version.to_lowercase() == "unreleased" {
            continue;
        }
        
        // Check each category in the version section
        for (category, items) in categories {
            for item in items {
                // Skip initial release entries as they're not relevant for moving
                if item.to_lowercase().contains("initial release") {
                    continue;
                }
                
                // Create regex pattern to find this item in the git diff
                // Using the ? operator for cleaner error propagation
                let escaped_item = regex::escape(item);
                let item_pattern = Regex::new(&format!(r"(?m)^\+.*{}.*$", escaped_item))?;
                
                // If the item is found in the diff, it should be moved
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
fn reorganize_changelog(
    content: &str,
    unreleased_section: &HashMap<String, Vec<String>>,
    entries_to_move: &[ChangelogEntry],
) -> Result<String> {
    let mut new_unreleased = unreleased_section.clone();
    
    for entry in entries_to_move {
        new_unreleased
            .entry(entry.category.to_owned())
            .or_default()
            .push(entry.content.to_owned());
    }
    
    let lines: Vec<&str> = content.lines().collect();
    let formatted_unreleased = format_unreleased_section(&new_unreleased);
    
    let unreleased_idx = lines.iter().position(|&line| UNRELEASED_SECTION_PATTERN.is_match(line));
    
    if let Some(idx) = unreleased_idx {
        format_with_existing_unreleased(&lines, idx, &formatted_unreleased, entries_to_move)
    } else {
        format_with_new_unreleased(&lines, &formatted_unreleased, entries_to_move)
    }
}

fn format_unreleased_section(
    unreleased: &HashMap<String, Vec<String>>,
) -> String {
    let mut formatted = String::with_capacity(1024); // Pre-allocate space to reduce reallocations
    
    // Sort categories for consistent output
    let mut categories: Vec<_> = unreleased.keys().collect();
    categories.sort(); // Ensure consistent ordering
    
    for category_key in categories {
        if let Some(items) = unreleased.get(category_key) {
            if !items.is_empty() {
                formatted.push_str("### ");
                formatted.push_str(category_key);
                formatted.push_str("\n");
                
                for item in items {
                    formatted.push_str("- ");
                    formatted.push_str(item);
                    formatted.push_str("\n");
                }
                formatted.push('\n');
            }
        }
    }
    
    formatted
}

fn format_with_existing_unreleased(
    lines: &[&str],
    unreleased_idx: usize,
    formatted_unreleased: &str,
    entries_to_move: &[ChangelogEntry],
) -> Result<String> {
    let mut new_content = String::new();
    
    // Add everything up to and including the unreleased header
    for line in lines.iter().take(unreleased_idx + 1) {
        new_content.push_str(line);
        new_content.push('\n');
    }
    
    // Add the formatted unreleased section
    new_content.push_str(formatted_unreleased);
    
    // Find the next version section
    let next_version_idx = lines.iter()
        .skip(unreleased_idx + 1)
        .position(|&line| SEMVER_VERSION_PATTERN.is_match(line))
        .map(|pos| pos + unreleased_idx + 1)
        .unwrap_or(lines.len());
    
    // Create regex for moved items
    let moved_items_regex = create_moved_items_regex(entries_to_move)?;
    
    // Add remaining content, excluding moved items
    for line in lines.iter().skip(next_version_idx) {
        if !moved_items_regex.is_match(line) {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }
    
    Ok(new_content)
}

fn format_with_new_unreleased(
    lines: &[&str],
    formatted_unreleased: &str,
    entries_to_move: &[ChangelogEntry],
) -> Result<String> {
    let mut new_content = String::new();
    
    // Find the title (or use line 0)
    let title_idx = lines.iter().position(|&line| line.starts_with("# ")).unwrap_or(0);
    
    // Add up to the title
    for line in lines.iter().take(title_idx + 1) {
        new_content.push_str(line);
        new_content.push('\n');
    }
    
    // Add unreleased section
    new_content.push_str("\n## [Unreleased]\n\n");
    new_content.push_str(formatted_unreleased);
    
    // Create regex for moved items
    let moved_items_regex = create_moved_items_regex(entries_to_move)?;
    
    // Add remaining content, excluding moved items
    for line in lines.iter().skip(title_idx + 1) {
        if !moved_items_regex.is_match(line) {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }
    
    Ok(new_content)
}
