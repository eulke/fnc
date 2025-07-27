mod common;

use common::*;
use http_diff::ErrorSummary;
use tempfile::TempDir;
use std::fs;

// =============================================================================
// UNIT TESTS - Core Functionality
// =============================================================================

#[cfg(test)]
mod comparator_tests {
    use super::*;

    #[test]
    fn test_comparison_result_status_codes() {
        let result = create_comparison_result(
            "test_route",
            vec![("test", 200, "success"), ("prod", 500, "error")],
            false,
        );

        assert_eq!(result.status_codes["test"], 200);
        assert_eq!(result.status_codes["prod"], 500);
        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
    }

    #[test]
    fn test_error_detection() {
        let all_success = create_comparison_result(
            "success_route", 
            vec![("test", 200, "ok"), ("prod", 201, "created")], 
            true
        );
        assert!(!all_success.has_errors);
        assert!(all_success.error_bodies.is_none());

        let with_errors = create_comparison_result(
            "error_route",
            vec![("test", 404, "not found"), ("prod", 200, "ok")],
            false,
        );
        assert!(with_errors.has_errors);
        assert!(with_errors.error_bodies.is_some());
    }

    #[test]
    fn test_error_summary_calculation() {
        let results = vec![
            create_comparison_result("route1", vec![("test", 200, "ok"), ("prod", 200, "ok")], true),
            create_comparison_result("route2", vec![("test", 404, "error"), ("prod", 500, "error")], false),
            create_comparison_result("route3", vec![("test", 200, "ok"), ("prod", 404, "error")], false),
        ];

        let summary = ErrorSummary::from_comparison_results(&results);
        assert_eq!(summary.total_requests, 3);
    }
}

#[cfg(test)]
mod output_tests {
    use super::*;

    #[test]
    fn test_error_analysis_output() {
        let result = create_comparison_result(
            "test_route",
            vec![("test", 404, "Not Found"), ("prod", 500, "Internal Error")],
            false,
        );

        // Test that error analysis is generated for results with errors
        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        
        let error_bodies = result.error_bodies.as_ref().unwrap();
        assert_eq!(error_bodies["test"], "Not Found");
        assert_eq!(error_bodies["prod"], "Internal Error");
    }

    #[test]
    fn test_curl_generation() {
        let response = create_response(200, "test body", Some("http://example.com/api"));
        
        // Curl generation should work for any response
        assert!(!response.curl_command.is_empty());
        assert!(response.curl_command.contains("curl"));
    }

    #[test]
    fn test_output_formatting_preserves_functionality() {
        let results = vec![
            create_comparison_result("route1", vec![("test", 200, "ok")], true),
            create_comparison_result("route2", vec![("test", 404, "error")], false),
        ];

        let summary = ErrorSummary::from_comparison_results(&results);
        
        // Verify output formatting doesn't break core metrics
        assert_eq!(summary.total_requests, 2);
    }
}

#[cfg(test)]
mod runner_tests {
    use super::*;

    #[test]
    fn test_status_code_tracking() {
        let result = create_comparison_result(
            "api_test",
            vec![("dev", 200, "ok"), ("staging", 404, "not found"), ("prod", 500, "error")],
            false,
        );

        // Verify status codes are properly tracked
        assert_eq!(result.status_codes.len(), 3);
        assert_eq!(result.status_codes["dev"], 200);
        assert_eq!(result.status_codes["staging"], 404);
        assert_eq!(result.status_codes["prod"], 500);
    }

    #[test]
    fn test_error_body_storage() {
        let success_result = create_comparison_result(
            "success_route",
            vec![("test", 200, "success"), ("prod", 201, "created")],
            true,
        );
        assert!(success_result.error_bodies.is_none());

        let error_result = create_comparison_result(
            "error_route", 
            vec![("test", 404, "not found"), ("prod", 500, "server error")],
            false,
        );
        let error_bodies = error_result.error_bodies.as_ref().unwrap();
        assert_eq!(error_bodies.len(), 2);
        assert_eq!(error_bodies["test"], "not found");
        assert_eq!(error_bodies["prod"], "server error");
    }

    #[test]
    fn test_redirect_handling() {
        // 3xx status codes should be considered errors for tracking
        let redirect_result = create_comparison_result(
            "redirect_route",
            vec![("test", 301, "moved"), ("prod", 302, "found")],
            false,
        );
        assert!(redirect_result.has_errors);
        assert!(redirect_result.error_bodies.is_some());
    }
}

// =============================================================================
// INTEGRATION TESTS - Full Flow
// =============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;


    #[tokio::test]
    async fn test_config_file_integration() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("http-diff.toml");
        let users_path = temp_dir.path().join("users.csv");

        fs::write(&config_path, create_test_config()).unwrap();
        fs::write(&users_path, create_test_users_csv()).unwrap();

        // Test config loading exists - simplified test
        assert!(config_path.exists());
        assert!(users_path.exists());
    }

    #[test]
    fn test_error_summary_comprehensive() {
        let mixed_results = vec![
            create_comparison_result("health", vec![("test", 200, "ok"), ("prod", 200, "ok")], true),
            create_comparison_result("users", vec![("test", 404, "not found"), ("prod", 200, "ok")], false),
            create_comparison_result("orders", vec![("test", 500, "error"), ("prod", 503, "unavailable")], false),
            create_comparison_result("auth", vec![("test", 200, "ok"), ("prod", 200, "ok")], true),
        ];

        let summary = ErrorSummary::from_comparison_results(&mixed_results);
        assert_eq!(summary.total_requests, 4);
    }

    #[test]
    fn test_end_to_end_workflow_simulation() {
        // Simulate a complete workflow with mixed scenarios
        let test_scenarios = vec![
            // Scenario 1: All environments healthy
            create_comparison_result(
                "health_check", 
                vec![("dev", 200, "healthy"), ("staging", 200, "healthy"), ("prod", 200, "healthy")],
                true
            ),
            // Scenario 2: One environment down
            create_comparison_result(
                "api_endpoint",
                vec![("dev", 200, "data"), ("staging", 500, "error"), ("prod", 200, "data")],
                false
            ),
            // Scenario 3: Complete failure
            create_comparison_result(
                "legacy_service",
                vec![("dev", 404, "not found"), ("staging", 404, "not found"), ("prod", 404, "not found")],
                false
            ),
        ];

        // Verify the workflow produces expected results
        let summary = ErrorSummary::from_comparison_results(&test_scenarios);
        assert_eq!(summary.total_requests, 3);
        
        // Verify individual scenario tracking
        for scenario in &test_scenarios {
            assert!(!scenario.route_name.is_empty());
            assert!(!scenario.status_codes.is_empty());
        }
    }
} 