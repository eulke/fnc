use std::collections::HashMap;

use regex::Regex;

use crate::utils::*;
use crate::{config::ChangelogConfig, types::*};

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

pub struct Formatter {
    config: ChangelogConfig,
    format: ChangelogFormat,
}

impl Formatter {
    pub fn new(config: ChangelogConfig, format: ChangelogFormat) -> Self {
        Self { config, format }
    }

    pub fn format_header(&self, version: &str, date: &str, author: &str) -> String {
        match self.format {
            ChangelogFormat::Standard => self
                .config
                .version_header_format
                .replace("{0}", version)
                .replace("{1}", date)
                .replace("{2}", author),
            ChangelogFormat::GitHub => format!("## [{version}] - {date}"),
        }
    }

    pub fn format_unreleased_section(&self, unreleased: &HashMap<String, Vec<String>>) -> String {
        let mut formatted = String::with_capacity(1024); // Pre-allocate space to reduce reallocations

        // Sort categories for consistent output
        let mut categories: Vec<_> = unreleased.keys().collect();
        categories.sort(); // Ensure consistent ordering

        for category_key in categories {
            if let Some(items) = unreleased.get(category_key) {
                if !items.is_empty() {
                    formatted.push_str("### ");
                    formatted.push_str(category_key);
                    formatted.push('\n');

                    for item in items {
                        formatted.push_str("- ");
                        formatted.push_str(item);
                        formatted.push('\n');
                    }
                    formatted.push('\n');
                }
            }
        }

        formatted
    }

    pub fn format_with_existing_unreleased(
        &self,
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
        let next_version_idx = lines
            .iter()
            .skip(unreleased_idx + 1)
            .position(|&line| SEMVER_VERSION_PATTERN.is_match(line))
            .map_or(lines.len(), |pos| pos + unreleased_idx + 1);

        for line in lines.iter().skip(next_version_idx) {
            let should_exclude = Self::should_exclude_line(entries_to_move, line)?;
            if !should_exclude {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }

        Ok(new_content)
    }

    pub fn format_with_new_unreleased(
        &self,
        lines: &[&str],
        formatted_unreleased: &str,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        let mut new_content = String::new();

        // Find the title (or use line 0)
        let title_idx = lines
            .iter()
            .position(|&line| line.starts_with("# "))
            .unwrap_or(0);

        for line in lines.iter().take(title_idx + 1) {
            new_content.push_str(line);
            new_content.push('\n');
        }

        new_content.push_str("\n## [Unreleased]\n\n");
        new_content.push_str(formatted_unreleased);

        for line in lines.iter().skip(title_idx + 1) {
            let should_exclude = Self::should_exclude_line(entries_to_move, line)?;
            if !should_exclude {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }

        Ok(new_content)
    }

    fn should_exclude_line(entries_to_move: &[ChangelogEntry], line: &str) -> Result<bool> {
        if entries_to_move.is_empty() {
            return Ok(false);
        }

        let pattern = entries_to_move
            .iter()
            .map(|entry| regex::escape(&entry.content))
            .collect::<Vec<_>>()
            .join("|");

        let regex = Regex::new(&format!(r"- ({pattern})"))?;
        Ok(regex.is_match(line))
    }
}
