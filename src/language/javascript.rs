use crate::ports::{AuthorInfo, ChangelogOperations, PackageOperations};
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::process::Command;

use super::common::Changelog;

pub struct Javascript;

impl PackageOperations for Javascript {
    fn increment_pkg_version(
        &self,
        version: &str,
        author: &AuthorInfo,
    ) -> Result<(), Box<dyn Error>> {
        npm_version(version)?;
        Changelog::write_version(version, author)?;
        Ok(())
    }

    fn current_pkg_version(&self) -> String {
        let package_json = fs::read_to_string("package.json").expect("Failed to read package.json");
        let package_json: Value =
            serde_json::from_str(&package_json).expect("Failed to parse package.json");

        package_json["version"]
            .as_str()
            .expect("Failed to get version from package.json")
            .to_owned()
    }
}

fn npm_version(version: &str) -> Result<(), Box<dyn Error>> {
    Command::new("npm").arg("version").arg(version).output()?;
    Ok(())
}
