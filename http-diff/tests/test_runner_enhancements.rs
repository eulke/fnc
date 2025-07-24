use http_diff::client::HttpResponse;
use http_diff::comparator::{ErrorSummary, ResponseComparator};
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

#[cfg(test)]
mod response_collection_tests {
    use super::*;

    #[test]
    fn test_status_code_extraction_from_responses() {
        // Test that ComparisonResult correctly extracts status codes from HTTP responses
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_response_with_status(200, "OK"));
        responses.insert("staging".to_string(), create_response_with_status(201, "Created"));

        let comparator = ResponseComparator::new();
        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        ).expect("Comparison should succeed");

        // Verify status codes are correctly extracted
        assert_eq!(result.status_codes.get("prod"), Some(&200u16));
        assert_eq!(result.status_codes.get("staging"), Some(&201u16));
        assert!(!result.has_errors); // Both 2xx should not have errors
        assert!(result.error_bodies.is_none()); // No error bodies for successful requests
    }

    #[test]
    fn test_response_body_capture_for_failed_requests_4xx() {
        // Test that 4xx responses have their bodies captured
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_response_with_status(404, "Not Found"));
        responses.insert("staging".to_string(), create_response_with_status(404, "Not Found"));

        let comparator = ResponseComparator::new();
        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        ).expect("Comparison should succeed");

        // Verify error detection and body capture
        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        
        let error_bodies = result.error_bodies.unwrap();
        assert_eq!(error_bodies.get("prod"), Some(&"Not Found".to_string()));
        assert_eq!(error_bodies.get("staging"), Some(&"Not Found".to_string()));
    }

    #[test]
    fn test_response_body_capture_for_failed_requests_5xx() {
        // Test that 5xx responses have their bodies captured
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_response_with_status(500, "Internal Server Error"));
        responses.insert("staging".to_string(), create_response_with_status(502, "Bad Gateway"));

        let comparator = ResponseComparator::new();
        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        ).expect("Comparison should succeed");

        // Verify error detection and body capture
        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        
        let error_bodies = result.error_bodies.unwrap();
        assert_eq!(error_bodies.get("prod"), Some(&"Internal Server Error".to_string()));
        assert_eq!(error_bodies.get("staging"), Some(&"Bad Gateway".to_string()));
    }

    #[test]
    fn test_no_body_storage_for_successful_requests() {
        // Test memory efficiency: successful requests don't store response bodies in error_bodies
        let large_response_body = "A".repeat(10000); // 10KB response
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_response_with_status(200, &large_response_body));
        responses.insert("staging".to_string(), create_response_with_status(200, &large_response_body));

        let comparator = ResponseComparator::new();
        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        ).expect("Comparison should succeed");

        // Verify no error bodies are stored for successful requests
        assert!(!result.has_errors);
        assert!(result.error_bodies.is_none()); // Memory efficient: no storage for successful requests
        
        // Verify the actual response bodies are still available in the responses field
        assert_eq!(result.responses.get("prod").unwrap().body, large_response_body);
        assert_eq!(result.responses.get("staging").unwrap().body, large_response_body);
    }

    #[test]
    fn test_mixed_success_failure_selective_body_storage() {
        // Test that only failed requests store bodies in error_bodies
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_response_with_status(200, "Success response"));
        responses.insert("staging".to_string(), create_response_with_status(500, "Error response"));

        let comparator = ResponseComparator::new();
        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        ).expect("Comparison should succeed");

        // Verify selective error body storage
        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        
        let error_bodies = result.error_bodies.unwrap();
        assert!(!error_bodies.contains_key("prod")); // Successful response should not store error body
        assert_eq!(error_bodies.get("staging"), Some(&"Error response".to_string())); // Failed response should store error body
    }

    #[test]
    fn test_3xx_redirects_considered_errors() {
        // Test that 3xx status codes are considered errors (non-2xx)
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_response_with_status(301, "Moved Permanently"));
        responses.insert("staging".to_string(), create_response_with_status(302, "Found"));

        let comparator = ResponseComparator::new();
        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        ).expect("Comparison should succeed");

        // Verify 3xx are treated as errors
        assert!(result.has_errors);
        assert!(result.error_bodies.is_some());
        
        let error_bodies = result.error_bodies.unwrap();
        assert_eq!(error_bodies.get("prod"), Some(&"Moved Permanently".to_string()));
        assert_eq!(error_bodies.get("staging"), Some(&"Found".to_string()));
    }

    #[test]
    fn test_status_code_boundary_conditions() {
        // Test edge cases around 2xx boundary
        let test_cases = vec![
            (199, true, "199 should be considered error"),
            (200, false, "200 should be considered success"),
            (299, false, "299 should be considered success"),
            (300, true, "300 should be considered error"),
        ];

        for (status_code, should_have_errors, description) in test_cases {
            let mut responses = HashMap::new();
            responses.insert("env1".to_string(), create_response_with_status(status_code, "test body"));
            responses.insert("env2".to_string(), create_response_with_status(status_code, "test body"));

            let comparator = ResponseComparator::new();
            let result = comparator.compare_responses(
                "test_route".to_string(),
                HashMap::new(),
                responses,
            ).expect("Comparison should succeed");

            assert_eq!(result.has_errors, should_have_errors, "{}", description);
            
            if should_have_errors {
                assert!(result.error_bodies.is_some());
                let error_bodies = result.error_bodies.unwrap();
                assert_eq!(error_bodies.get("env1"), Some(&"test body".to_string()));
                assert_eq!(error_bodies.get("env2"), Some(&"test body".to_string()));
            } else {
                assert!(result.error_bodies.is_none());
            }
        }
    }
}

