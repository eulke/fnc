use crate::error::ChangelogError;
use crate::types::{ChangelogEntry, Result};
use regex::Regex;

pub fn build_item_pattern(item: &str) -> Result<Regex> {
    let escaped_item = regex::escape(item);
    Regex::new(&format!(r"(?m)^\+.*{escaped_item}.*$"))
        .map_err(|e| ChangelogError::ParseError(e.to_string()))
}

pub fn build_entries_filter_pattern(entries: &[ChangelogEntry]) -> Result<Option<Regex>> {
    if entries.is_empty() {
        return Ok(None);
    }

    let pattern = entries
        .iter()
        .map(|entry| regex::escape(&entry.content))
        .collect::<Vec<_>>()
        .join("|");

    Regex::new(&format!(r"- ({pattern})"))
        .map(Some)
        .map_err(|e| ChangelogError::ParseError(e.to_string()))
}

pub fn build_version_pattern(version: Option<&str>) -> String {
    match version {
        Some(v) => format!(r"## \[{}\]", regex::escape(v)),
        None => r"## \[\d+\.\d+\.\d+\]".to_string(),
    }
}

pub fn find_pattern_in_content(content: &str, pattern: &str) -> Result<Option<(usize, usize)>> {
    let regex = Regex::new(pattern).map_err(|e| ChangelogError::ParseError(e.to_string()))?;

    match regex.find(content) {
        Some(m) => {
            let next_match = regex
                .find_at(content, m.start() + 1)
                .map_or(content.len(), |m| m.start());

            Ok(Some((m.start(), next_match)))
        }
        None => Ok(None),
    }
}
