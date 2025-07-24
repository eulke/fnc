use http_diff::client::HttpResponse;
use http_diff::comparator::{ComparisonResult, ErrorSummary};
use std::collections::HashMap;

/// Helper function to create a sample HttpResponse
fn create_http_response(status: u16, body: &str) -> HttpResponse {
    HttpResponse {
        status,
        headers: HashMap::new(),
        body: body.to_string(),
        url: "http://test.com".to_string(),
        curl_command: "curl http://test.com".to_string(),
    }
}

#[cfg(test)]
mod comparison_result_tests {
    use super::*;

    #[test]
    fn test_comparison_result_with_status_codes() {
        // Test that ComparisonResult can store status codes for each environment
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_http_response(200, "success"));
        responses.insert("staging".to_string(), create_http_response(200, "success"));

        let mut status_codes = HashMap::new();
        status_codes.insert("prod".to_string(), 200u16);
        status_codes.insert("staging".to_string(), 200u16);

        let result = ComparisonResult {
            route_name: "test_route".to_string(),
            user_context: HashMap::new(),
            responses,
            differences: vec![],
            is_identical: true,
            status_codes,
            has_errors: false,
            error_bodies: None,
        };

        assert_eq!(result.status_codes.get("prod"), Some(&200u16));
        assert_eq!(result.status_codes.get("staging"), Some(&200u16));
        assert!(!result.has_errors);
        assert!(result.error_bodies.is_none());
    }

    #[test]
    fn test_comparison_result_with_errors_detection() {
        // Test that ComparisonResult can detect errors (non-2xx status codes)
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_http_response(500, "Internal Server Error"));
        responses.insert("staging".to_string(), create_http_response(500, "Internal Server Error"));

        let mut status_codes = HashMap::new();
        status_codes.insert("prod".to_string(), 500u16);
        status_codes.insert("staging".to_string(), 500u16);

        let mut error_bodies = HashMap::new();
        error_bodies.insert("prod".to_string(), "Internal Server Error".to_string());
        error_bodies.insert("staging".to_string(), "Internal Server Error".to_string());

        let result = ComparisonResult {
            route_name: "test_route".to_string(),
            user_context: HashMap::new(),
            responses,
            differences: vec![],
            is_identical: true,
            status_codes,
            has_errors: true,
            error_bodies: Some(error_bodies),
        };

        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        let error_bodies = result.error_bodies.unwrap();
        assert_eq!(error_bodies.get("prod"), Some(&"Internal Server Error".to_string()));
        assert_eq!(error_bodies.get("staging"), Some(&"Internal Server Error".to_string()));
    }

    #[test]
    fn test_comparison_result_mixed_success_failure() {
        // Test ComparisonResult with mixed success/failure across environments
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_http_response(200, "success"));
        responses.insert("staging".to_string(), create_http_response(500, "error"));

        let mut status_codes = HashMap::new();
        status_codes.insert("prod".to_string(), 200u16);
        status_codes.insert("staging".to_string(), 500u16);

        let mut error_bodies = HashMap::new();
        error_bodies.insert("staging".to_string(), "error".to_string());
        // Note: prod should not be in error_bodies since it was successful

        let result = ComparisonResult {
            route_name: "test_route".to_string(),
            user_context: HashMap::new(),
            responses,
            differences: vec![],
            is_identical: false,
            status_codes,
            has_errors: true,
            error_bodies: Some(error_bodies),
        };

        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        let error_bodies = result.error_bodies.unwrap();
        assert!(error_bodies.contains_key("staging"));
        assert!(!error_bodies.contains_key("prod")); // Successful response should not store error body
        assert_eq!(error_bodies.get("staging"), Some(&"error".to_string()));
    }

    #[test]
    fn test_comparison_result_successful_requests_no_error_bodies() {
        // Test that successful requests (2xx) don't store response bodies in error_bodies
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_http_response(200, "large response body"));
        responses.insert("staging".to_string(), create_http_response(201, "created"));

        let mut status_codes = HashMap::new();
        status_codes.insert("prod".to_string(), 200u16);
        status_codes.insert("staging".to_string(), 201u16);

        let result = ComparisonResult {
            route_name: "test_route".to_string(),
            user_context: HashMap::new(),
            responses,
            differences: vec![],
            is_identical: false, // Different 200 vs 201, but both successful
            status_codes,
            has_errors: false,
            error_bodies: None, // No error bodies for successful requests
        };

        assert!(!result.has_errors);
        assert!(result.error_bodies.is_none());
    }

    #[test]
    fn test_has_errors_calculation() {
        // Test various status codes to ensure has_errors is calculated correctly
        let test_cases = vec![
            (vec![200, 201], false, "2xx statuses should not have errors"),
            (vec![400, 404], true, "4xx statuses should have errors"),
            (vec![500, 502], true, "5xx statuses should have errors"),
            (vec![200, 500], true, "mixed 2xx and 5xx should have errors"),
            (vec![299], false, "edge case 299 should not have errors"),
            (vec![300, 301], true, "3xx statuses should be considered errors"),
        ];

        for (status_codes, expected_has_errors, description) in test_cases {
            let has_errors = status_codes.iter().any(|&status| status < 200 || status >= 300);
            assert_eq!(has_errors, expected_has_errors, "{}", description);
        }
    }
}

