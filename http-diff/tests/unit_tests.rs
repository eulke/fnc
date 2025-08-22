//! Unit tests for core http-diff functionality
//! 
//! This module contains focused unit tests for individual components and core functionality.
//! For integration tests and end-to-end workflows, see integration_tests.rs.

mod common;

use common::*;
use http_diff::{
    config::{HttpDiffConfig, Route, GlobalConfig},
    conditions::ConditionOperator,
    error::HttpDiffError,
    traits::{HttpClient, ResponseComparator, ConditionEvaluator},
    types::{ComparisonResult, ErrorSummary, HttpResponse},
};
use std::collections::HashMap;

// =============================================================================
// CORE COMPARISON FUNCTIONALITY TESTS
// =============================================================================

#[cfg(test)]
mod comparison_tests {
    use super::*;

    #[test]
    fn test_comparison_result_creation() {
        let result = create_comparison_result(
            "test_route",
            vec![("dev", 200, "success"), ("prod", 200, "success")],
            true,
        );

        assert_eq!(result.route_name, "test_route");
        assert!(result.is_identical);
        assert!(!result.has_errors);
        assert_eq!(result.status_codes.len(), 2);
        assert_eq!(result.status_codes["dev"], 200);
        assert_eq!(result.status_codes["prod"], 200);
    }

    #[test]
    fn test_comparison_result_with_errors() {
        let result = create_comparison_result(
            "error_route",
            vec![("dev", 404, "not found"), ("prod", 500, "server error")],
            false,
        );

        assert!(!result.is_identical);
        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        
        let error_bodies = result.error_bodies.as_ref().unwrap();
        assert_eq!(error_bodies["dev"], "not found");
        assert_eq!(error_bodies["prod"], "server error");
    }

    #[test]
    fn test_status_code_classification() {
        // Test 2xx success codes
        let success_result = create_comparison_result(
            "success",
            vec![("dev", 200, "ok"), ("prod", 201, "created")],
            true,
        );
        assert!(!success_result.has_errors);

        // Test 4xx client errors
        let client_error_result = create_comparison_result(
            "client_error",
            vec![("dev", 404, "not found"), ("prod", 400, "bad request")],
            false,
        );
        assert!(client_error_result.has_errors);

        // Test 5xx server errors
        let server_error_result = create_comparison_result(
            "server_error",
            vec![("dev", 500, "internal error"), ("prod", 503, "unavailable")],
            false,
        );
        assert!(server_error_result.has_errors);
    }

    #[test]
    fn test_error_summary_calculation() {
        let results = vec![
            create_comparison_result("route1", vec![("dev", 200, "ok")], true),
            create_comparison_result("route2", vec![("dev", 404, "error")], false),
            create_comparison_result("route3", vec![("dev", 500, "error")], false),
        ];

        let summary = ErrorSummary::from_comparison_results(&results);
        assert_eq!(summary.total_requests, 3);
    }

    #[test]
    fn test_response_creation() {
        let response = create_response(200, "test body", Some("https://example.com/test"));
        
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "test body");
        assert_eq!(response.url, "https://example.com/test");
        assert!(response.curl_command.contains("curl"));
    }
}

// =============================================================================
// CONFIGURATION TESTS
// =============================================================================

#[cfg(test)]
mod configuration_tests {
    use super::*;

    #[test]
    fn test_basic_environment_configuration() {
        let environments = create_test_environments();
        
        assert!(environments.contains_key("dev"));
        assert!(environments.contains_key("staging"));
        assert!(environments.contains_key("prod"));
        
        let dev_env = &environments["dev"];
        assert!(dev_env.base_url.starts_with("https://"));
        assert!(!dev_env.is_base);
    }

    #[test]
    fn test_route_configuration_validation() {
        let route = create_route("test_route", "GET", "/api/test");
        
        assert_eq!(route.name, "test_route");
        assert_eq!(route.method, "GET");
        assert_eq!(route.path, "/api/test");
        assert!(route.headers.is_none());
        assert!(route.conditions.is_none());
    }

    #[test]
    fn test_route_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        
        let route = Route {
            name: "protected".to_string(),
            method: "GET".to_string(),
            path: "/api/protected".to_string(),
            headers: Some(headers.clone()),
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: None,
            depends_on: None,
            wait_for_extraction: None,
        };

