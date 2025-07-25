//! HTTP Diff - Multi-environment HTTP request testing and comparison tool
//! 
//! This crate provides functionality to execute HTTP requests across multiple
//! configurable environments and compare responses to identify differences.

pub mod config;
pub mod client;
pub mod runner;
pub mod comparator;
pub mod output;
pub mod error;

// Re-export main types for convenience
pub use config::{HttpDiffConfig, Environment, Route, UserData};
pub use client::HttpClient;
pub use runner::TestRunner;
pub use comparator::{ResponseComparator, ComparisonResult, DiffViewStyle};
pub use output::CurlGenerator;
pub use error::{HttpDiffError, Result};

/// Execute HTTP diff testing with the given configuration
pub async fn run_http_diff(
    config: HttpDiffConfig,
    environments: Option<Vec<String>>,
) -> Result<Vec<ComparisonResult>> {
    let runner = TestRunner::new(config)?;
    runner.execute(environments).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Test that all modules can be imported and basic types work
    #[test]
    fn test_module_imports() {
        // Test that we can create basic configuration structures
        let environments = HashMap::new();
        let routes = Vec::new();
        
        let config = HttpDiffConfig {
            environments,
            global: None,
            routes,
        };

        // Test that validation works (should fail with empty config)
        assert!(config.validate().is_err());
    }

    /// Test that error types work correctly
    #[test]
    fn test_error_types() {
        let error = HttpDiffError::invalid_config("test error");
        assert!(error.to_string().contains("Invalid configuration"));
        
        let error = HttpDiffError::NoEnvironments;
        assert!(error.to_string().contains("No environments"));
    }

    /// Test that UserData can be created and used
    #[test]
    fn test_user_data() {
        let mut data = HashMap::new();
        data.insert("userId".to_string(), "123".to_string());
        data.insert("siteId".to_string(), "MCO".to_string());
        
        let user_data = UserData { data };
        
        assert_eq!(user_data.data.get("userId"), Some(&"123".to_string()));
        assert_eq!(user_data.data.get("siteId"), Some(&"MCO".to_string()));
    }
} 