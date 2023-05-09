use crate::ports::{AuthorInfo, PackageOperations};
use std::{error::Error, fs};
use toml::Value;

use super::common::ChangelogWriter;

pub struct Rust;
impl ChangelogWriter for Rust {}
impl PackageOperations for Rust {
    fn increment_pkg_version(
        &self,
        version: &str,
        author: &AuthorInfo,
    ) -> Result<(), Box<dyn Error>> {
        Rust::increment_version(version, author)
    }

    fn current_pkg_version(&self) -> String {
        let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
        let cargo_toml: Value = toml::from_str(&cargo_toml).expect("Failed to parse Cargo.toml");
        cargo_toml["package"]["version"]
            .as_str()
            .expect("Failed to get version from Cargo.toml")
            .to_owned()
    }
}
