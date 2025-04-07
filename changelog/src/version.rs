use crate::formatter::HeaderFormatter;
use crate::position::{find_first_version_position, find_unreleased_position};
use crate::types::Result;
use chrono::Local;

pub trait VersionUpdater {
    fn update_with_version(&self, content: &str, version: &str, author: &str) -> Result<String>;
}

pub struct DefaultVersionUpdater<'a> {
    header_formatter: &'a dyn HeaderFormatter,
}

impl<'a> DefaultVersionUpdater<'a> {
    pub fn new(header_formatter: &'a dyn HeaderFormatter) -> Self {
        Self { header_formatter }
    }

    fn get_unreleased_position(&self, content: &str) -> Option<usize> {
        find_unreleased_position(content)
    }

    fn rewrite_for_new_version(
        &self,
        content: &str,
        pos: usize,
        new_version_header: &str,
    ) -> String {
        let mut new_lines: Vec<&str> = content.lines().collect();
        let unreleased_line = new_lines[pos];

        if unreleased_line.trim() == "## [Unreleased]" {
            new_lines[pos] = new_version_header;
        } else {
            new_lines.insert(pos + 1, "");
            new_lines.insert(pos + 2, new_version_header);

            let relative_pos = new_lines[pos + 3..]
                .iter()
                .position(|line| line.starts_with("## ["))
                .unwrap_or(new_lines.len() - pos - 3);

            let next_version_header_idx = relative_pos + pos + 3;
            if !new_lines[next_version_header_idx - 1].trim().is_empty() {
                new_lines.insert(next_version_header_idx, "");
            }
        }

        let mut result = new_lines.join("\n");
        if content.ends_with('\n') {
            result.push('\n');
        }
        result
    }

    fn add_new_version_before_last(&self, content: &str, new_version_header: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        // Find the first version section using the position module
        let (mut insert_pos, _) = find_first_version_position(&lines);

        // Create the new content with the version inserted before the first version
        let mut lines_vec = lines.to_vec();

        // Ensure there's an empty line before the new version (if not at start of file)
        if insert_pos > 0 && !lines_vec[insert_pos - 1].trim().is_empty() {
            lines_vec.insert(insert_pos, "");
            insert_pos += 1;
        }

        // Insert the new version header
        lines_vec.insert(insert_pos, new_version_header);

        // Ensure there's an empty line after the new version
        if insert_pos + 1 >= lines_vec.len() || !lines_vec[insert_pos + 1].trim().is_empty() {
            lines_vec.insert(insert_pos + 1, "");
        }

        lines_vec.join("\n") + "\n"
    }
}

impl VersionUpdater for DefaultVersionUpdater<'_> {
    fn update_with_version(&self, content: &str, version: &str, author: &str) -> Result<String> {
        let date = Local::now().format("%Y-%m-%d").to_string();
        let new_version_header = self.header_formatter.format(version, &date, author);

        let unreleased_pos = self.get_unreleased_position(content);
        let new_content = match unreleased_pos {
            Some(pos) => self.rewrite_for_new_version(content, pos, &new_version_header),
            None => self.add_new_version_before_last(content, &new_version_header),
        };

        Ok(new_content)
    }
}
