use std::collections::HashMap;

/// Default threshold for large response processing (50KB)
pub const DEFAULT_LARGE_RESPONSE_THRESHOLD: usize = 50_000;

/// HTTP response data with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub url: String,
    pub curl_command: String,
}

impl HttpResponse {
    /// Create a new HTTP response
    pub fn new(
        status: u16,
        headers: HashMap<String, String>,
        body: String,
        url: String,
        curl_command: String,
    ) -> Self {
        Self {
            status,
            headers,
            body,
            url,
            curl_command,
        }
    }

    /// Check if the response indicates success (2xx status code)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Check if the response indicates an error (non-2xx status code)
    pub fn is_error(&self) -> bool {
        !self.is_success()
    }

    /// Get the number of lines in the response body (using efficient shared utility)
    pub fn line_count(&self) -> usize {
        crate::utils::response_summary::count_lines_efficient(&self.body)
    }
}

/// Result of comparing two HTTP responses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComparisonResult {
    pub route_name: String,
    pub user_context: HashMap<String, String>,
    pub responses: HashMap<String, HttpResponse>,
    pub differences: Vec<Difference>,
    pub is_identical: bool,
    // Error tracking fields
    pub status_codes: HashMap<String, u16>, // env_name -> status_code
    pub has_errors: bool,                   // true if any non-2xx status
    pub error_bodies: Option<HashMap<String, String>>, // env_name -> response_body (only for errors)
    /// Optional base environment used for orienting comparisons and diffs
    pub base_environment: Option<String>,
}

impl ComparisonResult {
    /// Create a new comparison result
    pub fn new(route_name: String, user_context: HashMap<String, String>) -> Self {
        Self {
            route_name,
            user_context,
            responses: HashMap::new(),
            differences: Vec::new(),
            is_identical: true,
            status_codes: HashMap::new(),
            has_errors: false,
            error_bodies: None,
            base_environment: None,
        }
    }

    /// Add a response for an environment
    pub fn add_response(&mut self, environment: String, response: HttpResponse) {
        self.status_codes
            .insert(environment.clone(), response.status);
        if response.is_error() {
            self.has_errors = true;
            if self.error_bodies.is_none() {
                self.error_bodies = Some(HashMap::new());
            }
            if let Some(ref mut error_bodies) = self.error_bodies {
                error_bodies.insert(environment.clone(), response.body.clone());
            }
        }
        self.responses.insert(environment, response);
    }

    /// Add a difference between responses
    pub fn add_difference(&mut self, difference: Difference) {
        self.is_identical = false;
        self.differences.push(difference);
    }

    /// Check if all responses have the same status code
    pub fn has_consistent_status(&self) -> bool {
        let statuses: Vec<u16> = self.status_codes.values().copied().collect();
        statuses.windows(2).all(|w| w[0] == w[1])
    }
}

/// Represents a difference between responses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Difference {
    pub category: DifferenceCategory,
    pub description: String,
    pub diff_output: Option<String>,
    /// Structured header diff data (avoids JSON serialization/deserialization)
    pub header_diff: Option<Vec<crate::comparison::analyzer::HeaderDiff>>,
    /// Structured body diff data (avoids JSON serialization/deserialization)
    pub body_diff: Option<crate::comparison::analyzer::BodyDiff>,
}

impl Difference {
    /// Create a new difference
    pub fn new(category: DifferenceCategory, description: String) -> Self {
        Self {
            category,
            description,
            diff_output: None,
            header_diff: None,
            body_diff: None,
        }
    }

    /// Create a new difference with diff output
    pub fn with_diff(
        category: DifferenceCategory,
        description: String,
        diff_output: String,
    ) -> Self {
        Self {
            category,
            description,
            diff_output: Some(diff_output),
            header_diff: None,
            body_diff: None,
        }
    }

    /// Create a header difference with structured data
    pub fn with_header_diff(
        description: String,
        header_diff: Vec<crate::comparison::analyzer::HeaderDiff>,
    ) -> Self {
        Self {
            category: DifferenceCategory::Headers,
            description,
            diff_output: None,
            header_diff: Some(header_diff),
            body_diff: None,
        }
    }

    /// Create a body difference with structured data
    pub fn with_body_diff(
        description: String,
        body_diff: crate::comparison::analyzer::BodyDiff,
    ) -> Self {
        Self {
            category: DifferenceCategory::Body,
            description,
            diff_output: None,
            header_diff: None,
            body_diff: Some(body_diff),
        }
    }
}

/// Categories of differences that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DifferenceCategory {
    Status,
    Headers,
    Body,
}

impl DifferenceCategory {
    /// Get a human-readable name for the category
    pub fn name(&self) -> &'static str {
        match self {
            DifferenceCategory::Status => "Status Code",
            DifferenceCategory::Headers => "Headers",
            DifferenceCategory::Body => "Response Body",
        }
    }
}

/// Error severity classification for failed requests
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorSeverity {
    Critical,   // 5xx errors
    Dependency, // 424, 502, 503 errors
    Client,     // 4xx errors
}

/// Diff view style configuration for text differences
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiffViewStyle {
    /// Traditional unified diff (up/down) - default, backward compatible
    Unified,
    /// Side-by-side diff view for easier comparison
    SideBySide,
}

/// Summary of error statistics across all comparison results
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ErrorSummary {
    pub total_requests: usize,
    pub successful_requests: usize, // 2xx status codes
    pub failed_requests: usize,     // non-2xx status codes
    pub identical_successes: usize, // identical 2xx responses
    pub identical_failures: usize,  // identical non-2xx responses
    pub mixed_responses: usize,     // different status codes across envs
}

