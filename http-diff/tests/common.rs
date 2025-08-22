//! Comprehensive test utilities for http-diff crate tests
//!
//! This module provides a centralized collection of mock implementations,
//! helper functions, and factory methods for all test scenarios.

use http_diff::{
    ComparisonResult, HttpResponse,
    config::{Environment, HttpDiffConfig, Route, UserData, ValueExtractionRule, ExtractorType},
    conditions::{ConditionOperator, ExecutionCondition},
    traits::{HttpClient, ResponseComparator, ConditionEvaluator},
    types::{DiffViewStyle, Difference, DifferenceCategory, ExtractedValue, ExtractionType},
    error::{HttpDiffError, Result},
};
use std::collections::HashMap;
use chrono;

/// Helper function to create HttpResponse with specific status and content
pub fn create_response(status: u16, body: &str, url: Option<&str>) -> HttpResponse {
    HttpResponse {
        status,
        headers: HashMap::new(),
        body: body.to_string(),
        url: url.unwrap_or("http://test.com").to_string(),
        curl_command: format!("curl {}", url.unwrap_or("http://test.com")),
    }
}

/// Helper function to create ComparisonResult for testing
pub fn create_comparison_result(
    route_name: &str,
    env_responses: Vec<(&str, u16, &str)>, // (env, status, body)
    is_identical: bool,
) -> ComparisonResult {
    let mut responses = HashMap::new();
    let mut status_codes = HashMap::new();
    let mut error_bodies = HashMap::new();
    let mut has_errors = false;

    for (env, status, body) in env_responses {
        responses.insert(env.to_string(), create_response(status, body, None));
        status_codes.insert(env.to_string(), status);

        if !(200..300).contains(&status) {
            has_errors = true;
            error_bodies.insert(env.to_string(), body.to_string());
        }
    }

    ComparisonResult {
        route_name: route_name.to_string(),
        user_context: HashMap::new(),
        responses,
        differences: vec![],
        is_identical,
        status_codes,
        has_errors,
        error_bodies: if error_bodies.is_empty() {
            None
        } else {
            Some(error_bodies)
        },
        base_environment: None,
    }
}

/// Create test configuration for integration tests
pub fn create_test_config() -> String {
    r#"[environments.test]
base_url = "http://127.0.0.1:8080"

[environments.prod]
base_url = "http://127.0.0.1:8081"

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "api/users"
method = "GET"
path = "/api/users"
"#
    .to_string()
}

/// Create test configuration with conditional routes
pub fn create_test_config_with_conditions() -> String {
    r#"[environments.test]
base_url = "http://127.0.0.1:8080"

[environments.prod]
base_url = "http://127.0.0.1:8081"

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "premium-api"
method = "GET"
path = "/api/premium/users"

[[routes.conditions]]
variable = "user_type"
operator = "equals"
value = "premium"

[[routes.conditions]]
variable = "user_id"
operator = "greater_than"
value = "1000"

[[routes]]
name = "debug-endpoint"
method = "GET"
path = "/debug/health"

[[routes.conditions]]
variable = "env.DEBUG_MODE"
operator = "equals"
value = "true"
"#
    .to_string()
}

/// Create test user data CSV content
pub fn create_test_users_csv() -> String {
    "userId,siteId,userName\n12345,MCO,test_user\n".to_string()
}

/// Create test user data CSV content with conditional fields
pub fn create_test_users_csv_with_conditions() -> String {
    "userId,user_type,user_id,userName\n12345,premium,1500,premium_user\n67890,basic,500,basic_user\n".to_string()
}

// Test helper functions that were previously in mocks

/// Helper to create a mock route for testing
pub fn create_mock_route(name: &str, method: &str, path: &str) -> http_diff::config::Route {
    http_diff::config::Route {
        name: name.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: None,
        extract: None,
        depends_on: None,
        wait_for_extraction: None,
    }
}

/// Helper to create a basic route for testing (alias for create_mock_route)
pub fn create_route(name: &str, method: &str, path: &str) -> http_diff::config::Route {
    create_mock_route(name, method, path)
}

