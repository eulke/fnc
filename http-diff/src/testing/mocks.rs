use crate::conditions::{ConditionResult, ExecutionCondition};
use crate::config::{Route, UserData, ValueExtractionRule, ExtractorType};
use crate::error::Result;
use crate::execution::progress::ProgressTracker;
use crate::testing::test_helpers::create_mock_response;
use crate::traits::{ConditionEvaluator, HttpClient, ResponseComparator, TestRunner};
use crate::types::{ComparisonResult, DiffViewStyle, Difference, DifferenceCategory, ExecutionError, HttpResponse};
use std::collections::HashMap;

/// Mock HTTP client for testing
#[derive(Clone)]
pub struct MockHttpClient {
    pub responses: HashMap<String, HttpResponse>,
    pub should_fail: bool,
    pub failure_message: String,
}

impl MockHttpClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            should_fail: false,
            failure_message: "Mock failure".to_string(),
        }
    }

    pub fn with_response(mut self, key: String, response: HttpResponse) -> Self {
        self.responses.insert(key, response);
        self
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

    fn make_key(route: &Route, environment: &str) -> String {
        format!("{}:{}", route.name, environment)
    }
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for MockHttpClient {
    async fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        _user_data: &UserData,
    ) -> Result<HttpResponse> {
        if self.should_fail {
            return Err(crate::error::HttpDiffError::general(&self.failure_message));
        }

        let key = Self::make_key(route, environment);
        self.responses.get(&key).cloned().ok_or_else(|| {
            crate::error::HttpDiffError::general(format!(
                "Mock response not found for key: {}",
                key
            ))
        })
    }
}

/// Mock response comparator for testing
pub struct MockResponseComparator {
    pub should_find_differences: bool,
    pub mock_differences: Vec<Difference>,
    pub headers_comparison_enabled: bool,
    pub diff_view_style: DiffViewStyle,
}

