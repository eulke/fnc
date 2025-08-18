use crate::comparison::content::{ContentNormalizer, HeaderNormalizer};
/// Response difference analysis and categorization
use crate::types::{Difference, DifferenceCategory, HttpResponse};
use std::collections::HashMap;

/// Raw header difference data for later formatting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeaderDiff {
    pub name: String,
    pub value1: Option<String>, // Value in first environment
    pub value2: Option<String>, // Value in second environment
}

/// Raw body difference data for later formatting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BodyDiff {
    pub normalized_body1: String,
    pub normalized_body2: String,
    pub is_large_response: bool,
    pub total_size: usize,
}

/// Difference analyzer that extracts raw difference data without formatting
pub struct DifferenceAnalyzer {
    content_normalizer: ContentNormalizer,
    header_normalizer: HeaderNormalizer,
    large_response_threshold: usize,
}

impl DifferenceAnalyzer {
    /// Create a new difference analyzer
    pub fn new(
        ignore_headers: Vec<String>,
        ignore_whitespace: bool,
        large_response_threshold: usize,
    ) -> Self {
        Self {
            content_normalizer: ContentNormalizer::new(ignore_whitespace),
            header_normalizer: HeaderNormalizer::new(ignore_headers),
            large_response_threshold,
        }
    }

    /// Analyze differences between two responses, returning raw data without formatting
    pub fn analyze_responses(
        &self,
        response1: &HttpResponse,
        response2: &HttpResponse,
        env1: &str,
        env2: &str,
        compare_headers: bool,
    ) -> Vec<Difference> {
        let mut differences = Vec::new();

        // Compare status codes
        if response1.status != response2.status {
            differences.push(Difference {
                category: DifferenceCategory::Status,
                description: format!(
                    "Status code differs between {} and {}: {} vs {}",
                    env1, env2, response1.status, response2.status
                ),
                diff_output: None,
                header_diff: None,
                body_diff: None,
            });
        }

        // Compare headers if enabled
        if compare_headers {
            if let Some(header_diff) = self.analyze_headers(&response1.headers, &response2.headers)
            {
                differences.push(Difference::with_header_diff(
                    "Header differences detected".to_string(),
                    header_diff,
                ));
            }
        }

        // Compare bodies
        if let Some(body_diff) = self.analyze_bodies(&response1.body, &response2.body) {
            differences.push(Difference::with_body_diff(
                "Body differences detected".to_string(),
                body_diff,
            ));
        }

        differences
    }

    /// Analyze header differences and return raw data
    fn analyze_headers(
        &self,
        headers1: &HashMap<String, String>,
        headers2: &HashMap<String, String>,
    ) -> Option<Vec<HeaderDiff>> {
        let normalized_headers1 = self.header_normalizer.normalize(headers1);
        let normalized_headers2 = self.header_normalizer.normalize(headers2);

        let mut header_differences = Vec::new();

        // Check for headers present in one but not the other
        for (lowercase_key, (original_key, value)) in &normalized_headers1 {
            if !normalized_headers2.contains_key(lowercase_key) {
                header_differences.push(HeaderDiff {
                    name: original_key.clone(),
                    value1: Some(value.clone()),
                    value2: None,
                });
            }
        }

        for (lowercase_key, (original_key, value)) in &normalized_headers2 {
            if !normalized_headers1.contains_key(lowercase_key) {
                header_differences.push(HeaderDiff {
                    name: original_key.clone(),
                    value1: None,
                    value2: Some(value.clone()),
                });
            }
        }

        // Check for headers with different values
        for (lowercase_key, (original_key1, value1)) in &normalized_headers1 {
            if let Some((_original_key2, value2)) = normalized_headers2.get(lowercase_key) {
                if value1 != value2 {
                    header_differences.push(HeaderDiff {
                        name: original_key1.clone(),
                        value1: Some(value1.clone()),
                        value2: Some(value2.clone()),
                    });
                }
            }
        }

        if header_differences.is_empty() {
            None
        } else {
            Some(header_differences)
        }
    }

    /// Analyze body differences and return raw data
    fn analyze_bodies(&self, body1: &str, body2: &str) -> Option<BodyDiff> {
        let normalized_body1 = self.content_normalizer.normalize(body1, None);
        let normalized_body2 = self.content_normalizer.normalize(body2, None);

        if normalized_body1 == normalized_body2 {
            return None;
        }

        let total_size = body1.len() + body2.len();
        let is_large_response = total_size > self.large_response_threshold;

        Some(BodyDiff {
            normalized_body1,
            normalized_body2,
            is_large_response,
            total_size,
        })
    }

