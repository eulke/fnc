use crate::formatter::{ChangelogRewriter, SectionFormatter};
use crate::position::{
    find_first_version_position, find_next_section_position, find_unreleased_position,
};
use crate::types::{ChangelogEntry, ChangelogItem, ChangelogSection, Result};
use std::collections::HashMap;

pub trait ChangelogReorganizer {
    fn reorganize(
        &self,
        content: &str,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String>;

    fn prepare_sections(
        &self,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Vec<ChangelogSection>;
}

pub struct DefaultReorganizer<'a> {
    rewriter: &'a dyn ChangelogRewriter,
    section_formatter: &'a dyn SectionFormatter,
}

impl ChangelogReorganizer for DefaultReorganizer<'_> {
    fn reorganize(
        &self,
        content: &str,
        unreleased_section: &HashMap<String, Vec<String>>,
        entries_to_move: &[ChangelogEntry],
    ) -> Result<String> {
        if entries_to_move.is_empty() {
            return Ok(content.to_string());
        }

        let sections = self.prepare_sections(unreleased_section, entries_to_move);

        let new_content = match self.get_unreleased_strategy(content) {
            UnreleasedStrategy::Create => self.rewrite_without_unreleased(content, &sections),
            UnreleasedStrategy::Update(idx) => {
                self.rewrite_with_existing_unreleased(content, idx, &sections)
            }
        }?;

        Ok(new_content)
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
}

impl<'a> DefaultReorganizer<'a> {
    pub fn new(
        rewriter: &'a dyn ChangelogRewriter,
        section_formatter: &'a dyn SectionFormatter,
    ) -> Self {
        Self {
            rewriter,
            section_formatter,
        }
    }

    fn get_unreleased_strategy(&self, content: &str) -> UnreleasedStrategy {
        match find_unreleased_position(content) {
            Some(idx) => UnreleasedStrategy::Update(idx),
            None => UnreleasedStrategy::Create,
        }
    }

    fn rewrite_without_unreleased(
        &self,
        content: &str,
        unreleased_sections: &[ChangelogSection],
    ) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();

        // Find the first version section using the position module
        let (insertion_idx, _) = find_first_version_position(&lines);

        let formatted_unreleased = self.format_unreleased_section(unreleased_sections);
        let entries_to_move = self.convert_sections_to_entries(unreleased_sections);

        self.rewriter.rewrite(
            &lines,
            insertion_idx,
            insertion_idx,
            Some("## [Unreleased]\n\n"),
            &formatted_unreleased,
            &entries_to_move,
        )
    }

    fn rewrite_with_existing_unreleased(
        &self,
        content: &str,
        unreleased_idx: usize,
        unreleased_sections: &[ChangelogSection],
    ) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let formatted_unreleased = self.format_unreleased_section(unreleased_sections);

        let next_section_idx = find_next_section_position(&lines, unreleased_idx);

        let entries_to_move = self.convert_sections_to_entries(unreleased_sections);

        self.rewriter.rewrite(
            &lines,
            unreleased_idx,
            next_section_idx,
            Some("## [Unreleased]\n"),
            &formatted_unreleased,
            &entries_to_move,
        )
    }
    fn format_unreleased_section(&self, unreleased_sections: &[ChangelogSection]) -> String {
        self.section_formatter
            .format("Unreleased", unreleased_sections)
    }

    fn convert_sections_to_entries(&self, sections: &[ChangelogSection]) -> Vec<ChangelogEntry> {
        sections
            .iter()
            .flat_map(|section| {
                let section_title = section.title.clone();
                section.items.iter().map(move |item| ChangelogEntry {
                    content: item.content.clone(),
                    category: section_title.clone(),
                })
            })
            .collect()
    }
}

pub enum UnreleasedStrategy {
    Create,
    Update(usize),
}
