//! Test for conditional route execution integration

use http_diff::{
    config::{HttpDiffConfig, Route, UserData},
    conditions::{ConditionEvaluatorImpl, ConditionOperator, ExecutionCondition},
    execution::TestRunnerImpl,
    traits::{ConditionEvaluator, TestRunner},
    types::HttpResponse,
};
use std::collections::HashMap;

// Mock implementations for testing
#[derive(Clone)]
struct MockHttpClient {
    responses: HashMap<String, HttpResponse>,
}

impl MockHttpClient {
    fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    fn with_response(mut self, key: String, response: HttpResponse) -> Self {
        self.responses.insert(key, response);
        self
    }
}

impl http_diff::traits::HttpClient for MockHttpClient {
    async fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        _user_data: &UserData,
    ) -> http_diff::error::Result<HttpResponse> {
        let key = format!("{}:{}", route.name, environment);
        self.responses
            .get(&key)
            .cloned()
            .ok_or_else(|| http_diff::error::HttpDiffError::general(format!("No response for {}", key)))
    }
}

#[derive(Clone)]
struct MockResponseComparator;

impl MockResponseComparator {
    fn new() -> Self {
        Self
    }
}

impl http_diff::traits::ResponseComparator for MockResponseComparator {
    fn compare_responses(
        &self,
        route_name: String,
        _user_context: HashMap<String, String>,
        responses: HashMap<String, HttpResponse>,
    ) -> http_diff::error::Result<http_diff::types::ComparisonResult> {
        let mut result = http_diff::types::ComparisonResult::new(route_name, HashMap::new());
        for (env, response) in responses {
            result.add_response(env, response);
        }
        Ok(result)
    }

    fn diff_view_style(&self) -> http_diff::types::DiffViewStyle {
        http_diff::types::DiffViewStyle::SideBySide
    }

    fn headers_comparison_enabled(&self) -> bool {
        false
    }
}

fn create_mock_response(status: u16, body: &str) -> HttpResponse {
    HttpResponse::new(
        status,
        HashMap::new(),
        body.to_string(),
        "https://example.com/test".to_string(),
        "curl 'https://example.com/test'".to_string(),
    )
}

#[tokio::test]
async fn test_conditional_route_execution_with_skipping() {
    // Create test configuration with conditional routes
    let mut environments = HashMap::new();
    environments.insert(
        "dev".to_string(),
        http_diff::config::Environment {
            base_url: "https://dev.example.com".to_string(),
            headers: None,
            is_base: false,
        },
    );
    environments.insert(
        "staging".to_string(),
        http_diff::config::Environment {
            base_url: "https://staging.example.com".to_string(),
            headers: None,
            is_base: false,
        },
    );

    // Route with condition that should execute (user_type = "premium")
    let premium_route = Route {
        name: "premium-endpoint".to_string(),
        method: "GET".to_string(),
        path: "/api/premium/data".to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: Some(vec![ExecutionCondition {
            variable: "user_type".to_string(),
            operator: ConditionOperator::Equals,
            value: "premium".to_string(),
        }]),
    };

    // Route with condition that should NOT execute (user_type = "basic")
    let basic_route = Route {
        name: "basic-endpoint".to_string(),
        method: "GET".to_string(),
        path: "/api/basic/data".to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: Some(vec![ExecutionCondition {
            variable: "user_type".to_string(),
            operator: ConditionOperator::Equals,
            value: "basic".to_string(),
        }]),
    };

    // Route without conditions (should always execute)
    let unconditional_route = Route {
        name: "public-endpoint".to_string(),
        method: "GET".to_string(),
        path: "/api/public/health".to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: None,
    };

    let config = HttpDiffConfig {
        environments,
        global: None,
        routes: vec![premium_route, basic_route, unconditional_route],
    };

    // Create user data with user_type = "premium"
    let mut user_data_map = HashMap::new();
    user_data_map.insert("user_type".to_string(), "premium".to_string());
    user_data_map.insert("user_id".to_string(), "123".to_string());
    let user_data = vec![UserData { data: user_data_map }];

    // Mock responses for routes that should execute
    let dev_premium_response = create_mock_response(200, "premium dev data");
    let staging_premium_response = create_mock_response(200, "premium staging data");
    let dev_public_response = create_mock_response(200, "public dev health");
    let staging_public_response = create_mock_response(200, "public staging health");

    let client = MockHttpClient::new()
        .with_response("premium-endpoint:dev".to_string(), dev_premium_response)
        .with_response("premium-endpoint:staging".to_string(), staging_premium_response)
        .with_response("public-endpoint:dev".to_string(), dev_public_response)
        .with_response("public-endpoint:staging".to_string(), staging_public_response);

    let comparator = MockResponseComparator::new();
    let condition_evaluator = ConditionEvaluatorImpl::new();
    let runner = TestRunnerImpl::new(config, client, comparator, condition_evaluator).unwrap();

    // Execute tests
    let result = runner
        .execute_with_data(&user_data, None, None, None)
        .await
        .unwrap();

    // Verify results:
    // - Should have 2 comparison results (premium-endpoint and public-endpoint)
    // - Should have 1 skipped route (basic-endpoint was skipped)
    // - Total requests executed: 2 routes * 2 environments = 4 requests
    assert_eq!(result.comparisons.len(), 2);
    assert_eq!(result.progress.skipped_routes, 1);
    assert_eq!(result.progress.total_requests, 4); // Only executable routes counted
    assert_eq!(result.progress.completed_requests, 4);

    // Verify specific routes were executed
    let route_names: Vec<&String> = result.comparisons.iter().map(|c| &c.route_name).collect();
    assert!(route_names.contains(&&"premium-endpoint".to_string()));
    assert!(route_names.contains(&&"public-endpoint".to_string()));

    // Verify basic-endpoint was not executed (not in comparisons)
    assert!(!route_names.contains(&&"basic-endpoint".to_string()));
}

