// Re-export semver for users of this library
pub use semver::{Version as SemverVersion, VersionReq};
use clap::ValueEnum;
use std::path::Path;

mod ecosystems;
mod error;

pub use ecosystems::{Ecosystem, EcosystemType};
pub use error::{VersionError, Result, ResultExt};

#[derive(Clone, Debug, ValueEnum)]
pub enum VersionType {
    Major,
    Minor,
    Patch,
}

// Central structure for version management
pub struct Version;

impl Version {
    // Parse a version string into a Version
    pub fn parse(version: &str) -> Result<SemverVersion> {
        SemverVersion::parse(version)
            .map_err(|e| VersionError::ParseError(e)
                .with_context(format!("Failed to parse version string: '{}'", version)))
    }

    // Increment a version based on the version type
    pub fn increment(version: &SemverVersion, version_type: &VersionType) -> Result<SemverVersion> {
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
    pub fn detect_ecosystem(dir_path: &Path) -> Result<EcosystemType> {
        ecosystems::detect_ecosystem(dir_path)
            .map_err(|e| e.with_context(format!("Failed to detect ecosystem in '{}'", dir_path.display())))
    }
    
    /// Read the current version from a project at the given path
    pub fn read_from_project(dir_path: &Path) -> Result<SemverVersion> {
        let ecosystem_type = Self::detect_ecosystem(dir_path)?;
        let ecosystem = ecosystems::create_ecosystem(&ecosystem_type);
        ecosystem.read_version(dir_path)
            .map_err(|e| e.with_context(format!("Failed to read version from {} project", &ecosystem_type)))
    }
    
    /// Write a specific version to a project at the given path
    pub fn write_to_project(dir_path: &Path, version: &SemverVersion) -> Result<()> {
        let ecosystem_type = Self::detect_ecosystem(dir_path)?;
        let ecosystem = ecosystems::create_ecosystem(&ecosystem_type);
        
        ecosystem.write_version(dir_path, version)
            .map_err(|e| e.with_context(format!("Failed to write version {} to project files", version)))
    }
    
    /// Update the version in a project at the given path
    pub fn update_in_project(dir_path: &Path, version_type: &VersionType) -> Result<SemverVersion> {
        let ecosystem_type = Self::detect_ecosystem(dir_path)?;
        let ecosystem = ecosystems::create_ecosystem(&ecosystem_type);
        
        // Read the current version
        let current_version = ecosystem.read_version(dir_path)
            .map_err(|e| e.with_context(format!("Failed to read current version from {} project", &ecosystem_type)))?;
        
        // Increment it
        let new_version = Self::increment(&current_version, version_type)
            .map_err(|e| e.with_context(format!("Failed to increment {:?} version from {}", version_type, current_version)))?;
        
        // Write it back
        ecosystem.write_version(dir_path, &new_version)
            .map_err(|e| e.with_context(format!("Failed to write new version {} to project files", new_version)))?;
        
        Ok(new_version)
    }
}
