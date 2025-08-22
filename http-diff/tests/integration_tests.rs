//! Integration tests for http-diff crate
//!
//! This module contains integration tests that focus on end-to-end workflows,
//! component integration, and full system testing scenarios.
//! For unit tests of individual components, see unit_tests.rs.

mod common;

use common::*;
use http_diff::{
    config::HttpDiffConfig,
    traits::{HttpClient, ResponseComparator, ConditionEvaluator},
    ErrorSummary,
};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// =============================================================================
// CONFIGURATION INTEGRATION TESTS
// =============================================================================

#[cfg(test)]
mod config_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_config_file_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("http-diff.toml");
        let users_path = temp_dir.path().join("users.csv");

        // Create complete configuration files
        fs::write(&config_path, create_test_config()).unwrap();
        fs::write(&users_path, create_test_users_csv()).unwrap();

        // Verify files exist and can be read
        assert!(config_path.exists());
        assert!(users_path.exists());
        
        // Test full config parsing workflow
        let config_content = fs::read_to_string(&config_path).unwrap();
        let config: HttpDiffConfig = toml::from_str(&config_content).unwrap();
        
        // Verify config structure is complete
        assert!(!config.environments.is_empty());
        assert!(!config.routes.is_empty());
        assert!(config.environments.contains_key("test"));
        assert!(config.environments.contains_key("prod"));
        
        // Test CSV parsing integration
        let csv_content = fs::read_to_string(&users_path).unwrap();
        assert!(csv_content.contains("userId,siteId,userName"));
        assert!(csv_content.contains("12345,MCO,test_user"));
    }

    #[test]
    fn test_conditional_config_parsing_integration() {
        // Test complete conditional configuration workflow
        let config_toml = create_test_config_with_conditions();
        let config: Result<HttpDiffConfig, _> = toml::from_str(&config_toml);
        
        assert!(config.is_ok(), "Configuration with conditions should parse successfully");
        
        let config = config.unwrap();
        assert_eq!(config.routes.len(), 3);
        
        // Verify premium-api route conditions integration
        let premium_route = config.routes.iter()
            .find(|r| r.name == "premium-api")
            .expect("Premium route should exist");
        
        assert!(premium_route.conditions.is_some());
        let conditions = premium_route.conditions.as_ref().unwrap();
        assert_eq!(conditions.len(), 2);
        
        // Verify integrated condition parsing
        assert_eq!(conditions[0].variable, "user_type");
        assert_eq!(conditions[0].value.as_ref().unwrap(), "premium");
        assert_eq!(conditions[1].variable, "user_id");
        assert_eq!(conditions[1].value.as_ref().unwrap(), "1000");
        
        // Test environment variable condition integration
        let debug_route = config.routes.iter()
            .find(|r| r.name == "debug-endpoint")
            .expect("Debug route should exist");
        
        assert!(debug_route.conditions.is_some());
        let debug_conditions = debug_route.conditions.as_ref().unwrap();
        assert_eq!(debug_conditions.len(), 1);
        assert_eq!(debug_conditions[0].variable, "env.DEBUG_MODE");
        
        // Verify unconditional route integration
        let health_route = config.routes.iter()
            .find(|r| r.name == "health")
            .expect("Health route should exist");
        
        assert!(health_route.conditions.is_none());
    }
    
    #[test]
    fn test_multi_environment_config_integration() {
        let config_toml = r#"
[environments.dev]
base_url = "https://dev.api.example.com"

[environments.staging]
base_url = "https://staging.api.example.com"

[environments.prod]
base_url = "https://api.example.com"
is_base = true

[[routes]]
name = "multi_env_test"
method = "GET"
path = "/api/status"
"#;
        
        let config: HttpDiffConfig = toml::from_str(config_toml).unwrap();
        
        // Verify all environments are configured
        assert_eq!(config.environments.len(), 3);
        assert!(config.environments["dev"].base_url.contains("dev"));
        assert!(config.environments["staging"].base_url.contains("staging"));
        assert!(config.environments["prod"].is_base);
        
        // Verify route applies to all environments
        assert_eq!(config.routes.len(), 1);
        let route = &config.routes[0];
        assert_eq!(route.name, "multi_env_test");
        assert_eq!(route.path, "/api/status");
    }
}