/// Helper to create mock user data for testing
pub fn create_mock_user_data(data: Vec<(&str, &str)>) -> http_diff::config::UserData {
    let mut user_data = HashMap::new();
    for (key, value) in data {
        user_data.insert(key.to_string(), value.to_string());
    }
    http_diff::config::UserData { data: user_data }
}

/// Helper to create mock HTTP response for testing
pub fn create_mock_response(status: u16, body: &str) -> http_diff::types::HttpResponse {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());

    http_diff::types::HttpResponse {
        status,
        headers,
        body: body.to_string(),
        url: "https://example.com/test".to_string(),
        curl_command: "curl 'https://example.com/test'".to_string(),
    }
}

// =============================================================================
// CENTRALIZED MOCK IMPLEMENTATIONS
// =============================================================================

/// Centralized MockHttpClient for all test scenarios
#[derive(Clone)]
pub struct TestMockHttpClient {
    pub responses: HashMap<String, HttpResponse>,
    pub should_fail: bool,
    pub failure_message: String,
    pub route_failures: HashMap<String, String>,
    pub extraction_failures: HashMap<String, Vec<String>>,
}

impl TestMockHttpClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            should_fail: false,
            failure_message: "Mock failure".to_string(),
            route_failures: HashMap::new(),
            extraction_failures: HashMap::new(),
        }
    }

    pub fn with_responses(mut self, responses: HashMap<String, HttpResponse>) -> Self {
        self.responses = responses;
        self
    }

    pub fn with_failure(mut self, message: String) -> Self {
        self.should_fail = true;
        self.failure_message = message;
        self
    }

    pub fn with_route_failure(mut self, route: String, message: String) -> Self {
        self.route_failures.insert(route, message);
        self
    }

    pub fn with_extraction_failure(mut self, route: String, failed_extractions: Vec<String>) -> Self {
        self.extraction_failures.insert(route, failed_extractions);
        self
    }

    pub fn has_response(&self, key: &str) -> bool {
        self.responses.contains_key(key)
    }

    pub fn extract_values(&self, route: &Route, response: &HttpResponse) -> Result<HashMap<String, String>> {
        let mut extracted_values = HashMap::new();

        if let Some(extractions) = &route.extract {
            for extraction_rule in extractions {
                // Check if this extraction should fail
                if let Some(failed_extractions) = self.extraction_failures.get(&route.name) {
                    if failed_extractions.contains(&extraction_rule.name) {
                        if extraction_rule.required {
                            return Err(HttpDiffError::general(
                                format!("Mock extraction failure for required field: {}", extraction_rule.name)
                            ));
                        } else {
                            if let Some(default) = &extraction_rule.default_value {
                                extracted_values.insert(extraction_rule.name.clone(), default.clone());
                            }
                            continue;
                        }
                    }
                }

                // Simulate extraction based on type
                let extracted_value = match extraction_rule.extractor_type {
                    ExtractorType::JsonPath => {
                        match extraction_rule.source.as_str() {
                            "$.token" => Some("mock_token_123".to_string()),
                            "$.user_id" => Some("1001".to_string()),
                            "$.users[0].id" => Some("1".to_string()),
                            "$.email" => Some("test@example.com".to_string()),
                            "$.profile.department" => Some("Engineering".to_string()),
                            "$.data.id" => Some("extracted_id_123".to_string()),
                            _ => extraction_rule.default_value.clone(),
                        }
                    },
                    ExtractorType::Regex => {
                        if extraction_rule.source.contains("token=") {
                            Some("regex_token_456".to_string())
                        } else {
                            extraction_rule.default_value.clone()
                        }
                    },
                    ExtractorType::Header => {
                        response.headers.get(&extraction_rule.source).cloned()
                            .or_else(|| extraction_rule.default_value.clone())
                    },
                    ExtractorType::StatusCode => {
                        Some(response.status.to_string())
                    },
                };

                if let Some(value) = extracted_value {
                    extracted_values.insert(extraction_rule.name.clone(), value);
                } else if extraction_rule.required {
                    return Err(HttpDiffError::general(
                        format!("Required extraction failed: {}", extraction_rule.name)
                    ));
                }
            }
        }

        Ok(extracted_values)
    }
}