    /// Check if responses have identical content
    pub fn are_identical(
        &self,
        response1: &HttpResponse,
        response2: &HttpResponse,
        compare_headers: bool,
    ) -> bool {
        // Check status codes
        if response1.status != response2.status {
            return false;
        }

        // Check headers if enabled
        if compare_headers {
            let normalized_headers1 = self.header_normalizer.normalize(&response1.headers);
            let normalized_headers2 = self.header_normalizer.normalize(&response2.headers);

            if normalized_headers1 != normalized_headers2 {
                return false;
            }
        }

        // Check bodies
        let normalized_body1 = self.content_normalizer.normalize(&response1.body, None);
        let normalized_body2 = self.content_normalizer.normalize(&response2.body, None);

        normalized_body1 == normalized_body2
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
    fn test_identical_responses() {
        let analyzer =
            DifferenceAnalyzer::new(vec![], true, crate::types::DEFAULT_LARGE_RESPONSE_THRESHOLD);

        let response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let response2 = create_test_response(200, r#"{"status": "ok"}"#);

        let differences = analyzer.analyze_responses(&response1, &response2, "test", "prod", false);
        assert!(differences.is_empty());

        assert!(analyzer.are_identical(&response1, &response2, false));
    }

    #[test]
    fn test_status_code_difference() {
        let analyzer =
            DifferenceAnalyzer::new(vec![], true, crate::types::DEFAULT_LARGE_RESPONSE_THRESHOLD);

        let response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let response2 = create_test_response(404, r#"{"error": "not found"}"#);

        let differences = analyzer.analyze_responses(&response1, &response2, "test", "prod", false);

        // Should have status and body differences
        assert_eq!(differences.len(), 2);

        let status_diff = differences
            .iter()
            .find(|d| d.category == DifferenceCategory::Status);
        assert!(status_diff.is_some());
        assert!(status_diff.unwrap().description.contains("200 vs 404"));

        assert!(!analyzer.are_identical(&response1, &response2, false));
    }

    #[test]
    fn test_body_difference_analysis() {
        let analyzer =
            DifferenceAnalyzer::new(vec![], true, crate::types::DEFAULT_LARGE_RESPONSE_THRESHOLD);

        let response1 = create_test_response(200, r#"{"status": "ok", "data": "test"}"#);
        let response2 = create_test_response(200, r#"{"status": "ok", "data": "prod"}"#);

        let differences = analyzer.analyze_responses(&response1, &response2, "test", "prod", false);

        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].category, DifferenceCategory::Body);

        // Should contain structured BodyDiff data
        assert!(differences[0].body_diff.is_some());
        let body_diff = differences[0].body_diff.as_ref().unwrap();
        assert!(!body_diff.is_large_response);
        assert!(body_diff.normalized_body1.contains("test"));
        assert!(body_diff.normalized_body2.contains("prod"));
    }

    #[test]
    fn test_header_difference_analysis() {
        let analyzer =
            DifferenceAnalyzer::new(vec![], true, crate::types::DEFAULT_LARGE_RESPONSE_THRESHOLD);

        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);

        response1
            .headers
            .insert("X-Version".to_string(), "1.0".to_string());
        response2
            .headers
            .insert("X-Version".to_string(), "2.0".to_string());

        let differences = analyzer.analyze_responses(&response1, &response2, "test", "prod", true);

        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].category, DifferenceCategory::Headers);

        // Should contain structured HeaderDiff data
        let header_diffs = differences[0].header_diff.as_ref().unwrap();

        assert_eq!(header_diffs.len(), 1);
        assert_eq!(header_diffs[0].name, "X-Version");
        assert_eq!(header_diffs[0].value1, Some("1.0".to_string()));
        assert_eq!(header_diffs[0].value2, Some("2.0".to_string()));

        // Without header comparison, should be identical
        assert!(analyzer.are_identical(&response1, &response2, false));
        // With header comparison, should not be identical
        assert!(!analyzer.are_identical(&response1, &response2, true));
    }

    #[test]
    fn test_large_response_detection() {
        let analyzer = DifferenceAnalyzer::new(vec![], true, 100); // Low threshold for testing

        let large_body1 = "x".repeat(60);
        let large_body2 = "y".repeat(60);

        let response1 = create_test_response(200, &large_body1);
        let response2 = create_test_response(200, &large_body2);

        let differences = analyzer.analyze_responses(&response1, &response2, "test", "prod", false);

        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].category, DifferenceCategory::Body);

        let body_diff = differences[0].body_diff.as_ref().unwrap();

        assert!(body_diff.is_large_response);
        assert_eq!(body_diff.total_size, 120);
    }

    #[test]
    fn test_ignored_headers() {
        let ignore_headers = vec!["date".to_string(), "x-request-id".to_string()];
        let analyzer = DifferenceAnalyzer::new(ignore_headers, true, 50_000);

        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);

        // Add headers that should be ignored
        response1.headers.insert(
            "date".to_string(),
            "Mon, 01 Jan 2024 00:00:00 GMT".to_string(),
        );
        response2.headers.insert(
            "date".to_string(),
            "Tue, 02 Jan 2024 00:00:00 GMT".to_string(),
        );

        let differences = analyzer.analyze_responses(&response1, &response2, "test", "prod", true);

        // Should be identical because ignored headers are not compared
        assert!(differences.is_empty());
        assert!(analyzer.are_identical(&response1, &response2, true));
    }
}
