use crate::client::HttpResponse;
use crate::error::{HttpDiffError, Result};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;

/// Result of comparing two HTTP responses
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub route_name: String,
    pub user_context: HashMap<String, String>,
    pub responses: HashMap<String, HttpResponse>,
    pub differences: Vec<Difference>,
    pub is_identical: bool,
}

/// Represents a difference between responses
#[derive(Debug, Clone)]
pub struct Difference {
    pub category: DifferenceCategory,
    pub description: String,
    pub diff_output: Option<String>,
}

/// Categories of differences that can be detected
#[derive(Debug, Clone, PartialEq)]
pub enum DifferenceCategory {
    Status,
    Headers,
    Body,
}

/// Response comparator with configurable comparison strategies
pub struct ResponseComparator {
    ignore_headers: Vec<String>,
    ignore_whitespace: bool,
    max_diff_lines: usize,
    large_response_threshold: usize,
    compare_headers: bool,
}

impl ResponseComparator {
    /// Create a new response comparator with default settings
    /// By default, compares only HTTP status and response body (headers comparison disabled)
    pub fn new() -> Self {
        Self {
            ignore_headers: vec![
                "date".to_string(),
                "server".to_string(),
                "x-request-id".to_string(),
                "x-correlation-id".to_string(),
            ],
            ignore_whitespace: true,
            max_diff_lines: 100,
            large_response_threshold: 50_000, // 50KB
            compare_headers: false, // Headers comparison disabled by default
        }
    }

    /// Create a comparator with custom settings
    pub fn with_settings(ignore_headers: Vec<String>, ignore_whitespace: bool) -> Self {
        Self {
            ignore_headers,
            ignore_whitespace,
            max_diff_lines: 100,
            large_response_threshold: 50_000,
            compare_headers: false, // Headers comparison disabled by default
        }
    }

    /// Create a comparator with full control over all settings
    pub fn with_full_settings(
        ignore_headers: Vec<String>, 
        ignore_whitespace: bool,
        compare_headers: bool,
        max_diff_lines: usize,
        large_response_threshold: usize,
    ) -> Self {
        Self {
            ignore_headers,
            ignore_whitespace,
            max_diff_lines,
            large_response_threshold,
            compare_headers,
        }
    }

    /// Enable headers comparison (disabled by default)
    pub fn with_headers_comparison(mut self) -> Self {
        self.compare_headers = true;
        self
    }

    /// Compare responses from multiple environments
    pub fn compare_responses(
        &self,
        route_name: String,
        user_context: HashMap<String, String>,
        responses: HashMap<String, HttpResponse>,
    ) -> Result<ComparisonResult> {
        if responses.len() < 2 {
            return Err(HttpDiffError::comparison_failed(
                "Need at least 2 responses to compare",
            ));
        }

        let mut differences = Vec::new();
        let environments: Vec<String> = responses.keys().cloned().collect();

        // Compare each pair of environments
        for i in 0..environments.len() {
            for j in i + 1..environments.len() {
                let env1 = &environments[i];
                let env2 = &environments[j];
                
                let response1 = &responses[env1];
                let response2 = &responses[env2];

                // Compare status codes
                if response1.status != response2.status {
                    differences.push(Difference {
                        category: DifferenceCategory::Status,
                        description: format!(
                            "Status code differs between {} and {}: {} vs {}",
                            env1, env2, response1.status, response2.status
                        ),
                        diff_output: None,
                    });
                }

                // Compare headers only if enabled
                if self.compare_headers {
                    let header_diffs = self.compare_headers(&response1.headers, &response2.headers, env1, env2);
                    differences.extend(header_diffs);
                }

                // Compare bodies
                if let Some(body_diff) = self.compare_bodies(&response1.body, &response2.body, env1, env2)? {
                    differences.push(body_diff);
                }
            }
        }

        let is_identical = differences.is_empty();

        Ok(ComparisonResult {
            route_name,
            user_context,
            responses,
            differences,
            is_identical,
        })
    }

