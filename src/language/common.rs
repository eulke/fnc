use std::{error::Error, fs};

use chrono::Local;
use regex::Regex;
use semver::Version;

use crate::ports::{AuthorInfo, ChangelogOperations};

pub struct Changelog;
impl ChangelogOperations for Changelog {
    fn write_version(version: &str, author: &AuthorInfo) -> Result<(), Box<dyn Error>> {
        let date = Local::now().format("%Y-%m-%d").to_string();
        let (changelog_content, changelog_path) = read_file()?;
        let semver_version = Self::read_version();

        let new_version_section = format!(
            "## [{}] {} _{} ({})_",
            version, date, author.name, author.email
        );

        let unreleased_regex = Regex::new(r"(?i)(.*unrelease[d]?.*)").unwrap();

        let updated_content = match unreleased_regex.is_match(&changelog_content) {
            true => update_unreleased(&unreleased_regex, &changelog_content, &new_version_section),
            false => add_semver(&changelog_content, &new_version_section, &semver_version),
        };

        fs::write(changelog_path, updated_content)?;
        Ok(())
    }

    fn read_version() -> String {
        let (changelog_content, _) = read_file().expect("Failed to read changelog");

        let re = Regex::new(r"\[(\d+\.\d+\.\d+)]").expect("Invalid regex");
        let mut max_version = Version::parse("0.0.0").expect("Invalid Semver version");

        for cap in re.captures_iter(&changelog_content) {
            let version_str = &cap[1];
            let version =
                Version::parse(version_str).expect("Invalid Semver version read on changelog");

            if version > max_version {
                max_version = version;
            }
        }

        max_version.to_string()
    }
}

fn read_file() -> Result<(String, &'static str), Box<dyn Error>> {
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
    let regex_pattern = format!("(.*{}.*)", semver_version);
    let semver_regex = Regex::new(&regex_pattern).unwrap();

    match semver_regex.is_match(changelog_content) {
        true => semver_regex
            .replace(changelog_content, |caps: &regex::Captures| {
                format!("{} \n\n{}", new_version_section, &caps[0])
            })
            .to_string(),
        false => format!("{}\n{}", changelog_content, new_version_section),
    }
}
