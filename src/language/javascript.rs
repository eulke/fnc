use crate::ports::{AuthorInfo, ChangelogOperations, PackageOperations};
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::process::Command;
use thiserror::Error;

use super::common::Changelog;

#[derive(Error, Debug)]
pub enum JavascriptError {
    #[error("Failed to read package.json: {0}")]
    PackageReadError(#[from] std::io::Error),
    
    #[error("Failed to parse package.json: {0}")]
    PackageParseError(#[from] serde_json::Error),
    
    #[error("Invalid version in package.json")]
    InvalidVersion,
    
    #[error("NPM command failed: {0}")]
    NpmError(String),
}

pub struct Javascript;

impl PackageOperations for Javascript {
    fn current_pkg_version(&self) -> String {
        self.read_version()
            .unwrap_or_else(|_| String::from("0.1.0"))
    }

    fn increment_pkg_version(
        &self,
        version: &str,
        author: &AuthorInfo,
    ) -> Result<(), Box<dyn Error>> {
        npm_version(version)?;
        Changelog::write_version(version, author)?;
        Ok(())
    }
}

impl Javascript {
    fn read_version(&self) -> Result<String, JavascriptError> {
        let package_json = fs::read_to_string("package.json")?;
        let package_json: Value = serde_json::from_str(&package_json)?;
        
        package_json["version"]
            .as_str()
            .map(String::from)
            .ok_or(JavascriptError::InvalidVersion)
    }
}

fn npm_version(version: &str) -> Result<(), Box<dyn Error>> {
    let output = Command::new("npm")
        .arg("version")
        .arg(version)
        .output()
        .map_err(|e| JavascriptError::NpmError(e.to_string()))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(Box::new(JavascriptError::NpmError(error.to_string())));
    }

    Ok(())
}