    /// Compare response headers with case-insensitive handling
    fn compare_headers(
        &self,
        headers1: &HashMap<String, String>,
        headers2: &HashMap<String, String>,
        env1: &str,
        env2: &str,
    ) -> Vec<Difference> {
        let mut differences = Vec::new();

        // Normalize headers to lowercase for comparison while preserving original case for display
        let normalize_headers = |headers: &HashMap<String, String>| -> HashMap<String, (String, String)> {
            headers
                .iter()
                .filter(|(key, _)| !self.ignore_headers.contains(&key.to_lowercase()))
                .map(|(k, v)| (k.to_lowercase(), (k.clone(), v.clone())))
                .collect()
        };

        let normalized_headers1 = normalize_headers(headers1);
        let normalized_headers2 = normalize_headers(headers2);

        // Check for headers present in one but not the other
        for (lowercase_key, (original_key, _)) in &normalized_headers1 {
            if !normalized_headers2.contains_key(lowercase_key) {
                differences.push(Difference {
                    category: DifferenceCategory::Headers,
                    description: format!(
                        "Header '{}' present in {} but missing in {}",
                        original_key, env1, env2
                    ),
                    diff_output: None,
                });
            }
        }

        for (lowercase_key, (original_key, _)) in &normalized_headers2 {
            if !normalized_headers1.contains_key(lowercase_key) {
                differences.push(Difference {
                    category: DifferenceCategory::Headers,
                    description: format!(
                        "Header '{}' present in {} but missing in {}",
                        original_key, env2, env1
                    ),
                    diff_output: None,
                });
            }
        }

        // Check for headers with different values
        for (lowercase_key, (original_key1, value1)) in &normalized_headers1 {
            if let Some((_original_key2, value2)) = normalized_headers2.get(lowercase_key) {
                if value1 != value2 {
                    let diff_output = self.generate_header_diff(original_key1, value1, value2, env1, env2);
                    differences.push(Difference {
                        category: DifferenceCategory::Headers,
                        description: format!(
                            "Header '{}' differs between {} and {}: '{}' vs '{}'",
                            original_key1, env1, env2, value1, value2
                        ),
                        diff_output: Some(diff_output),
                    });
                }
            }
        }

        differences
    }

    /// Generate a formatted diff for header values
    fn generate_header_diff(&self, header_name: &str, value1: &str, value2: &str, env1: &str, env2: &str) -> String {
        format!(
            "┌─ Header '{}' Comparison ─┐\n│ {}: {} │\n│ {}: {} │\n└{}┘",
            header_name,
            env1,
            value1,
            env2,
            value2,
            "─".repeat(header_name.len() + 24)
        )
    }

    /// Compare response bodies with enhanced diff visualization
    fn compare_bodies(
        &self,
        body1: &str,
        body2: &str,
        env1: &str,
        env2: &str,
    ) -> Result<Option<Difference>> {
        let (normalized_body1, normalized_body2) = if self.ignore_whitespace {
            (self.normalize_whitespace(body1), self.normalize_whitespace(body2))
        } else {
            (body1.to_string(), body2.to_string())
        };

        if normalized_body1 == normalized_body2 {
            return Ok(None);
        }

        // Generate GitHub-style side-by-side diff output
        let diff_output = self.generate_github_style_diff(&normalized_body1, &normalized_body2, env1, env2);

        Ok(Some(Difference {
            category: DifferenceCategory::Body,
            description: format!("Response body differs between {} and {}", env1, env2),
            diff_output: Some(diff_output),
        }))
    }

