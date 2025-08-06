use crate::config::{Route, UserData};
use crate::error::Result;
use crate::execution::progress::ProgressTracker;
use crate::traits::{
    HttpClient, RequestBuilder, ResponseComparator, ResponseConverter, TestRunner,
};
use crate::types::{ComparisonResult, DiffViewStyle, Difference, DifferenceCategory, HttpResponse};
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
        _error_collector: Option<Box<dyn crate::traits::ErrorCollector>>,
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
        ))
    }
}

/// Mock request builder for testing
pub struct MockRequestBuilder {
    pub should_fail: bool,
    pub failure_message: String,
}

impl MockRequestBuilder {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            failure_message: "Mock request builder failure".to_string(),
        }
    }

    pub fn with_failure(mut self, message: String) -> Self {
        self.should_fail = true;
        self.failure_message = message;
        self
    }
}

impl Default for MockRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestBuilder for MockRequestBuilder {
    async fn build_request(
        &self,
        _route: &Route,
        _environment: &str,
        _user_data: &UserData,
    ) -> Result<reqwest::Request> {
        if self.should_fail {
            return Err(crate::error::HttpDiffError::general(&self.failure_message));
        }

        // Create a basic mock request
        let url = url::Url::parse("https://example.com/test").unwrap();
        let request = reqwest::Request::new(reqwest::Method::GET, url);
        Ok(request)
    }
}

/// Mock response converter for testing
pub struct MockResponseConverter {
    pub mock_response: Option<HttpResponse>,
    pub should_fail: bool,
    pub failure_message: String,
}

impl MockResponseConverter {
    pub fn new() -> Self {
        Self {
            mock_response: None,
            should_fail: false,
            failure_message: "Mock response converter failure".to_string(),
        }
    }

    pub fn with_response(mut self, response: HttpResponse) -> Self {
        self.mock_response = Some(response);
        self
    }

    pub fn with_failure(mut self, message: String) -> Self {
        self.should_fail = true;
        self.failure_message = message;
        self
    }
}

impl Default for MockResponseConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseConverter for MockResponseConverter {
    async fn convert_response(
        &self,
        _response: reqwest::Response,
        curl_command: String,
    ) -> Result<HttpResponse> {
        if self.should_fail {
            return Err(crate::error::HttpDiffError::general(&self.failure_message));
        }

        if let Some(ref mock_response) = self.mock_response {
            let mut response = mock_response.clone();
            response.curl_command = curl_command;
            Ok(response)
        } else {
            // Return a default mock response
            Ok(HttpResponse {
                status: 200,
                headers: HashMap::new(),
                body: "Mock response body".to_string(),
                url: "https://example.com/test".to_string(),
                curl_command,
            })
        }
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
}