impl HttpClient for TestMockHttpClient {
    async fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        _user_data: &UserData,
    ) -> Result<HttpResponse> {
        if self.should_fail {
            return Err(HttpDiffError::general(&self.failure_message));
        }

        if let Some(error_message) = self.route_failures.get(&route.name) {
            return Err(HttpDiffError::general(error_message));
        }

        let key = format!("{}:{}", route.name, environment);
        self.responses.get(&key).cloned().ok_or_else(|| {
            HttpDiffError::general(format!(
                "Mock response not found for key: {}",
                key
            ))
        })
    }
}

/// Centralized MockResponseComparator for all test scenarios
#[derive(Clone)]
pub struct TestMockResponseComparator {
    pub should_find_differences: bool,
    pub mock_differences: Vec<Difference>,
    pub headers_comparison_enabled: bool,
    pub diff_view_style: DiffViewStyle,
}

impl TestMockResponseComparator {
    pub fn new() -> Self {
        Self {
            should_find_differences: false,
            mock_differences: Vec::new(),
            headers_comparison_enabled: false,
            diff_view_style: DiffViewStyle::SideBySide,
        }
    }

    pub fn with_differences(mut self, differences: Vec<Difference>) -> Self {
        self.should_find_differences = true;
        self.mock_differences = differences;
        self
    }

    pub fn with_headers_comparison(mut self) -> Self {
        self.headers_comparison_enabled = true;
        self
    }

    pub fn with_diff_style(mut self, style: DiffViewStyle) -> Self {
        self.diff_view_style = style;
        self
    }
}

impl ResponseComparator for TestMockResponseComparator {
    fn compare_responses(
        &self,
        route_name: String,
        user_context: HashMap<String, String>,
        responses: HashMap<String, HttpResponse>,
    ) -> Result<ComparisonResult> {
        let mut result = ComparisonResult::new(route_name, user_context);
        for (env, response) in responses {
            result.add_response(env, response);
        }
        
        if self.should_find_differences {
            result.is_identical = false;
            for diff in &self.mock_differences {
                result.differences.push(diff.clone());
            }
        }
        
        Ok(result)
    }

    fn diff_view_style(&self) -> DiffViewStyle {
        self.diff_view_style.clone()
    }

    fn headers_comparison_enabled(&self) -> bool {
        self.headers_comparison_enabled
    }
}

/// Centralized MockConditionEvaluator for all test scenarios
#[derive(Clone)]
pub struct TestMockConditionEvaluator {
    pub should_execute_results: HashMap<String, bool>,
    pub default_result: bool,
    pub should_fail: bool,
    pub failure_message: String,
}

impl TestMockConditionEvaluator {
    pub fn new() -> Self {
        Self {
            should_execute_results: HashMap::new(),
            default_result: true,
            should_fail: false,
            failure_message: "Mock condition evaluation failure".to_string(),
        }
    }

    pub fn with_route_result(mut self, route_name: String, should_execute: bool) -> Self {
        self.should_execute_results.insert(route_name, should_execute);
        self
    }

    pub fn with_default_result(mut self, default_result: bool) -> Self {
        self.default_result = default_result;
        self
    }

    pub fn with_failure(mut self, message: String) -> Self {
        self.should_fail = true;
        self.failure_message = message;
        self
    }
}

impl ConditionEvaluator for TestMockConditionEvaluator {
    fn should_execute_route(&self, route: &Route, _user_data: &UserData) -> Result<bool> {
        if self.should_fail {
            return Err(HttpDiffError::general(&self.failure_message));
        }

        Ok(self
            .should_execute_results
            .get(&route.name)
            .copied()
            .unwrap_or(self.default_result))
    }

    fn evaluate_conditions(
        &self,
        conditions: &[ExecutionCondition],
        _user_data: &UserData,
    ) -> Result<Vec<http_diff::conditions::ConditionResult>> {
        if self.should_fail {
            return Err(HttpDiffError::general(&self.failure_message));
        }

        let results = conditions
            .iter()
            .map(|condition| http_diff::conditions::ConditionResult {
                condition: condition.clone(),
                passed: self.default_result,
                actual_value: Some("mock_value".to_string()),
                reason: None,
            })
            .collect();

        Ok(results)
    }
}

