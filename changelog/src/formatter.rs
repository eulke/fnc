use regex::Regex;

use crate::regex_utils::build_entries_filter_pattern;
use crate::types::*;

// --- Traits ---

pub trait HeaderFormatter: Send + Sync {
    fn format(&self, version: &str, date: &str, author: &str) -> String;
    fn format_with_config(
        &self,
        version: &str,
        date: &str,
        author: &str,
        _template: Option<&str>,
    ) -> String {
        self.format(version, date, author)
    }
}

pub trait SectionFormatter: Send + Sync {
    fn format(&self, title: &str, sections: &[ChangelogSection]) -> String;
}

pub trait ChangelogRewriter: Send + Sync {
    fn rewrite(
        &self,
        lines: &[&str],
        insertion_idx: usize,
        filter_start_idx: usize,
        header_to_insert: Option<&str>,
        formatted_content: &str,
        entries_to_filter: &[ChangelogEntry],
    ) -> Result<String>;
}

// Header Formatters
#[derive(Debug, Clone)]
pub struct StandardHeaderFormatter {
    pub template: String,
}

impl HeaderFormatter for StandardHeaderFormatter {
    fn format(&self, version: &str, date: &str, author: &str) -> String {
        self.template
            .replace("{0}", version)
            .replace("{1}", date)
            .replace("{2}", author)
    }
}

#[derive(Debug, Clone)]
pub struct GitHubHeaderFormatter;

impl HeaderFormatter for GitHubHeaderFormatter {
    fn format(&self, version: &str, date: &str, _author: &str) -> String {
        format!("## [{version}] - {date}")
    }
}

// Section Formatter (Markdown)
#[derive(Debug, Clone)]
pub struct MarkdownSectionFormatter;

impl SectionFormatter for MarkdownSectionFormatter {
    fn format(&self, _title: &str, sections: &[ChangelogSection]) -> String {
        let mut formatted = String::with_capacity(1024);
        // Sort sections by title for consistent output
        let mut sorted_sections = sections.to_vec();
        sorted_sections.sort_by(|a, b| a.title.cmp(&b.title));

        for section in sorted_sections {
            if !section.items.is_empty() {
                formatted.push_str("### ");
                formatted.push_str(&section.title);
                formatted.push('\n');
                for item in &section.items {
                    formatted.push_str("- ");
                    formatted.push_str(&item.content);
                    formatted.push('\n');
                }
                formatted.push('\n');
            }
        }
        formatted.to_string()
    }
}

// Changelog Rewriter
#[derive(Debug, Clone)]
pub struct DefaultChangelogRewriter;

impl ChangelogRewriter for DefaultChangelogRewriter {
    fn rewrite(
        &self,
        lines: &[&str],
        insertion_idx: usize,
        filter_start_idx: usize,
        header_to_insert: Option<&str>,
        formatted_content: &str,
        entries_to_filter: &[ChangelogEntry],
    ) -> Result<String> {
        let filter_regex = self.build_filter_regex(entries_to_filter)?;
        let mut new_content = String::with_capacity(lines.len() * 50 + formatted_content.len());

        // 1. Add lines before the insertion point
        for line in lines.iter().take(insertion_idx) {
            new_content.push_str(line);
            new_content.push('\n');
        }

        // 2. Add the optional new header
        if let Some(header) = header_to_insert {
            new_content.push_str(header);
        }

        // 3. Add the newly formatted content section
        if !formatted_content.is_empty() {
            new_content.push_str(formatted_content);
        }

        // 4. Add the remaining lines, filtering as needed
        self.append_filtered_lines(
            &mut new_content,
            lines.iter().skip(filter_start_idx),
            filter_regex.as_ref(),
        );

        Ok(new_content.to_string())
    }
}

impl DefaultChangelogRewriter {
    fn build_filter_regex(&self, entries_to_filter: &[ChangelogEntry]) -> Result<Option<Regex>> {
        build_entries_filter_pattern(entries_to_filter)
    }

    fn append_filtered_lines<'a, I>(
        &self,
        target_string: &mut String,
        lines_iter: I,
        filter_regex: Option<&Regex>,
    ) where
        I: Iterator<Item = &'a &'a str>,
    {
        for line in lines_iter {
            if !self.should_exclude_line(filter_regex, line) {
                target_string.push_str(line);
                target_string.push('\n');
            }
        }
    }

    fn should_exclude_line(&self, filter_regex: Option<&Regex>, line: &str) -> bool {
        filter_regex.is_some_and(|regex| regex.is_match(line))
    }
}

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
