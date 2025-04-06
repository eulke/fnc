use crate::error::ChangelogError;
use crate::formatter::ChangelogFormat;
use crate::types::*;
use crate::utils::{SEMVER_VERSION_PATTERN, UNRELEASED_SECTION_PATTERN};
use crate::{config::ChangelogConfig, formatter::*, parser::Parser};
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

        let raw_content = fs::read_to_string(&path).map_err(ChangelogError::ReadError)?;

        let parser = Parser::new(config.clone());
        let sections = parser.parse(&raw_content)?;

        Ok(Self {
            path,
            content: raw_content,
            sections,
            config,
            format,
            parser,
        })
    }

    /// Reorganizes a changelog by moving entries to the unreleased section
    pub fn reorganize(
        &self,
        content: &str,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        let mut new_unreleased_map = unreleased_section.clone();

        for entry in entries_to_move {
            new_unreleased_map
                .entry(entry.category.clone())
                .or_default()
                .push(entry.content.clone());
        }

        // Convert HashMap to Vec<ChangelogSection>
        let new_unreleased_sections: Vec<ChangelogSection> = new_unreleased_map
            .into_iter()
            .map(|(title, items_str)| ChangelogSection {
                title,
                items: items_str
                    .into_iter()
                    .map(|content| ChangelogItem { content })
                    .collect(),
            })
            .collect();

        let section_formatter = MarkdownSectionFormatter;
        let rewriter = DefaultChangelogRewriter;

        let lines: Vec<&str> = content.lines().collect();
        let formatted_unreleased = section_formatter.format("Unreleased", &new_unreleased_sections);

        let unreleased_idx = lines
            .iter()
            .position(|&line| UNRELEASED_SECTION_PATTERN.is_match(line));

        unreleased_idx.map_or_else(
            || {
                // Case: No existing [Unreleased] section
                let title_idx = lines
                    .iter()
                    .position(|&line| line.starts_with("# "))
                    .unwrap_or(0);
                let insertion_idx = title_idx + 1;
                rewriter.rewrite(
                    &lines,
                    insertion_idx,                 // Insert after title
                    insertion_idx,                 // Filter after insertion
                    Some("\n## [Unreleased]\n\n"), // Add header
                    &formatted_unreleased,         // Content
                    entries_to_move,               // Entries to filter out later
                )
            },
            |idx| {
                // Case: Existing [Unreleased] section found
                let content_start_idx = idx + 1;
                let next_version_header_idx = lines
                    .iter()
                    .skip(content_start_idx)
                    .position(|&line| SEMVER_VERSION_PATTERN.is_match(line))
                    .map_or(lines.len(), |pos| pos + content_start_idx);

                rewriter.rewrite(
                    &lines,
                    content_start_idx,       // Insert content after header
                    next_version_header_idx, // Filter from next version onwards
                    None,                    // No new header needed
                    &formatted_unreleased,   // Content
                    entries_to_move,         // Entries to filter out
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
        let date = Local::now().format("%Y-%m-%d").to_string();
        let new_version_header =
            create_header_formatter(self.format, &self.config).format(version, &date, author);

        // Find the position of the ## [Unreleased] header
        let unreleased_header_pos = self
            .content
            .lines()
            .position(|line| UNRELEASED_SECTION_PATTERN.is_match(line));

        let new_content_string = match unreleased_header_pos {
            Some(pos) => {
                // Found [Unreleased], replace its header line
                // Ensure we handle potential trailing newline correctly
                let lines: Vec<&str> = self.content.lines().collect();
                let mut new_lines = Vec::with_capacity(lines.len() + 1);
                for (i, line) in lines.iter().enumerate() {
                    if i == pos {
                        new_lines.push(new_version_header.as_str());
                    } else {
                        new_lines.push(line);
                    }
                }
                // Find the position of the next version header, if any
                let next_version_header_pos_option = new_lines
                    .iter()
                    .skip(pos + 1)
                    .position(|line| SEMVER_VERSION_PATTERN.is_match(line));

                // If a next version header exists, ensure a blank line precedes it
                if let Some(relative_pos) = next_version_header_pos_option {
                    let next_version_header_idx = relative_pos + pos + 1;
                    // Check the line right before the next header
                    if !new_lines[next_version_header_idx - 1].trim().is_empty() {
                        new_lines.insert(next_version_header_idx, "");
                    }
                }

                // Reconstruct the string, adding a final newline if the original had one
                // or if the last line isn't empty
                let mut result = new_lines.join("\n");
                if self.content.ends_with('\n') || !new_lines.last().is_none_or(|l| l.is_empty()) {
                    result.push('\n');
                }
                result
            }
            None => {
                // No [Unreleased], insert after the main title
                let title_pos = self
                    .content
                    .lines()
                    .position(|line| line.starts_with("# "))
                    .unwrap_or(0); // Assume title is first line if not found

                let mut lines: Vec<&str> = self.content.lines().collect();
                // Insert with appropriate spacing
                lines.insert(title_pos + 1, ""); // Blank line
                lines.insert(title_pos + 2, &new_version_header);
                lines.insert(title_pos + 3, ""); // Blank line
                lines.join("\n") + "\n" // Add trailing newline
            }
        };

        fs::write(&self.path, &new_content_string).map_err(ChangelogError::ReadError)?;
        self.content = new_content_string;
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
}
