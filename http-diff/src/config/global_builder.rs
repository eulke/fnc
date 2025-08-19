//! Global configuration builder for fluent API construction
//!
//! This module provides a dedicated builder for global HTTP configurations
//! that can be accessed through the main configuration builder.

use crate::config::types::GlobalConfig;
use std::collections::HashMap;

/// Builder for global HTTP configuration with fluent API
#[derive(Debug, Default)]
pub struct GlobalConfigBuilder {
    config: GlobalConfig,
}

impl GlobalConfigBuilder {
    /// Create a new global config builder
    pub fn new() -> Self {
        Self {
            config: GlobalConfig::default(),
        }
    }

    /// Create a builder from existing global config
    pub fn from_config(config: GlobalConfig) -> Self {
        Self { config }
    }

    /// Set timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = Some(seconds);
        self
    }

    /// Set whether to follow redirects
    pub fn follow_redirects(mut self, follow: bool) -> Self {
        self.config.follow_redirects = Some(follow);
        self
    }

    /// Set maximum number of concurrent requests
    pub fn max_concurrent_requests(mut self, max_concurrent: usize) -> Self {
        self.config.max_concurrent_requests = Some(max_concurrent);
        self
    }

    /// Set global headers (replaces any existing headers)
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.config.headers = Some(headers);
        self
    }

    /// Add a single global header
    pub fn header<S: Into<String>>(mut self, key: S, value: S) -> Self {
        if self.config.headers.is_none() {
            self.config.headers = Some(HashMap::new());
        }
        self.config
            .headers
            .as_mut()
            .unwrap()
            .insert(key.into(), value.into());
        self
    }

    /// Build the global configuration
    pub fn build(self) -> GlobalConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_builder() {
        let config = GlobalConfigBuilder::new()
            .timeout(30)
            .follow_redirects(true)
            .max_concurrent_requests(5)
            .header("Authorization", "Bearer token")
            .header("Accept", "application/json")
            .build();

        assert_eq!(config.timeout_seconds, Some(30));
        assert_eq!(config.follow_redirects, Some(true));
        assert_eq!(config.max_concurrent_requests, Some(5));

        let headers = config.headers.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&"Bearer token".to_string())
        );
        assert_eq!(headers.get("Accept"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_global_config_builder_from_existing() {
        let existing = GlobalConfig {
            timeout_seconds: Some(60),
            follow_redirects: Some(false),
            max_concurrent_requests: Some(20),
            headers: None,
            params: None,
        };

        let config = GlobalConfigBuilder::from_config(existing)
            .timeout(30) // override
            .max_concurrent_requests(15) // override
            .header("X-Custom", "value")
            .build();

        assert_eq!(config.timeout_seconds, Some(30));
        assert_eq!(config.follow_redirects, Some(false));
        assert_eq!(config.max_concurrent_requests, Some(15));

        let headers = config.headers.unwrap();
        assert_eq!(headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_global_config_builder_headers_replacement() {
        let mut initial_headers = HashMap::new();
        initial_headers.insert("Initial".to_string(), "Value".to_string());

        let mut replacement_headers = HashMap::new();
        replacement_headers.insert("Replacement".to_string(), "Value".to_string());

        let config = GlobalConfigBuilder::new()
            .headers(initial_headers)
            .headers(replacement_headers) // Should replace, not merge
            .build();

        let headers = config.headers.unwrap();
        assert!(!headers.contains_key("Initial"));
        assert_eq!(headers.get("Replacement"), Some(&"Value".to_string()));
    }
}
