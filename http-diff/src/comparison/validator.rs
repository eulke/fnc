/// Response validation and similarity scoring utilities
use crate::types::HttpResponse;
use crate::error::{HttpDiffError, Result};
use std::collections::HashMap;

/// Response validator for checking comparison prerequisites
pub struct ResponseValidator;

impl ResponseValidator {
    /// Validate that responses can be compared
    pub fn validate_responses(responses: &HashMap<String, HttpResponse>) -> Result<()> {
        if responses.len() < 2 {
            return Err(HttpDiffError::comparison_failed(
                "Need at least 2 responses to compare",
            ));
        }

        // Additional validation could go here:
        // - Check for required headers
        // - Validate response format
        // - Check for minimum response size
        // etc.

        Ok(())
    }

    /// Check if all responses have successful status codes
    pub fn all_responses_successful(responses: &HashMap<String, HttpResponse>) -> bool {
        responses.values().all(|response| response.is_success())
    }

    /// Check if any responses have error status codes
    pub fn has_error_responses(responses: &HashMap<String, HttpResponse>) -> bool {
        responses.values().any(|response| response.is_error())
    }

    /// Get error responses grouped by environment
    pub fn get_error_responses(responses: &HashMap<String, HttpResponse>) -> HashMap<String, String> {
        responses
            .iter()
            .filter(|(_, response)| response.is_error())
            .map(|(env, response)| (env.clone(), response.body.clone()))
            .collect()
    }

    /// Extract status codes from all responses
    pub fn extract_status_codes(responses: &HashMap<String, HttpResponse>) -> HashMap<String, u16> {
        responses
            .iter()
            .map(|(env, response)| (env.clone(), response.status))
            .collect()
    }
}


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
    fn test_response_validation() {
        // Valid case: 2 responses
        let mut valid_responses = HashMap::new();
        valid_responses.insert("test".to_string(), create_test_response(200, "ok"));
        valid_responses.insert("prod".to_string(), create_test_response(200, "ok"));

        assert!(ResponseValidator::validate_responses(&valid_responses).is_ok());

        // Invalid case: 1 response
        let mut invalid_responses = HashMap::new();
        invalid_responses.insert("test".to_string(), create_test_response(200, "ok"));

        assert!(ResponseValidator::validate_responses(&invalid_responses).is_err());

        // Invalid case: 0 responses
        let empty_responses = HashMap::new();
        assert!(ResponseValidator::validate_responses(&empty_responses).is_err());
    }

    #[test]
    fn test_success_response_detection() {
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, "ok"));
        responses.insert("prod".to_string(), create_test_response(201, "created"));

        assert!(ResponseValidator::all_responses_successful(&responses));
        assert!(!ResponseValidator::has_error_responses(&responses));

        // Add an error response
        responses.insert("staging".to_string(), create_test_response(404, "not found"));

        assert!(!ResponseValidator::all_responses_successful(&responses));
        assert!(ResponseValidator::has_error_responses(&responses));
    }

    #[test]
    fn test_error_response_extraction() {
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, "ok"));
        responses.insert("prod".to_string(), create_test_response(404, "not found"));
        responses.insert("staging".to_string(), create_test_response(500, "server error"));

        let error_responses = ResponseValidator::get_error_responses(&responses);

        assert_eq!(error_responses.len(), 2);
        assert_eq!(error_responses.get("prod"), Some(&"not found".to_string()));
        assert_eq!(error_responses.get("staging"), Some(&"server error".to_string()));
        assert!(!error_responses.contains_key("test"));
    }

    #[test]
    fn test_status_code_extraction() {
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, "ok"));
        responses.insert("prod".to_string(), create_test_response(404, "not found"));

        let status_codes = ResponseValidator::extract_status_codes(&responses);

        assert_eq!(status_codes.len(), 2);
        assert_eq!(status_codes.get("test"), Some(&200));
        assert_eq!(status_codes.get("prod"), Some(&404));
    }

}