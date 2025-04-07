use crate::diff::{DefaultDiffHandler, DiffHandler};
use crate::formatter::{
    ChangelogFormat, ChangelogRewriter, DefaultChangelogRewriter, GitHubHeaderFormatter,
    HeaderFormatter, MarkdownSectionFormatter, SectionFormatter, StandardHeaderFormatter,
};
use crate::reorganizer::{ChangelogReorganizer, DefaultReorganizer};
use crate::types::{ChangelogEntry, ChangelogSections, Result};

use crate::regex_utils::{build_version_pattern, find_pattern_in_content};
use crate::version::{DefaultVersionUpdater, VersionUpdater};
use crate::{
    config::ChangelogConfig,
    parser::{ChangelogParser, Parser},
};
use std::collections::HashMap;

pub struct Changelog {
    content: String,
    sections: ChangelogSections,
    config: ChangelogConfig,
    format: ChangelogFormat,
    rewriter: Box<dyn ChangelogRewriter>,
    section_formatter: Box<dyn SectionFormatter>,
    header_formatter: Box<dyn HeaderFormatter>,
}

impl Changelog {
    pub fn new(content: String, config: ChangelogConfig, format: ChangelogFormat) -> Result<Self> {
        let parser = Parser::new(config.ignore_duplicates);
        let sections = parser.parse(&content)?;

        let header_formatter: Box<dyn HeaderFormatter> = match format {
            ChangelogFormat::Standard => Box::new(StandardHeaderFormatter {
                template: String::from("## [{0}] {1} _{2}_"),
            }),
            ChangelogFormat::GitHub => Box::new(GitHubHeaderFormatter),
        };

        Ok(Self {
            content,
            sections,
            config,
            format,
            rewriter: Box::new(DefaultChangelogRewriter),
            section_formatter: Box::new(MarkdownSectionFormatter),
            header_formatter,
        })
    }

    pub fn with_rewriter(mut self, rewriter: Box<dyn ChangelogRewriter>) -> Self {
        self.rewriter = rewriter;
        self
    }

    pub fn with_section_formatter(mut self, formatter: Box<dyn SectionFormatter>) -> Self {
        self.section_formatter = formatter;
        self
    }

    pub fn with_header_formatter(mut self, formatter: Box<dyn HeaderFormatter>) -> Self {
        self.header_formatter = formatter;
        self
    }

    pub fn reorganize(
        &self,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        if entries_to_move.is_empty() {
            return Ok(self.content.clone());
        }

        let reorganizer = DefaultReorganizer::new(&*self.rewriter, &*self.section_formatter);
        let reorganizer: &dyn ChangelogReorganizer = &reorganizer;
        reorganizer.reorganize(&self.content, unreleased_section, entries_to_move)
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

    pub fn replace_unreleased(&self, version: &str, author: &str) -> Result<String> {
        let updater = DefaultVersionUpdater::new(&*self.header_formatter);
        updater.update_with_version(&self.content, version, author)
    }

    pub fn extract_changes(&self, version: Option<&str>) -> Result<String> {
        let version_pattern = build_version_pattern(version);

        let range = find_pattern_in_content(&self.content, &version_pattern)?
            .ok_or(crate::error::ChangelogError::MissingVersionSection)?;

        let section_content = self.content[range.0..range.1].trim();
        Ok(section_content.to_string())
    }

    pub fn fix_with_diff(&self, diff: &str) -> Result<(String, bool, usize)> {
        let unreleased_section = self.unreleased_section();
        let version_sections = self.version_sections();

        let reorganizer = DefaultReorganizer::new(&*self.rewriter, &*self.section_formatter);
        let diff_handler = DefaultDiffHandler::new(&reorganizer);

        diff_handler.fix(&self.content, diff, &unreleased_section, &version_sections)
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
