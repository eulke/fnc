use std::{error::Error, fs};
use thiserror::Error;
use chrono::Local;
use regex::Regex;
use semver::Version;

use crate::ports::{AuthorInfo, ChangelogOperations};

#[derive(Error, Debug)]
pub enum ChangelogError {
    #[error("Failed to read changelog: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Invalid regex pattern: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("Invalid semver version: {0}")]
    SemverError(#[from] semver::Error),
}

pub struct Changelog;

impl ChangelogOperations for Changelog {
    fn read_version() -> String {
        Self::read_version_internal().unwrap_or_else(|_| String::from("0.1.0"))
    }

    fn write_version(version: &str, author: &AuthorInfo) -> Result<(), Box<dyn Error>> {
        let date = Local::now().format("%Y-%m-%d").to_string();
        let (changelog_content, changelog_path) = read_file()?;
        let semver_version = Self::read_version();

        let new_version_section = format!(
            "## [{}] {} _{} ({})_",
            version, date, author.name, author.email
        );

        let unreleased_regex = Regex::new(r"(?i)(.*unrelease[d]?.*)")?;
        
        let updated_content = if unreleased_regex.is_match(&changelog_content) {
            update_unreleased(&unreleased_regex, &changelog_content, &new_version_section)
        } else {
            add_semver(&changelog_content, &new_version_section, &semver_version)
        };

        fs::write(changelog_path, updated_content)?;
        Ok(())
    }
}

impl Changelog {
    fn read_version_internal() -> Result<String, ChangelogError> {
        let changelog_path = "CHANGELOG.md";
        let changelog_content = fs::read_to_string(changelog_path).map_err(ChangelogError::ReadError)?;
        let re = Regex::new(r"\[(\d+\.\d+\.\d+)\]")?;
        let mut max_version = Version::parse("0.0.0")?;

        for cap in re.captures_iter(&changelog_content) {
            let version_str = &cap[1];
            let version = Version::parse(version_str)?;

            if version > max_version {
                max_version = version;
            }
        }

        Ok(max_version.to_string())
    }
}

fn read_file() -> Result<(String, &'static str), std::io::Error> {
    let changelog_path = "CHANGELOG.md";
    let changelog_content = fs::read_to_string(changelog_path)?;
    Ok((changelog_content, changelog_path))
}

fn update_unreleased(regex: &Regex, changelog_content: &str, new_version_section: &str) -> String {
    regex
        .replace(changelog_content, new_version_section)
        .to_string()
}

fn add_semver(changelog_content: &str, new_version_section: &str, semver_version: &str) -> String {
    let regex_pattern = format!(r"(.*{}.*)", semver_version);
    let semver_regex = Regex::new(&regex_pattern).unwrap_or_else(|_| {
        // If the regex creation fails, we'll fall back to appending at the end
        Regex::new(r"^.*$").unwrap()
    });

    if semver_regex.is_match(changelog_content) {
        semver_regex
            .replace(changelog_content, |caps: &regex::Captures| {
                format!("{} \n\n{}", new_version_section, &caps[0])
            })
            .to_string()
    } else {
        format!("{}\n{}", changelog_content, new_version_section)
    }
}