impl Default for ErrorSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorSummary {
    /// Create a new empty error summary
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            identical_successes: 0,
            identical_failures: 0,
            mixed_responses: 0,
        }
    }

    /// Calculate error summary from comparison results
    pub fn from_comparison_results(results: &[ComparisonResult]) -> Self {
        let mut summary = Self::new();
        summary.total_requests = results.len();

        for result in results {
            let statuses: Vec<u16> = result.status_codes.values().cloned().collect();
            let all_successful = statuses.iter().all(|&status| (200..300).contains(&status));
            let all_same_status = statuses.windows(2).all(|w| w[0] == w[1]);

            // First check if all status codes are the same
            if all_same_status {
                if all_successful {
                    summary.successful_requests += 1;
                    if result.is_identical {
                        summary.identical_successes += 1;
                    }
                } else {
                    summary.failed_requests += 1;
                    if result.is_identical {
                        summary.identical_failures += 1;
                    }
                }
            } else {
                // Different status codes across environments = mixed responses
                summary.failed_requests += 1;
                summary.mixed_responses += 1;
            }
        }

        summary
    }
}

/// Types of execution errors that can occur during test runs
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExecutionErrorType {
    /// Error during HTTP request execution
    RequestError,
    /// Error during response comparison
    ComparisonError,
    /// General execution error (semaphore, task management, etc.)
    ExecutionError,
}

/// Represents an execution error with context
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionError {
    /// Type of error that occurred
    pub error_type: ExecutionErrorType,
    /// Route name where error occurred
    pub route: String,
    /// Environment name (if applicable)
    pub environment: Option<String>,
    /// Error message
    pub message: String,
}

impl ExecutionError {
    /// Create a new request error
    pub fn request_error(route: String, environment: String, message: String) -> Self {
        Self {
            error_type: ExecutionErrorType::RequestError,
            route,
            environment: Some(environment),
            message,
        }
    }

    /// Create a new comparison error
    pub fn comparison_error(route: String, message: String) -> Self {
        Self {
            error_type: ExecutionErrorType::ComparisonError,
            route,
            environment: None,
            message,
        }
    }

    /// Create a new general execution error
    pub fn general_execution_error(message: String) -> Self {
        Self {
            error_type: ExecutionErrorType::ExecutionError,
            route: "unknown".to_string(),
            environment: None,
            message,
        }
    }
}

/// Comprehensive result of test runner execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    /// Comparison results from successful test runs
    pub comparisons: Vec<ComparisonResult>,
    /// Progress information
    pub progress: crate::execution::progress::ProgressTracker,
    /// Collection of errors that occurred during execution
    pub errors: Vec<ExecutionError>,
}

impl ExecutionResult {
    /// Create a new execution result
    pub fn new(
        comparisons: Vec<ComparisonResult>,
        progress: crate::execution::progress::ProgressTracker,
        errors: Vec<ExecutionError>,
    ) -> Self {
        Self {
            comparisons,
            progress,
            errors,
        }
    }

    /// Check if execution had any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get errors by type
    pub fn errors_by_type(&self, error_type: ExecutionErrorType) -> Vec<&ExecutionError> {
        self.errors
            .iter()
            .filter(|e| e.error_type == error_type)
            .collect()
    }

    /// Get request errors
    pub fn request_errors(&self) -> Vec<&ExecutionError> {
        self.errors_by_type(ExecutionErrorType::RequestError)
    }

    /// Get comparison errors
    pub fn comparison_errors(&self) -> Vec<&ExecutionError> {
        self.errors_by_type(ExecutionErrorType::ComparisonError)
    }

    /// Get execution errors
    pub fn execution_errors(&self) -> Vec<&ExecutionError> {
        self.errors_by_type(ExecutionErrorType::ExecutionError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_response() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = HttpResponse::new(
            200,
            headers,
            r#"{"message": "success"}"#.to_string(),
            "https://api.example.com/test".to_string(),
            "curl -X GET https://api.example.com/test".to_string(),
        );

        assert!(response.is_success());
        assert!(!response.is_error());
        assert_eq!(response.status, 200);
        assert_eq!(response.line_count(), 1);
    }

    #[test]
    fn test_comparison_result() {
        let mut result = ComparisonResult::new("test_route".to_string(), HashMap::new());

        let response = HttpResponse::new(
            200,
            HashMap::new(),
            "test body".to_string(),
            "https://api.example.com".to_string(),
            "curl command".to_string(),
        );

        result.add_response("dev".to_string(), response);
        assert!(!result.has_errors);
        assert!(result.has_consistent_status());
    }

    #[test]
    fn test_error_summary() {
        let summary = ErrorSummary {
            total_requests: 10,
            successful_requests: 8,
            failed_requests: 2,
            identical_successes: 6,
            identical_failures: 1,
            mixed_responses: 1,
        };

        // Test basic summary properties
        assert_eq!(summary.total_requests, 10);
        assert_eq!(summary.successful_requests, 8);
        assert_eq!(summary.failed_requests, 2);
    }

    #[test]
    fn test_difference_category() {
        assert_eq!(DifferenceCategory::Status.name(), "Status Code");
        assert_eq!(DifferenceCategory::Headers.name(), "Headers");
        assert_eq!(DifferenceCategory::Body.name(), "Response Body");
    }
}