    /// Generate GitHub-style side-by-side diff with line numbers and context
    fn generate_github_style_diff(&self, text1: &str, text2: &str, env1: &str, env2: &str) -> String {
        let total_size = text1.len() + text2.len();
        
        // For very large responses, provide a summary instead of full diff
        if total_size > self.large_response_threshold {
            return self.generate_large_response_summary(text1, text2, env1, env2);
        }
        
        let diff = TextDiff::from_lines(text1, text2);
        let mut output = String::new();
        
        // Header
        output.push_str("┌─────────────────────────────────────────────────────────────────────────────────────┐\n");
        output.push_str(&format!("│ {} vs {} - Response Body Comparison{}", 
            env1.to_uppercase(), 
            env2.to_uppercase(),
            " ".repeat(42_usize.saturating_sub(env1.len()).saturating_sub(env2.len()))
        ));
        output.push_str("│\n");
        output.push_str("├─────────────────────────────────────────────────────────────────────────────────────┤\n");

        let mut line_num1 = 1;
        let mut line_num2 = 1;
        let mut context_count = 0;
        let mut diff_lines_shown = 0;
        const MAX_CONTEXT: usize = 3;

        for (i, group) in diff.grouped_ops(MAX_CONTEXT).iter().enumerate() {
            if diff_lines_shown >= self.max_diff_lines {
                output.push_str("│ ... (diff truncated, too many differences)                                         │\n");
                break;
            }
            
            if i > 0 {
                output.push_str("│ @@ ... @@ (context omitted)                                                        │\n");
            }

            for op in group {
                for change in diff.iter_changes(op) {
                    if diff_lines_shown >= self.max_diff_lines {
                        break;
                    }
                    
                    let (prefix, line_indicator, line_content) = match change.tag() {
                        ChangeTag::Delete => {
                            let content = format!("- {:<3} │ {}", line_num1, change.value().trim_end());
                            line_num1 += 1;
                            diff_lines_shown += 1;
                            ("│ ", "🔴", content)
                        }
                        ChangeTag::Insert => {
                            let content = format!("+ {:<3} │ {}", line_num2, change.value().trim_end());
                            line_num2 += 1;
                            diff_lines_shown += 1;
                            ("│ ", "🟢", content)
                        }
                        ChangeTag::Equal => {
                            let content = format!("  {:<3} │ {}", line_num1, change.value().trim_end());
                            line_num1 += 1;
                            line_num2 += 1;
                            context_count += 1;
                            if context_count > MAX_CONTEXT * 2 {
                                continue;
                            }
                            ("│ ", "  ", content)
                        }
                    };

                    // Truncate long lines for readability
                    let truncated_content = if line_content.len() > 85 {
                        format!("{}...", &line_content[..82])
                    } else {
                        line_content
                    };

                    output.push_str(&format!("{}{} {:<85}│\n", prefix, line_indicator, truncated_content));
                }
            }
        }

        output.push_str("└─────────────────────────────────────────────────────────────────────────────────────┘\n");
        
        // Add summary
        let lines1 = text1.lines().count();
        let lines2 = text2.lines().count();
        output.push_str(&format!("\n📊 Comparison Summary:\n"));
        output.push_str(&format!("   {} response: {} lines\n", env1, lines1));
        output.push_str(&format!("   {} response: {} lines\n", env2, lines2));
        
        if lines1 != lines2 {
            output.push_str(&format!("   Line count difference: {}\n", (lines1 as i32 - lines2 as i32).abs()));
        }

        output
    }