// =============================================================================
// FACTORY FUNCTIONS FOR CONFIGURATIONS
// =============================================================================

/// Create a standard environment configuration
pub fn create_test_environments() -> HashMap<String, Environment> {
    let mut environments = HashMap::new();
    environments.insert("dev".to_string(), Environment {
        base_url: "https://dev.example.com".to_string(),
        headers: None,
        is_base: false,
    });
    environments.insert("staging".to_string(), Environment {
        base_url: "https://staging.example.com".to_string(),
        headers: None,
        is_base: false,
    });
    environments.insert("prod".to_string(), Environment {
        base_url: "https://prod.example.com".to_string(),
        headers: None,
        is_base: true,
    });
    environments
}

/// Create a route with extraction rules
pub fn create_route_with_extraction(
    name: &str,
    method: &str,
    path: &str,
    extractions: Vec<ValueExtractionRule>,
    depends_on: Option<Vec<String>>,
) -> Route {
    Route {
        name: name.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: None,
        extract: if extractions.is_empty() { None } else { Some(extractions) },
        depends_on: depends_on.clone(),
        wait_for_extraction: depends_on.as_ref().map(|_| true),
    }
}

/// Create a route with conditions
pub fn create_route_with_conditions(
    name: &str,
    method: &str,
    path: &str,
    conditions: Vec<ExecutionCondition>,
) -> Route {
    Route {
        name: name.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        headers: None,
        params: None,
        base_urls: None,
        body: None,
        conditions: Some(conditions),
        extract: None,
        depends_on: None,
        wait_for_extraction: None,
    }
}

/// Create an extraction rule
pub fn create_extraction_rule(
    name: &str,
    extractor_type: &str,
    source: &str,
    required: bool,
    default_value: Option<String>,
) -> ValueExtractionRule {
    let ext_type = match extractor_type {
        "JsonPath" => ExtractorType::JsonPath,
        "Regex" => ExtractorType::Regex,
        "Header" => ExtractorType::Header,
        "StatusCode" => ExtractorType::StatusCode,
        _ => ExtractorType::JsonPath,
    };
    
    ValueExtractionRule {
        name: name.to_string(),
        extractor_type: ext_type,
        source: source.to_string(),
        default_value,
        required,
    }
}

/// Create an execution condition
pub fn create_execution_condition(
    variable: &str,
    operator: ConditionOperator,
    value: Option<&str>,
) -> ExecutionCondition {
    ExecutionCondition {
        variable: variable.to_string(),
        operator,
        value: value.map(|v| v.to_string()),
    }
}

/// Helper to create an execution condition with string operator
pub fn create_execution_condition_str(
    variable: &str,
    operator: &str, 
    value: Option<&str>,
) -> ExecutionCondition {
    let op = match operator {
        "equals" => ConditionOperator::Equals,
        "greater_than" => ConditionOperator::GreaterThan,
        "less_than" => ConditionOperator::LessThan,
        "contains" => ConditionOperator::Contains,
        _ => ConditionOperator::Equals,
    };
    create_execution_condition(variable, op, value)
}

/// Create test user data with common patterns
pub fn create_test_user_data(user_type: &str, user_id: &str) -> UserData {
    let mut data = HashMap::new();
    data.insert("userId".to_string(), user_id.to_string());
    data.insert("userName".to_string(), format!("user_{}", user_id));
    data.insert("user_type".to_string(), user_type.to_string());
    data.insert("environment".to_string(), "test".to_string());
    UserData { data }
}

/// Create test user data from key-value pairs
pub fn create_user_data(data: Vec<(&str, &str)>) -> UserData {
    let mut user_data = HashMap::new();
    for (key, value) in data {
        user_data.insert(key.to_string(), value.to_string());
    }
    UserData { data: user_data }
}

/// Create multiple test users
pub fn create_test_users(count: usize) -> Vec<UserData> {
    (0..count).map(|i| {
        create_user_data(vec![
            ("userId", &format!("user_{}", i)),
            ("userName", &format!("test_user_{}", i)),
            ("user_type", if i % 2 == 0 { "premium" } else { "basic" }),
            ("environment", "test"),
        ])
    }).collect()
}

