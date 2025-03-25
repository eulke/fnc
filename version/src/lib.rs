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
    
    #[error("{0}: {1}")]
    WithContext(String, Box<VersionError>),
}

impl VersionError {
    /// Add context to an error
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        VersionError::WithContext(context.into(), Box::new(self))
    }
    
    /// Get a user-friendly message for command line display
    pub fn user_message(&self) -> String {
        match self {
            VersionError::ParseError(e) => format!("Invalid version format: {}", e),
            VersionError::NoEcosystemDetected => "Could not detect project type. Supported project types: JavaScript, Rust, Python".to_string(),
            VersionError::VersionNotFound => "Could not find version in project files".to_string(),
            VersionError::WithContext(ctx, err) => format!("{}: {}", ctx, err.user_message()),
            _ => format!("{}", self),
        }
    }
}

// Helper trait for adding context to results
pub trait ResultExt<T, E> {
    fn with_context<C, F>(self, context: F) -> std::result::Result<T, VersionError>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: Into<VersionError>,
{
    fn with_context<C, F>(self, context: F) -> std::result::Result<T, VersionError>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|err| {
            let version_err: VersionError = err.into();
            version_err.with_context(context())
        })
    }
}

// Central structure for version management
pub struct Version;

impl Version {
    // Parse a version string into a Version
    pub fn parse(version: &str) -> Result<SemverVersion, VersionError> {
        SemverVersion::parse(version)
            .map_err(|e| VersionError::ParseError(e)
                .with_context(format!("Failed to parse version string: '{}'", version)))
    }

    // Increment a version based on the version type
    pub fn increment(version: &SemverVersion, version_type: &VersionType) -> Result<SemverVersion, VersionError> {
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
            .map_err(|e| e.with_context(format!("Failed to detect ecosystem in '{}'", dir_path.display())))
    }
    
    /// Read the current version from a project at the given path
    pub fn read_from_project(dir_path: &Path) -> Result<SemverVersion, VersionError> {
        let ecosystem_type = Self::detect_ecosystem(dir_path)?;
        let ecosystem = ecosystems::create_ecosystem(&ecosystem_type);
        ecosystem.read_version(dir_path)
            .map_err(|e| e.with_context(format!("Failed to read version from {} project", &ecosystem_type)))
    }
    
    /// Update the version in a project at the given path
    pub fn update_in_project(dir_path: &Path, version_type: &VersionType) -> Result<SemverVersion, VersionError> {
        let ecosystem_type = Self::detect_ecosystem(dir_path)?;
        let ecosystem = ecosystems::create_ecosystem(&ecosystem_type);
        
        // Read the current version
        let current_version = ecosystem.read_version(dir_path)
            .map_err(|e| e.with_context(format!("Failed to read current version from {} project", &ecosystem_type)))?;
        
        // Increment it
        let new_version = Self::increment(&current_version, &version_type)
            .map_err(|e| e.with_context(format!("Failed to increment {:?} version from {}", version_type, current_version)))?;
        
        // Write it back
        ecosystem.write_version(dir_path, &new_version)
            .map_err(|e| e.with_context(format!("Failed to write new version {} to project files", new_version)))?;
        
        Ok(new_version)
    }
}
