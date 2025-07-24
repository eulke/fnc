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

/// Helper function to create ComparisonResult with error tracking for CLI testing
fn create_cli_comparison_result(
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
mod cli_error_analysis_integration_tests {
    use super::*;

    #[test]
    fn test_cli_displays_error_analysis_when_errors_present() {
        // Test that CLI integration properly displays error analysis section when errors are present
        let results = vec![
            create_cli_comparison_result("health", vec![("prod", 500, "Internal Server Error"), ("staging", 500, "Internal Server Error")], true),
            create_cli_comparison_result("api/users", vec![("prod", 404, "Users not found"), ("staging", 200, "{\"users\": []}")], false),
        ];

        // Generate CLI output using the same function the CLI uses
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify error analysis section appears in CLI output
        assert!(cli_output.contains("==== Error Analysis ===="));
        assert!(cli_output.contains("🚨 2 requests failed (non-2xx status codes)"));
        assert!(cli_output.contains("⚠️  1 requests failed identically across environments"));
        assert!(cli_output.contains("🔄 1 requests had different status codes across environments"));

        // Verify specific route error details are shown
        assert!(cli_output.contains("📍 Route 'health'"));
        assert!(cli_output.contains("📍 Route 'api/users'"));
        assert!(cli_output.contains("Status codes:"));
        assert!(cli_output.contains("prod: 500"));
        assert!(cli_output.contains("staging: 500"));
        assert!(cli_output.contains("prod: 404"));
        assert!(cli_output.contains("staging: 200"));

        // Verify response bodies are shown for error responses
        assert!(cli_output.contains("Response bodies:"));
        assert!(cli_output.contains("Internal Server Error"));
        assert!(cli_output.contains("Users not found"));
    }

    #[test]
    fn test_cli_no_error_analysis_when_all_successful() {
        // Test that CLI integration does NOT display error analysis when all requests are successful
        let results = vec![
            create_cli_comparison_result("health", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),
            create_cli_comparison_result("users", vec![("prod", 201, "Created"), ("staging", 201, "Created")], true),
        ];

        // Generate CLI output using the same function the CLI uses
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify NO error analysis section appears
        assert!(!cli_output.contains("==== Error Analysis ===="));
        assert!(!cli_output.contains("🚨"));
        assert!(!cli_output.contains("⚠️"));
        assert!(!cli_output.contains("🔄"));

        // Verify successful completion message
        assert!(cli_output.contains("✅ All responses are identical across environments!"));
        
        // Verify traditional success rate is displayed
        assert!(cli_output.contains("🔍 Test Results Summary"));
        assert!(cli_output.contains("Success Rate: 100.0%"));
    }

    #[test]
    fn test_cli_backward_compatibility_with_existing_flags() {
        // Test that CLI integration maintains backward compatibility with existing functionality
        let results = vec![
            create_cli_comparison_result("health", vec![("prod", 200, "Test Environment OK"), ("staging", 200, "Production Environment OK")], false), // same status, different body
        ];

        // Generate CLI output
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify traditional success rate calculation is maintained (body differences = not identical)
        assert!(cli_output.contains("🔍 Test Results Summary: 1 total, 0 identical, 1 different - Success Rate: 0.0%"));
        
        // Verify traditional differences section is still shown for body differences
        assert!(cli_output.contains("❌ Differences Found:"));
        assert!(cli_output.contains("📍 Route 'health'"));
        
        // Verify NO error analysis section (both are 2xx status codes)
        assert!(!cli_output.contains("==== Error Analysis ===="));
        
        // This proves backward compatibility: differences in body/headers are still detected 
        // and reported as before, while error analysis only appears for status code issues
    }

    #[test]
    fn test_cli_success_rate_calculation_unchanged() {
        // Test that the existing success rate calculation logic remains exactly the same
        let results = vec![
            create_cli_comparison_result("health", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),
            create_cli_comparison_result("api/users", vec![("prod", 200, "{\"users\": [\"test\"]}"), ("staging", 200, "{\"users\": [\"prod\"]}")], false), // same status, different body
        ];

        // Generate CLI output
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify traditional success rate: 1 identical, 1 different = 50% success rate
        assert!(cli_output.contains("🔍 Test Results Summary: 2 total, 1 identical, 1 different - Success Rate: 50.0%"));
        
        // Verify differences section for body differences
        assert!(cli_output.contains("❌ Differences Found:"));
        
        // Verify NO error analysis (all 2xx responses)
        assert!(!cli_output.contains("==== Error Analysis ===="));
        
        // This confirms the existing success rate calculation is based on response similarity,
        // not HTTP status codes, and remains unchanged
    }

    #[test]
    fn test_cli_enhanced_output_in_mixed_scenarios() {
        // Test CLI output with both traditional differences AND error analysis
        let results = vec![
            create_cli_comparison_result("health", vec![("prod", 200, "Test OK"), ("staging", 200, "Prod OK")], false),  // successful but different bodies (traditional difference)
            create_cli_comparison_result("api/users", vec![("prod", 404, "Not Found"), ("staging", 500, "Internal Error")], false),   // error responses - different status codes (error analysis)
            create_cli_comparison_result("api/status", vec![("prod", 200, "Available"), ("staging", 200, "Available")], true),      // identical successful responses
        ];

        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify comprehensive output includes both traditional and error analysis
        assert!(cli_output.contains("🔍 Test Results Summary: 3 total, 1 identical, 2 different"));
        
        // Verify error analysis section appears
        assert!(cli_output.contains("==== Error Analysis ===="));
        assert!(cli_output.contains("🚨 1 requests failed (non-2xx status codes)"));
        assert!(cli_output.contains("🔄 1 requests had different status codes across environments"));
        
        // Verify traditional differences section also appears
        assert!(cli_output.contains("❌ Differences Found:"));
        
        // Verify both types of issues are reported properly
        assert!(cli_output.contains("📍 Route 'health'")); // Body difference
        assert!(cli_output.contains("📍 Route 'api/users'")); // Status code difference
        
        // This demonstrates that both traditional diff reporting and new error analysis
        // work together seamlessly in the CLI
    }

    #[test]
    fn test_cli_error_analysis_with_response_body_truncation() {
        // Test that large error response bodies are properly truncated in CLI output
        let large_error_body = "Error: ".to_string() + &"A".repeat(1000); // 1006 chars
        let results = vec![
            create_cli_comparison_result("api/upload", vec![("prod", 413, &large_error_body)], false),
        ];

        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Should display error analysis
        assert!(cli_output.contains("==== Error Analysis ===="));
        assert!(cli_output.contains("Response bodies:"));
        
        // Should truncate long response bodies
        assert!(cli_output.contains("prod: Error: AAA")); // Should start with the error
        assert!(cli_output.contains("... (truncated)")); // Should indicate truncation
        assert!(!cli_output.contains(&"A".repeat(500))); // Should not contain the full body
    }

    #[test]
    fn test_cli_error_summary_integration() {
        // Test that ErrorSummary metrics are correctly reflected in CLI output
        let results = vec![
            create_cli_comparison_result("route1", vec![("prod", 200, "OK"), ("staging", 200, "OK")], true),   // successful
            create_cli_comparison_result("route2", vec![("prod", 500, "Internal Server Error"), ("staging", 500, "Internal Server Error")], true),   // identical failure
            create_cli_comparison_result("route3", vec![("prod", 200, "OK"), ("staging", 404, "Not Found")], false),  // mixed
            create_cli_comparison_result("route4", vec![("prod", 503, "Service Unavailable"), ("staging", 502, "Bad Gateway")], false),  // different failures
        ];

        let summary = ErrorSummary::from_comparison_results(&results);
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify ErrorSummary calculations are reflected in output
        assert_eq!(summary.total_requests, 4);
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(summary.failed_requests, 3);
        assert_eq!(summary.identical_failures, 1);
        assert_eq!(summary.mixed_responses, 2);

        // Check that CLI output reflects these metrics
        assert!(cli_output.contains("🚨 3 requests failed (non-2xx status codes)"));
        assert!(cli_output.contains("⚠️  1 requests failed identically across environments"));
        assert!(cli_output.contains("🔄 2 requests had different status codes across environments"));
    }

    #[test]
    fn test_cli_maintains_existing_ui_structure() {
        // Test that the enhanced CLI output maintains the existing UI structure and doesn't break existing patterns
        let results = vec![
            create_cli_comparison_result("route1", vec![("prod", 200, "OK"), ("staging", 200, "Different")], false), // not identical - has differences
            create_cli_comparison_result("route2", vec![("prod", 500, "Error"), ("staging", 500, "Error")], true),   // identical error
        ];

        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify the order and structure of output sections
        let summary_pos = cli_output.find("🔍 Test Results Summary").unwrap();
        let error_analysis_pos = cli_output.find("==== Error Analysis ====").unwrap();
        let success_message_pos = cli_output.find("✅ All responses are identical across environments!");
        let differences_section_pos = cli_output.find("❌ Differences Found:");

        // Error analysis should come after the summary
        assert!(error_analysis_pos > summary_pos);
        
        // Success message should not appear when there are differences
        assert!(success_message_pos.is_none(), "Success message should not appear when there are differences");
        
        // Differences section should appear since one route is not identical
        assert!(differences_section_pos.is_some(), "Differences section should appear");
        
        // Verify that both error analysis and differences can coexist
        assert!(cli_output.contains("==== Error Analysis ===="));
        assert!(cli_output.contains("❌ Differences Found:"));
        
        // This verifies that the enhanced output maintains logical flow and structure
    }
} 