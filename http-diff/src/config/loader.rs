use crate::config::types::{HttpDiffConfig, UserData};
use crate::error::{HttpDiffError, Result};
use std::collections::HashMap;
use std::path::Path;

/// Configuration loader trait
pub trait ConfigLoader {
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<HttpDiffConfig>;
    fn load_with_validation<P: AsRef<Path>>(path: P) -> Result<HttpDiffConfig>;
}

/// Default configuration loader implementation
pub struct DefaultConfigLoader;

impl ConfigLoader for DefaultConfigLoader {
    /// Load configuration from http-diff.toml file
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<HttpDiffConfig> {
        let content = std::fs::read_to_string(&path)
            .map_err(|_| HttpDiffError::ConfigNotFound {
                path: path.as_ref().to_path_buf(),
            })?;
        
        let config: HttpDiffConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with enhanced error context
    fn load_with_validation<P: AsRef<Path>>(path: P) -> Result<HttpDiffConfig> {
        let path_ref = path.as_ref();
        
        // Check if file exists and provide helpful error message
        if !path_ref.exists() {
            return Err(HttpDiffError::ConfigNotFound {
                path: path_ref.to_path_buf(),
            });
        }

        let content = std::fs::read_to_string(path_ref)
            .map_err(HttpDiffError::Io)?;

        // Parse TOML with enhanced error context
        let config: HttpDiffConfig = toml::from_str(&content)
            .map_err(|e| {
                HttpDiffError::invalid_config(format!(
                    "Failed to parse TOML in {}: {}",
                    path_ref.display(),
                    e
                ))
            })?;

        Ok(config)
    }
}

/// Load user data from CSV file
pub fn load_user_data<P: AsRef<Path>>(path: P) -> Result<Vec<UserData>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    
    let mut users = Vec::new();
    for result in reader.records() {
        let record = result?;
        let mut data = HashMap::new();
        
        for (i, header) in headers.iter().enumerate() {
            if let Some(value) = record.get(i) {
                data.insert(header.to_string(), value.to_string());
            }
        }
        
        users.push(UserData { data });
    }
    
    Ok(users)
}

// Convenience functions maintaining the API
impl HttpDiffConfig {
    /// Create a new config builder
    pub fn builder() -> crate::config::builder::HttpDiffConfigBuilder {
        crate::config::builder::HttpDiffConfigBuilder::new()
    }

    /// Load configuration from http-diff.toml file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        DefaultConfigLoader::load_from_file(path)
    }

    /// Load configuration with enhanced error context
    pub fn load_with_validation<P: AsRef<Path>>(path: P) -> Result<Self> {
        DefaultConfigLoader::load_with_validation(path)
    }
}