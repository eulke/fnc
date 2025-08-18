use crate::config::types::HttpDiffConfig;
use crate::error::{HttpDiffError, Result};
use crate::traits::ConfigValidator;
use std::path::Path;

/// Configuration validator implementation
pub struct ConfigValidatorImpl;

impl ConfigValidator for ConfigValidatorImpl {
    type Config = HttpDiffConfig;

    /// Validate configuration (uses enhanced validation with default context)
    fn validate(&self, config: &HttpDiffConfig) -> Result<()> {
        self.validate_with_context(config, "configuration")
    }
}

impl ConfigValidatorImpl {
    /// Create a new validator
    pub fn new() -> Self {
        Self
    }

    /// Validation with enhanced error context
    pub fn validate_with_context<P: AsRef<Path>>(
        &self,
        config: &HttpDiffConfig,
        config_path: P,
    ) -> Result<()> {
        let config_path_str = config_path.as_ref().to_string_lossy();

        if config.environments.is_empty() {
            return Err(HttpDiffError::invalid_config(format!(
                "No environments configured in {}. Add at least one environment to [environments] section.",
                config_path_str
            )));
        }

        if config.routes.is_empty() {
            return Err(HttpDiffError::invalid_config(format!(
                "No routes configured in {}. Add at least one [[routes]] entry.",
                config_path_str
            )));
        }

        // Ensure no more than one base environment is selected
        let base_count = config.environments.values().filter(|e| e.is_base).count();
        if base_count > 1 {
            return Err(HttpDiffError::invalid_config(format!(
                "Multiple environments are marked as base in {}; only one is allowed",
                config_path_str
            )));
        }

        // Validate HTTP methods and environment references
        for route in &config.routes {
            let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
            if !valid_methods.contains(&route.method.as_str()) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Invalid HTTP method '{}' in route '{}'. Valid methods: {}",
                    route.method,
                    route.name,
                    valid_methods.join(", ")
                )));
            }

            // Validate base URL overrides reference existing environments
            if let Some(base_urls) = &route.base_urls {
                for env_name in base_urls.keys() {
                    if !config.environments.contains_key(env_name) {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Route '{}' references unknown environment '{}' in base_urls. Available environments: {}",
                            route.name,
                            env_name,
                            config.environments.keys().map(String::as_str).collect::<Vec<_>>().join(", ")
                        )));
                    }
                }
            }
        }

        // Validate URLs in environments
        for (env_name, env) in &config.environments {
            if url::Url::parse(&env.base_url).is_err() {
                return Err(HttpDiffError::invalid_config(format!(
                    "Invalid base_url '{}' in environment '{}'. Must be a valid URL.",
                    env.base_url, env_name
                )));
            }
        }

        // Validate timeout if specified
        if let Some(global) = &config.global {
            if let Some(timeout) = global.timeout_seconds {
                if timeout == 0 || timeout > 300 {
                    return Err(HttpDiffError::invalid_config(
                        "timeout_seconds must be between 1 and 300 seconds".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

impl Default for ConfigValidatorImpl {
    fn default() -> Self {
        Self::new()
    }
}

// Add convenience methods to HttpDiffConfig
impl HttpDiffConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        let validator = ConfigValidatorImpl::new();
        validator.validate(self)
    }
}
