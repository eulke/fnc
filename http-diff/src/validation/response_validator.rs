use crate::error::{HttpDiffError, Result};
use crate::types::HttpResponse;
use std::collections::HashMap;

/// Response validation utilities
pub struct ResponseValidatorImpl;

impl ResponseValidatorImpl {
    /// Create a new response validator
    pub fn new() -> Self {
        Self
    }

    /// Validate that we have enough responses for comparison
    pub fn validate_responses(responses: &HashMap<String, HttpResponse>) -> Result<()> {
        if responses.len() < 2 {
            return Err(HttpDiffError::comparison_failed(format!(
                "Need at least 2 responses for comparison, got {}",
                responses.len()
            )));
        }
        Ok(())
    }

    /// Extract status codes from responses
    pub fn extract_status_codes(responses: &HashMap<String, HttpResponse>) -> HashMap<String, u16> {
        responses
            .iter()
            .map(|(env, response)| (env.clone(), response.status))
            .collect()
    }

    /// Check if any responses have error status codes
    pub fn has_error_responses(responses: &HashMap<String, HttpResponse>) -> bool {
        responses.values().any(|response| response.is_error())
    }

    /// Get error response bodies
    pub fn get_error_responses(
        responses: &HashMap<String, HttpResponse>,
    ) -> HashMap<String, String> {
        responses
            .iter()
            .filter(|(_, response)| response.is_error())
            .map(|(env, response)| (env.clone(), response.body.clone()))
            .collect()
    }

    /// Validate response content type
    pub fn validate_content_type(response: &HttpResponse, expected: &str) -> bool {
        response
            .headers
            .get("content-type")
            .map(|ct| ct.contains(expected))
            .unwrap_or(false)
    }

    /// Check if response body is valid JSON
    pub fn is_valid_json(response: &HttpResponse) -> bool {
        serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
    }

    /// Validate response size (useful for detecting truncated responses)
    pub fn validate_response_size(response: &HttpResponse, max_size: usize) -> Result<()> {
        if response.body.len() > max_size {
            return Err(HttpDiffError::comparison_failed(format!(
                "Response body too large: {} bytes (max: {})",
                response.body.len(),
                max_size
            )));
        }
        Ok(())
    }

    /// Check if all responses have consistent content types
    pub fn validate_consistent_content_types(
        responses: &HashMap<String, HttpResponse>,
    ) -> Result<()> {
        let content_types: Vec<_> = responses
            .values()
            .filter_map(|r| r.headers.get("content-type"))
            .collect();

        if content_types.len() > 1 {
            let first_ct = content_types[0];
            if !content_types.iter().all(|ct| *ct == first_ct) {
                return Err(HttpDiffError::comparison_failed(
                    "Inconsistent content types across environments".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate that responses are suitable for comparison
    pub fn validate_for_comparison(responses: &HashMap<String, HttpResponse>) -> Result<()> {
        Self::validate_responses(responses)?;

        // Additional validations can be added here:
        // - Check for consistent content types
        // - Validate response sizes
        // - Check for required headers

        Ok(())
    }
}

impl Default for ResponseValidatorImpl {
    fn default() -> Self {
        Self::new()
    }
}
