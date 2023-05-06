use regex::Regex;
use semver::Version;
use serde_json::Value;
use std::fs;

pub enum Language {
    Rust,
    JavaScript,
    Go,
    Java,
}

pub fn detect_language() -> Option<Language> {
    if fs::metadata("Cargo.toml").is_ok() {
        return Some(Language::Rust);
    }
    if fs::metadata("package.json").is_ok() {
        return Some(Language::JavaScript);
    }
    if fs::metadata("go.mod").is_ok() {
        return Some(Language::Go);
    }
    if fs::metadata("pom.xml").is_ok() {
        return Some(Language::Java);
    }
    None
}

pub fn get_current(language: &Language) -> String {
    match language {
        Language::Rust => get_rust_version(),
        Language::JavaScript => get_js_version(),
        Language::Go | Language::Java => get_changelog_version(),
    }
}

fn get_rust_version() -> String {
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let cargo_toml: Value = toml::from_str(&cargo_toml).expect("Failed to parse Cargo.toml");
    cargo_toml["package"]["version"]
        .as_str()
        .expect("Failed to get version from Cargo.toml")
        .to_owned()
}

fn get_js_version() -> String {
    let package_json = fs::read_to_string("package.json").expect("Failed to read package.json");
    let package_json: Value =
        serde_json::from_str(&package_json).expect("Failed to parse package.json");

    package_json["version"]
        .as_str()
        .expect("Failed to get version from package.json")
        .to_owned()
}

fn get_changelog_version() -> String {
    let changelog = fs::read_to_string("CHANGELOG.md").expect("Failed to read CHANGELOG.md");

    let re = Regex::new(r"\[(\d+\.\d+\.\d+)\]").expect("Invalid regex");
    let mut max_version = Version::parse("0.0.0").expect("Invalid Semver version");

    for cap in re.captures_iter(&changelog) {
        let version_str = &cap[1];
        let version =
            Version::parse(version_str).expect("Invalid Semver version read on changelog");

        if version > max_version {
            max_version = version;
        }
    }

    max_version.to_string()
}

pub fn increment(semver: &str, version: &str) -> String {
    let mut parsed_version = Version::parse(semver).expect("Failed to parse version");

    match version {
        "major" => {
            parsed_version.major += 1;
            parsed_version.minor = 0;
            parsed_version.patch = 0;
        }
        "minor" => {
            parsed_version.minor += 1;
            parsed_version.patch = 0;
        }
        "patch" => {
            parsed_version.patch += 1;
        }
        _ => panic!("Invalid version level. Only 'major', 'minor' and 'patch' are allowed."),
    }

    parsed_version.to_string()
}
