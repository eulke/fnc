use crate::formatter::ChangelogFormat;
use crate::types::*;
use crate::utils::{SEMVER_VERSION_PATTERN, UNRELEASED_SECTION_PATTERN};
use crate::{config::ChangelogConfig, error::ChangelogError, formatter::Formatter, parser::Parser};
use chrono::Local;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a changelog file with its contents and structured sections
pub struct Changelog {
    path: PathBuf,
    content: String,
    sections: ChangelogSections,
    config: ChangelogConfig,
    format: ChangelogFormat,
    parser: Parser,
    formatter: Formatter,
}

impl Changelog {
    /// Creates a new Changelog instance from a file path
    /// Creates a new Changelog instance from a file path
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    pub fn new(
        path: impl Into<PathBuf>,
        config: ChangelogConfig,
        format: ChangelogFormat,
    ) -> Result<Self> {
        let path = path.into();

        if !path.exists() {
            return Err(ChangelogError::Other(format!(
                "Changelog file not found at {path:?}"
            )));
        }

        let raw_content = fs::read_to_string(&path).map_err(ChangelogError::ReadError)?;

        let formatter = Formatter::new(config.clone(), format);
        let parser = Parser::new(config.clone());
        let sections = parser.parse(&raw_content)?;

        Ok(Self {
            path,
            content: raw_content,
            sections,
            config,
            format,
            parser,
            formatter,
        })
    }

    /// Reorganizes a changelog by moving entries to the unreleased section
    pub fn reorganize(
        &self,
        content: &str,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        let mut new_unreleased = unreleased_section.clone();

        for entry in entries_to_move {
            new_unreleased
                .entry(entry.category.clone())
                .or_default()
                .push(entry.content.clone());
        }

        let lines: Vec<&str> = content.lines().collect();
        let formatted_unreleased = self.formatter.format_unreleased_section(&new_unreleased);

        let unreleased_idx = lines
            .iter()
            .position(|&line| UNRELEASED_SECTION_PATTERN.is_match(line));

        unreleased_idx.map_or_else(
            || {
                self.formatter.format_with_new_unreleased(
                    &lines,
                    &formatted_unreleased,
                    entries_to_move,
                )
            },
            |idx| {
                self.formatter.format_with_existing_unreleased(
                    &lines,
                    idx,
                    &formatted_unreleased,
                    entries_to_move,
                )
            },
        )
    }

    /// Gets a reference to all changelog sections
    #[must_use]
    pub const fn sections(&self) -> &ChangelogSections {
        &self.sections
    }

    /// Gets the path to the changelog file
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets the raw content of the changelog
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Gets the unreleased section, or an empty map if none exists
    #[must_use]
    pub fn unreleased_section(&self) -> HashMap<String, Vec<String>> {
        self.sections.get("unreleased").cloned().unwrap_or_default()
    }

    /// Gets all version sections except unreleased
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// Returns an error if changelog update operations fail
    pub fn update_with_version(&mut self, version: &str, author: &str) -> Result<()> {
        let today = Local::now().format(&self.config.date_format).to_string();
        let version_header = self.formatter.format_header(version, &today, author);

        let new_content = if UNRELEASED_SECTION_PATTERN.is_match(&self.content) {
            UNRELEASED_SECTION_PATTERN
                .replace(&self.content, &version_header)
                .to_string()
        } else if let Some(first_match) = SEMVER_VERSION_PATTERN.find(&self.content) {
            let (before, after) = self.content.split_at(first_match.start());
            format!("{before}{version_header}\n\n{after}")
        } else {
            format!("{version_header}\n\n{}", self.content)
        };

        fs::write(&self.path, &new_content).map_err(ChangelogError::ReadError)?;
        self.content = new_content;
        // Use the stored parser instance
        self.sections = self.parser.parse(&self.content)?;

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
    /// The extracted changes as a string
    ///
    /// # Errors
    ///
    /// Returns an error if the parsing fails or if the requested version cannot be found
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
            .map_or(self.content.len(), |m| m.start());

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
    /// A tuple with (`were_entries_moved`, `number_of_entries_moved`)
    ///
    /// # Errors
    ///
    /// Returns an error if parsing the diff fails or if sections cannot be processed properly
    pub fn fix_with_diff(&mut self, diff: &str) -> Result<(bool, usize)> {
        let unreleased_section = self.unreleased_section();
        let version_sections = self.version_sections();
        let verbose = self.config.verbose;

        if verbose {
            println!(
                "Found {} version sections in changelog",
                version_sections.len()
            );
            println!(
                "Unreleased section has {} categories",
                unreleased_section.len()
            );
        }

        let entries_to_move = Self::identify_entries_in_diff(diff, &version_sections, verbose)?;

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

        let new_content = self.reorganize(&self.content, &unreleased_section, &entries_to_move)?;

        fs::write(&self.path, &new_content).map_err(ChangelogError::ReadError)?;
        self.content = new_content;
        // Use the stored parser instance
        self.sections = self.parser.parse(&self.content)?;

        Ok((true, entries_to_move.len()))
    }

    /// Returns an iterator over all changelog entries
    pub fn iter_entries(&self) -> impl Iterator<Item = (&str, &str, &str)> + '_ {
        self.sections.iter().flat_map(|(version, categories)| {
            categories.iter().flat_map(move |(category, items)| {
                items
                    .iter()
                    .map(move |item| (version.as_str(), category.as_str(), item.as_str()))
            })
        })
    }

    /// Gets the format of the changelog
    #[must_use]
    pub const fn format(&self) -> ChangelogFormat {
        self.format
    }

    /// Sets the format of the changelog
    pub fn set_format(&mut self, format: ChangelogFormat) {
        self.format = format;
    }

    /// Gets a reference to the changelog's configuration
    #[must_use]
    pub const fn config(&self) -> &ChangelogConfig {
        &self.config
    }

    /// Sets the changelog's configuration
    pub fn set_config(&mut self, config: ChangelogConfig) {
        self.config = config;
    }

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
                    let item_pattern = Regex::new(&format!(r"(?m)^\+.*{escaped_item}.*$"))?;

                    // If the item is found in the diff, it should be moved
                    if item_pattern.is_match(diff) {
                        if verbose {
                            println!("Found '{item}' in diff from main branch");
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
}
