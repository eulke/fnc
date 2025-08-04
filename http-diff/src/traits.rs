use crate::config::{Route, UserData};
use crate::types::{HttpResponse, ComparisonResult};
use crate::error::Result;
use std::collections::HashMap;
use std::future::Future;

/// Trait for HTTP client implementations
pub trait HttpClient: Send + Sync + Clone {
    /// Execute a request for a specific route, environment, and user data
    fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> impl Future<Output = Result<HttpResponse>> + Send;
}

/// Trait for response comparison
pub trait ResponseComparator: Send + Sync {
    /// Compare responses from multiple environments
    fn compare_responses(
        &self,
        route_name: String,
        user_context: HashMap<String, String>,
        responses: HashMap<String, HttpResponse>,
    ) -> Result<ComparisonResult>;

    /// Get the configured diff view style
    fn diff_view_style(&self) -> crate::types::DiffViewStyle;

    /// Check if headers comparison is enabled
    fn headers_comparison_enabled(&self) -> bool;
}

/// Trait for test runners
pub trait TestRunner: Send + Sync {
    /// Execute HTTP diff tests
    fn execute(
        &self,
        environments: Option<Vec<String>>,
        routes: Option<Vec<String>>,
    ) -> impl Future<Output = Result<Vec<ComparisonResult>>> + Send;

    /// Execute HTTP diff tests with progress tracking
    fn execute_with_progress(
        &self,
        environments: Option<Vec<String>>,
        routes: Option<Vec<String>>,
        progress_callback: Option<ProgressCallback>,
    ) -> impl Future<Output = Result<(Vec<ComparisonResult>, crate::execution::progress::ProgressTracker)>> + Send;
}

/// Trait for request building
pub trait RequestBuilder: Send + Sync {
    /// Build an HTTP request from route configuration
    fn build_request(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> impl Future<Output = Result<reqwest::Request>> + Send;
}

/// Trait for response conversion
pub trait ResponseConverter: Send + Sync {
    /// Convert reqwest Response to our HttpResponse
    fn convert_response(
        &self,
        response: reqwest::Response,
        curl_command: String,
    ) -> impl Future<Output = Result<HttpResponse>> + Send;
}

/// Trait for URL building
pub trait UrlBuilder: Send + Sync {
    /// Build a complete URL for a request
    fn build_url(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<url::Url>;
}

/// Trait for configuration validation
pub trait ConfigValidator: Send + Sync {
    type Config;
    
    /// Validate configuration
    fn validate(&self, config: &Self::Config) -> Result<()>;
}

/// Type alias for progress callback to reduce complexity
pub type ProgressCallback = Box<dyn Fn(&crate::execution::progress::ProgressTracker) + Send + Sync>;

/// Trait for progress reporting
pub trait ProgressReporter: Send + Sync {
    /// Report progress update
    fn report_progress(&self, progress: &crate::execution::progress::ProgressTracker);
}