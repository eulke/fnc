use crate::error::ChangelogError;
use crate::formatter::{
    ChangelogFormat, ChangelogRewriter, DefaultChangelogRewriter, GitHubHeaderFormatter,
    HeaderFormatter, MarkdownSectionFormatter, SectionFormatter, StandardHeaderFormatter,
};
use crate::types::{ChangelogEntry, ChangelogItem, ChangelogSection, ChangelogSections, Result};
use crate::utils::{SEMVER_VERSION_PATTERN, UNRELEASED_SECTION_PATTERN};
use crate::{config::ChangelogConfig, parser::Parser};
use chrono::Local;
use regex::Regex;
use std::collections::HashMap;

enum UnreleasedStrategy {
    Create,
    Update(usize),
}

pub struct Changelog {
    content: String,
    sections: ChangelogSections,
    config: ChangelogConfig,
    format: ChangelogFormat,
}

impl Changelog {
    pub fn new(content: String, config: ChangelogConfig, format: ChangelogFormat) -> Result<Self> {
        let parser = Parser::new(config.ignore_duplicates);
        let sections = parser.parse(&content)?;

        Ok(Self {
            content,
            sections,
            config,
            format,
        })
    }

    pub fn reorganize(
        &self,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        if entries_to_move.is_empty() {
            return Ok(self.content.clone());
        }

        let new_content = match self.get_unreleased_strategy(&self.content) {
            UnreleasedStrategy::Create => {
                self.rewrite_without_unreleased(unreleased_section, entries_to_move)
            }
            UnreleasedStrategy::Update(idx) => {
                self.rewrite_with_existing_unreleased(idx, unreleased_section, entries_to_move)
            }
        }?;

        Ok(new_content)
    }

    fn get_unreleased_strategy(&self, content: &str) -> UnreleasedStrategy {
        let lines: Vec<&str> = content.lines().collect();
        let unreleased_idx = lines
            .iter()
            .position(|&line| UNRELEASED_SECTION_PATTERN.is_match(line));

        match unreleased_idx {
            Some(idx) => UnreleasedStrategy::Update(idx),
            None => UnreleasedStrategy::Create,
        }
    }

    fn rewrite_without_unreleased(
        &self,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        let unreleased_sections = self.prepare_sections(unreleased_section, entries_to_move);
        let section_formatter = MarkdownSectionFormatter;
        let rewriter = DefaultChangelogRewriter;

        let lines: Vec<&str> = self.content.lines().collect();
        let title_idx = lines
            .iter()
            .position(|&line| line.starts_with("# "))
            .unwrap_or(0);
        let insertion_idx = title_idx + 1;
        let formatted_unreleased = section_formatter.format("Unreleased", &unreleased_sections);

        rewriter.rewrite(
            &lines,
            insertion_idx,                 // Insert after title
            insertion_idx,                 // Filter after insertion
            Some("\n## [Unreleased]\n\n"), // Add header
            &formatted_unreleased,         // Content
            entries_to_move,               // Entries to filter out later
        )
    }

    fn rewrite_with_existing_unreleased(
        &self,
        unreleased_idx: usize,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        let unreleased_sections = self.prepare_sections(unreleased_section, entries_to_move);
        let section_formatter = MarkdownSectionFormatter;
        let rewriter = DefaultChangelogRewriter;

        let lines: Vec<&str> = self.content.lines().collect();
        let content_start_idx = unreleased_idx + 1;
        let next_version_header_idx = lines
            .iter()
            .skip(content_start_idx)
            .position(|&line| SEMVER_VERSION_PATTERN.is_match(line))
            .map_or(lines.len(), |pos| pos + content_start_idx);

        let formatted_unreleased = section_formatter.format("Unreleased", &unreleased_sections);

        rewriter.rewrite(
            &lines,
            content_start_idx,       // Insert content after header
            next_version_header_idx, // Filter from next version onwards
            None,                    // No new header needed
            &formatted_unreleased,   // Content
            entries_to_move,         // Entries to filter out
        )
    }

