use crate::ports::PackageOperations;
use crate::semver::Language;
use chrono::prelude::*;
use git2::Config;
use regex::Regex;
use std::fs;
use std::process::Command;

pub struct Adapter;

impl PackageOperations for Adapter {
    fn increment_version(&self, version: &str, language: &Language) {
        match language {
            Language::JavaScript => increment_version_js(version),
            Language::Go | Language::Java => increment_version_changelog(version),
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

    increment_version_changelog(version);

    if output.status.success() {
        println!("Updated package.json version to: {}", version);
    } else {
        panic!("Failed to update package.json version");
    }
}

fn increment_version_rust(version: &str) {
    println!("{}", version)
}

fn increment_version_changelog(version: &str) {
    let changelog_path = "Changelog.md";
    let changelog_content =
        fs::read_to_string(changelog_path).expect("Failed to read Changelog.md");

    let unreleased_regex = Regex::new(r"(?i)unrelease").expect("Failed to compile regex");
    let updated_content = unreleased_regex.replace(&changelog_content, version);

    let config = Config::open_default().expect("Failed to open git config");
    let author_name = config.get_string("user.name").unwrap_or_default();
    let author_email = config.get_string("user.email").unwrap_or_default();

    let date = Local::now().format("%Y-%m-%d").to_string();

    let final_content = format!(
        "{} - {} - _{} ({})_",
        updated_content, date, author_name, author_email
    );
    fs::write(changelog_path, final_content).expect("Failed to write Changelog.md");
}