#[cfg(test)]
mod error_summary_integration_tests {
    use super::*;

    #[test]
    fn test_error_summary_from_mixed_comparison_results() {
        // Test ErrorSummary generation from real ComparisonResult objects
        let comparator = ResponseComparator::new();
        let mut results = Vec::new();

        // Create various scenarios
        
        // 1. Successful identical responses
        let mut success_responses = HashMap::new();
        success_responses.insert("prod".to_string(), create_response_with_status(200, "OK"));
        success_responses.insert("staging".to_string(), create_response_with_status(200, "OK"));
        
        let success_result = comparator.compare_responses(
            "success_route".to_string(),
            HashMap::new(),
            success_responses,
        ).expect("Comparison should succeed");
        results.push(success_result);

        // 2. Failed identical responses
        let mut error_responses = HashMap::new();
        error_responses.insert("prod".to_string(), create_response_with_status(500, "Server Error"));
        error_responses.insert("staging".to_string(), create_response_with_status(500, "Server Error"));
        
        let error_result = comparator.compare_responses(
            "error_route".to_string(),
            HashMap::new(),
            error_responses,
        ).expect("Comparison should succeed");
        results.push(error_result);

        // 3. Mixed success/failure
        let mut mixed_responses = HashMap::new();
        mixed_responses.insert("prod".to_string(), create_response_with_status(200, "OK"));
        mixed_responses.insert("staging".to_string(), create_response_with_status(404, "Not Found"));
        
        let mixed_result = comparator.compare_responses(
            "mixed_route".to_string(),
            HashMap::new(),
            mixed_responses,
        ).expect("Comparison should succeed");
        results.push(mixed_result);

        // Generate ErrorSummary
        let summary = ErrorSummary::from_comparison_results(&results);

        // Verify summary statistics
        assert_eq!(summary.total_requests, 3);
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(summary.failed_requests, 2);
        assert_eq!(summary.identical_successes, 1);
        assert_eq!(summary.identical_failures, 1);
        assert_eq!(summary.mixed_responses, 1);
    }

    #[test]
    fn test_memory_efficiency_with_large_error_responses() {
        // Test that error body storage doesn't consume excessive memory for large responses
        let large_error_body = "E".repeat(50000); // 50KB error response
        let normal_success_body = "OK";

        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_response_with_status(200, normal_success_body));
        responses.insert("staging".to_string(), create_response_with_status(500, &large_error_body));

        let comparator = ResponseComparator::new();
        let result = comparator.compare_responses(
            "test_route".to_string(),
            HashMap::new(),
            responses,
        ).expect("Comparison should succeed");

        // Verify error body is captured for failed request
        assert!(result.has_errors);
        let error_bodies = result.error_bodies.unwrap();
        assert_eq!(error_bodies.get("staging"), Some(&large_error_body));
        
        // Verify successful request doesn't duplicate body storage
        assert!(!error_bodies.contains_key("prod"));
        
        // Verify original responses are still available
        assert_eq!(result.responses.get("prod").unwrap().body, normal_success_body);
        assert_eq!(result.responses.get("staging").unwrap().body, large_error_body);
    }
} 