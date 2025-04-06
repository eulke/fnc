use std::collections::HashMap;

use crate::{error::ChangelogError, types::*, utils::*};

#[derive(Debug, Clone)]
struct ParserState {
    current_version: Option<String>,
    current_category: Option<String>,
}

impl ParserState {
    fn new() -> Self {
        Self {
            current_version: None,
            current_category: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parser {
    ignore_duplicates: bool,
}

impl Parser {
    pub fn new(ignore_duplicates: bool) -> Self {
        Self { ignore_duplicates }
    }

    pub fn parse(&self, content: &str) -> Result<ChangelogSections> {
        let mut sections: ChangelogSections = HashMap::new();
        let mut state = ParserState::new();

        for (line_num, line) in content.lines().enumerate() {
            self.parse_line(line.trim(), &mut state, &mut sections, line_num)?;
        }

        Ok(sections)
    }

    fn parse_line(
        &self,
        line: &str,
        state: &mut ParserState,
        sections: &mut ChangelogSections,
        line_num: usize,
    ) -> Result<()> {
        if let Some(captures) = VERSION_HEADER_PATTERN.captures(line) {
            self.handle_version_header(captures, state, sections);
        } else if let Some(captures) = CHANGELOG_CATEGORY_PATTERN.captures(line) {
            self.handle_category_header(captures, state, sections)?;
        } else if let Some(captures) = CHANGELOG_ITEM_PATTERN.captures(line) {
            self.handle_item_line(captures, state, sections);
        } else {
            self.handle_other_line(line, state, line_num)?;
        }
        Ok(())
    }

    fn handle_version_header(
        &self,
        captures: regex::Captures,
        state: &mut ParserState,
        sections: &mut ChangelogSections,
    ) {
        if let Some(version_match) = captures.get(1) {
            let version = version_match.as_str().to_lowercase();
            state.current_version = Some(version.clone());
            state.current_category = None;
            sections.entry(version).or_default();
        }
    }

    fn handle_category_header(
        &self,
        captures: regex::Captures,
        state: &mut ParserState,
        sections: &mut ChangelogSections,
    ) -> Result<()> {
        if let (Some(version), Some(category_match)) = (&state.current_version, captures.get(1)) {
            let category = category_match.as_str().to_string();
            state.current_category = Some(category.clone());

            if let Some(version_map) = sections.get_mut(version) {
                if version_map.contains_key(&category) && !self.ignore_duplicates {
                    return Err(ChangelogError::DuplicateCategory(category, version.clone()));
                }
                version_map.entry(category).or_default();
            }
        }
        Ok(())
    }

    fn handle_item_line(
        &self,
        captures: regex::Captures,
        state: &ParserState,
        sections: &mut ChangelogSections,
    ) {
        if let (Some(version), Some(category), Some(item_match)) = (
            &state.current_version,
            &state.current_category,
            captures.get(1),
        ) {
            let item = item_match.as_str().to_string();
            if let Some(categories) = sections.get_mut(version) {
                if let Some(items) = categories.get_mut(category) {
                    if !self.ignore_duplicates && items.contains(&item) {
                        // Skip duplicate items silently if ignore_duplicates is false
                    } else if !items.contains(&item) {
                        // Add item only if it's not already present (handles both cases)
                        items.push(item);
                    }
                }
            }
        }
    }

    fn handle_other_line(&self, line: &str, state: &ParserState, line_num: usize) -> Result<()> {
        if !line.is_empty()
            && !line.starts_with('#') // Allow comment lines anywhere
            && state.current_version.is_some()
            && state.current_category.is_none()
            && !VERSION_HEADER_PATTERN.is_match(line) // Ensure it's not another version header
            && !CHANGELOG_CATEGORY_PATTERN.is_match(line)
        // Ensure it's not a category header we missed
        {
            // This condition targets lines that appear *after* a version header
            // but *before* any category header for that version, and are not
            // empty, comments, or valid headers themselves.
            return Err(ChangelogError::InvalidFormat(
                line_num + 1,
                format!(
                    "Expected category header '### Category Name' after version header, but found: {line}"
                ),
            ));
        }
        // Ignore empty lines, comments, lines before the first version,
        // or lines within a category section that aren't items (e.g., blank lines).
        Ok(())
    }
}