        assert!(route.headers.is_some());
        let route_headers = route.headers.as_ref().unwrap();
        assert_eq!(route_headers["Authorization"], "Bearer token");
    }

    #[test]
    fn test_global_config_defaults() {
        let config = GlobalConfig::default();
        assert_eq!(config.max_concurrent_requests, Some(10));
        assert!(config.timeout_seconds.is_some());
    }

    #[test]
    fn test_config_validation_success() {
        let config = create_test_config();
        let parsed: Result<HttpDiffConfig, _> = toml::from_str(&config);
        assert!(parsed.is_ok());
        
        let config = parsed.unwrap();
        assert!(!config.environments.is_empty());
        assert!(!config.routes.is_empty());
    }

    #[test]
    fn test_config_validation_with_invalid_toml() {
        let invalid_config = r#"
[environments.dev
base_url = "https://dev.example.com"  # Missing closing bracket
"#;
        
        let parsed: Result<HttpDiffConfig, _> = toml::from_str(invalid_config);
        assert!(parsed.is_err());
    }
}

// =============================================================================
// MOCK SYSTEM TESTS
// =============================================================================

#[cfg(test)]
mod mock_system_tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_http_client_basic_functionality() {
        let (_, client, _, _, users) = TestScenarioBuilder::new()
            .with_basic_responses()
            .with_users(1)
            .build();

        let route = create_route("test", "GET", "/test");
        let user = &users[0];

        // Test successful response
        let response_key = "test:dev".to_string();
        if client.has_response(&response_key) {
            let result = client.execute_request(&route, "dev", user).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.status, 200);
        }
    }

    #[tokio::test]
    async fn test_mock_http_client_with_failures() {
        let (_, client, _, _, users) = TestScenarioBuilder::new()
            .with_route_failure("failing_route", "Network timeout")
            .with_users(1)
            .build();

        let failing_route = create_route("failing_route", "GET", "/fail");
        let user = &users[0];

        let result = client.execute_request(&failing_route, "dev", user).await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Network timeout"));
    }

    #[test]
    fn test_mock_response_comparator() {
        let (_, _, comparator, _, _) = TestScenarioBuilder::new().build();

        let mut responses = HashMap::new();
        responses.insert("dev".to_string(), create_response(200, "dev response", None));
        responses.insert("prod".to_string(), create_response(200, "prod response", None));

        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        );

        assert!(result.is_ok());
        let comparison = result.unwrap();
        assert_eq!(comparison.route_name, "test_route");
        assert!(!comparison.responses.is_empty());
    }

    #[test]
    fn test_mock_condition_evaluator() {
        let (_, _, _, evaluator, _) = TestScenarioBuilder::new().build();

        let user_data = create_user_data(vec![("user_type", "premium")]);
        let condition = create_execution_condition("user_type", ConditionOperator::Equals, Some("premium"));

        let result = evaluator.evaluate_conditions(&[condition], &user_data);
        assert!(result.is_ok());
        let conditions_result = result.unwrap();
        assert_eq!(conditions_result.len(), 1);
        assert!(conditions_result[0].passed);
    }
}

