use crate::error::{Result, VersionError};
use once_cell::sync::Lazy;
use semver::Version as SemverVersion;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Cache for ecosystem detection results to avoid redundant filesystem operations
static ECOSYSTEM_CACHE: Lazy<Arc<Mutex<HashMap<PathBuf, EcosystemType>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Represents the type of ecosystem (language/framework)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcosystemType {
    JavaScript, // package.json
    Rust,       // Cargo.toml
    Python,     // pyproject.toml or setup.py
                // Add more as needed
}

// Implement Display trait for EcosystemType for better error messages
impl fmt::Display for EcosystemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::JavaScript => "JavaScript",
            Self::Rust => "Rust",
            Self::Python => "Python",
        };
        write!(f, "{name}")
    }
}

/// Trait for ecosystem-specific version operations
pub trait Ecosystem {
    /// Read the current version from a project
    /// Read the current version from a project
    ///
    /// # Errors
    /// Returns error if version cannot be read or parsed
    fn read_version(&self, dir_path: &Path) -> Result<SemverVersion>;

    /// Write a new version to a project
    /// Write a new version to a project
    ///
    /// # Errors
    /// Returns error if version cannot be written
    fn write_version(&self, dir_path: &Path, version: &SemverVersion) -> Result<()>;
}

/// Create an ecosystem implementation based on the type
pub fn create_ecosystem(ecosystem_type: EcosystemType) -> Box<dyn Ecosystem> {
    match ecosystem_type {
        EcosystemType::JavaScript => Box::new(JavaScriptEcosystem),
        EcosystemType::Rust => Box::new(RustEcosystem),
        EcosystemType::Python => Box::new(PythonEcosystem),
    }
}

/// Detect the ecosystem type from a directory
pub fn detect_ecosystem(dir_path: &Path) -> Result<EcosystemType> {
    let path_buf = dir_path.to_path_buf();

    if let Ok(cache) = ECOSYSTEM_CACHE.lock() {
        if let Some(ecosystem) = cache.get(&path_buf) {
            return Ok(*ecosystem);
        }
    }

    if !dir_path.is_dir() {
        return Err(VersionError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Directory not found",
        )));
    }

    let ecosystem_markers = [
        ("package.json", EcosystemType::JavaScript),
        ("Cargo.toml", EcosystemType::Rust),
    ];

    for (marker, ecosystem) in ecosystem_markers {
        if dir_path.join(marker).exists() {
            update_ecosystem_cache(&path_buf, ecosystem);
            return Ok(ecosystem);
        }
    }

    let pyproject_toml_path = dir_path.join("pyproject.toml");
    let setup_py_path = dir_path.join("setup.py");
    if pyproject_toml_path.exists() || setup_py_path.exists() {
        update_ecosystem_cache(&path_buf, EcosystemType::Python);
        return Ok(EcosystemType::Python);
    }

    Err(VersionError::NoEcosystemDetected)
}

fn update_ecosystem_cache(path: &Path, ecosystem: EcosystemType) {
    if let Ok(mut cache) = ECOSYSTEM_CACHE.lock() {
        cache.insert(path.to_path_buf(), ecosystem);
    }
}

//=============== JavaScript Ecosystem Implementation ===============//

/// JavaScript ecosystem (package.json)
struct JavaScriptEcosystem;

#[derive(Serialize, Deserialize, Debug)]
struct PackageJson {
    version: String,
    #[serde(flatten)]
    other: std::collections::HashMap<String, Value>,
}

impl Ecosystem for JavaScriptEcosystem {
    fn read_version(&self, dir_path: &Path) -> Result<SemverVersion> {
        let package_json_path = dir_path.join("package.json");
        let content = fs::read_to_string(package_json_path)?;

        let package_json: PackageJson = serde_json::from_str(&content).map_err(|e| {
            VersionError::ParseFileError(format!("Failed to parse package.json: {e}"))
        })?;

        let version = SemverVersion::parse(&package_json.version)?;
        Ok(version)
    }

    fn write_version(&self, dir_path: &Path, version: &SemverVersion) -> Result<()> {
        use std::process::Command;

        let npm_version_cmd = Command::new("npm")
            .current_dir(dir_path)
            .arg("version")
            .arg(version.to_string())
            .arg("--no-git-tag-version")
            .output()
            .map_err(|e| VersionError::Other(format!("Failed to run npm version command: {e}")))?;

        if !npm_version_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&npm_version_cmd.stderr);
            return Err(VersionError::Other(format!(
                "npm version command failed: {stderr}"
            )));
        }

        Ok(())
    }
}

//=============== Rust Ecosystem Implementation ===============//

/// Rust ecosystem (Cargo.toml)
struct RustEcosystem;

impl Ecosystem for RustEcosystem {
    fn read_version(&self, dir_path: &Path) -> Result<SemverVersion> {
        let cargo_toml_path = dir_path.join("Cargo.toml");
        let content = fs::read_to_string(cargo_toml_path)?;

        let cargo_toml: toml::Table = toml::from_str(&content).map_err(|e| {
            VersionError::ParseFileError(format!("Failed to parse Cargo.toml: {e}"))
        })?;

        let version = cargo_toml
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .ok_or(VersionError::VersionNotFound)?;

        let version = SemverVersion::parse(version)?;
        Ok(version)
    }

    fn write_version(&self, dir_path: &Path, version: &SemverVersion) -> Result<()> {
        let cargo_toml_path = dir_path.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml_path)?;

        // Parse TOML
        let mut cargo_toml: toml::Table = toml::from_str(&content).map_err(|e| {
            VersionError::ParseFileError(format!("Failed to parse Cargo.toml: {e}"))
        })?;

