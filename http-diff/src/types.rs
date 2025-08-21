use std::collections::HashMap;
use crate::utils::environment_utils::{EnvironmentOrderResolver, OrderedEnvironmentResponses, OrderedStatusCodes, EnvironmentValidator};

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

    /// Get ordered responses using environment resolver
    pub fn get_ordered_responses(&self, resolver: &EnvironmentOrderResolver) -> OrderedEnvironmentResponses {
        OrderedEnvironmentResponses::new(resolver, self.responses.clone())
    }

    /// Get ordered status codes using environment resolver
    pub fn get_ordered_status_codes(&self, resolver: &EnvironmentOrderResolver) -> OrderedStatusCodes {
        OrderedStatusCodes::new(resolver, self.status_codes.clone())
    }

    /// Get ordered environment names using resolver
    pub fn get_ordered_environment_names(&self, resolver: &EnvironmentOrderResolver) -> Vec<String> {
        resolver.extract_ordered_environments(&self.responses)
    }

    /// Create environment resolver from this comparison result
    pub fn create_environment_resolver(&self) -> EnvironmentOrderResolver {
        EnvironmentOrderResolver::from_responses(&self.responses, self.base_environment.clone())
    }

    /// Get the first response in deterministic environment order (safe replacement for .iter().next())
    pub fn get_first_response_ordered(&self) -> Option<(String, &HttpResponse)> {
        let resolver = self.create_environment_resolver();
        let ordered_environments = self.get_ordered_environment_names(&resolver);
        if let Some(first_env) = ordered_environments.first() {
            self.responses.get(first_env).map(|response| (first_env.clone(), response))
        } else {
            None
        }
    }

    /// Get the first response data only (for summary displays)
    pub fn get_first_response_data(&self) -> Option<&HttpResponse> {
        self.get_first_response_ordered().map(|(_, response)| response)
    }

    /// Get environment names in deterministic order (cached for performance)
    pub fn get_environment_names_ordered(&self) -> Vec<String> {
        let resolver = self.create_environment_resolver();
        self.get_ordered_environment_names(&resolver)
    }

    /// Validate environment consistency for this result
    pub fn validate_environment_consistency(&self) -> crate::error::Result<()> {
        let resolver = self.create_environment_resolver();
        EnvironmentValidator::validate_comparison_result(self, &resolver)
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
    /// Chain execution metadata (if applicable)
    pub chain_metadata: Option<ChainExecutionMetadata>,
}

impl ExecutionResult {
    /// Create a new execution result
    pub fn new(
        comparisons: Vec<ComparisonResult>,
        progress: crate::execution::progress::ProgressTracker,
        errors: Vec<ExecutionError>,
        chain_metadata: Option<ChainExecutionMetadata>,
    ) -> Self {
        Self {
            comparisons,
            progress,
            errors,
            chain_metadata,
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
    
    /// Check if this was a chain execution
    pub fn is_chain_execution(&self) -> bool {
        self.chain_metadata.is_some() || self.progress.is_chain_execution()
    }
    
    /// Get chain execution metadata if available
    pub fn get_chain_metadata(&self) -> Option<&ChainExecutionMetadata> {
        self.chain_metadata.as_ref()
    }
}

/// Represents a value extracted from an HTTP response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedValue {
    /// The key/name of the extracted value
    pub key: String,
    /// The extracted value as a string
    pub value: String,
    /// The extraction rule that was used
    pub extraction_rule: String,
    /// The extraction type (JsonPath, Regex, Header, StatusCode)
    pub extraction_type: ExtractionType,
    /// The environment from which this value was extracted
    pub environment: String,
    /// The route name from which this value was extracted
    pub route_name: String,
    /// Timestamp when the value was extracted
    pub extracted_at: chrono::DateTime<chrono::Utc>,
}

impl ExtractedValue {
    /// Create a new extracted value
    pub fn new(
        key: String,
        value: String,
        extraction_rule: String,
        extraction_type: ExtractionType,
        environment: String,
        route_name: String,
    ) -> Self {
        Self {
            key,
            value,
            extraction_rule,
            extraction_type,
            environment,
            route_name,
            extracted_at: chrono::Utc::now(),
        }
    }
}

/// Context information for value extraction operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValueExtractionContext {
    /// The route name being processed
    pub route_name: String,
    /// The environment being processed
    pub environment: String,
    /// The HTTP response from which values are being extracted
    pub response: HttpResponse,
    /// User data context for the current request
    pub user_context: HashMap<String, String>,
    /// Timestamp when extraction started
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl ValueExtractionContext {
    /// Create a new value extraction context
    pub fn new(
        route_name: String,
        environment: String,
        response: HttpResponse,
        user_context: HashMap<String, String>,
    ) -> Self {
        Self {
            route_name,
            environment,
            response,
            user_context,
            started_at: chrono::Utc::now(),
        }
    }
}

/// Types of value extraction supported
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ExtractionType {
    /// Extract values using JSONPath expressions
    JsonPath,
    /// Extract values using regular expressions
    Regex,
    /// Extract header values
    Header,
    /// Extract HTTP status code
    StatusCode,
}

impl ExtractionType {
    /// Get a human-readable name for the extraction type
    pub fn name(&self) -> &'static str {
        match self {
            ExtractionType::JsonPath => "JsonPath",
            ExtractionType::Regex => "Regex",
            ExtractionType::Header => "Header",
            ExtractionType::StatusCode => "StatusCode",
        }
    }
}