// =============================================================================
// CHAIN CONFIGURATION FACTORIES
// =============================================================================

/// Create a basic login -> list -> detail chain configuration
pub fn create_basic_chain_config() -> HttpDiffConfig {
    let environments = create_test_environments();
    
    HttpDiffConfig {
        environments,
        global: None,
        routes: vec![
            create_route_with_extraction(
                "login",
                "POST", 
                "/auth/login",
                vec![
                    create_extraction_rule("auth_token", "JsonPath", "$.token", true, None),
                    create_extraction_rule("logged_user_id", "JsonPath", "$.user_id", false, None),
                ],
                None,
            ),
            create_route_with_extraction(
                "user_list",
                "GET",
                "/api/users", 
                vec![
                    create_extraction_rule("first_user_id", "JsonPath", "$.users[0].id", false, Some("1".to_string())),
                ],
                Some(vec!["login".to_string()]),
            ),
            create_route_with_extraction(
                "user_detail",
                "GET",
                "/api/users/{first_user_id}",
                vec![
                    create_extraction_rule("user_email", "JsonPath", "$.email", false, None),
                    create_extraction_rule("department", "JsonPath", "$.profile.department", false, Some("Unknown".to_string())),
                ],
                Some(vec!["user_list".to_string()]),
            ),
        ],
    }
}

/// Create a configuration with conditional routes
pub fn create_conditional_chain_config() -> HttpDiffConfig {
    let environments = create_test_environments();
    
    let routes = vec![
        create_mock_route("health", "GET", "/health"),
        create_route_with_conditions(
            "premium-api",
            "GET",
            "/api/premium/users",
            vec![
                create_execution_condition("user_type", ConditionOperator::Equals, Some("premium")),
                create_execution_condition("user_id", ConditionOperator::GreaterThan, Some("1000")),
            ],
        ),
        create_route_with_conditions(
            "debug-endpoint",
            "GET",
            "/debug/health",
            vec![
                create_execution_condition("env.DEBUG_MODE", ConditionOperator::Equals, Some("true")),
            ],
        ),
    ];

    HttpDiffConfig {
        environments,
        global: None,
        routes,
    }
}

/// Create a complex multi-level dependency configuration
pub fn create_complex_chain_config() -> HttpDiffConfig {
    let environments = create_test_environments();
    
    let routes = vec![
        // Level 0: Authentication
        create_route_with_extraction(
            "auth",
            "POST",
            "/auth/token",
            vec![
                create_extraction_rule(
                    "access_token",
                    "JsonPath",
                    "$.access_token",
                    true,
                    None,
                ),
            ],
            None,
        ),
        // Level 1: Organization lookup
        create_route_with_extraction(
            "organization",
            "GET",
            "/api/organizations/current",
            vec![
                create_extraction_rule(
                    "org_id",
                    "JsonPath",
                    "$.id",
                    true,
                    None,
                ),
                create_extraction_rule(
                    "org_name",
                    "JsonPath",
                    "$.name",
                    false,
                    None,
                ),
            ],
            Some(vec!["auth".to_string()]),
        ),
        // Level 2: Projects and Teams (parallel)
        create_route_with_extraction(
            "projects",
            "GET",
            "/api/organizations/{org_id}/projects",
            vec![
                create_extraction_rule(
                    "project_id",
                    "JsonPath",
                    "$.projects[0].id",
                    true,
                    None,
                ),
            ],
            Some(vec!["organization".to_string()]),
        ),
        create_route_with_extraction(
            "teams",
            "GET",
            "/api/organizations/{org_id}/teams",
            vec![
                create_extraction_rule(
                    "team_id",
                    "JsonPath",
                    "$.teams[0].id",
                    true,
                    None,
                ),
            ],
            Some(vec!["organization".to_string()]),
        ),
        // Level 3: Project details (depends on both projects and teams)
        Route {
            name: "project_details".to_string(),
            method: "GET".to_string(),
            path: "/api/projects/{project_id}".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer {access_token}".to_string());
                headers
            }),
            params: Some({
                let mut params = HashMap::new();
                params.insert("include_team".to_string(), "{team_id}".to_string());
                params
            }),
            base_urls: None,
            body: None,
            conditions: None,
            extract: None,
            depends_on: Some(vec!["projects".to_string(), "teams".to_string()]),
            wait_for_extraction: Some(true),
        },
    ];

    HttpDiffConfig {
        environments,
        global: None,
        routes,
    }
}