#[tokio::test]
async fn test_condition_evaluation_error_handling() {
    // Test that condition evaluation errors are handled gracefully
    let mut environments = HashMap::new();
    environments.insert(
        "dev".to_string(),
        http_diff::config::Environment {
            base_url: "https://dev.example.com".to_string(),
            headers: None,
            is_base: false,
        },
    );

    // Route with condition referencing non-existent variable
    let route_with_missing_var = Route {
        name: "test-route".to_string(),
        method: "GET".to_string(),
        path: "/api/test".to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: Some(vec![ExecutionCondition {
            variable: "nonexistent_field".to_string(),
            operator: ConditionOperator::Equals,
            value: "some_value".to_string(),
        }]),
    };

    let config = HttpDiffConfig {
        environments,
        global: None,
        routes: vec![route_with_missing_var],
    };

    // User data without the referenced field
    let mut user_data_map = HashMap::new();
    user_data_map.insert("user_id".to_string(), "123".to_string());
    let user_data = vec![UserData { data: user_data_map }];

    let client = MockHttpClient::new();
    let comparator = MockResponseComparator::new();
    let condition_evaluator = ConditionEvaluatorImpl::new();
    let runner = TestRunnerImpl::new(config, client, comparator, condition_evaluator).unwrap();

    // Execute tests
    let result = runner
        .execute_with_data(&user_data, None, None, None)
        .await
        .unwrap();

    // Should have no comparisons and 1 skipped route due to condition evaluation failure
    assert_eq!(result.comparisons.len(), 0);
    assert_eq!(result.progress.skipped_routes, 1);
    assert_eq!(result.progress.total_requests, 0); // No requests executed
}

#[test]
fn test_condition_evaluator_integration() {
    // Test the condition evaluator directly
    let evaluator = ConditionEvaluatorImpl::new();
    
    // Create test route with conditions
    let route = Route {
        name: "test".to_string(),
        method: "GET".to_string(),
        path: "/test".to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: Some(vec![
            ExecutionCondition {
                variable: "user_type".to_string(),
                operator: ConditionOperator::Equals,
                value: "premium".to_string(),
            },
            ExecutionCondition {
                variable: "status".to_string(),
                operator: ConditionOperator::Contains,
                value: "active".to_string(),
            },
        ]),
    };

    // Test with matching user data (all conditions pass)
    let mut matching_data = HashMap::new();
    matching_data.insert("user_type".to_string(), "premium".to_string());
    matching_data.insert("status".to_string(), "active_user".to_string());
    let matching_user = UserData { data: matching_data };

    assert_eq!(evaluator.should_execute_route(&route, &matching_user).unwrap(), true);

    // Test with non-matching user data (one condition fails)
    let mut non_matching_data = HashMap::new();
    non_matching_data.insert("user_type".to_string(), "basic".to_string());
    non_matching_data.insert("status".to_string(), "active_user".to_string());
    let non_matching_user = UserData { data: non_matching_data };

    assert_eq!(evaluator.should_execute_route(&route, &non_matching_user).unwrap(), false);

    // Test with route without conditions (should always execute)
    let unconditional_route = Route {
        name: "unconditional".to_string(),
        method: "GET".to_string(),
        path: "/public".to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: None,
    };

    assert_eq!(evaluator.should_execute_route(&unconditional_route, &matching_user).unwrap(), true);
    assert_eq!(evaluator.should_execute_route(&unconditional_route, &non_matching_user).unwrap(), true);
}