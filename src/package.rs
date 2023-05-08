use crate::ports::{AuthorInfo, PackageOperations};
use crate::semver::Language;
use chrono::prelude::*;
use regex::Regex;
use std::fs;
use std::process::Command;

pub struct Adapter;

impl PackageOperations for Adapter {
    fn increment_version(&self, version: &str, language: &Language, author: &AuthorInfo) {
        match language {
            Language::JavaScript => {
                increment_version_js(version);
                increment_version_changelog(version, author);
            }
            Language::Go | Language::Java => increment_version_changelog(version, author),
            Language::Rust => increment_version_rust(version),
        }
    }
}

fn increment_version_js(version: &str) {
    let output = Command::new("npm")
        .arg("version")
        .arg(version)
        .output()
        .expect("Failed to run npm version");

    if output.status.success() {
        println!("Updated package.json version to: {}", version);
    } else {
        panic!("Failed to update package.json version");
    }
}

fn increment_version_rust(version: &str) {
    println!("{}", version)
}

fn increment_version_changelog(version: &str, author: &AuthorInfo) {
    let date = Local::now().format("%Y-%m-%d").to_string();

    let changelog_path = "Changelog.md";
    let changelog_content =
        fs::read_to_string(changelog_path).expect("Failed to read Changelog.md");

    let unreleased_regex =
        Regex::new(r"(?i)\[?(unrelease[d]?)\]?").expect("Failed to compile regex");
    let updated_content = unreleased_regex
        .replace(
            &changelog_content,
            format!(
                "[{}] {} _{} ({})_",
                version, date, author.name, author.email
            ),
        )
        .to_string();

    fs::write(changelog_path, updated_content).expect("Failed to write Changelog.md");
}
