/// Response comparison module with pure business logic
pub mod content;
pub mod analyzer;
pub mod validator;

use crate::error::Result;
use crate::types::{HttpResponse, ComparisonResult, DiffViewStyle};
use analyzer::DifferenceAnalyzer;
use validator::ResponseValidator;
use std::collections::HashMap;

/// Response comparator with configurable comparison strategies - pure business logic only
pub struct ResponseComparator {
    analyzer: DifferenceAnalyzer,
    compare_headers: bool,
    diff_view_style: DiffViewStyle,
}

impl ResponseComparator {
    /// Create a new response comparator with default settings
    /// By default, compares only HTTP status and response body (headers comparison disabled)
    pub fn new() -> Self {
        let ignore_headers = vec![
            "date".to_string(),
            "server".to_string(),
            "x-request-id".to_string(),
            "x-correlation-id".to_string(),
        ];

        Self {
            analyzer: DifferenceAnalyzer::new(ignore_headers, true, 50_000),
            compare_headers: false,
            diff_view_style: DiffViewStyle::Unified,
        }
    }

    /// Create a comparator with custom settings
    pub fn with_settings(ignore_headers: Vec<String>, ignore_whitespace: bool) -> Self {
        Self {
            analyzer: DifferenceAnalyzer::new(ignore_headers, ignore_whitespace, 50_000),
            compare_headers: false,
            diff_view_style: DiffViewStyle::Unified,
        }
    }

    /// Create a comparator with full control over all settings
    pub fn with_full_settings(
        ignore_headers: Vec<String>,
        ignore_whitespace: bool,
        compare_headers: bool,
        large_response_threshold: usize,
    ) -> Self {
        Self {
            analyzer: DifferenceAnalyzer::new(ignore_headers, ignore_whitespace, large_response_threshold),
            compare_headers,
            diff_view_style: DiffViewStyle::Unified,
        }
    }

    /// Enable headers comparison (disabled by default)
    pub fn with_headers_comparison(mut self) -> Self {
        self.compare_headers = true;
        self
    }

    /// Set the diff view style (unified or side-by-side)
    pub fn with_diff_view_style(mut self, style: DiffViewStyle) -> Self {
        self.diff_view_style = style;
        self
    }

    /// Enable side-by-side diff view for easier comparison
    pub fn with_side_by_side_diff(mut self) -> Self {
        self.diff_view_style = DiffViewStyle::SideBySide;
        self
    }

    /// Compare responses from multiple environments
    pub fn compare_responses(
        &self,
        route_name: String,
        user_context: HashMap<String, String>,
        responses: HashMap<String, HttpResponse>,
    ) -> Result<ComparisonResult> {
        // Validate responses
        ResponseValidator::validate_responses(&responses)?;

        let mut differences = Vec::new();
        let environments: Vec<String> = responses.keys().cloned().collect();

        // Compare each pair of environments
        for i in 0..environments.len() {
            for j in i + 1..environments.len() {
                let env1 = &environments[i];
                let env2 = &environments[j];

                let response1 = &responses[env1];
                let response2 = &responses[env2];

                let pair_differences = self.analyzer.analyze_responses(
                    response1,
                    response2,
                    env1,
                    env2,
                    self.compare_headers,
                );

                differences.extend(pair_differences);
            }
        }

        let is_identical = differences.is_empty();

        // Extract status codes and error information
        let status_codes = ResponseValidator::extract_status_codes(&responses);
        let has_errors = ResponseValidator::has_error_responses(&responses);
        let error_bodies = if has_errors {
            Some(ResponseValidator::get_error_responses(&responses))
        } else {
            None
        };

        Ok(ComparisonResult {
            route_name,
            user_context,
            responses,
            differences,
            is_identical,
            status_codes,
            has_errors,
            error_bodies,
        })
    }

    /// Get the configured diff view style for use by renderers
    pub fn diff_view_style(&self) -> DiffViewStyle {
        self.diff_view_style.clone()
    }

    /// Check if headers comparison is enabled
    pub fn headers_comparison_enabled(&self) -> bool {
        self.compare_headers
    }

}

impl Default for ResponseComparator {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export key types for convenience
pub use analyzer::{HeaderDiff, BodyDiff};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_response(status: u16, body: &str) -> HttpResponse {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        HttpResponse {
            status,
            headers,
            body: body.to_string(),
            url: "https://example.com/api/test".to_string(),
            curl_command: "curl 'https://example.com/api/test'".to_string(),
        }
    }

    #[test]
    fn test_identical_responses() {
        let comparator = ResponseComparator::new();

        let mut responses = HashMap::new();
        responses.insert(
            "test".to_string(),
            create_test_response(200, r#"{"status": "ok"}"#),
        );
        responses.insert(
            "prod".to_string(),
            create_test_response(200, r#"{"status": "ok"}"#),
        );

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(result.is_identical);
        assert!(result.differences.is_empty());
    }

    #[test]
    fn test_different_status_codes() {
        let comparator = ResponseComparator::new();

        let mut responses = HashMap::new();
        responses.insert(
            "test".to_string(),
            create_test_response(200, r#"{"status": "ok"}"#),
        );
        responses.insert(
            "prod".to_string(),
            create_test_response(404, r#"{"error": "not found"}"#),
        );

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(!result.is_identical);
        assert_eq!(result.differences.len(), 2); // Status + body difference
    }

    #[test]
    fn test_headers_comparison_disabled_by_default() {
        let comparator = ResponseComparator::new();

        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);

        // Add different headers
        response1.headers.insert("X-Version".to_string(), "1.0".to_string());
        response2.headers.insert("X-Version".to_string(), "2.0".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Should be identical because headers comparison is disabled by default
        assert!(result.is_identical);
        assert!(!comparator.headers_comparison_enabled());
    }

    #[test]
    fn test_headers_comparison_enabled() {
        let comparator = ResponseComparator::new().with_headers_comparison();

        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);

        response1.headers.insert("X-Version".to_string(), "1.0".to_string());
        response2.headers.insert("X-Version".to_string(), "2.0".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(!result.is_identical);
        assert!(comparator.headers_comparison_enabled());
    }

    #[test]
    fn test_diff_view_style_configuration() {
        let default_comparator = ResponseComparator::new();
        assert_eq!(default_comparator.diff_view_style(), DiffViewStyle::Unified);

        let side_by_side_comparator = ResponseComparator::new().with_side_by_side_diff();
        assert_eq!(side_by_side_comparator.diff_view_style(), DiffViewStyle::SideBySide);
    }


    #[test]
    fn test_error_response_handling() {
        let comparator = ResponseComparator::new();

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, "ok"));
        responses.insert("prod".to_string(), create_test_response(404, "not found"));

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        
        let error_bodies = result.error_bodies.unwrap();
        assert_eq!(error_bodies.get("prod"), Some(&"not found".to_string()));
    }

    #[test]
    fn test_invalid_response_count() {
        let comparator = ResponseComparator::new();

        // Test with only one response
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, "ok"));

        let result = comparator.compare_responses("test-route".to_string(), HashMap::new(), responses);
        assert!(result.is_err());
        
        // Test with empty responses
        let empty_responses = HashMap::new();
        let result = comparator.compare_responses("test-route".to_string(), HashMap::new(), empty_responses);
        assert!(result.is_err());
    }
}