/// Configuration for a value extraction rule
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionRule {
    /// The key/name for the extracted value
    pub key: String,
    /// The extraction type
    pub extraction_type: ExtractionType,
    /// The extraction pattern/expression
    pub pattern: String,
    /// Optional default value if extraction fails
    pub default_value: Option<String>,
    /// Whether this extraction is required (fails if not found)
    pub required: bool,
}

impl ExtractionRule {
    /// Create a new extraction rule
    pub fn new(
        key: String,
        extraction_type: ExtractionType,
        pattern: String,
    ) -> Self {
        Self {
            key,
            extraction_type,
            pattern,
            default_value: None,
            required: false,
        }
    }

    /// Set the default value for this extraction rule
    pub fn with_default_value(mut self, default_value: String) -> Self {
        self.default_value = Some(default_value);
        self
    }

    /// Mark this extraction rule as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// Result of a value extraction operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionResult {
    /// Successfully extracted values
    pub extracted_values: Vec<ExtractedValue>,
    /// Extraction errors that occurred
    pub errors: Vec<ExtractionError>,
    /// The context in which extraction was performed
    pub context: ValueExtractionContext,
}

impl ExtractionResult {
    /// Create a new extraction result
    pub fn new(context: ValueExtractionContext) -> Self {
        Self {
            extracted_values: Vec::new(),
            errors: Vec::new(),
            context,
        }
    }

    /// Add an extracted value
    pub fn add_value(&mut self, value: ExtractedValue) {
        self.extracted_values.push(value);
    }

    /// Add an extraction error
    pub fn add_error(&mut self, error: ExtractionError) {
        self.errors.push(error);
    }

    /// Check if extraction was successful (no errors)
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if extraction has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get extracted values by key
    pub fn get_value_by_key(&self, key: &str) -> Option<&ExtractedValue> {
        self.extracted_values.iter().find(|v| v.key == key)
    }

    /// Get all extracted values as a key-value map
    pub fn to_key_value_map(&self) -> HashMap<String, String> {
        self.extracted_values
            .iter()
            .map(|v| (v.key.clone(), v.value.clone()))
            .collect()
    }
}

/// Represents an error that occurred during value extraction
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionError {
    /// The extraction rule that failed
    pub rule: ExtractionRule,
    /// Error message describing what went wrong
    pub message: String,
    /// The environment where the error occurred
    pub environment: String,
    /// The route name where the error occurred
    pub route_name: String,
    /// Timestamp when the error occurred
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

impl ExtractionError {
    /// Create a new extraction error
    pub fn new(
        rule: ExtractionRule,
        message: String,
        environment: String,
        route_name: String,
    ) -> Self {
        Self {
            rule,
            message,
            environment,
            route_name,
            occurred_at: chrono::Utc::now(),
        }
    }
}

/// Metadata about chain execution for analysis and debugging
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainExecutionMetadata {
    /// Total number of execution batches
    pub total_batches: usize,
    /// Number of routes with dependencies
    pub dependent_routes: usize,
    /// Number of routes with extraction rules
    pub extraction_routes: usize,
    /// Total number of values extracted across all routes
    pub total_extracted_values: usize,
    /// Number of extraction errors that occurred
    pub extraction_errors: usize,
    /// Execution time per batch (in milliseconds)
    pub batch_execution_times: Vec<u64>,
    /// Whether any routes had to wait for dependency completion
    pub had_dependency_waits: bool,
}

impl ChainExecutionMetadata {
    /// Create new chain execution metadata
    pub fn new(total_batches: usize) -> Self {
        Self {
            total_batches,
            dependent_routes: 0,
            extraction_routes: 0,
            total_extracted_values: 0,
            extraction_errors: 0,
            batch_execution_times: Vec::with_capacity(total_batches),
            had_dependency_waits: false,
        }
    }
    
    /// Record batch execution time
    pub fn add_batch_time(&mut self, time_ms: u64) {
        self.batch_execution_times.push(time_ms);
    }
    
    /// Get average batch execution time
    pub fn average_batch_time(&self) -> f64 {
        if self.batch_execution_times.is_empty() {
            0.0
        } else {
            self.batch_execution_times.iter().sum::<u64>() as f64 / self.batch_execution_times.len() as f64
        }
    }
    
    /// Get the longest batch execution time
    pub fn max_batch_time(&self) -> u64 {
        self.batch_execution_times.iter().copied().max().unwrap_or(0)
    }
    
    /// Get the shortest batch execution time
    pub fn min_batch_time(&self) -> u64 {
        self.batch_execution_times.iter().copied().min().unwrap_or(0)
    }
    
    /// Check if extraction was used
    pub fn used_extraction(&self) -> bool {
        self.total_extracted_values > 0 || self.extraction_routes > 0
    }
    
    /// Get extraction success rate
    pub fn extraction_success_rate(&self) -> f64 {
        if self.extraction_routes == 0 {
            100.0
        } else {
            let successful_extractions = if self.extraction_errors < self.total_extracted_values {
                self.total_extracted_values - self.extraction_errors
            } else {
                0
            };
            (successful_extractions as f64 / self.total_extracted_values as f64) * 100.0
        }
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
