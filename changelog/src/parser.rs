use std::collections::HashMap;

use crate::{ChangelogConfig, error::ChangelogError, types::*, utils::*};

#[derive(Debug, Clone)]
pub struct Parser {
    config: ChangelogConfig,
}

impl Parser {
    pub fn new(config: ChangelogConfig) -> Self {
        Self { config }
    }

    pub fn parse(&self, content: &str) -> Result<ChangelogSections> {
        let ignore_duplicates = self.config.ignore_duplicates;

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
                    (&current_version, &current_category, captures.get(1))
                {
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
            } else if !line.is_empty()
                && !line.starts_with('#')
                && current_version.is_some()
                && current_category.is_none()
            {
                return Err(ChangelogError::InvalidFormat(
                    line_num + 1,
                    format!("Expected category header but found: {line}"),
                ));
            }
        }

        Ok(sections)
    }
}
