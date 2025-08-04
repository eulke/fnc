//! HTTP Diff - Multi-environment HTTP request testing and comparison tool
//! 
//! This crate provides functionality to execute HTTP requests across multiple
//! configurable environments and compare responses to identify differences.

// Core modules
pub mod config;
pub mod error;
pub mod types;

// Shared utility modules
pub mod url_builder;
pub mod formatter;

// Main functionality modules
pub mod client;
pub mod runner;
pub mod comparison;
pub mod curl;
pub mod documentation;
pub mod error_analysis;
pub mod renderers;

// Re-export main types for convenience
pub use config::{HttpDiffConfig, HttpDiffConfigBuilder, Environment, Route, UserData};
pub use types::{
    HttpResponse, ComparisonResult, Difference, DifferenceCategory, 
    DiffViewStyle, ErrorSummary, ProgressConfig, ProgressInfo
};
pub use client::HttpClient;
pub use runner::{TestRunner, run_http_diff_concurrent};
pub use comparison::ResponseComparator;
pub use curl::{CurlGenerator, CurlCommand};
pub use documentation::generate_request_documentation;
pub use renderers::{OutputRenderer, CliRenderer, JsonRenderer, HtmlRenderer, ComparisonFormatter, TableBuilder, TableStyle};
pub use error::{HttpDiffError, Result};

// Re-export utility modules for advanced usage
pub use url_builder::UrlBuilder;
pub use formatter::{TextFormatter, FormatterConfig, DiffStyle};

/// Execute HTTP diff testing with the given configuration
pub async fn run_http_diff(
    config: HttpDiffConfig,
    environments: Option<Vec<String>>,
    routes: Option<Vec<String>>,
) -> Result<Vec<ComparisonResult>> {
    let runner = TestRunner::new(config)?;
    runner.execute(environments, routes).await
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

    /// Test that shared utilities work
    #[test]
    fn test_shared_utilities() {
        // Test formatter utilities
        use formatter::{shell, text};
        
        assert_eq!(shell::escape_argument("simple"), "simple");
        assert_eq!(text::line_count("line1\nline2"), 2);
        
        // Test table builder
        let mut builder = TableBuilder::new();
        builder.headers(vec!["Name", "Value"]);
        builder.row(vec!["test", "123"]);
        let table = builder.build();
        
        assert!(!table.is_empty());
        
        // Test URL builder utilities
        use url_builder::utils;
        
        assert!(utils::has_path_parameters("/api/users/{userId}"));
        assert!(!utils::has_path_parameters("/api/users"));
    }

    /// Test type construction and methods
    #[test] 
    fn test_types() {
        let response = types::HttpResponse::new(
            200,
            HashMap::new(),
            "test body".to_string(),
            "https://api.example.com/test".to_string(),
            "curl command".to_string(),
        );

        assert!(response.is_success());
        assert!(!response.is_error());

        let mut comparison_result = types::ComparisonResult::new(
            "test_route".to_string(),
            HashMap::new(),
        );

        comparison_result.add_response("dev".to_string(), response);
        assert!(!comparison_result.has_errors);
        assert!(comparison_result.is_identical);
    }
} 