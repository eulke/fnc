// Re-export semver for users of this library
pub use semver::{Version as SemverVersion, VersionReq};
use clap::ValueEnum;

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
    pub fn parse(version: &str) -> Result<SemverVersion, Box<dyn std::error::Error>> {
        Ok(SemverVersion::parse(version)?)
    }

    // Increment a version based on the version type
    pub fn increment(version: &str, version_type: VersionType) -> Result<SemverVersion, Box<dyn std::error::Error>> {
        let version = Self::parse(version)?;
        
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
}