impl MockResponseComparator {
    pub fn new() -> Self {
        Self {
            should_find_differences: false,
            mock_differences: Vec::new(),
            headers_comparison_enabled: false,
            diff_view_style: DiffViewStyle::Unified,
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

impl Default for MockResponseComparator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseComparator for MockResponseComparator {
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

/// Mock test runner for testing
pub struct MockTestRunner {
    pub mock_results: Vec<ComparisonResult>,
    pub should_fail: bool,
    pub failure_message: String,
}

impl MockTestRunner {
    pub fn new() -> Self {
        Self {
            mock_results: Vec::new(),
            should_fail: false,
            failure_message: "Mock test runner failure".to_string(),
        }
    }

    pub fn with_results(mut self, results: Vec<ComparisonResult>) -> Self {
        self.mock_results = results;
        self
    }

    pub fn with_failure(mut self, message: String) -> Self {
        self.should_fail = true;
        self.failure_message = message;
        self
    }
}

impl Default for MockTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRunner for MockTestRunner {
    async fn execute_with_data(
        &self,
        _user_data: &[crate::config::UserData],
        _environments: Option<Vec<String>>,
        _routes: Option<Vec<String>>,
        _progress_callback: Option<Box<dyn Fn(&ProgressTracker) + Send + Sync>>,
    ) -> Result<crate::types::ExecutionResult> {
        if self.should_fail {
            return Err(crate::error::HttpDiffError::general(&self.failure_message));
        }
        let progress = ProgressTracker::new(self.mock_results.len());
        Ok(crate::types::ExecutionResult::new(
            self.mock_results.clone(),
            progress,
            Vec::new(), // No errors in mock
            None, // No chain metadata in mock
        ))
    }
}

/// Mock condition evaluator for testing
pub struct MockConditionEvaluator {
    pub should_execute_results: HashMap<String, bool>,
    pub default_result: bool,
    pub should_fail: bool,
    pub failure_message: String,
}

impl MockConditionEvaluator {
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

impl Default for MockConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionEvaluator for MockConditionEvaluator {
    fn should_execute_route(&self, route: &Route, _user_data: &UserData) -> Result<bool> {
        if self.should_fail {
            return Err(crate::error::HttpDiffError::condition_evaluation_failed(
                &route.name,
                &self.failure_message,
            ));
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
    ) -> Result<Vec<ConditionResult>> {
        if self.should_fail {
            return Err(crate::error::HttpDiffError::general(&self.failure_message));
        }

        // Create mock results for all conditions
        let results = conditions
            .iter()
            .map(|condition| ConditionResult {
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
// CHAIN-AWARE MOCK IMPLEMENTATIONS
// =============================================================================

/// Mock HTTP client with chain-aware capabilities for testing
#[derive(Clone)]
pub struct ChainAwareMockHttpClient {
    pub responses: HashMap<String, HttpResponse>,
    pub extraction_responses: HashMap<String, HashMap<String, String>>, // route -> extracted values
    pub should_fail_on_route: HashMap<String, String>, // route -> error message
    pub should_simulate_extraction_failure: HashMap<String, Vec<String>>, // route -> failed extraction names
}

impl ChainAwareMockHttpClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            extraction_responses: HashMap::new(),
            should_fail_on_route: HashMap::new(),
            should_simulate_extraction_failure: HashMap::new(),
        }
    }

    pub fn with_chain_responses(mut self, responses: HashMap<String, HttpResponse>) -> Self {
        self.responses = responses;
        self
    }

    pub fn with_extraction_values(
        mut self, 
        route: String, 
        extractions: HashMap<String, String>
    ) -> Self {
        self.extraction_responses.insert(route, extractions);
        self
    }

    pub fn with_route_failure(mut self, route: String, message: String) -> Self {
        self.should_fail_on_route.insert(route, message);
        self
    }

    pub fn with_extraction_failure(mut self, route: String, failed_extractions: Vec<String>) -> Self {
        self.should_simulate_extraction_failure.insert(route, failed_extractions);
        self
    }

    /// Simulate value extraction from response for testing
    pub fn extract_values(&self, route: &Route, response: &HttpResponse) -> Result<HashMap<String, String>> {
        let mut extracted_values = HashMap::new();

        if let Some(extractions) = &route.extract {
            for extraction_rule in extractions {
                // Check if this extraction should fail
                if let Some(failed_extractions) = self.should_simulate_extraction_failure.get(&route.name) {
                    if failed_extractions.contains(&extraction_rule.name) {
                        if extraction_rule.required {
                            return Err(crate::error::HttpDiffError::general(
                                format!("Mock extraction failure for required field: {}", extraction_rule.name)
                            ));
                        } else {
                            // Use default value or skip
                            if let Some(default) = &extraction_rule.default_value {
                                extracted_values.insert(extraction_rule.name.clone(), default.clone());
                            }
                            continue;
                        }
                    }
                }

                // Use predefined extraction values if available
                if let Some(route_extractions) = self.extraction_responses.get(&route.name) {
                    if let Some(value) = route_extractions.get(&extraction_rule.name) {
                        extracted_values.insert(extraction_rule.name.clone(), value.clone());
                        continue;
                    }
                }

                // Otherwise, simulate extraction based on type
                let extracted_value = match extraction_rule.extractor_type {
                    ExtractorType::JsonPath => {
                        // Simulate JsonPath extraction
                        match extraction_rule.source.as_str() {
                            "$.token" => Some("mock_token_123".to_string()),
                            "$.user_id" => Some("1001".to_string()),
                            "$.users[0].id" => Some("1".to_string()),
                            "$.email" => Some("test@example.com".to_string()),
                            "$.profile.department" => Some("Engineering".to_string()),
                            _ => extraction_rule.default_value.clone(),
                        }
                    },
                    ExtractorType::Regex => {
                        // Simulate regex extraction
                        if extraction_rule.source.contains("token=") {
                            Some("regex_token_456".to_string())
                        } else {
                            extraction_rule.default_value.clone()
                        }
                    },
                    ExtractorType::Header => {
                        // Simulate header extraction
                        response.headers.get(&extraction_rule.source).cloned()
                            .or_else(|| extraction_rule.default_value.clone())
                    },
                    ExtractorType::StatusCode => {
                        // Extract status code
                        Some(response.status.to_string())
                    },
                };

                if let Some(value) = extracted_value {
                    extracted_values.insert(extraction_rule.name.clone(), value);
                } else if extraction_rule.required {
                    return Err(crate::error::HttpDiffError::general(
                        format!("Required extraction failed: {}", extraction_rule.name)
                    ));
                }
            }
        }

        Ok(extracted_values)
    }
}

impl Default for ChainAwareMockHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for ChainAwareMockHttpClient {
    async fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        _user_data: &UserData,
    ) -> Result<HttpResponse> {
        // Check if this route should fail
        if let Some(error_message) = self.should_fail_on_route.get(&route.name) {
            return Err(crate::error::HttpDiffError::general(error_message));
        }

        let key = format!("{}:{}", route.name, environment);
        self.responses.get(&key).cloned().ok_or_else(|| {
            crate::error::HttpDiffError::general(format!(
                "Chain-aware mock response not found for key: {}",
                key
            ))
        })
    }
}

/// Mock test runner specifically for chain testing scenarios
pub struct MockChainTestRunner {
    pub execution_results: HashMap<String, Vec<ComparisonResult>>, // user_data_key -> results
    pub should_fail_on_user: HashMap<String, String>, // user_data_key -> error message
    pub simulate_extraction_context: bool,
    pub mock_extracted_values: HashMap<String, HashMap<String, String>>, // route -> extractions
}

impl MockChainTestRunner {
    pub fn new() -> Self {
        Self {
            execution_results: HashMap::new(),
            should_fail_on_user: HashMap::new(),
            simulate_extraction_context: false,
            mock_extracted_values: HashMap::new(),
        }
    }

    pub fn with_user_results(mut self, user_key: String, results: Vec<ComparisonResult>) -> Self {
        self.execution_results.insert(user_key, results);
        self
    }

    pub fn with_user_failure(mut self, user_key: String, message: String) -> Self {
        self.should_fail_on_user.insert(user_key, message);
        self
    }

    pub fn with_extraction_context(mut self) -> Self {
        self.simulate_extraction_context = true;
        self
    }

    pub fn with_extracted_values(
        mut self,
        route: String,
        extractions: HashMap<String, String>
    ) -> Self {
        self.mock_extracted_values.insert(route, extractions);
        self
    }

    /// Generate a key for user data to use in lookups
    fn user_data_key(user_data: &UserData) -> String {
        if let Some(user_id) = user_data.data.get("userId") {
            user_id.clone()
        } else if let Some(user_name) = user_data.data.get("userName") {
            user_name.clone()
        } else {
            "default_user".to_string()
        }
    }
}

impl Default for MockChainTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRunner for MockChainTestRunner {
    async fn execute_with_data(
        &self,
        user_data: &[crate::config::UserData],
        _environments: Option<Vec<String>>,
        _routes: Option<Vec<String>>,
        _progress_callback: Option<Box<dyn Fn(&ProgressTracker) + Send + Sync>>,
    ) -> Result<crate::types::ExecutionResult> {
        let mut all_results = Vec::new();
        let mut all_errors = Vec::new();

        for user in user_data {
            let user_key = Self::user_data_key(user);

            // Check if this user should fail
            if let Some(error_message) = self.should_fail_on_user.get(&user_key) {
                all_errors.push(ExecutionError::general_execution_error(error_message.clone()));
                continue;
            }

            // Get results for this user
            if let Some(user_results) = self.execution_results.get(&user_key) {
                all_results.extend(user_results.clone());
            }
        }

        let progress = ProgressTracker::new(all_results.len());
        Ok(crate::types::ExecutionResult::new(
            all_results,
            progress,
            all_errors,
            None, // No chain metadata in mock
        ))
    }
}

/// Builder for creating comprehensive chain test scenarios
pub struct ChainTestScenarioBuilder {
    http_client: ChainAwareMockHttpClient,
    test_runner: MockChainTestRunner,
    routes: Vec<Route>,
    user_data: Vec<UserData>,
}

impl ChainTestScenarioBuilder {
    pub fn new() -> Self {
        Self {
            http_client: ChainAwareMockHttpClient::new(),
            test_runner: MockChainTestRunner::new(),
            routes: Vec::new(),
            user_data: Vec::new(),
        }
    }

    pub fn with_login_chain(mut self) -> Self {
        // Add basic login -> list -> detail chain
        let login_route = Route {
            name: "login".to_string(),
            method: "POST".to_string(),
            path: "/auth/login".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: Some(r#"{"username": "test"}"#.to_string()),
            conditions: None,
            extract: Some(vec![
                ValueExtractionRule {
                    name: "auth_token".to_string(),
                    extractor_type: ExtractorType::JsonPath,
                    source: "$.token".to_string(),
                    default_value: None,
                    required: true,
                },
            ]),
            depends_on: None,
            wait_for_extraction: None,
        };

        let list_route = Route {
            name: "list".to_string(),
            method: "GET".to_string(),
            path: "/api/items".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer {auth_token}".to_string());
                headers
            }),
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: Some(vec![
                ValueExtractionRule {
                    name: "first_item_id".to_string(),
                    extractor_type: ExtractorType::JsonPath,
                    source: "$.items[0].id".to_string(),
                    default_value: None,
                    required: true,
                },
            ]),
            depends_on: Some(vec!["login".to_string()]),
            wait_for_extraction: Some(true),
        };

        let detail_route = Route {
            name: "detail".to_string(),
            method: "GET".to_string(),
            path: "/api/items/{first_item_id}".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer {auth_token}".to_string());
                headers
            }),
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: None,
            depends_on: Some(vec!["list".to_string()]),
            wait_for_extraction: Some(true),
        };

        self.routes.extend(vec![login_route, list_route, detail_route]);

        // Add corresponding mock responses
        let mut responses = HashMap::new();
        responses.insert(
            "login:dev".to_string(),
            create_mock_response(200, r#"{"token": "test_token_123"}"#)
        );
        responses.insert(
            "list:dev".to_string(),
            create_mock_response(200, r#"{"items": [{"id": "item_1"}, {"id": "item_2"}]}"#)
        );
        responses.insert(
            "detail:dev".to_string(),
            create_mock_response(200, r#"{"id": "item_1", "name": "Test Item", "status": "active"}"#)
        );

        self.http_client = self.http_client.with_chain_responses(responses);
        self
    }

    pub fn with_extraction_failure(mut self, route: String, failed_extractions: Vec<String>) -> Self {
        self.http_client = self.http_client.with_extraction_failure(route, failed_extractions);
        self
    }

    pub fn with_network_failure(mut self, route: String) -> Self {
        self.http_client = self.http_client.with_route_failure(route, "Network failure".to_string());
        self
    }

    pub fn with_test_user(mut self, user_data: UserData) -> Self {
        self.user_data.push(user_data);
        self
    }

    pub fn build(self) -> (ChainAwareMockHttpClient, MockChainTestRunner, Vec<Route>, Vec<UserData>) {
        (self.http_client, self.test_runner, self.routes, self.user_data)
    }
}

impl Default for ChainTestScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating test data
pub mod test_helpers {
    use super::*;

    pub fn create_mock_response(status: u16, body: &str) -> HttpResponse {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        HttpResponse {
            status,
            headers,
            body: body.to_string(),
            url: "https://example.com/test".to_string(),
            curl_command: "curl 'https://example.com/test'".to_string(),
        }
    }

    pub fn create_mock_route(name: &str, method: &str, path: &str) -> Route {
        Route {
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

    pub fn create_mock_route_with_conditions(
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

    pub fn create_mock_user_data(data: Vec<(&str, &str)>) -> UserData {
        let mut user_data = HashMap::new();
        for (key, value) in data {
            user_data.insert(key.to_string(), value.to_string());
        }
        UserData { data: user_data }
    }

    pub fn create_mock_difference(category: DifferenceCategory, description: &str) -> Difference {
        Difference::new(category, description.to_string())
    }

    /// Create a route with extraction rules for chain testing
    pub fn create_mock_route_with_extraction(
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

    /// Create a mock extraction rule
    pub fn create_mock_extraction_rule(
        name: &str,
        extractor_type: ExtractorType,
        source: &str,
        required: bool,
        default_value: Option<String>,
    ) -> ValueExtractionRule {
        ValueExtractionRule {
            name: name.to_string(),
            extractor_type,
            source: source.to_string(),
            default_value,
            required,
        }
    }

    /// Create a mock response with custom headers for extraction testing
    pub fn create_mock_response_with_headers(
        status: u16,
        body: &str,
        custom_headers: HashMap<String, String>,
    ) -> HttpResponse {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.extend(custom_headers);

        HttpResponse {
            status,
            headers,
            body: body.to_string(),
            url: "https://example.com/test".to_string(),
            curl_command: "curl 'https://example.com/test'".to_string(),
        }
    }

    /// Create a complete chain configuration for testing
    pub fn create_chain_config_for_testing() -> (Vec<Route>, Vec<UserData>) {
        let routes = vec![
            // Step 1: Login
            create_mock_route_with_extraction(
                "login",
                "POST",
                "/auth/login",
                vec![
                    create_mock_extraction_rule(
                        "auth_token",
                        ExtractorType::JsonPath,
                        "$.token",
                        true,
                        None,
                    ),
                    create_mock_extraction_rule(
                        "user_id",
                        ExtractorType::JsonPath,
                        "$.user_id",
                        false,
                        Some("0".to_string()),
                    ),
                ],
                None,
            ),
            // Step 2: Get user list (depends on login)
            create_mock_route_with_extraction(
                "user_list",
                "GET",
                "/api/users",
                vec![
                    create_mock_extraction_rule(
                        "first_user_id",
                        ExtractorType::JsonPath,
                        "$.users[0].id",
                        true,
                        None,
                    ),
                ],
                Some(vec!["login".to_string()]),
            ),
            // Step 3: Get user details (depends on user_list)
            create_mock_route_with_extraction(
                "user_detail",
                "GET",
                "/api/users/{first_user_id}",
                vec![
                    create_mock_extraction_rule(
                        "user_email",
                        ExtractorType::JsonPath,
                        "$.email",
                        false,
                        Some("unknown@example.com".to_string()),
                    ),
                ],
                Some(vec!["user_list".to_string()]),
            ),
        ];

        let user_data = vec![
            create_mock_user_data(vec![
                ("userId", "test_user_1"),
                ("userName", "Alice"),
                ("environment", "test"),
            ]),
            create_mock_user_data(vec![
                ("userId", "test_user_2"),
                ("userName", "Bob"),
                ("environment", "test"),
            ]),
        ];

        (routes, user_data)
    }

    /// Create mock responses for a complete chain scenario
    pub fn create_chain_mock_responses() -> HashMap<String, HttpResponse> {
        let mut responses = HashMap::new();

        // Login responses
        responses.insert(
            "login:dev".to_string(),
            create_mock_response(200, r#"{"token": "dev_auth_token", "user_id": 1001}"#)
        );
        responses.insert(
            "login:staging".to_string(),
            create_mock_response(200, r#"{"token": "staging_auth_token", "user_id": 1001}"#)
        );

        // User list responses
        responses.insert(
            "user_list:dev".to_string(),
            create_mock_response(200, r#"{"users": [{"id": 101, "name": "Alice"}, {"id": 102, "name": "Bob"}]}"#)
        );
        responses.insert(
            "user_list:staging".to_string(),
            create_mock_response(200, r#"{"users": [{"id": 101, "name": "Alice"}, {"id": 102, "name": "Bob"}]}"#)
        );

        // User detail responses
        responses.insert(
            "user_detail:dev".to_string(),
            create_mock_response(200, r#"{"id": 101, "name": "Alice", "email": "alice@dev.example.com", "role": "admin"}"#)
        );
        responses.insert(
            "user_detail:staging".to_string(),
            create_mock_response(200, r#"{"id": 101, "name": "Alice", "email": "alice@staging.example.com", "role": "admin"}"#)
        );

        responses
    }

    /// Create mock responses with extraction failures
    pub fn create_chain_mock_responses_with_failures() -> HashMap<String, HttpResponse> {
        let mut responses = HashMap::new();

        // Login response missing token (extraction failure)
        responses.insert(
            "login:dev".to_string(),
            create_mock_response(200, r#"{"user_id": 1001, "message": "login successful"}"#) // Missing token
        );

        // User list with valid response
        responses.insert(
            "user_list:dev".to_string(),
            create_mock_response(200, r#"{"users": [{"id": 101, "name": "Alice"}]}"#)
        );

        responses
    }

    /// Create a scenario for testing extraction types
    pub fn create_extraction_types_scenario() -> (Route, HttpResponse) {
        let mut custom_headers = HashMap::new();
        custom_headers.insert("X-Request-ID".to_string(), "req-123-456".to_string());
        custom_headers.insert("X-Session-Token".to_string(), "session-token=abc123".to_string());

        let route = create_mock_route_with_extraction(
            "extraction_test",
            "GET",
            "/api/test",
            vec![
                create_mock_extraction_rule(
                    "json_id",
                    ExtractorType::JsonPath,
                    "$.data.id",
                    false,
                    Some("default_id".to_string()),
                ),
                create_mock_extraction_rule(
                    "regex_token",
                    ExtractorType::Regex,
                    r"session-token=([a-zA-Z0-9]+)",
                    false,
                    None,
                ),
                create_mock_extraction_rule(
                    "request_id",
                    ExtractorType::Header,
                    "X-Request-ID",
                    false,
                    None,
                ),
                create_mock_extraction_rule(
                    "response_status",
                    ExtractorType::StatusCode,
                    "",
                    false,
                    Some("200".to_string()),
                ),
            ],
            None,
        );

        let response = create_mock_response_with_headers(
            200,
            r#"{"data": {"id": "extracted_id_123", "name": "test"}}"#,
            custom_headers,
        );

        (route, response)
    }
}
