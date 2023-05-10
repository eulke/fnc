use std::{error::Error, fs};

use chrono::Local;
use regex::Regex;

use crate::ports::AuthorInfo;

pub trait ChangelogWriter {
    fn increment_version(version: &str, author: &AuthorInfo) -> Result<(), Box<dyn Error>> {
        let date = Local::now().format("%Y-%m-%d").to_string();

        let changelog_path = "Changelog.md";
        let changelog_content = fs::read_to_string(changelog_path)?;

        let unreleased_regex = Regex::new(r"(?i)\[?(unrelease[d]?)\]?")?;

        let updated_content = unreleased_regex
            .replace_all(
                &changelog_content,
                format!(
                    "[{}] {} _{} ({})_",
                    version, date, author.name, author.email
                ),
            )
            .to_string();

        fs::write(changelog_path, updated_content)?;
        Ok(())
    }
}
