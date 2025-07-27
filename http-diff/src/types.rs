
use std::collections::HashMap;

/// HTTP response data with metadata
#[derive(Debug, Clone)]
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

    /// Get the content type from headers
    pub fn content_type(&self) -> Option<&String> {
        self.headers.get("content-type")
            .or_else(|| self.headers.get("Content-Type"))
    }

    /// Get the response size in bytes
    pub fn size(&self) -> usize {
        self.body.len()
    }

    /// Get the number of lines in the response body
    pub fn line_count(&self) -> usize {
        self.body.lines().count()
    }
}

/// Result of comparing two HTTP responses
#[derive(Debug, Clone)]
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
        }
    }

    /// Add a response for an environment
    pub fn add_response(&mut self, environment: String, response: HttpResponse) {
        self.status_codes.insert(environment.clone(), response.status);
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
        let statuses: Vec<u16> = self.status_codes.values().cloned().collect();
        statuses.windows(2).all(|w| w[0] == w[1])
    }

    /// Get the primary environment (first one in responses)
    pub fn primary_environment(&self) -> Option<&String> {
        self.responses.keys().next()
    }

    /// Get all environment names
    pub fn environments(&self) -> Vec<&String> {
        self.responses.keys().collect()
    }
}

/// Represents a difference between responses
#[derive(Debug, Clone)]
pub struct Difference {
    pub category: DifferenceCategory,
    pub description: String,
    pub diff_output: Option<String>,
}

impl Difference {
    /// Create a new difference
    pub fn new(category: DifferenceCategory, description: String) -> Self {
        Self {
            category,
            description,
            diff_output: None,
        }
    }

    /// Create a new difference with diff output
    pub fn with_diff(category: DifferenceCategory, description: String, diff_output: String) -> Self {
        Self {
            category,
            description,
            diff_output: Some(diff_output),
        }
    }
}

/// Categories of differences that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Get an emoji icon for the category
    pub fn icon(&self) -> &'static str {
        match self {
            DifferenceCategory::Status => "🚨",
            DifferenceCategory::Headers => "📝",
            DifferenceCategory::Body => "📄",
        }
    }
}

/// Error severity classification for failed requests
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    Critical,    // 5xx errors
    Dependency,  // 424, 502, 503 errors
    Client,      // 4xx errors
}

/// Diff view style configuration for text differences
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffViewStyle {
    /// Traditional unified diff (up/down) - default, backward compatible
    Unified,
    /// Side-by-side diff view for easier comparison
    SideBySide,
}

impl Default for DiffViewStyle {
    fn default() -> Self {
        DiffViewStyle::Unified
    }
}

/// Summary of error statistics across all comparison results
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSummary {
    pub total_requests: usize,
    pub successful_requests: usize, // 2xx status codes
    pub failed_requests: usize,     // non-2xx status codes
    pub identical_successes: usize, // identical 2xx responses
    pub identical_failures: usize,  // identical non-2xx responses
    pub mixed_responses: usize,     // different status codes across envs
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
            let all_successful = statuses.iter().all(|&status| status >= 200 && status < 300);
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

    /// Get the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Get the failure rate as a percentage
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.failed_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Check if there were any failures
    pub fn has_failures(&self) -> bool {
        self.failed_requests > 0
    }

    /// Check if all requests were successful
    pub fn all_successful(&self) -> bool {
        self.total_requests > 0 && self.failed_requests == 0
    }
}

impl Default for ErrorSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a curl command with metadata
#[derive(Debug, Clone)]
pub struct CurlCommand {
    pub route_name: String,
    pub environment: String,
    pub user_context: HashMap<String, String>,
    pub command: String,
}

impl CurlCommand {
    /// Create a new curl command
    pub fn new(
        route_name: String,
        environment: String,
        user_context: HashMap<String, String>,
        command: String,
    ) -> Self {
        Self {
            route_name,
            environment,
            user_context,
            command,
        }
    }

    /// Get a unique identifier for this command
    pub fn id(&self) -> String {
        format!("{}_{}", self.route_name, self.environment)
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        format!("{} on {}", self.route_name, self.environment)
    }
}

/// Configuration for progress tracking during test execution
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    /// Whether to show progress bars
    pub show_progress: bool,
    /// Update interval for progress callbacks (in milliseconds)
    pub update_interval_ms: u64,
    /// Whether to show estimated time remaining
    pub show_eta: bool,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            show_progress: true,
            update_interval_ms: 100,
            show_eta: true,
        }
    }
}

/// Progress tracking information
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub total_requests: usize,
    pub completed_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub start_time: std::time::Instant,
}

impl ProgressInfo {
    /// Create new progress info
    pub fn new(total_requests: usize) -> Self {
        Self {
            total_requests,
            completed_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            start_time: std::time::Instant::now(),
        }
    }

    /// Mark a request as completed
    pub fn complete_request(&mut self, success: bool) {
        self.completed_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
    }

    /// Get progress as a percentage
    pub fn progress_percentage(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.completed_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Estimate remaining time
    pub fn estimated_remaining(&self) -> Option<std::time::Duration> {
        if self.completed_requests == 0 || self.completed_requests >= self.total_requests {
            return None;
        }

        let elapsed = self.elapsed();
        let avg_time_per_request = elapsed.as_secs_f64() / self.completed_requests as f64;
        let remaining_requests = self.total_requests - self.completed_requests;
        let estimated_seconds = avg_time_per_request * remaining_requests as f64;

        Some(std::time::Duration::from_secs_f64(estimated_seconds))
    }

    /// Check if all requests are completed
    pub fn is_complete(&self) -> bool {
        self.completed_requests >= self.total_requests
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
        assert_eq!(response.content_type(), Some(&"application/json".to_string()));
        assert!(response.size() > 0);
    }

    #[test]
    fn test_comparison_result() {
        let mut result = ComparisonResult::new(
            "test_route".to_string(),
            HashMap::new(),
        );

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

        assert_eq!(summary.success_rate(), 80.0);
        assert_eq!(summary.failure_rate(), 20.0);
        assert!(summary.has_failures());
        assert!(!summary.all_successful());
    }

    #[test]
    fn test_progress_info() {
        let mut progress = ProgressInfo::new(10);
        
        assert_eq!(progress.progress_percentage(), 0.0);
        assert!(!progress.is_complete());

        progress.complete_request(true);
        progress.complete_request(false);

        assert_eq!(progress.progress_percentage(), 20.0);
        assert_eq!(progress.successful_requests, 1);
        assert_eq!(progress.failed_requests, 1);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_difference_category() {
        assert_eq!(DifferenceCategory::Status.name(), "Status Code");
        assert_eq!(DifferenceCategory::Headers.name(), "Headers");
        assert_eq!(DifferenceCategory::Body.name(), "Response Body");

        assert_eq!(DifferenceCategory::Status.icon(), "🚨");
        assert_eq!(DifferenceCategory::Headers.icon(), "📝");
        assert_eq!(DifferenceCategory::Body.icon(), "📄");
    }

    #[test]
    fn test_curl_command() {
        let command = CurlCommand::new(
            "get_user".to_string(),
            "dev".to_string(),
            HashMap::new(),
            "curl -X GET https://api.dev.example.com/users".to_string(),
        );

        assert_eq!(command.id(), "get_user_dev");
        assert_eq!(command.description(), "get_user on dev");
    }
} 