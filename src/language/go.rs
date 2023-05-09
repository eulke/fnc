use crate::ports::{AuthorInfo, PackageOperations};
use regex::Regex;
use semver::Version;
use std::{error::Error, fs};

use super::common::ChangelogWriter;

pub struct Go;

impl ChangelogWriter for Go {}
impl PackageOperations for Go {
    fn increment_pkg_version(
        &self,
        version: &str,
        author: &AuthorInfo,
    ) -> Result<(), Box<dyn Error>> {
        Go::increment_version(version, author)
    }

    fn current_pkg_version(&self) -> String {
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
}
