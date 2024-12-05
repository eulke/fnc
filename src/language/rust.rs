use crate::ports::{AuthorInfo, ChangelogOperations, PackageOperations};
use std::{error::Error, fs};
use thiserror::Error;
use toml::Value;

use super::common::Changelog;

#[derive(Error, Debug)]
pub enum RustError {
    #[error("Failed to read Cargo.toml: {0}")]
    CargoReadError(#[from] std::io::Error),
    
    #[error("Failed to parse Cargo.toml: {0}")]
    CargoParseError(#[from] toml::de::Error),
    
    #[error("Invalid version in Cargo.toml")]
    InvalidVersion,
}

pub struct Rust;

impl PackageOperations for Rust {
    fn current_pkg_version(&self) -> String {
        self.read_version()
            .unwrap_or_else(|_| String::from("0.1.0"))
    }

    fn increment_pkg_version(
        &self,
        version: &str,
        author: &AuthorInfo,
    ) -> Result<(), Box<dyn Error>> {
        // Update Cargo.toml
        let cargo_content = fs::read_to_string("Cargo.toml")?;
        let mut cargo_toml: Value = toml::from_str(&cargo_content)?;
        
        if let Some(package) = cargo_toml.get_mut("package") {
            if let Some(ver) = package.get_mut("version") {
                *ver = toml::Value::String(version.to_string());
                let updated_content = toml::to_string(&cargo_toml)?;
                fs::write("Cargo.toml", updated_content)?;
            }
        }

        // Update changelog
        Changelog::write_version(version, author)?;
        Ok(())
    }
}

impl Rust {
    fn read_version(&self) -> Result<String, RustError> {
        let cargo_toml = fs::read_to_string("Cargo.toml")?;
        let cargo_toml: Value = toml::from_str(&cargo_toml)?;
        
        cargo_toml["package"]["version"]
            .as_str()
            .map(String::from)
            .ok_or(RustError::InvalidVersion)
    }
}