    /// Generate summary for very large responses
    fn generate_large_response_summary(&self, text1: &str, text2: &str, env1: &str, env2: &str) -> String {
        let mut output = String::new();
        
        output.push_str("┌─────────────────────────────────────────────────────────────────────────────────────┐\n");
        output.push_str("│ 🔍 Large Response Comparison Summary                                                    │\n");
        output.push_str("├─────────────────────────────────────────────────────────────────────────────────────┤\n");
        output.push_str("│ ⚠️  Responses are too large for detailed diff - showing summary only                   │\n");
        output.push_str("└─────────────────────────────────────────────────────────────────────────────────────┘\n");
        
        let lines1 = text1.lines().count();
        let lines2 = text2.lines().count();
        let size1 = text1.len();
        let size2 = text2.len();
        
        output.push_str(&format!("\n📊 Size Comparison:\n"));
        output.push_str(&format!("   {} response: {} bytes, {} lines\n", env1, size1, lines1));
        output.push_str(&format!("   {} response: {} bytes, {} lines\n", env2, size2, lines2));
        output.push_str(&format!("   Size difference: {} bytes\n", (size1 as i32 - size2 as i32).abs()));
        
        if lines1 != lines2 {
            output.push_str(&format!("   Line count difference: {}\n", (lines1 as i32 - lines2 as i32).abs()));
        }
        
        // Try to detect what kind of differences exist
        let first_lines1: Vec<_> = text1.lines().take(10).collect();
        let first_lines2: Vec<_> = text2.lines().take(10).collect();
        
        if first_lines1 != first_lines2 {
            output.push_str("\n🔍 Sample Differences (first 10 lines):\n");
            for (i, (line1, line2)) in first_lines1.iter().zip(first_lines2.iter()).enumerate() {
                if line1 != line2 {
                    output.push_str(&format!("   Line {}: content differs\n", i + 1));
                    if output.lines().count() > 20 { // Limit output
                        output.push_str("   ... (more differences)\n");
                        break;
                    }
                }
            }
        }
        
        output.push_str("\n💡 Tip: Use curl commands or reduce response size for detailed comparison\n");
        output
    }

    /// Normalize whitespace in text for comparison with content-type awareness
    fn normalize_whitespace(&self, text: &str) -> String {
        self.normalize_by_content_type(text, None)
    }

    /// Normalize content based on detected or specified content type
    fn normalize_by_content_type(&self, text: &str, content_type: Option<&str>) -> String {
        // Auto-detect content type if not provided
        let detected_content_type = content_type.unwrap_or_else(|| self.detect_content_type(text));

        match detected_content_type {
            "application/json" | "json" => self.normalize_json(text),
            "application/xml" | "text/xml" | "xml" => self.normalize_xml(text),
            "text/html" | "html" => self.normalize_html(text),
            "text/plain" | "text" | _ => self.normalize_plain_text(text),
        }
    }

    /// Detect content type from text content
    fn detect_content_type(&self, text: &str) -> &str {
        let trimmed = text.trim();
        
        if (trimmed.starts_with('{') && trimmed.ends_with('}')) || 
           (trimmed.starts_with('[') && trimmed.ends_with(']')) {
            if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                return "application/json";
            }
        }
        
        if trimmed.starts_with('<') && trimmed.contains('>') {
            return "application/xml";
        }
        