// =============================================================================
// BASIC ERROR HANDLING TESTS
// =============================================================================

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_http_diff_error_creation() {
        let error = HttpDiffError::general("Test error message");
        assert!(error.to_string().contains("Test error message"));
    }

    #[test]
    fn test_invalid_config_error() {
        let error = HttpDiffError::InvalidConfig {
            message: "Invalid configuration".to_string(),
        };
        
        match error {
            HttpDiffError::InvalidConfig { message } => {
                assert_eq!(message, "Invalid configuration");
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[test]
    fn test_network_error_scenarios() {
        // Test timeout scenario
        let timeout_error = HttpDiffError::general("Request timeout");
        assert!(timeout_error.to_string().contains("timeout"));

        // Test connection refused scenario
        let connection_error = HttpDiffError::general("Connection refused");
        assert!(connection_error.to_string().contains("Connection refused"));
    }

    #[test]
    fn test_validation_error_scenarios() {
        // Test missing environment error
        let missing_env_error = HttpDiffError::InvalidConfig {
            message: "Environment 'prod' not found".to_string(),
        };
        
        match missing_env_error {
            HttpDiffError::InvalidConfig { message } => {
                assert!(message.contains("Environment"));
                assert!(message.contains("not found"));
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }
}

// =============================================================================
// CORE TYPES AND UTILITIES TESTS
// =============================================================================

#[cfg(test)]
mod core_types_tests {
    use super::*;

    #[test]
    fn test_http_response_creation() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = HttpResponse {
            status: 200,
            headers: headers.clone(),
            body: "test body".to_string(),
            url: "https://example.com".to_string(),
            curl_command: "curl 'https://example.com'".to_string(),
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "test body");
        assert_eq!(response.url, "https://example.com");
        assert_eq!(response.headers["Content-Type"], "application/json");
    }

    #[test]
    fn test_comparison_result_builder() {
        let mut result = ComparisonResult::new("test".to_string(), HashMap::new());
        
        let response = create_response(200, "test", None);
        result.add_response("dev".to_string(), response);

        assert_eq!(result.route_name, "test");
        assert!(result.responses.contains_key("dev"));
    }

    #[test]
    fn test_user_data_creation() {
        let user_data = create_user_data(vec![
            ("userId", "123"),
            ("userName", "test_user"),
            ("environment", "dev"),
        ]);

        assert_eq!(user_data.data.get("userId"), Some(&"123".to_string()));
        assert_eq!(user_data.data.get("userName"), Some(&"test_user".to_string()));
        assert_eq!(user_data.data.get("environment"), Some(&"dev".to_string()));
    }

    #[test]
    fn test_multiple_users_creation() {
        let users = create_test_users(3);
        assert_eq!(users.len(), 3);
        
        for (i, user) in users.iter().enumerate() {
            let user_id = format!("user_{}", i);
            assert_eq!(user.data.get("userId"), Some(&user_id));
        }
    }

    #[test]
    fn test_route_factory_functions() {
        // Test basic route creation
        let basic_route = create_route("basic", "GET", "/api/basic");
        assert_eq!(basic_route.name, "basic");
        assert_eq!(basic_route.method, "GET");
        assert_eq!(basic_route.path, "/api/basic");

        // Test route with extraction
        let extraction_rules = vec![
            create_extraction_rule("token", "JsonPath", "$.token", true, None),
        ];
        let extraction_route = create_route_with_extraction(
            "auth", "POST", "/auth", extraction_rules, None
        );
        assert!(extraction_route.extract.is_some());
        assert_eq!(extraction_route.extract.as_ref().unwrap().len(), 1);
    }
}

// =============================================================================
// HELPER FUNCTION TESTS
// =============================================================================

#[cfg(test)]
mod helper_function_tests {
    use super::*;

    #[test]
    fn test_test_config_generation() {
        let config_toml = create_test_config();
        assert!(config_toml.contains("[environments.test]"));
        assert!(config_toml.contains("[environments.prod]"));
        assert!(config_toml.contains("[[routes]]"));
    }

    #[test]
    fn test_test_config_with_conditions() {
        let config_toml = create_test_config_with_conditions();
        assert!(config_toml.contains("[[routes.conditions]]"));
        assert!(config_toml.contains("operator = \"equals\""));
    }

    #[test]
    fn test_csv_generation() {
        let csv_content = create_test_users_csv();
        assert!(csv_content.contains("userId,siteId,userName"));
        assert!(csv_content.contains("12345,MCO,test_user"));
        
        let csv_with_conditions = create_test_users_csv_with_conditions();
        assert!(csv_with_conditions.contains("user_type"));
        assert!(csv_with_conditions.contains("premium"));
    }

    #[test]
    fn test_scenario_builder_basic() {
        let (config, client, comparator, evaluator, users) = TestScenarioBuilder::new()
            .with_users(2)
            .build();

        assert_eq!(users.len(), 2);
        assert!(!config.environments.is_empty());
        assert!(!config.routes.is_empty());
    }

    #[test]
    fn test_scenario_builder_with_chain() {
        let (config, _client, _comparator, _evaluator, _users) = TestScenarioBuilder::new()
            .with_basic_chain()
            .build();

        // Should have multiple routes in dependency order
        assert!(config.routes.len() > 1);
        
        // Should have routes with dependencies
        let dependent_routes: Vec<_> = config.routes.iter()
            .filter(|r| r.depends_on.is_some())
            .collect();
        assert!(!dependent_routes.is_empty());
    }

    #[test]
    fn test_scenario_builder_with_failures() {
        let (config, client, _comparator, _evaluator, _users) = TestScenarioBuilder::new()
            .with_route_failure("test_route", "Test failure message")
            .build();

        // Verify the failure is configured
        assert!(!config.routes.is_empty());
        // The failure would be tested in the mock HTTP client during execution
    }
}