// =============================================================================
// END-TO-END WORKFLOW TESTS
// =============================================================================

#[cfg(test)]
mod workflow_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_http_request_response_workflow() {
        let (config, client, comparator, evaluator, users) = TestScenarioBuilder::new()
            .with_basic_responses()
            .with_users(2)
            .build();

        // Test complete request-response workflow
        for route in &config.routes {
            for user in &users {
                for env in config.environments.keys() {
                    // Skip if conditions don't match (simulated)
                    if let Some(_conditions) = &route.conditions {
                        let should_execute = evaluator.should_execute_route(route, user).unwrap();
                        if !should_execute {
                            continue;
                        }
                    }
                    
                    // Execute request
                    let response_key = format!("{}:{}", route.name, env);
                    if client.has_response(&response_key) {
                        let result = client.execute_request(route, env, user).await;
                        assert!(result.is_ok(), "Request should succeed for route {} in {}", route.name, env);
                        
                        let response = result.unwrap();
                        assert!(!response.curl_command.is_empty());
                        assert!(!response.url.is_empty());
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_multi_environment_comparison_workflow() {
        let (config, client, comparator, _evaluator, users) = TestScenarioBuilder::new()
            .with_basic_responses()
            .with_users(1)
            .build();

        let user = &users[0];
        
        // Test complete multi-environment workflow
        for route in &config.routes {
            let mut responses = HashMap::new();
            
            // Collect responses from all environments
            for env in config.environments.keys() {
                let response_key = format!("{}:{}", route.name, env);
                if client.has_response(&response_key) {
                    let response = client.execute_request(route, env, user).await.unwrap();
                    responses.insert(env.clone(), response);
                }
            }
            
            if !responses.is_empty() {
                // Compare responses across environments
                let comparison_result = comparator.compare_responses(
                    route.name.clone(),
                    HashMap::new(),
                    responses,
                ).unwrap();
                
                assert_eq!(comparison_result.route_name, route.name);
                assert!(!comparison_result.responses.is_empty());
            }
        }
    }

    #[test]
    fn test_error_handling_workflow_integration() {
        let mixed_results = vec![
            // Success scenario
            create_comparison_result(
                "health_check",
                vec![("dev", 200, "healthy"), ("prod", 200, "healthy")],
                true,
            ),
            // Client error scenario
            create_comparison_result(
                "user_endpoint",
                vec![("dev", 404, "user not found"), ("prod", 200, "user data")],
                false,
            ),
            // Server error scenario
            create_comparison_result(
                "payment_service",
                vec![("dev", 500, "database error"), ("prod", 503, "service unavailable")],
                false,
            ),
            // Mixed scenario
            create_comparison_result(
                "order_api",
                vec![("dev", 200, "orders"), ("prod", 404, "orders not found")],
                false,
            ),
        ];

        // Test comprehensive error analysis workflow
        let summary = ErrorSummary::from_comparison_results(&mixed_results);
        assert_eq!(summary.total_requests, 4);
        
        // Verify error categorization workflow
        let mut success_count = 0;
        let mut error_count = 0;
        
        for result in &mixed_results {
            if result.has_errors {
                error_count += 1;
                assert!(result.error_bodies.is_some());
            } else {
                success_count += 1;
                assert!(result.error_bodies.is_none());
            }
        }
        
        assert_eq!(success_count, 1);
        assert_eq!(error_count, 3);
    }

    #[test]
    fn test_end_to_end_conditional_workflow() {
        let (config, _client, _comparator, evaluator, _users) = TestScenarioBuilder::new()
            .with_conditional_chain()
            .with_condition_result("premium-api", true)
            .with_condition_result("debug-endpoint", false)
            .build();
        
        // Create test users with different characteristics
        let premium_user = create_user_data(vec![
            ("user_type", "premium"),
            ("user_id", "1500"),
            ("userName", "premium_user"),
        ]);
        
        let basic_user = create_user_data(vec![
            ("user_type", "basic"),
            ("user_id", "500"),
            ("userName", "basic_user"),
        ]);
        
        // Test conditional execution workflow
        for route in &config.routes {
            match route.name.as_str() {
                "health" => {
                    // Health route should execute for all users
                    assert!(evaluator.should_execute_route(route, &premium_user).unwrap());
                    assert!(evaluator.should_execute_route(route, &basic_user).unwrap());
                }
                "premium-api" => {
                    // Premium API should be controlled by condition evaluator mock
                    let should_execute = evaluator.should_execute_route(route, &premium_user).unwrap();
                    // Mock is configured to return true for premium-api
                    assert!(should_execute);
                }
                "debug-endpoint" => {
                    // Debug endpoint should be controlled by condition evaluator mock
                    let should_execute = evaluator.should_execute_route(route, &premium_user).unwrap();
                    // Mock is configured to return false for debug-endpoint
                    assert!(!should_execute);
                }
                _ => {}
            }
        }
    }
}

// =============================================================================
// CHAIN EXECUTION INTEGRATION TESTS
// =============================================================================

#[cfg(test)]
mod chain_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_chain_execution_workflow() {
        let (config, client, _comparator, _evaluator, users) = TestScenarioBuilder::new()
            .with_basic_chain()
            .with_users(1)
            .build();
            
        let user = &users[0];
        
        // Test complete chain execution workflow
        // 1. Login route (no dependencies)
        let login_route = config.routes.iter().find(|r| r.name == "login").unwrap();
        assert!(login_route.depends_on.is_none());
        
        // 2. User list route (depends on login)
        let user_list_route = config.routes.iter().find(|r| r.name == "user_list").unwrap();
        assert!(user_list_route.depends_on.is_some());
        assert!(user_list_route.depends_on.as_ref().unwrap().contains(&"login".to_string()));
        
        // 3. User detail route (depends on user_list)
        let user_detail_route = config.routes.iter().find(|r| r.name == "user_detail").unwrap();
        assert!(user_detail_route.depends_on.is_some());
        assert!(user_detail_route.depends_on.as_ref().unwrap().contains(&"user_list".to_string()));
        
        // Test extraction workflow integration
        assert!(login_route.extract.is_some());
        assert!(user_list_route.extract.is_some());
        assert!(user_detail_route.extract.is_some());
        
        // Verify extraction rules
        let login_extractions = login_route.extract.as_ref().unwrap();
        assert!(login_extractions.iter().any(|e| e.name == "auth_token"));
        assert!(login_extractions.iter().any(|e| e.name == "logged_user_id"));
    }

    #[tokio::test]
    async fn test_extraction_and_dependency_workflow() {
        let (config, client, _comparator, _evaluator, users) = TestScenarioBuilder::new()
            .with_basic_chain()
            .build();
            
        let user = &users[0];
        
        // Test extraction workflow with mocked responses
        for route in &config.routes {
            if let Some(extractions) = &route.extract {
                for env in config.environments.keys() {
                    let response_key = format!("{}:{}", route.name, env);
                    if client.has_response(&response_key) {
                        let response = client.execute_request(route, env, user).await.unwrap();
                        
                        // Test extraction process integration
                        let extracted = client.extract_values(route, &response).unwrap();
                        
                        // Verify extractions based on route type
                        match route.name.as_str() {
                            "login" => {
                                assert!(extracted.contains_key("auth_token") || 
                                       extractions.iter().find(|e| e.name == "auth_token").map_or(true, |e| !e.required));
                            }
                            "user_list" => {
                                assert!(extracted.contains_key("first_user_id") ||
                                       extractions.iter().find(|e| e.name == "first_user_id").map_or(true, |e| !e.required));
                            }
                            "user_detail" => {
                                // User detail may extract email and department
                                // These are non-required, so extraction may succeed or use defaults
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_complex_dependency_workflow() {
        let (config, _client, _comparator, _evaluator, _users) = TestScenarioBuilder::new()
            .with_complex_chain()
            .build();
        
        // Test complex multi-level dependency structure
        let route_deps: HashMap<String, Vec<String>> = config.routes.iter()
            .filter_map(|r| {
                r.depends_on.as_ref().map(|deps| (r.name.clone(), deps.clone()))
            })
            .collect();
        
        // Verify dependency structure
        if let Some(org_deps) = route_deps.get("organization") {
            assert!(org_deps.contains(&"auth".to_string()));
        }
        
        if let Some(project_deps) = route_deps.get("projects") {
            assert!(project_deps.contains(&"organization".to_string()));
        }
        
        if let Some(team_deps) = route_deps.get("teams") {
            assert!(team_deps.contains(&"organization".to_string()));
        }
        
        if let Some(detail_deps) = route_deps.get("project_details") {
            assert!(detail_deps.contains(&"projects".to_string()));
            assert!(detail_deps.contains(&"teams".to_string()));
        }
    }
}

// =============================================================================
// ERROR SCENARIO INTEGRATION TESTS
// =============================================================================

#[cfg(test)]
mod error_scenario_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_network_failure_handling_workflow() {
        let (config, client, _comparator, _evaluator, users) = TestScenarioBuilder::new()
            .with_route_failure("failing_route", "Network connection timeout")
            .build();
            
        let user = &users[0];
        
        // Create a route that should fail
        let failing_route = create_route("failing_route", "GET", "/api/fail");
        
        // Test network failure workflow
        for env in config.environments.keys() {
            let result = client.execute_request(&failing_route, env, user).await;
            assert!(result.is_err());
            
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Network connection timeout"));
        }
    }

    #[tokio::test]
    async fn test_extraction_failure_workflow() {
        let (config, client, _comparator, _evaluator, users) = TestScenarioBuilder::new()
            .with_extraction_failure("extraction_test", vec!["required_field", "optional_field"])
            .build();
            
        let user = &users[0];
        
        // Create route with required and optional extractions
        let extraction_route = create_route_with_extraction(
            "extraction_test",
            "GET",
            "/api/extract",
            vec![
                create_extraction_rule("required_field", "JsonPath", "$.required", true, None),
                create_extraction_rule("optional_field", "JsonPath", "$.optional", false, Some("default".to_string())),
                create_extraction_rule("working_field", "JsonPath", "$.working", false, None),
            ],
            None,
        );
        
        // Test extraction failure workflow
        for env in config.environments.keys() {
            let response_key = format!("{}:{}", extraction_route.name, env);
            
            // For this test, we'll create a mock response
            let mock_response = create_response(200, r#"{"working": "value"}"#, None);
            
            // Test extraction failures
            let extraction_result = client.extract_values(&extraction_route, &mock_response);
            
            // Should fail because required_field extraction is configured to fail
            assert!(extraction_result.is_err());
            let error = extraction_result.unwrap_err();
            assert!(error.to_string().contains("required_field"));
        }
    }

    #[test]
    fn test_mixed_error_scenario_workflow() {
        // Create a realistic mixed error scenario
        let results = vec![
            // Successful health check
            create_comparison_result(
                "health",
                vec![("dev", 200, "OK"), ("prod", 200, "OK")],
                true,
            ),
            // Authentication failure in staging
            create_comparison_result(
                "auth",
                vec![("dev", 200, "token"), ("staging", 401, "unauthorized"), ("prod", 200, "token")],
                false,
            ),
            // Database error in production
            create_comparison_result(
                "user_data",
                vec![("dev", 200, "users"), ("staging", 200, "users"), ("prod", 500, "database error")],
                false,
            ),
            // Service unavailable across environments
            create_comparison_result(
                "payment_service",
                vec![("dev", 503, "unavailable"), ("staging", 503, "unavailable"), ("prod", 503, "unavailable")],
                false,
            ),
        ];
        
        // Test comprehensive error analysis
        let summary = ErrorSummary::from_comparison_results(&results);
        assert_eq!(summary.total_requests, 4);
        
        // Verify error distribution
        let error_results: Vec<_> = results.iter().filter(|r| r.has_errors).collect();
        assert_eq!(error_results.len(), 3);
        
        // Test environment-specific error analysis
        let mut env_error_counts = HashMap::new();
        for result in &results {
            if let Some(error_bodies) = &result.error_bodies {
                for env in error_bodies.keys() {
                    *env_error_counts.entry(env.clone()).or_insert(0) += 1;
                }
            }
        }
        
        // Verify error distribution makes sense
        assert!(env_error_counts.get("dev").copied().unwrap_or(0) <= 2); // dev has fewer errors
        assert!(env_error_counts.get("prod").copied().unwrap_or(0) >= 1); // prod has some errors
    }
}

// =============================================================================
// PERFORMANCE AND SCALABILITY INTEGRATION TESTS
// =============================================================================

#[cfg(test)]
mod performance_integration_tests {
    use super::*;

    #[test]
    fn test_large_configuration_parsing() {
        // Test parsing performance with larger configurations
        let mut config_builder = String::new();
        config_builder.push_str("[environments.dev]\nbase_url = \"https://dev.example.com\"\n\n");
        config_builder.push_str("[environments.prod]\nbase_url = \"https://prod.example.com\"\n\n");
        
        // Add many routes
        for i in 0..100 {
            config_builder.push_str(&format!(
                "[[routes]]\nname = \"route_{}\"\nmethod = \"GET\"\npath = \"/api/route/{}\"\n\n",
                i, i
            ));
        }
        
        // Test that large configs can be parsed efficiently
        let start = std::time::Instant::now();
        let config: Result<HttpDiffConfig, _> = toml::from_str(&config_builder);
        let parse_duration = start.elapsed();
        
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.routes.len(), 100);
        assert_eq!(config.environments.len(), 2);
        
        // Parsing should be reasonably fast
        assert!(parse_duration.as_millis() < 1000, "Config parsing took too long: {:?}", parse_duration);
    }

    #[test]
    fn test_multiple_user_workflow_integration() {
        let (config, _client, _comparator, _evaluator, _users) = TestScenarioBuilder::new()
            .with_users(50) // Test with many users
            .build();
        
        // Create multiple user scenarios
        let users = create_test_users(50);
        
        // Verify user data variety
        let premium_users: Vec<_> = users.iter()
            .filter(|u| u.data.get("user_type") == Some(&"premium".to_string()))
            .collect();
        let basic_users: Vec<_> = users.iter()
            .filter(|u| u.data.get("user_type") == Some(&"basic".to_string()))
            .collect();
        
        assert!(!premium_users.is_empty());
        assert!(!basic_users.is_empty());
        assert_eq!(premium_users.len() + basic_users.len(), 50);
        
        // Test that each user has unique identifiers
        let user_ids: std::collections::HashSet<_> = users.iter()
            .filter_map(|u| u.data.get("userId"))
            .collect();
        assert_eq!(user_ids.len(), 50); // All users should have unique IDs
    }
}

// =============================================================================
// BACKWARD COMPATIBILITY INTEGRATION TESTS
// =============================================================================

#[cfg(test)]
mod compatibility_integration_tests {
    use super::*;

    #[test]
    fn test_legacy_config_format_compatibility() {
        // Test that old config formats still work
        let legacy_config = r#"
[environments.test]
base_url = "http://localhost:8080"

[environments.prod]
base_url = "http://api.example.com"

[[routes]]
name = "simple"
method = "GET"
path = "/health"
"#;
        
        let config: Result<HttpDiffConfig, _> = toml::from_str(legacy_config);
        assert!(config.is_ok());
        
        let config = config.unwrap();
        assert_eq!(config.environments.len(), 2);
        assert_eq!(config.routes.len(), 1);
        
        // Verify legacy format fields are properly handled
        let route = &config.routes[0];
        assert_eq!(route.name, "simple");
        assert_eq!(route.method, "GET");
        assert_eq!(route.path, "/health");
        assert!(route.conditions.is_none());
        assert!(route.extract.is_none());
        assert!(route.depends_on.is_none());
    }

    #[test] 
    fn test_configuration_evolution_workflow() {
        // Test that newer features don't break basic functionality
        let basic_config = create_test_config();
        let advanced_config = create_test_config_with_conditions();
        
        // Both configs should parse successfully
        let basic: HttpDiffConfig = toml::from_str(&basic_config).unwrap();
        let advanced: HttpDiffConfig = toml::from_str(&advanced_config).unwrap();
        
        // Basic config should have simpler routes
        assert!(basic.routes.iter().all(|r| r.conditions.is_none()));
        
        // Advanced config should have conditional routes
        assert!(advanced.routes.iter().any(|r| r.conditions.is_some()));
        
        // Both should have valid environment configurations
        assert!(!basic.environments.is_empty());
        assert!(!advanced.environments.is_empty());
    }
}
