use once_cell::sync::Lazy;
use regex::Regex;

pub static SEMVER_VERSION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"## \[\d+\.\d+\.\d+\]").expect("Failed to compile semver regex"));

pub static UNRELEASED_SECTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)## \[(un|un-)?released\]").expect("Failed to compile unreleased section regex")
});

pub static CHANGELOG_CATEGORY_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"### (.+)").expect("Failed to compile category regex"));

pub static CHANGELOG_ITEM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"- (.+)").expect("Failed to compile item regex"));

pub static VERSION_HEADER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)##\s*\[\s*((?:un|un-)?released|\d+\.\d+\.\d+)\s*\]")
        .expect("Failed to compile version header regex")
});
