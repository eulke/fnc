// Re-export semver for users of this library
pub use semver::{Version as SemverVersion, VersionReq};
use clap::ValueEnum;
use std::path::{Path, PathBuf};

mod ecosystems;
mod error;

pub use ecosystems::{Ecosystem, EcosystemType};
pub use error::{VersionError, Result, ResultExt};

/// The type of version increment to perform.
///
/// This enum is used to specify how a semantic version should be incremented:
/// - `Major`: Increment the major version (x.0.0) when making incompatible API changes
/// - `Minor`: Increment the minor version (0.x.0) when adding functionality in a backward compatible manner
/// - `Patch`: Increment the patch version (0.0.x) when making backward compatible bug fixes
#[derive(Clone, Debug, ValueEnum)]
pub enum VersionType {
    Major,
    Minor,
    Patch,
}

/// Central structure for version management across different project ecosystems.
///
/// The `Version` struct provides methods to:
/// - Parse version strings into semantic versions
/// - Increment versions according to semantic versioning rules
/// - Detect project ecosystem types
/// - Read versions from different project file formats
/// - Write updated versions back to project files
///
/// # Examples
///
/// ```no_run
/// use version::{Version, VersionType};
/// use std::path::Path;
///
/// // Parse a version string
/// let version_str = "1.2.3";
/// let semver = Version::parse(version_str).unwrap();
/// assert_eq!(semver.to_string(), "1.2.3");
///
/// // Increment a version
/// let new_version = Version::increment(&semver, &VersionType::Minor).unwrap();
/// assert_eq!(new_version.to_string(), "1.3.0");
///
/// // Read version from a project
/// let project_dir = Path::new(".");
/// let current_version = Version::read_from_project(project_dir).unwrap();
///
/// // Update version in a project
/// let updated_version = Version::update_in_project(project_dir, &VersionType::Patch).unwrap();
/// ```
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
        let mut new_version = version.clone();
        
        match version_type {
            VersionType::Major => {
                new_version.major += 1;
                new_version.minor = 0;
                new_version.patch = 0;
            },
            VersionType::Minor => {
                new_version.minor += 1;
                new_version.patch = 0;
            },
            VersionType::Patch => {
                new_version.patch += 1;
            },
        }
        
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
    
    /// Synchronize versions across multiple projects in different directories
    ///
    /// This function reads the version from the primary project and applies it
    /// to all dependency projects, which is useful for monorepos with cross-ecosystem
    /// dependencies.
    ///
    /// # Arguments
    ///
    /// * `primary_dir` - The directory of the primary project
    /// * `dependency_dirs` - A list of directories containing dependent projects
    ///
    /// # Returns
    ///
    /// A result containing the synchronized version
    pub fn sync_across_projects(primary_dir: &Path, dependency_dirs: &[&Path]) -> Result<SemverVersion> {
        // Read the version from the primary project
        let version = Self::read_from_project(primary_dir)?;
        
        // Apply the version to all dependency projects
        for dir in dependency_dirs {
            Self::write_to_project(dir, &version)
                .map_err(|e| e.with_context(format!(
                    "Failed to synchronize version {} to project at {}", 
                    version, 
                    dir.display()
                )))?;
        }
        
        Ok(version)
    }
    
    /// Find all projects in subdirectories and return their ecosystem types and paths
    ///
    /// # Arguments
    ///
    /// * `root_dir` - The root directory to scan for projects
    ///
    /// # Returns
    ///
    /// A result containing a vector of tuples with (path, ecosystem_type)
    pub fn discover_projects(root_dir: &Path) -> Result<Vec<(PathBuf, EcosystemType)>> {
        use std::fs;
        
        let mut projects = Vec::new();
        
        // Scan the root directory
        if let Ok(entries) = fs::read_dir(root_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    // Try to detect ecosystem in this directory
                    if let Ok(ecosystem) = Self::detect_ecosystem(&path) {
                        projects.push((path.clone(), ecosystem));
                    }
                    
                    // Recursively scan subdirectory if it's not a hidden directory
                    if let Some(dir_name) = path.file_name() {
                        if let Some(dir_name_str) = dir_name.to_str() {
                            if !dir_name_str.starts_with('.') {
                                // Avoid going too deep by limiting to a reasonable depth
                                let max_depth = 3;
                                Self::discover_projects_recursive(&path, &mut projects, 1, max_depth)?;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(projects)
    }
    
    /// Helper method for recursive project discovery
    fn discover_projects_recursive(
        dir: &Path,
        projects: &mut Vec<(PathBuf, EcosystemType)>,
        current_depth: usize,
        max_depth: usize
    ) -> Result<()> {
        use std::fs;
        
        // Don't go deeper than the max depth
        if current_depth > max_depth {
            return Ok(());
        }
        
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    // Try to detect ecosystem in this directory
                    if let Ok(ecosystem) = Self::detect_ecosystem(&path) {
                        projects.push((path.clone(), ecosystem));
                    }
                    
                    // Recursively scan subdirectory if it's not a hidden directory
                    if let Some(dir_name) = path.file_name() {
                        if let Some(dir_name_str) = dir_name.to_str() {
                            if !dir_name_str.starts_with('.') && dir_name_str != "node_modules" && dir_name_str != "target" {
                                Self::discover_projects_recursive(&path, projects, current_depth + 1, max_depth)?;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}
