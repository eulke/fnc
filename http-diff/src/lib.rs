//! HTTP Diff - Multi-environment HTTP request testing and comparison tool
//!
//! This crate provides functionality to execute HTTP requests across multiple
//! configurable environments and compare responses to identify differences.

// Core trait definitions
pub mod traits;

// Core modules - restructured for better organization
pub mod config;
pub mod error;
pub mod types;

// New structured modules
pub mod execution;
pub mod http;

// Business logic modules
pub mod analysis;
pub mod comparison;
pub mod conditions;
pub mod curl;

// Presentation modules
pub mod renderers;

// Shared utility modules
pub mod output_manager;
pub mod url_builder;
pub mod utils;

// Testing utilities
#[cfg(test)]
pub mod testing;

// Re-export core traits
pub use traits::{ConditionEvaluator, ConfigValidator, HttpClient, ProgressCallback, ResponseComparator, TestRunner};

// Re-export analysis types
pub use analysis::{ErrorAnalysis, ErrorAnalyzer, ErrorClassifierImpl, ErrorGroup, RouteError};

// Re-export main types
pub use config::{Environment, HttpDiffConfig, HttpDiffConfigBuilder, Route, UserData};
pub use conditions::{ConditionEvaluatorImpl, ConditionOperator, ConditionResult, ExecutionCondition};
pub use types::{
    ComparisonResult, DiffViewStyle, Difference, DifferenceCategory, ErrorSummary, ExecutionError,
    ExecutionErrorType, ExecutionResult, HttpResponse,
};

// Re-export implementations (clean API without "Impl" suffix)
pub use comparison::{ResponseComparator as DefaultResponseComparator, ResponseValidatorImpl};
pub use execution::{DefaultTestRunner, ProgressTracker, TestRunnerImpl};
pub use http::HttpClientImpl as DefaultHttpClient;

// Re-export renderers
pub use renderers::cli::{ComparisonFormatter, ErrorRenderer, TableBuilder, TableStyle};
pub use renderers::{CliRenderer, InteractiveRenderer, OutputRenderer, TuiRenderer};

// Re-export error types
pub use error::{HttpDiffError, Result};

// Re-export utility modules
pub use output_manager::{OutputCategory, OutputManager};
pub use url_builder::UrlBuilder;

/// Create a test runner with default implementations
pub fn create_default_test_runner(config: HttpDiffConfig) -> Result<DefaultTestRunner> {
    let client = DefaultHttpClient::new(config.clone())?;
    let comparator = DefaultResponseComparator::new();
    let condition_evaluator = ConditionEvaluatorImpl::new();
    DefaultTestRunner::new(config, client, comparator, condition_evaluator)
}

/// Execute HTTP diff testing with clean architecture - requires explicit user data
pub async fn run_http_diff_with_data(
    config: HttpDiffConfig,
    user_data: &[UserData],
    environments: Option<Vec<String>>,
    routes: Option<Vec<String>>,
    progress_callback: Option<ProgressCallback>,
) -> Result<ExecutionResult> {
    let runner = create_default_test_runner(config)?;
    runner
        .execute_with_data(user_data, environments, routes, progress_callback)
        .await
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
        // Test basic string line counting (now done directly)
        assert_eq!("line1\nline2".lines().count(), 2);

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

        let mut comparison_result =
            types::ComparisonResult::new("test_route".to_string(), HashMap::new());

        comparison_result.add_response("dev".to_string(), response);
        assert!(!comparison_result.has_errors);
        assert!(comparison_result.is_identical);
    }

    /// Test factory functions
    #[test]
    fn test_factory_functions() {
        let config = HttpDiffConfig::builder()
            .environment("test", "https://test.example.com", None)
            .get_route("health", "/health")
            .build()
            .unwrap();

        // Test creating HTTP client directly
        let client = DefaultHttpClient::new(config.clone());
        assert!(client.is_ok());

        // Test creating default test runner
        let runner = create_default_test_runner(config);
        assert!(runner.is_ok());
    }

    #[cfg(test)]
    mod mock_tests {
        use super::*;
        use crate::testing::mocks::test_helpers::*;
        use crate::testing::mocks::*;

        #[tokio::test]
        async fn test_mock_http_client() {
            let route = create_mock_route("test", "GET", "/test");
            let response = create_mock_response(200, "test body");

            let client =
                MockHttpClient::new().with_response("test:dev".to_string(), response.clone());

            let user_data = create_mock_user_data(vec![("userId", "123")]);

            let result = client.execute_request(&route, "dev", &user_data).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().body, "test body");
        }

        #[test]
        fn test_mock_response_comparator() {
            let comparator = MockResponseComparator::new();

            let responses = HashMap::new();
            let result =
                comparator.compare_responses("test".to_string(), HashMap::new(), responses);

            assert!(result.is_ok());
            assert!(result.unwrap().is_identical);
        }

        #[tokio::test]
        async fn test_mock_test_runner() {
            let runner = MockTestRunner::new();
            let user_data = vec![create_mock_user_data(vec![("userId", "123")])];
            let result = runner.execute_with_data(&user_data, None, None, None).await;
            assert!(result.is_ok());
            let execution_result = result.unwrap();
            assert!(!execution_result.has_errors());
        }
    }
}