    fn prepare_sections(
        &self,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Vec<ChangelogSection> {
        let mut new_unreleased_map = unreleased_section.clone();

        for entry in entries_to_move {
            new_unreleased_map
                .entry(entry.category.clone())
                .or_default()
                .push(entry.content.clone());
        }

        // Convert HashMap to Vec<ChangelogSection>
        new_unreleased_map
            .into_iter()
            .map(|(title, items_str)| ChangelogSection {
                title,
                items: items_str
                    .into_iter()
                    .map(|content| ChangelogItem { content })
                    .collect(),
            })
            .collect()
    }

    pub const fn sections(&self) -> &ChangelogSections {
        &self.sections
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn unreleased_section(&self) -> HashMap<String, Vec<String>> {
        self.sections.get("unreleased").cloned().unwrap_or_default()
    }

    pub fn version_sections(&self) -> ChangelogSections {
        self.sections
            .iter()
            .filter(|(k, _)| k.to_lowercase() != "unreleased")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn update_with_version(&self, version: &str, author: &str) -> Result<String> {
        let date = Local::now().format("%Y-%m-%d").to_string();
        let header_formatter: Box<dyn HeaderFormatter> = match self.format {
            ChangelogFormat::Standard => Box::new(StandardHeaderFormatter {
                template: self.config.version_header_format.clone(),
            }),
            ChangelogFormat::GitHub => Box::new(GitHubHeaderFormatter),
        };

        let new_version_header = header_formatter.format(version, &date, author);

        let new_content = match self.get_unreleased_position() {
            Some(pos) => self.rewrite_for_new_version(pos, &new_version_header),
            None => self.add_new_version_after_title(&new_version_header),
        };

        Ok(new_content)
    }

    fn get_unreleased_position(&self) -> Option<usize> {
        self.content
            .lines()
            .position(|line| UNRELEASED_SECTION_PATTERN.is_match(line))
    }

    fn rewrite_for_new_version(&self, pos: usize, new_version_header: &str) -> String {
        let lines: Vec<&str> = self.content.lines().collect();
        let mut new_lines = Vec::with_capacity(lines.len() + 1);

        for (i, line) in lines.iter().enumerate() {
            if i == pos {
                new_lines.push(new_version_header);
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

        // Reconstruct the string with proper newline handling
        let mut result = new_lines.join("\n");
        if self.content.ends_with('\n') {
            result.push('\n');
        }
        result
    }

    fn add_new_version_after_title(&self, new_version_header: &str) -> String {
        let title_pos = self
            .content
            .lines()
            .position(|line| line.starts_with("# "))
            .unwrap_or(0);

        let mut lines: Vec<&str> = self.content.lines().collect();
        // Insert with appropriate spacing
        lines.insert(title_pos + 1, ""); // Blank line
        lines.insert(title_pos + 2, new_version_header);
        lines.insert(title_pos + 3, ""); // Blank line
        lines.join("\n") + "\n" // Add trailing newline
    }

    pub fn extract_changes(&self, version: Option<&str>) -> Result<String> {
        let version_pattern = match version {
            Some(v) => format!(r"## \[{}\]", regex::escape(v)),
            None => r"## \[\d+\.\d+\.\d+\]".to_string(),
        };

        let version_regex =
            Regex::new(&version_pattern).map_err(|e| ChangelogError::ParseError(e.to_string()))?;

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

    pub fn fix_with_diff(&self, diff: &str) -> Result<(String, bool, usize)> {
        let unreleased_section = self.unreleased_section();
        let version_sections = self.version_sections();
        let entries_to_move = Self::identify_entries_in_diff(diff, &version_sections)?;

        if entries_to_move.is_empty() {
            return Ok((self.content.clone(), false, 0));
        }

        let new_content = self.reorganize(&unreleased_section, &entries_to_move)?;
        Ok((new_content, true, entries_to_move.len()))
    }

    fn identify_entries_in_diff(
        diff: &str,
        version_sections: &ChangelogSections,
    ) -> Result<Vec<ChangelogEntry>> {
        let mut entries_to_move = Vec::with_capacity(16);

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
                    let item_pattern = Regex::new(&format!(r"(?m)^\+.*{escaped_item}.*$"))?;

                    if item_pattern.is_match(diff) {
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

    pub fn iter_entries(&self) -> impl Iterator<Item = (&str, &str, &str)> + '_ {
        self.sections.iter().flat_map(|(version, categories)| {
            categories.iter().flat_map(move |(category, items)| {
                items
                    .iter()
                    .map(move |item| (version.as_str(), category.as_str(), item.as_str()))
            })
        })
    }

    pub const fn format(&self) -> ChangelogFormat {
        self.format
    }

    pub const fn config(&self) -> &ChangelogConfig {
        &self.config
    }
}