        // Update version
        if let Some(package) = cargo_toml.get_mut("package") {
            if let Some(package_table) = package.as_table_mut() {
                if let Some(v) = package_table.get_mut("version") {
                    *v = toml::Value::String(version.to_string());
                }
            }
        }

        // Convert back to string and write
        let updated_content = toml::to_string(&cargo_toml)
            .map_err(|e| VersionError::Other(format!("Failed to serialize Cargo.toml: {e}")))?;

        fs::write(cargo_toml_path, updated_content)?;
        Ok(())
    }
}

//=============== Python Ecosystem Implementation ===============//

/// Python ecosystem (pyproject.toml or setup.py)
struct PythonEcosystem;

impl Ecosystem for PythonEcosystem {
    fn read_version(&self, dir_path: &Path) -> Result<SemverVersion> {
        // First try pyproject.toml
        let pyproject_toml_path = dir_path.join("pyproject.toml");
        if pyproject_toml_path.exists() {
            return Self::read_from_pyproject_toml(&pyproject_toml_path);
        }

        // Then try setup.py
        let setup_py_path = dir_path.join("setup.py");
        if setup_py_path.exists() {
            return Self::read_from_setup_py(&setup_py_path);
        }

        Err(VersionError::VersionNotFound)
    }

    fn write_version(&self, dir_path: &Path, version: &SemverVersion) -> Result<()> {
        // First try pyproject.toml
        let pyproject_toml_path = dir_path.join("pyproject.toml");
        if pyproject_toml_path.exists() {
            return Self::write_to_pyproject_toml(&pyproject_toml_path, version);
        }

        // Then try setup.py
        let setup_py_path = dir_path.join("setup.py");
        if setup_py_path.exists() {
            return Self::write_to_setup_py(&setup_py_path, version);
        }

        Err(VersionError::VersionNotFound)
    }
}

impl PythonEcosystem {
    fn read_from_pyproject_toml(path: &Path) -> Result<SemverVersion> {
        let content = fs::read_to_string(path)?;

        let pyproject: toml::Table = toml::from_str(&content).map_err(|e| {
            VersionError::ParseFileError(format!("Failed to parse pyproject.toml: {e}"))
        })?;

        Self::find_version_in_pyproject(&pyproject)
            .ok_or(VersionError::VersionNotFound)
            .and_then(|version_str| SemverVersion::parse(&version_str).map_err(Into::into))
    }

    fn find_version_in_pyproject(pyproject: &toml::Table) -> Option<String> {
        let pep621_version = pyproject
            .get("project")
            .and_then(|p| p.as_table())
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        pep621_version.or_else(|| {
            pyproject
                .get("tool")
                .and_then(|t| t.as_table())
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.as_table())
                .and_then(|p| p.get("version"))
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
    }

    fn write_to_pyproject_toml(path: &Path, version: &SemverVersion) -> Result<()> {
        let content = fs::read_to_string(path)?;

        let mut pyproject: toml::Table = toml::from_str(&content).map_err(|e| {
            VersionError::ParseFileError(format!("Failed to parse pyproject.toml: {e}"))
        })?;

        // Try to update version in different possible locations

        // Standard poetry/pep621 location
        let mut updated = false;
        if let Some(project) = pyproject.get_mut("project") {
            if let Some(project_table) = project.as_table_mut() {
                if project_table.contains_key("version") {
                    project_table.insert(
                        "version".to_string(),
                        toml::Value::String(version.to_string()),
                    );
                    updated = true;
                }
            }
        }

        // Legacy poetry location
        if !updated {
            if let Some(tool) = pyproject.get_mut("tool") {
                if let Some(tool_table) = tool.as_table_mut() {
                    if let Some(poetry) = tool_table.get_mut("poetry") {
                        if let Some(poetry_table) = poetry.as_table_mut() {
                            if poetry_table.contains_key("version") {
                                poetry_table.insert(
                                    "version".to_string(),
                                    toml::Value::String(version.to_string()),
                                );
                                updated = true;
                            }
                        }
                    }
                }
            }
        }

        if !updated {
            return Err(VersionError::VersionNotFound);
        }

        // Write back
        let updated_content = toml::to_string(&pyproject)
            .map_err(|e| VersionError::Other(format!("Failed to serialize pyproject.toml: {e}")))?;

        fs::write(path, updated_content)?;
        Ok(())
    }

    fn read_from_setup_py(path: &Path) -> Result<SemverVersion> {
        let content = fs::read_to_string(path)?;

        // This is a simple regex-based approach; a more robust solution might require
        // actually executing the Python code or using an AST parser
        let version_regex = regex::Regex::new(r#"version\s*=\s*['"]([0-9]+\.[0-9]+\.[0-9]+)['"]"#)
            .map_err(|_| VersionError::Other("Failed to compile regex".to_string()))?;

        version_regex
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .ok_or(VersionError::VersionNotFound)
            .and_then(|version_str| SemverVersion::parse(version_str).map_err(Into::into))
    }

    fn write_to_setup_py(path: &Path, version: &SemverVersion) -> Result<()> {
        let content = fs::read_to_string(path)?;

        // Replace version using regex
        let version_regex =
            regex::Regex::new(r#"(version\s*=\s*['"])([0-9]+\.[0-9]+\.[0-9]+)(['"])"#)
                .map_err(|_| VersionError::Other("Failed to compile regex".to_string()))?;

        let new_content = version_regex.replace_all(&content, |caps: &regex::Captures| {
            format!("{}{}{}", &caps[1], version, &caps[3])
        });

        // Check if replacement actually happened
        if new_content == content {
            return Err(VersionError::VersionNotFound);
        }

        fs::write(path, new_content.as_bytes())?;
        Ok(())
    }
}
