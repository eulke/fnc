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

/// Similarity calculator for measuring response likeness
pub struct SimilarityCalculator;

impl SimilarityCalculator {
    /// Calculate a simple similarity score between two text bodies
    pub fn calculate_text_similarity(text1: &str, text2: &str) -> f64 {
        if text1 == text2 {
            return 1.0;
        }
        
        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();
        
        if lines1.is_empty() && lines2.is_empty() {
            return 1.0;
        }
        
        let max_lines = lines1.len().max(lines2.len());
        if max_lines == 0 {
            return 1.0;
        }
        
        let matching_lines = lines1.iter()
            .zip(lines2.iter())
            .filter(|(l1, l2)| l1 == l2)
            .count();
            
        matching_lines as f64 / max_lines as f64
    }

    /// Calculate similarity score between two responses
    pub fn calculate_response_similarity(response1: &HttpResponse, response2: &HttpResponse) -> f64 {
        let mut score = 0.0;
        let mut factors = 0.0;

        // Status code similarity (binary: 1.0 if same, 0.0 if different)
        if response1.status == response2.status {
            score += 1.0;
        }
        factors += 1.0;

        // Body similarity
        let body_similarity = Self::calculate_text_similarity(&response1.body, &response2.body);
        score += body_similarity;
        factors += 1.0;

        // Header similarity (simple count-based approach)
        let header_similarity = Self::calculate_header_similarity(&response1.headers, &response2.headers);
        score += header_similarity;
        factors += 1.0;

        score / factors
    }

    /// Calculate header similarity based on common headers
    fn calculate_header_similarity(headers1: &HashMap<String, String>, headers2: &HashMap<String, String>) -> f64 {
        if headers1.is_empty() && headers2.is_empty() {
            return 1.0;
        }

        let all_headers: std::collections::HashSet<_> = headers1.keys()
            .chain(headers2.keys())
            .collect();

        if all_headers.is_empty() {
            return 1.0;
        }

        let matching_headers = all_headers.iter()
            .filter(|&key| {
                headers1.get(*key) == headers2.get(*key) && 
                headers1.contains_key(*key) && 
                headers2.contains_key(*key)
            })
            .count();

        matching_headers as f64 / all_headers.len() as f64
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

    #[test]
    fn test_text_similarity() {
        // Identical text
        assert_eq!(SimilarityCalculator::calculate_text_similarity("hello", "hello"), 1.0);

        // Empty text
        assert_eq!(SimilarityCalculator::calculate_text_similarity("", ""), 1.0);

        // Partially similar
        let score = SimilarityCalculator::calculate_text_similarity("line1\nline2", "line1\nline3");
        assert!(score > 0.0 && score < 1.0);

        // Completely different
        let score = SimilarityCalculator::calculate_text_similarity("hello", "world");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_response_similarity() {
        let response1 = create_test_response(200, "hello world");
        let response2 = create_test_response(200, "hello world");
        
        // Identical responses should have similarity 1.0
        let score = SimilarityCalculator::calculate_response_similarity(&response1, &response2);
        assert_eq!(score, 1.0);

        let response3 = create_test_response(404, "error message");
        
        // Different responses should have lower similarity
        let score = SimilarityCalculator::calculate_response_similarity(&response1, &response3);
        assert!(score < 1.0);
    }

    #[test]
    fn test_header_similarity() {
        let mut headers1 = HashMap::new();
        headers1.insert("content-type".to_string(), "application/json".to_string());
        headers1.insert("x-version".to_string(), "1.0".to_string());

        let mut headers2 = HashMap::new();
        headers2.insert("content-type".to_string(), "application/json".to_string());
        headers2.insert("x-version".to_string(), "1.0".to_string());

        // Identical headers
        let score = SimilarityCalculator::calculate_header_similarity(&headers1, &headers2);
        assert_eq!(score, 1.0);

        // Partially different headers
        headers2.insert("x-version".to_string(), "2.0".to_string());
        let score = SimilarityCalculator::calculate_header_similarity(&headers1, &headers2);
        assert!(score < 1.0);

        // Empty headers
        let empty1 = HashMap::new();
        let empty2 = HashMap::new();
        let score = SimilarityCalculator::calculate_header_similarity(&empty1, &empty2);
        assert_eq!(score, 1.0);
    }
}