// =============================================================================
// RESPONSE FACTORIES
// =============================================================================

/// Create mock responses for the basic chain
pub fn create_basic_chain_responses() -> HashMap<String, HttpResponse> {
    let mut responses = HashMap::new();
    
    // Login responses
    responses.insert(
        "login:dev".to_string(),
        create_mock_response(200, r#"{"token": "dev_token_123", "user_id": 1001}"#)
    );
    responses.insert(
        "login:staging".to_string(),
        create_mock_response(200, r#"{"token": "staging_token_456", "user_id": 1001}"#)
    );
    
    // User list responses
    responses.insert(
        "user_list:dev".to_string(),
        create_mock_response(200, r#"{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]}"#)
    );
    responses.insert(
        "user_list:staging".to_string(),
        create_mock_response(200, r#"{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]}"#)
    );
    
    // User detail responses
    responses.insert(
        "user_detail:dev".to_string(),
        create_mock_response(200, r#"{"id": 1, "name": "Alice", "email": "alice@dev.com", "profile": {"department": "Engineering"}}"#)
    );
    responses.insert(
        "user_detail:staging".to_string(),
        create_mock_response(200, r#"{"id": 1, "name": "Alice", "email": "alice@staging.com", "profile": {"department": "Engineering"}}"#)
    );
    
    responses
}

/// Create mock responses for conditional routes
pub fn create_conditional_responses() -> HashMap<String, HttpResponse> {
    let mut responses = HashMap::new();
    
    responses.insert(
        "health:dev".to_string(),
        create_mock_response(200, r#"{"status": "healthy"}"#)
    );
    responses.insert(
        "health:staging".to_string(),
        create_mock_response(200, r#"{"status": "healthy"}"#)
    );
    
    responses.insert(
        "premium-api:dev".to_string(),
        create_mock_response(200, r#"{"premium_data": "exclusive_content"}"#)
    );
    responses.insert(
        "premium-api:staging".to_string(),
        create_mock_response(200, r#"{"premium_data": "exclusive_content"}"#)
    );
    
    responses.insert(
        "debug-endpoint:dev".to_string(),
        create_mock_response(200, r#"{"debug_info": "detailed_logs"}"#)
    );
    responses.insert(
        "debug-endpoint:staging".to_string(),
        create_mock_response(200, r#"{"debug_info": "detailed_logs"}"#)
    );
    
    responses
}

/// Create responses with various error scenarios
pub fn create_error_scenario_responses() -> HashMap<String, HttpResponse> {
    let mut responses = HashMap::new();
    
    // Success responses
    responses.insert(
        "success_route:dev".to_string(),
        create_mock_response(200, r#"{"status": "ok"}"#)
    );
    responses.insert(
        "another_success:dev".to_string(),
        create_mock_response(200, r#"{"status": "ok"}"#)
    );
    
    // Error responses
    responses.insert(
        "error_route:dev".to_string(),
        create_mock_response(500, r#"{"error": "Internal server error"}"#)
    );
    responses.insert(
        "unauthorized_route:dev".to_string(),
        create_mock_response(401, r#"{"error": "Unauthorized"}"#)
    );
    responses.insert(
        "not_found_route:dev".to_string(),
        create_mock_response(404, r#"{"error": "Not found"}"#)
    );
    
    responses
}

// =============================================================================
// PERFORMANCE TESTING UTILITIES
// =============================================================================

/// Create a linear chain of routes for performance testing
pub fn create_linear_chain_routes(count: usize) -> Vec<Route> {
    (0..count).map(|i| {
        let depends_on = if i == 0 {
            None
        } else {
            Some(vec![format!("route_{}", i - 1)])
        };

        create_route_with_extraction(
            &format!("route_{}", i),
            "GET",
            &format!("/api/route/{}", i),
            if i < count - 1 {
                vec![create_extraction_rule(
                    &format!("value_{}", i),
                    "JsonPath",
                    &format!("$.data.value_{}", i),
                    false,
                    None,
                )]
            } else {
                vec![]
            },
            depends_on,
        )
    }).collect()
}

/// Create wide dependency routes (few roots, many dependents)
pub fn create_wide_dependency_routes(root_count: usize, dependent_count: usize) -> Vec<Route> {
    let mut routes = Vec::new();

    // Create root routes
    for i in 0..root_count {
        routes.push(create_route_with_extraction(
            &format!("root_{}", i),
            "POST",
            &format!("/auth/root_{}", i),
            vec![create_extraction_rule(
                &format!("root_token_{}", i),
                "JsonPath",
                "$.token",
                true,
                None,
            )],
            None,
        ));
    }

    // Create dependent routes
    for i in 0..dependent_count {
        let root_index = i % root_count;
        routes.push(Route {
            name: format!("dependent_{}", i),
            method: "GET".to_string(),
            path: format!("/api/dependent/{}", i),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), 
                              format!("Bearer {{root_token_{}}}", root_index));
                headers
            }),
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: None,
            depends_on: Some(vec![format!("root_{}", root_index)]),
            wait_for_extraction: Some(true),
        });
    }

    routes
}

/// Create performance test responses
pub fn create_performance_test_responses(route_count: usize) -> HashMap<String, HttpResponse> {
    let mut responses = HashMap::new();
    
    for i in 0..route_count {
        responses.insert(
            format!("route_{}:dev", i),
            create_mock_response(200, &format!(r#"{{"data": {{"value_{}": "extracted_{}", "id": {}}}}}"#, i, i, i))
        );
    }
    
    responses
}

// =============================================================================
// EXTRACTED VALUE UTILITIES
// =============================================================================

/// Create mock extracted values for testing
pub fn create_extracted_values(route_name: &str, environment: &str, values: Vec<(&str, &str)>) -> Vec<ExtractedValue> {
    values.into_iter().map(|(key, value)| {
        ExtractedValue {
            key: key.to_string(),
            value: value.to_string(),
            extraction_rule: format!("$.{}", key),
            extraction_type: ExtractionType::JsonPath,
            environment: environment.to_string(),
            route_name: route_name.to_string(),
            extracted_at: chrono::Utc::now(),
        }
    }).collect()
}

/// Create test extractions for context manager testing
pub fn create_test_extractions(count: usize) -> HashMap<String, String> {
    (0..count).map(|i| {
        (format!("extracted_value_{}", i), format!("value_{}", i))
    }).collect()
}

// =============================================================================
// DIFFERENCE FACTORIES
// =============================================================================

/// Create mock differences for comparison testing
pub fn create_mock_differences() -> Vec<Difference> {
    vec![
        Difference::new(
            DifferenceCategory::Body,
            "Response bodies differ".to_string()
        ),
        Difference::new(
            DifferenceCategory::Status,
            "Status codes differ: 200 vs 404".to_string()
        ),
        Difference::new(
            DifferenceCategory::Headers,
            "Content-Type headers differ".to_string()
        ),
    ]
}

// =============================================================================
// INTEGRATION TEST BUILDERS
// =============================================================================

/// Builder for creating comprehensive test scenarios
pub struct TestScenarioBuilder {
    config: Option<HttpDiffConfig>,
    responses: HashMap<String, HttpResponse>,
    user_data: Vec<UserData>,
    should_find_differences: bool,
    mock_differences: Vec<Difference>,
    route_failures: HashMap<String, String>,
    extraction_failures: HashMap<String, Vec<String>>,
    condition_results: HashMap<String, bool>,
}

impl TestScenarioBuilder {
    pub fn new() -> Self {
        Self {
            config: None,
            responses: HashMap::new(),
            user_data: Vec::new(),
            should_find_differences: false,
            mock_differences: Vec::new(),
            route_failures: HashMap::new(),
            extraction_failures: HashMap::new(),
            condition_results: HashMap::new(),
        }
    }

    pub fn with_basic_chain(mut self) -> Self {
        self.config = Some(create_basic_chain_config());
        self.responses = create_basic_chain_responses();
        self
    }

    pub fn with_conditional_chain(mut self) -> Self {
        self.config = Some(create_conditional_chain_config());
        self.responses = create_conditional_responses();
        self
    }

    pub fn with_complex_chain(mut self) -> Self {
        self.config = Some(create_complex_chain_config());
        self
    }

    pub fn with_users(mut self, count: usize) -> Self {
        self.user_data = create_test_users(count);
        self
    }

    pub fn with_user_data(mut self, user_data: Vec<UserData>) -> Self {
        self.user_data = user_data;
        self
    }

    pub fn with_differences(mut self, differences: Vec<Difference>) -> Self {
        self.should_find_differences = true;
        self.mock_differences = differences;
        self
    }

    pub fn with_route_failure(mut self, route: &str, message: &str) -> Self {
        self.route_failures.insert(route.to_string(), message.to_string());
        self
    }

    pub fn with_extraction_failure(mut self, route: &str, failed_extractions: Vec<&str>) -> Self {
        self.extraction_failures.insert(
            route.to_string(), 
            failed_extractions.into_iter().map(|s| s.to_string()).collect()
        );
        self
    }

    pub fn with_condition_result(mut self, route: &str, should_execute: bool) -> Self {
        self.condition_results.insert(route.to_string(), should_execute);
        self
    }

    pub fn with_basic_responses(mut self) -> Self {
        // Create basic responses for common test scenarios
        let mut responses = HashMap::new();
        
        // Health check responses
        responses.insert("health:dev".to_string(), create_response(200, r#"{"status": "healthy"}"#, None));
        responses.insert("health:staging".to_string(), create_response(200, r#"{"status": "healthy"}"#, None));
        responses.insert("health:prod".to_string(), create_response(200, r#"{"status": "healthy"}"#, None));
        
        // API responses
        responses.insert("api/users:dev".to_string(), create_response(200, r#"{"users": [{"id": 1, "name": "Test User"}]}"#, None));
        responses.insert("api/users:staging".to_string(), create_response(200, r#"{"users": [{"id": 1, "name": "Test User"}]}"#, None));
        responses.insert("api/users:prod".to_string(), create_response(200, r#"{"users": [{"id": 1, "name": "Test User"}]}"#, None));
        
        // Test route responses
        responses.insert("test:dev".to_string(), create_response(200, r#"{"result": "success"}"#, None));
        responses.insert("test:staging".to_string(), create_response(200, r#"{"result": "success"}"#, None));
        responses.insert("test:prod".to_string(), create_response(200, r#"{"result": "success"}"#, None));
        
        self.responses.extend(responses);
        self
    }

    pub fn build(self) -> (
        HttpDiffConfig,
        TestMockHttpClient,
        TestMockResponseComparator,
        TestMockConditionEvaluator,
        Vec<UserData>
    ) {
        let config = self.config.unwrap_or_else(|| {
            // Create a simple default config if none specified
            let environments = create_test_environments();
            let routes = vec![
                create_route("health", "GET", "/health"),
                create_route("api/users", "GET", "/api/users"),
            ];
            HttpDiffConfig {
                environments,
                global: None,
                routes,
            }
        });
        
        let mut http_client = TestMockHttpClient::new().with_responses(self.responses);
        for (route, message) in self.route_failures {
            http_client = http_client.with_route_failure(route, message);
        }
        for (route, failures) in self.extraction_failures {
            http_client = http_client.with_extraction_failure(route, failures);
        }
        
        let mut comparator = TestMockResponseComparator::new();
        if self.should_find_differences {
            comparator = comparator.with_differences(self.mock_differences);
        }
        
        let mut condition_evaluator = TestMockConditionEvaluator::new();
        for (route, result) in self.condition_results {
            condition_evaluator = condition_evaluator.with_route_result(route, result);
        }
        
        let user_data = if self.user_data.is_empty() {
            create_test_users(1)
        } else {
            self.user_data
        };
        
        (config, http_client, comparator, condition_evaluator, user_data)
    }
}