        "text/plain"
    }

    /// Normalize JSON content for semantic comparison
    fn normalize_json(&self, text: &str) -> String {
        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(json_value) => {
                // Pretty print with consistent formatting
                serde_json::to_string_pretty(&json_value)
                    .unwrap_or_else(|_| text.to_string())
            }
            Err(_) => {
                // Fallback to plain text normalization if JSON parsing fails
                self.normalize_plain_text(text)
            }
        }
    }

    /// Normalize XML content (basic implementation)
    fn normalize_xml(&self, text: &str) -> String {
        // Basic XML normalization - remove extra whitespace between tags
        // Note: For production use, consider using a proper XML parser
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Normalize HTML content (basic implementation)
    fn normalize_html(&self, text: &str) -> String {
        // Basic HTML normalization - similar to XML but preserve some structure
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Normalize plain text content
    fn normalize_plain_text(&self, text: &str) -> String {
        text.lines()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join("\n")
    }


}

impl Default for ResponseComparator {
    fn default() -> Self {
        Self::new()
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
        let comparator = ResponseComparator::new();
        
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, r#"{"status": "ok"}"#));
        responses.insert("prod".to_string(), create_test_response(200, r#"{"status": "ok"}"#));

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        assert!(result.is_identical);
        assert!(result.differences.is_empty());
    }

    #[test]
    fn test_different_status_codes() {
        let comparator = ResponseComparator::new();
        
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, r#"{"status": "ok"}"#));
        responses.insert("prod".to_string(), create_test_response(404, r#"{"error": "not found"}"#));

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        assert!(!result.is_identical);
        assert_eq!(result.differences.len(), 2); // Status + body difference
        
        let status_diff = result.differences.iter()
            .find(|d| d.category == DifferenceCategory::Status);
        assert!(status_diff.is_some());
    }

    #[test]
    fn test_different_bodies() {
        let comparator = ResponseComparator::new();
        
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, r#"{"status": "ok", "data": "test"}"#));
        responses.insert("prod".to_string(), create_test_response(200, r#"{"status": "ok", "data": "prod"}"#));

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        assert!(!result.is_identical);
        
        let body_diff = result.differences.iter()
            .find(|d| d.category == DifferenceCategory::Body);
        assert!(body_diff.is_some());
        assert!(body_diff.unwrap().diff_output.is_some());
    }

    #[test]
    fn test_header_case_insensitive_comparison() {
        let comparator = ResponseComparator::new().with_headers_comparison();
        
        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);
        
        // Add headers with different cases
        response1.headers.insert("Content-Type".to_string(), "application/json".to_string());
        response2.headers.insert("content-type".to_string(), "application/json".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        // Should be identical even with different header case
        assert!(result.is_identical);
    }

    #[test]
    fn test_header_value_differences() {
        let comparator = ResponseComparator::new().with_headers_comparison();
        
        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);
        
        response1.headers.insert("X-Version".to_string(), "1.0".to_string());
        response2.headers.insert("X-Version".to_string(), "2.0".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        assert!(!result.is_identical);
        
        let header_diff = result.differences.iter()
            .find(|d| d.category == DifferenceCategory::Headers);
        assert!(header_diff.is_some());
        
        let diff = header_diff.unwrap();
        assert!(diff.description.contains("X-Version"));
        assert!(diff.description.contains("1.0"));
        assert!(diff.description.contains("2.0"));
        assert!(diff.diff_output.is_some());
    }

    #[test]
    fn test_json_semantic_comparison() {
        let comparator = ResponseComparator::new();
        
        // Same JSON data but with different formatting
        let json1 = r#"{"name":"John","age":30,"city":"NYC"}"#;
        let json2 = r#"{
  "age": 30,
  "city": "NYC",
  "name": "John"
}"#;

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, json1));
        responses.insert("prod".to_string(), create_test_response(200, json2));

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        // Should be identical due to JSON semantic comparison
        assert!(result.is_identical);
    }

    #[test]
    fn test_content_type_detection() {
        let comparator = ResponseComparator::new();
        
        // Test JSON detection
        assert_eq!(comparator.detect_content_type(r#"{"key": "value"}"#), "application/json");
        assert_eq!(comparator.detect_content_type(r#"[1, 2, 3]"#), "application/json");
        
        // Test XML detection
        assert_eq!(comparator.detect_content_type("<root><item>value</item></root>"), "application/xml");
        
        // Test plain text fallback
        assert_eq!(comparator.detect_content_type("This is plain text"), "text/plain");
        assert_eq!(comparator.detect_content_type("{invalid json"), "text/plain");
    }

    #[test]
    fn test_github_style_diff_generation() {
        let comparator = ResponseComparator::new();
        
        let text1 = "line1\nline2\nline3";
        let text2 = "line1\nmodified_line2\nline3";
        
        let diff_output = comparator.generate_github_style_diff(text1, text2, "test", "prod");
        
        assert!(diff_output.contains("TEST vs PROD"));
        assert!(diff_output.contains("🔴"));  // Delete indicator
        assert!(diff_output.contains("🟢"));  // Insert indicator
        assert!(diff_output.contains("line2"));
        assert!(diff_output.contains("modified_line2"));
        assert!(diff_output.contains("📊 Comparison Summary"));
    }

    #[test]
    fn test_large_response_handling() {
        let comparator = ResponseComparator::new();
        
        // Create large JSON responses
        let mut large_json1 = serde_json::Map::new();
        let mut large_json2 = serde_json::Map::new();
        
        for i in 0..1000 {
            large_json1.insert(format!("key_{}", i), serde_json::Value::String(format!("value_{}", i)));
            large_json2.insert(format!("key_{}", i), serde_json::Value::String(
                if i == 500 { "different_value".to_string() } else { format!("value_{}", i) }
            ));
        }
        
        let json1 = serde_json::to_string(&large_json1).unwrap();
        let json2 = serde_json::to_string(&large_json2).unwrap();

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, &json1));
        responses.insert("prod".to_string(), create_test_response(200, &json2));

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        assert!(!result.is_identical);
        assert_eq!(result.differences.len(), 1);
        assert_eq!(result.differences[0].category, DifferenceCategory::Body);
    }

    #[test]
    fn test_ignored_headers() {
        let comparator = ResponseComparator::new().with_headers_comparison();
        
        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);
        
        // Add headers that should be ignored
        response1.headers.insert("date".to_string(), "Mon, 01 Jan 2024 00:00:00 GMT".to_string());
        response2.headers.insert("date".to_string(), "Tue, 02 Jan 2024 00:00:00 GMT".to_string());
        response1.headers.insert("x-request-id".to_string(), "req-123".to_string());
        response2.headers.insert("x-request-id".to_string(), "req-456".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        // Should be identical because ignored headers are not compared
        assert!(result.is_identical);
    }

    #[test]
    fn test_custom_comparator_settings() {
        let ignore_headers = vec!["custom-header".to_string()];
        let comparator = ResponseComparator::with_full_settings(ignore_headers, false, true, 100, 50_000);
        
        let mut response1 = create_test_response(200, "  line1  \n  line2  ");
        let mut response2 = create_test_response(200, "line1\nline2");
        
        response1.headers.insert("custom-header".to_string(), "value1".to_string());
        response2.headers.insert("custom-header".to_string(), "value2".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        // Should not be identical because whitespace normalization is disabled
        assert!(!result.is_identical);
        
        // Custom header should be ignored (even with headers comparison enabled)
        let header_diffs: Vec<_> = result.differences.iter()
            .filter(|d| d.category == DifferenceCategory::Headers)
            .collect();
        assert!(header_diffs.is_empty());
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

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        // Should be identical because headers comparison is disabled by default
        assert!(result.is_identical);
        
        // No header differences should be reported
        let header_diffs: Vec<_> = result.differences.iter()
            .filter(|d| d.category == DifferenceCategory::Headers)
            .collect();
        assert!(header_diffs.is_empty());
    }

    #[test]
    fn test_default_comparison_scope() {
        let comparator = ResponseComparator::new();
        
        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(404, r#"{"error": "not found"}"#);
        
        // Add different headers (should be ignored)
        response1.headers.insert("X-Version".to_string(), "1.0".to_string());
        response2.headers.insert("X-Version".to_string(), "2.0".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator.compare_responses(
            "test-route".to_string(),
            HashMap::new(),
            responses,
        ).unwrap();

        // Should not be identical due to status and body differences
        assert!(!result.is_identical);
        
        // Should have status and body differences, but no header differences
        let status_diffs: Vec<_> = result.differences.iter()
            .filter(|d| d.category == DifferenceCategory::Status)
            .collect();
        assert!(!status_diffs.is_empty());
        
        let body_diffs: Vec<_> = result.differences.iter()
            .filter(|d| d.category == DifferenceCategory::Body)
            .collect();
        assert!(!body_diffs.is_empty());
        
        let header_diffs: Vec<_> = result.differences.iter()
            .filter(|d| d.category == DifferenceCategory::Headers)
            .collect();
        assert!(header_diffs.is_empty()); // Headers not compared by default
    }
} 