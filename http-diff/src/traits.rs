use crate::config::{Route, UserData};
use crate::conditions::{ConditionResult, ExecutionCondition};
use crate::error::Result;
use crate::types::{ComparisonResult, HttpResponse, ExtractionRule, ExtractionResult, ValueExtractionContext};
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

/// Trait for test runners with clean architecture
pub trait TestRunner: Send + Sync {
    /// Execute HTTP diff tests with provided user data and comprehensive error handling
    fn execute_with_data(
        &self,
        user_data: &[crate::config::UserData],
        environments: Option<Vec<String>>,
        routes: Option<Vec<String>>,
        progress_callback: Option<ProgressCallback>,
    ) -> impl Future<Output = Result<crate::types::ExecutionResult>> + Send;
}

/// Trait for configuration validation
pub trait ConfigValidator: Send + Sync {
    type Config;

    /// Validate configuration
    fn validate(&self, config: &Self::Config) -> Result<()>;
}

/// Trait for evaluating execution conditions
pub trait ConditionEvaluator: Send + Sync {
    /// Evaluate if a route should be executed based on conditions and user data
    fn should_execute_route(&self, route: &Route, user_data: &UserData) -> Result<bool>;

    /// Evaluate all conditions for a route
    fn evaluate_conditions(
        &self,
        conditions: &[ExecutionCondition],
        user_data: &UserData,
    ) -> Result<Vec<ConditionResult>>;
}

/// Trait for value extraction from HTTP responses
pub trait ValueExtractor: Send + Sync {
    /// Extract values from an HTTP response according to extraction rules
    fn extract_values(
        &self,
        context: &ValueExtractionContext,
        rules: &[ExtractionRule],
    ) -> Result<ExtractionResult>;

    /// Extract a single value using a specific rule
    fn extract_single_value(
        &self,
        context: &ValueExtractionContext,
        rule: &ExtractionRule,
    ) -> Result<Option<String>>;

    /// Check if this extractor supports the given extraction rule
    fn supports_rule(&self, rule: &ExtractionRule) -> bool;
}

/// Type alias for progress callback to reduce complexity
pub type ProgressCallback = Box<dyn Fn(&crate::execution::progress::ProgressTracker) + Send + Sync>;
