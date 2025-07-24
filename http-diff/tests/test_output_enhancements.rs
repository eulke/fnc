use http_diff::client::HttpResponse;
use http_diff::comparator::{ComparisonResult, ErrorSummary};
use http_diff::output::CurlGenerator;

use std::collections::HashMap;

/// Helper function to create HttpResponse with specific status
fn create_response_with_status(status: u16, body: &str) -> HttpResponse {
    HttpResponse {
        status,
        headers: HashMap::new(),
        body: body.to_string(),
        url: "http://test.com".to_string(),
        curl_command: "curl http://test.com".to_string(),
    }
}

/// Helper function to create ComparisonResult with error tracking
fn create_test_comparison_result(
    route_name: &str,
    env_statuses: Vec<(&str, u16, &str)>, // (env, status, body)
    is_identical: bool,
) -> ComparisonResult {
    let mut responses = HashMap::new();
    let mut status_codes = HashMap::new();
    let mut error_bodies = HashMap::new();
    let mut has_errors = false;

    for (env, status, body) in env_statuses {
        responses.insert(env.to_string(), create_response_with_status(status, body));
        status_codes.insert(env.to_string(), status);
        
        if status < 200 || status >= 300 {
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
        error_bodies: if error_bodies.is_empty() { None } else { Some(error_bodies) },
    }
}

#[cfg(test)]
mod error_analysis_output_tests {
    use super::*;

    #[test]
    fn test_error_analysis_section_appears_when_errors_present() {
        // Test that error analysis section is displayed when there are failed requests
        let results = vec![
            create_test_comparison_result("health", vec![("prod", 500, "Internal Server Error"), ("staging", 500, "Internal Server Error")], true),
            create_test_comparison_result("users", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should contain error analysis section
        assert!(output.contains("==== Error Analysis ===="));
        assert!(output.contains("🚨 1 requests failed (non-2xx status codes)"));
        assert!(output.contains("⚠️  1 requests failed identically across environments"));
    }

    #[test]
    fn test_no_error_analysis_section_when_all_successful() {
        // Test that error analysis section is NOT displayed when all requests are successful
        let results = vec![
            create_test_comparison_result("health", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),
            create_test_comparison_result("users", vec![("prod", 201, "Created"), ("staging", 201, "Created")], true),
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should NOT contain error analysis section
        assert!(!output.contains("==== Error Analysis ===="));
        assert!(!output.contains("🚨"));
        assert!(!output.contains("⚠️"));
        assert!(output.contains("✅ All responses are identical across environments!"));
    }

    #[test]
    fn test_detailed_error_information_formatting() {
        // Test detailed error information display format
        let results = vec![
            create_test_comparison_result("api/health", vec![("prod", 500, "Internal Server Error"), ("staging", 502, "Bad Gateway")], false),
            create_test_comparison_result("api/users", vec![("prod", 404, "Not Found"), ("staging", 404, "Not Found")], true),
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should contain detailed error information
        assert!(output.contains("==== Error Analysis ===="));
        assert!(output.contains("🚨 2 requests failed (non-2xx status codes)"));
        assert!(output.contains("⚠️  1 requests failed identically across environments"));
        assert!(output.contains("🔄 1 requests had different status codes across environments"));
        
        // Should contain specific route error details
        assert!(output.contains("📍 Route 'api/health'"));
        assert!(output.contains("📍 Route 'api/users'"));
        assert!(output.contains("Status codes:"));
        assert!(output.contains("prod: 500"));
        assert!(output.contains("staging: 502"));
        assert!(output.contains("prod: 404"));
        assert!(output.contains("staging: 404"));
    }

    #[test]
    fn test_response_body_display_for_errors() {
        // Test that error response bodies are displayed
        let results = vec![
            create_test_comparison_result("api/auth", 
                vec![("prod", 401, "Unauthorized: Invalid token"), 
                     ("staging", 401, "Unauthorized: Invalid token")], true),
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should display error response bodies
        assert!(output.contains("Response bodies:"));
        assert!(output.contains("prod: Unauthorized: Invalid token"));
        assert!(output.contains("staging: Unauthorized: Invalid token"));
    }

    #[test]
    fn test_response_body_truncation_for_large_errors() {
        // Test that large error response bodies are truncated appropriately
        let large_error_body = "Error: ".to_string() + &"A".repeat(1000); // 1006 chars
        let results = vec![
            create_test_comparison_result("api/upload", 
                vec![("prod", 413, &large_error_body)], false),
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should truncate long response bodies
        assert!(output.contains("Response bodies:"));
        assert!(output.contains("prod: Error: AAA")); // Should start with the error
        assert!(output.contains("... (truncated)")); // Should indicate truncation
        assert!(!output.contains(&"A".repeat(500))); // Should not contain the full body
    }

    #[test]
    fn test_mixed_success_failure_scenarios() {
        // Test output for mixed success/failure scenarios
        let results = vec![
            create_test_comparison_result("health", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),  // success
            create_test_comparison_result("auth", vec![("prod", 200, "OK"), ("staging", 401, "Unauthorized")], false),   // mixed
            create_test_comparison_result("db", vec![("prod", 500, "Database Error"), ("staging", 500, "Database Error")], true),      // identical failure
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should show comprehensive error analysis
        assert!(output.contains("==== Error Analysis ===="));
        assert!(output.contains("🚨 2 requests failed (non-2xx status codes)"));
        assert!(output.contains("⚠️  1 requests failed identically across environments"));
        assert!(output.contains("🔄 1 requests had different status codes across environments"));
        
        // Should show both similarity and error metrics
        assert!(output.contains("🔍 Test Results Summary: 3 total, 2 identical, 1 different"));
    }

    #[test]
    fn test_status_code_grouping_in_output() {
        // Test that status codes are grouped and displayed properly
        let results = vec![
            create_test_comparison_result("api/4xx", vec![("prod", 404, "Not Found"), ("staging", 400, "Bad Request")], false),
            create_test_comparison_result("api/5xx", vec![("prod", 500, "Internal Server Error"), ("staging", 502, "Bad Gateway")], false),
            create_test_comparison_result("api/3xx", vec![("prod", 301, "Moved Permanently"), ("staging", 302, "Found")], false),
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should display all different status codes
        assert!(output.contains("🚨 3 requests failed (non-2xx status codes)"));
        assert!(output.contains("🔄 3 requests had different status codes across environments"));
        
        // Should show each status code
        assert!(output.contains("404")); // 4xx
        assert!(output.contains("400"));
        assert!(output.contains("500")); // 5xx  
        assert!(output.contains("502"));
        assert!(output.contains("301")); // 3xx
        assert!(output.contains("302"));
    }

    #[test]
    fn test_colored_output_formatting_for_error_types() {
        // Test that different error types have appropriate formatting/icons
        let results = vec![
            create_test_comparison_result("client_error", vec![("prod", 404, "Not Found"), ("staging", 404, "Not Found")], false),
            create_test_comparison_result("server_error", vec![("prod", 500, "Internal Server Error"), ("staging", 500, "Internal Server Error")], false),
            create_test_comparison_result("redirect", vec![("prod", 301, "Moved Permanently"), ("staging", 301, "Moved Permanently")], false),
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should use appropriate icons and formatting
        assert!(output.contains("🚨")); // Error icon
        assert!(output.contains("📍")); // Route indicator
        assert!(output.contains("==== Error Analysis ===="));
    }

    #[test] 
    fn test_error_analysis_preserves_existing_functionality() {
        // Test that error analysis doesn't break existing success rate and difference reporting
        let results = vec![
            create_test_comparison_result("success1", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),
            create_test_comparison_result("success2", vec![("prod", 201, "Created"), ("staging", 201, "Created")], true),
            create_test_comparison_result("different", vec![("prod", 200, "OK"), ("staging", 200, "OK")], false), // same status, different body
        ];

        let output = CurlGenerator::format_comparison_results(&results);

        // Should maintain existing success rate calculation
        assert!(output.contains("🔍 Test Results Summary: 3 total, 2 identical, 1 different - Success Rate: 66.7%"));
        
        // Should NOT show error analysis section (all are 2xx)
        assert!(!output.contains("==== Error Analysis ===="));
        
        // Should show differences section for body differences
        assert!(output.contains("❌ Differences Found:"));
    }
}

#[cfg(test)]
mod error_summary_integration_output_tests {
    use super::*;

    #[test]
    fn test_error_summary_metrics_in_output() {
        // Test that ErrorSummary metrics are correctly reflected in output
        let results = vec![
            create_test_comparison_result("route1", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),   // successful
            create_test_comparison_result("route2", vec![("prod", 500, "Internal Server Error"), ("staging", 500, "Internal Server Error")], true),   // identical failure
            create_test_comparison_result("route3", vec![("prod", 200, "OK"), ("staging", 404, "Not Found")], false),  // mixed
            create_test_comparison_result("route4", vec![("prod", 503, "Service Unavailable"), ("staging", 502, "Bad Gateway")], false),  // different failures
        ];

        let summary = ErrorSummary::from_comparison_results(&results);
        let output = CurlGenerator::format_comparison_results(&results);

        // Verify ErrorSummary calculations are reflected in output
        assert_eq!(summary.total_requests, 4);
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(summary.failed_requests, 3);
        assert_eq!(summary.identical_failures, 1);
        assert_eq!(summary.mixed_responses, 2);

        // Check that output reflects these metrics
        assert!(output.contains("🚨 3 requests failed (non-2xx status codes)"));
        assert!(output.contains("⚠️  1 requests failed identically across environments"));
        assert!(output.contains("🔄 2 requests had different status codes across environments"));
    }

    #[test]
    fn test_output_with_no_failed_requests() {
        // Test output when ErrorSummary shows no failures
        let results = vec![
            create_test_comparison_result("route1", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),
            create_test_comparison_result("route2", vec![("prod", 201, "Created"), ("staging", 201, "Created")], true),
        ];

        let summary = ErrorSummary::from_comparison_results(&results);
        let output = CurlGenerator::format_comparison_results(&results);

        // Verify no failures in summary
        assert_eq!(summary.failed_requests, 0);
        assert_eq!(summary.successful_requests, 2);

        // Output should not show error analysis
        assert!(!output.contains("==== Error Analysis ===="));
        assert!(output.contains("✅ All responses are identical across environments!"));
    }
} 