use crate::regex_utils::build_item_pattern;
use crate::reorganizer::ChangelogReorganizer;
use crate::types::{ChangelogEntry, ChangelogSections, Result};
use std::collections::HashMap;

pub trait DiffHandler {
    fn fix(
        &self,
        content: &str,
        diff: &str,
        unreleased_section: &HashMap<String, Vec<String>>,
        version_sections: &ChangelogSections,
    ) -> Result<(String, bool, usize)>;

    fn identify_entries(
        diff: &str,
        version_sections: &ChangelogSections,
    ) -> Result<Vec<ChangelogEntry>>;
}

pub struct DefaultDiffHandler<'a> {
    reorganizer: &'a dyn ChangelogReorganizer,
}

impl<'a> DefaultDiffHandler<'a> {
    pub fn new(reorganizer: &'a dyn ChangelogReorganizer) -> Self {
        Self { reorganizer }
    }
}

impl DiffHandler for DefaultDiffHandler<'_> {
    fn fix(
        &self,
        content: &str,
        diff: &str,
        unreleased_section: &HashMap<String, Vec<String>>,
        version_sections: &ChangelogSections,
    ) -> Result<(String, bool, usize)> {
        let entries_to_move = Self::identify_entries(diff, version_sections)?;

        if entries_to_move.is_empty() {
            return Ok((content.to_string(), false, 0));
        }

        let new_content =
            self.reorganizer
                .reorganize(content, unreleased_section, &entries_to_move)?;
        Ok((new_content, true, entries_to_move.len()))
    }

    fn identify_entries(
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

                    let item_pattern = build_item_pattern(item)?;

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
}
