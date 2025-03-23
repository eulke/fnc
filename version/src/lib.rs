// Re-export semver for users of this library
pub use semver::{Version as SemverVersion, VersionReq};
use clap::ValueEnum;
use std::path::Path;
use thiserror::Error;

mod ecosystems;
pub use ecosystems::{Ecosystem, EcosystemType};

#[derive(Clone, Debug, ValueEnum)]
pub enum VersionType {
    Major,
    Minor,
    Patch,
}

#[derive(Error, Debug)]
pub enum VersionError {
    #[error("Failed to parse version: {0}")]
    ParseError(#[from] semver::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse file: {0}")]
    ParseFileError(String),
    
    #[error("Version not found in file")]
    VersionNotFound,
    
    #[error("Unsupported ecosystem")]
    UnsupportedEcosystem,
    
    #[error("No ecosystem detected")]
    NoEcosystemDetected,
    
    #[error("Other error: {0}")]
    Other(String),
}

// Central structure for version management
pub struct Version;

impl Version {
    // Parse a version string into a Version
    pub fn parse(version: &str) -> Result<SemverVersion, VersionError> {
        Ok(SemverVersion::parse(version)?)
    }

    // Increment a version based on the version type
    pub fn increment(version: SemverVersion, version_type: VersionType) -> Result<SemverVersion, VersionError> {
        let new_version = match version_type {
            VersionType::Major => SemverVersion {
                major: version.major + 1,
                minor: 0,
                patch: 0,
                pre: version.pre.clone(),
                build: version.build.clone(),
            },
            VersionType::Minor => SemverVersion {
                major: version.major,
                minor: version.minor + 1,
                patch: 0,
                pre: version.pre.clone(),
                build: version.build.clone(),
            },
            VersionType::Patch => SemverVersion {
                major: version.major,
                minor: version.minor,
                patch: version.patch + 1,
                pre: version.pre.clone(),
                build: version.build.clone(),
            },
        };

        Ok(new_version)
    }
    
    /// Detect the ecosystem type from a directory
    pub fn detect_ecosystem(dir_path: &Path) -> Result<EcosystemType, VersionError> {
        ecosystems::detect_ecosystem(dir_path)
    }
    
    /// Read the current version from a project at the given path
    pub fn read_from_project(dir_path: &Path) -> Result<SemverVersion, VersionError> {
        let ecosystem_type = Self::detect_ecosystem(dir_path)?;
        let ecosystem = ecosystems::create_ecosystem(ecosystem_type);
        ecosystem.read_version(dir_path)
    }
    
    /// Update the version in a project at the given path
    pub fn update_in_project(dir_path: &Path, version_type: VersionType) -> Result<SemverVersion, VersionError> {
        let ecosystem_type = Self::detect_ecosystem(dir_path)?;
        let ecosystem = ecosystems::create_ecosystem(ecosystem_type);
        
        // Read the current version
        let current_version = ecosystem.read_version(dir_path)?;
        
        // Increment it
        let new_version = Self::increment(current_version, version_type)?;
        
        // Write it back
        ecosystem.write_version(dir_path, &new_version)?;
        
        Ok(new_version)
    }
}