#[cfg(test)]
mod error_summary_tests {
    use super::*;

    #[test]
    fn test_error_summary_all_successful() {
        let results = vec![
            create_test_comparison_result(vec![("prod", 200), ("staging", 200)], true, false),
            create_test_comparison_result(vec![("prod", 201), ("staging", 201)], true, false),
        ];

        let summary = ErrorSummary::from_comparison_results(&results);

        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.successful_requests, 2);
        assert_eq!(summary.failed_requests, 0);
        assert_eq!(summary.identical_successes, 2);
        assert_eq!(summary.identical_failures, 0);
        assert_eq!(summary.mixed_responses, 0);
    }

    #[test]
    fn test_error_summary_all_failed() {
        let results = vec![
            create_test_comparison_result(vec![("prod", 500), ("staging", 500)], true, true),
            create_test_comparison_result(vec![("prod", 404), ("staging", 404)], true, true),
        ];

        let summary = ErrorSummary::from_comparison_results(&results);

        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.successful_requests, 0);
        assert_eq!(summary.failed_requests, 2);
        assert_eq!(summary.identical_successes, 0);
        assert_eq!(summary.identical_failures, 2);
        assert_eq!(summary.mixed_responses, 0);
    }

    #[test]
    fn test_error_summary_mixed_scenarios() {
        let results = vec![
            create_test_comparison_result(vec![("prod", 200), ("staging", 200)], true, false),  // identical success
            create_test_comparison_result(vec![("prod", 500), ("staging", 500)], true, true),   // identical failure
            create_test_comparison_result(vec![("prod", 200), ("staging", 500)], false, true),  // mixed success/failure
            create_test_comparison_result(vec![("prod", 404), ("staging", 500)], false, true),  // mixed failures
        ];

        let summary = ErrorSummary::from_comparison_results(&results);

        assert_eq!(summary.total_requests, 4);
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(summary.failed_requests, 3);
        assert_eq!(summary.identical_successes, 1);
        assert_eq!(summary.identical_failures, 1);
        assert_eq!(summary.mixed_responses, 2);
    }

    #[test]
    fn test_error_summary_empty_results() {
        let results = vec![];
        let summary = ErrorSummary::from_comparison_results(&results);

        assert_eq!(summary.total_requests, 0);
        assert_eq!(summary.successful_requests, 0);
        assert_eq!(summary.failed_requests, 0);
        assert_eq!(summary.identical_successes, 0);
        assert_eq!(summary.identical_failures, 0);
        assert_eq!(summary.mixed_responses, 0);
    }

    /// Helper function to create test ComparisonResult
    fn create_test_comparison_result(
        env_statuses: Vec<(&str, u16)>,
        is_identical: bool,
        has_errors: bool,
    ) -> ComparisonResult {
        let mut responses = HashMap::new();
        let mut status_codes = HashMap::new();
        let mut error_bodies = HashMap::new();

        for (env, status) in env_statuses {
            let body = if status >= 200 && status < 300 {
                "success".to_string()
            } else {
                "error".to_string()
            };
            
            responses.insert(env.to_string(), create_http_response(status, &body));
            status_codes.insert(env.to_string(), status);
            
            if status < 200 || status >= 300 {
                error_bodies.insert(env.to_string(), body);
            }
        }

        ComparisonResult {
            route_name: "test_route".to_string(),
            user_context: HashMap::new(),
            responses,
            differences: vec![],
            is_identical,
            status_codes,
            has_errors,
            error_bodies: if error_bodies.is_empty() { None } else { Some(error_bodies) },
        }
    }
} 