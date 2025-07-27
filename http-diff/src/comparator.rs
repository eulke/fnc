use crate::error::{HttpDiffError, Result};
use crate::types::{HttpResponse, ComparisonResult, Difference, DifferenceCategory, DiffViewStyle};

/// Represents a header difference for grouped display
#[derive(Debug, Clone)]
struct HeaderDiff {
    name: String,
    value1: Option<String>, // Value in first environment
    value2: Option<String>, // Value in second environment  
}
use crate::table_builder::{TableBuilder, presets};
use crate::formatter::TextFormatter;
use std::collections::HashMap;



/// Response comparator with configurable comparison strategies
pub struct ResponseComparator {
    ignore_headers: Vec<String>,
    ignore_whitespace: bool,
    large_response_threshold: usize,
    compare_headers: bool,
    diff_view_style: DiffViewStyle,
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
            large_response_threshold: 50_000,        // 50KB
            compare_headers: false,                  // Headers comparison disabled by default
            diff_view_style: DiffViewStyle::Unified, // Backward compatible default
        }
    }

    /// Create a comparator with custom settings
    pub fn with_settings(ignore_headers: Vec<String>, ignore_whitespace: bool) -> Self {
        Self {
            ignore_headers,
            ignore_whitespace,
            large_response_threshold: 50_000,
            compare_headers: false, // Headers comparison disabled by default
            diff_view_style: DiffViewStyle::Unified, // Backward compatible default
        }
    }

    /// Create a comparator with full control over all settings
    pub fn with_full_settings(
        ignore_headers: Vec<String>,
        ignore_whitespace: bool,
        compare_headers: bool,
        large_response_threshold: usize,
    ) -> Self {
        Self {
            ignore_headers,
            ignore_whitespace,
            large_response_threshold,
            compare_headers,
            diff_view_style: DiffViewStyle::Unified, // Backward compatible default
        }
    }

    /// Enable headers comparison (disabled by default)
    pub fn with_headers_comparison(mut self) -> Self {
        self.compare_headers = true;
        self
    }

    /// Set the diff view style (unified or side-by-side)
    pub fn with_diff_view_style(mut self, style: DiffViewStyle) -> Self {
        self.diff_view_style = style;
        self
    }

    /// Enable side-by-side diff view for easier comparison
    pub fn with_side_by_side_diff(mut self) -> Self {
        self.diff_view_style = DiffViewStyle::SideBySide;
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
                    let header_diffs =
                        self.compare_headers(&response1.headers, &response2.headers, env1, env2);
                    differences.extend(header_diffs);
                }

                // Compare bodies
                if let Some(body_diff) =
                    self.compare_bodies(&response1.body, &response2.body, env1, env2)?
                {
                    differences.push(body_diff);
                }
            }
        }

        let is_identical = differences.is_empty();

        // Extract status codes from responses
        let mut status_codes = HashMap::new();
        let mut error_bodies = HashMap::new();
        let mut has_errors = false;

        for (env_name, response) in &responses {
            status_codes.insert(env_name.clone(), response.status);

            // Check if this is an error response (non-2xx)
            if response.status < 200 || response.status >= 300 {
                has_errors = true;
                error_bodies.insert(env_name.clone(), response.body.clone());
            }
        }

        let error_bodies = if error_bodies.is_empty() {
            None
        } else {
            Some(error_bodies)
        };

        Ok(ComparisonResult {
            route_name,
            user_context,
            responses,
            differences,
            is_identical,
            status_codes,
            has_errors,
            error_bodies,
        })
    }

    /// Compare response headers with case-insensitive handling and grouped output
    fn compare_headers(
        &self,
        headers1: &HashMap<String, String>,
        headers2: &HashMap<String, String>,
        env1: &str,
        env2: &str,
    ) -> Vec<Difference> {
        // Normalize headers to lowercase for comparison while preserving original case for display
        let normalize_headers =
            |headers: &HashMap<String, String>| -> HashMap<String, (String, String)> {
                headers
                    .iter()
                    .filter(|(key, _)| !self.ignore_headers.contains(&key.to_lowercase()))
                    .map(|(k, v)| (k.to_lowercase(), (k.clone(), v.clone())))
                    .collect()
            };

        let normalized_headers1 = normalize_headers(headers1);
        let normalized_headers2 = normalize_headers(headers2);

        // Collect all header differences for grouped display
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

        // Generate grouped output if there are any header differences
        if !header_differences.is_empty() {
            let diff_output = self.generate_headers_diff_table(&header_differences, env1, env2);

            vec![Difference {
                category: DifferenceCategory::Headers,
                description: String::new(),
                diff_output: Some(diff_output),
            }]
        } else {
            Vec::new()
        }
    }

    /// Generate a grouped table for all header differences respecting diff view style
    fn generate_headers_diff_table(
        &self,
        header_differences: &[HeaderDiff],
        env1: &str,
        env2: &str,
    ) -> String {
        match self.diff_view_style {
            DiffViewStyle::Unified => {
                self.generate_headers_unified_table(header_differences, env1, env2)
            }
            DiffViewStyle::SideBySide => {
                self.generate_headers_side_by_side_table(header_differences, env1, env2)
            }
        }
    }

    /// Generate unified diff table for headers 
    fn generate_headers_unified_table(
        &self,
        header_differences: &[HeaderDiff],
        env1: &str,
        env2: &str,
    ) -> String {
        use crate::table_builder::{TableBuilder, TableStyle, cells};
        
        let mut table = TableBuilder::new();
        table.apply_style(TableStyle::Diff); // Remove horizontal lines
        table.headers(vec!["Header", "Environment", "Value"]);
        
        for diff in header_differences {
            if let Some(value1) = &diff.value1 {
                table.styled_row(vec![
                    cells::normal(&diff.name),
                    cells::removed(&format!("- {}", env1.to_uppercase())),
                    cells::removed(value1)
                ]);
            }
            if let Some(value2) = &diff.value2 {
                table.styled_row(vec![
                    cells::normal(&diff.name),
                    cells::added(&format!("+ {}", env2.to_uppercase())),
                    cells::added(value2)
                ]);
            }
        }
        
        format!("{}", table.build())
    }

    /// Generate side-by-side diff table for headers
    fn generate_headers_side_by_side_table(
        &self,
        header_differences: &[HeaderDiff],
        env1: &str,
        env2: &str,
    ) -> String {
        use crate::table_builder::{TableBuilder, TableStyle, cells};
        
        let mut table = TableBuilder::new();
        table.apply_style(TableStyle::Diff);
        table.headers(vec!["Header", &env1.to_uppercase(), &env2.to_uppercase()]);
        
        for diff in header_differences {
            let left_value = match &diff.value1 {
                Some(value) => cells::normal(value),
                None => cells::muted("(missing)")
            };
            
            let right_value = match &diff.value2 {
                Some(value) => cells::normal(value),
                None => cells::muted("(missing)")
            };
            
            // Color the header name if values are different
            let header_cell = if diff.value1 != diff.value2 {
                cells::bold(&diff.name)
            } else {
                cells::normal(&diff.name)
            };
            
            table.styled_row(vec![header_cell, left_value, right_value]);
        }
        
        format!("{}", table.build())
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
            (
                self.normalize_whitespace(body1),
                self.normalize_whitespace(body2),
            )
        } else {
            (body1.to_string(), body2.to_string())
        };

        if normalized_body1 == normalized_body2 {
            return Ok(None);
        }

        // Generate diff output based on configured style
        let diff_output = match self.diff_view_style {
            DiffViewStyle::Unified => {
                self.generate_unified_diff(&normalized_body1, &normalized_body2, env1, env2)
            }
            DiffViewStyle::SideBySide => {
                self.generate_side_by_side_diff(&normalized_body1, &normalized_body2, env1, env2)
            }
        };

        Ok(Some(Difference {
            category: DifferenceCategory::Body,
            description: format!("Body Comparison"),
            diff_output: Some(diff_output),
        }))
    }

    /// Create a clean diff table with optimal sizing and ANSI support
    fn create_diff_table(&self) -> TableBuilder {
        presets::diff_table()
    }

    /// Generate unified diff using prettydiff's native styling with proper table handling
    fn generate_unified_diff(&self, text1: &str, text2: &str, env1: &str, env2: &str) -> String {
        let total_size = text1.len() + text2.len();

        // For very large responses, provide a summary instead of full diff
        if total_size > self.large_response_threshold {
            return self.generate_large_response_summary(text1, text2, env1, env2);
        }

        // Use TextFormatter for unified diff generation  
        let formatter = TextFormatter::new();
        let diff_output = formatter.unified_diff(text1, text2, env1, env2);

        // Create table using shared implementation
        let mut table = self.create_diff_table();
        for line in diff_output.lines() {
            table.line(line);
        }
        let table_output = table.build();

        format!("\n{}\n", table_output)
    }

    /// Generate side-by-side diff using proper table rendering with automatic terminal width detection
    fn generate_side_by_side_diff(
        &self,
        text1: &str,
        text2: &str,
        env1: &str,
        env2: &str,
    ) -> String {
        let total_size = text1.len() + text2.len();

        // For very large responses, provide a summary instead of full diff
        if total_size > self.large_response_threshold {
            return self.generate_large_response_summary(text1, text2, env1, env2);
        }

        // Use TextFormatter for side-by-side diff generation
        let formatter = TextFormatter::new();
        formatter.side_by_side_diff(text1, text2, env1, env2)
    }

    /// Generate summary for very large responses using proper table formatting
    fn generate_large_response_summary(
        &self,
        text1: &str,
        text2: &str,
        env1: &str,
        env2: &str,
    ) -> String {
        // Create a simple summary table for large responses
        let lines1 = text1.lines().count();
        let lines2 = text2.lines().count();
        let size1 = text1.len();
        let size2 = text2.len();

        let mut table = presets::summary_table(vec!["Environment", "Response Size", "Line Count"]);
        table.row(vec![env1, &format!("{} bytes", size1), &lines1.to_string()]);
        table.row(vec![env2, &format!("{} bytes", size2), &lines2.to_string()]);
        let table = table.build();

        let mut output = String::new();
        output.push_str("\n🔍 Large Response Comparison Summary\n");
        output.push_str("⚠️  Responses are too large for detailed diff - showing summary only\n\n");
        output.push_str(&table);

        // Add size difference analysis
        output.push_str("\n📈 Differences:\n");
        let size_diff = (size1 as i64 - size2 as i64).abs();
        output.push_str(&format!("   Size difference: {} bytes\n", size_diff));

        if lines1 != lines2 {
            let line_diff = (lines1 as i64 - lines2 as i64).abs();
            output.push_str(&format!("   Line count difference: {} lines\n", line_diff));
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

        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
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
                serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| text.to_string())
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
        responses.insert(
            "test".to_string(),
            create_test_response(200, r#"{"status": "ok"}"#),
        );
        responses.insert(
            "prod".to_string(),
            create_test_response(200, r#"{"status": "ok"}"#),
        );

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(result.is_identical);
        assert!(result.differences.is_empty());
    }

    #[test]
    fn test_different_status_codes() {
        let comparator = ResponseComparator::new();

        let mut responses = HashMap::new();
        responses.insert(
            "test".to_string(),
            create_test_response(200, r#"{"status": "ok"}"#),
        );
        responses.insert(
            "prod".to_string(),
            create_test_response(404, r#"{"error": "not found"}"#),
        );

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(!result.is_identical);
        assert_eq!(result.differences.len(), 2); // Status + body difference

        let status_diff = result
            .differences
            .iter()
            .find(|d| d.category == DifferenceCategory::Status);
        assert!(status_diff.is_some());
    }

    #[test]
    fn test_different_bodies() {
        let comparator = ResponseComparator::new();

        let mut responses = HashMap::new();
        responses.insert(
            "test".to_string(),
            create_test_response(200, r#"{"status": "ok", "data": "test"}"#),
        );
        responses.insert(
            "prod".to_string(),
            create_test_response(200, r#"{"status": "ok", "data": "prod"}"#),
        );

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(!result.is_identical);

        let body_diff = result
            .differences
            .iter()
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
        response1
            .headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        response2
            .headers
            .insert("content-type".to_string(), "application/json".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Should be identical even with different header case
        assert!(result.is_identical);
    }

    #[test]
    fn test_header_value_differences() {
        let comparator = ResponseComparator::new().with_headers_comparison();

        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);

        response1
            .headers
            .insert("X-Version".to_string(), "1.0".to_string());
        response2
            .headers
            .insert("X-Version".to_string(), "2.0".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        assert!(!result.is_identical);

        let header_diff = result
            .differences
            .iter()
            .find(|d| d.category == DifferenceCategory::Headers);
        assert!(header_diff.is_some());

        let diff = header_diff.unwrap();
        assert_eq!(diff.description, "");
        assert!(diff.diff_output.is_some());
        
        // Check that the grouped header output contains the expected information
        let diff_output = diff.diff_output.as_ref().unwrap();
        assert!(diff_output.contains("X-Version"));
        assert!(diff_output.contains("1.0"));
        assert!(diff_output.contains("2.0"));
    }

    #[test]
    fn test_grouped_headers_diff_styles() {
        // Test that headers are properly grouped in both unified and side-by-side styles
        let unified_comparator = ResponseComparator::new()
            .with_headers_comparison()
            .with_diff_view_style(DiffViewStyle::Unified);
        
        let side_by_side_comparator = ResponseComparator::new()
            .with_headers_comparison()
            .with_diff_view_style(DiffViewStyle::SideBySide);

        let mut response1 = create_test_response(200, r#"{"status": "ok"}"#);
        let mut response2 = create_test_response(200, r#"{"status": "ok"}"#);

        // Add multiple header differences
        response1.headers.insert("X-Version".to_string(), "1.0".to_string());
        response1.headers.insert("X-Environment".to_string(), "prod".to_string());
        response2.headers.insert("X-Version".to_string(), "2.0".to_string());
        response2.headers.insert("X-Environment".to_string(), "staging".to_string());
        response2.headers.insert("X-New-Header".to_string(), "new-value".to_string());

        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), response1);
        responses.insert("test".to_string(), response2);

        // Test unified format
        let unified_result = unified_comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses.clone())
            .unwrap();

        let unified_header_diff = unified_result
            .differences
            .iter()
            .find(|d| d.category == DifferenceCategory::Headers)
            .unwrap();

        let unified_output = unified_header_diff.diff_output.as_ref().unwrap();
        assert!(unified_output.contains("X-Version"));
        assert!(unified_output.contains("X-Environment"));
        assert!(unified_output.contains("X-New-Header"));

        // Test side-by-side format
        let side_by_side_result = side_by_side_comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        let side_by_side_header_diff = side_by_side_result
            .differences
            .iter()
            .find(|d| d.category == DifferenceCategory::Headers)
            .unwrap();

        let side_by_side_output = side_by_side_header_diff.diff_output.as_ref().unwrap();
        assert!(side_by_side_output.contains("PROD"));
        assert!(side_by_side_output.contains("TEST"));
        assert!(side_by_side_output.contains("X-Version"));
        assert!(side_by_side_output.contains("(missing)"));
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

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Should be identical due to JSON semantic comparison
        assert!(result.is_identical);
    }

    #[test]
    fn test_content_type_detection() {
        let comparator = ResponseComparator::new();

        // Test JSON detection
        assert_eq!(
            comparator.detect_content_type(r#"{"key": "value"}"#),
            "application/json"
        );
        assert_eq!(
            comparator.detect_content_type(r#"[1, 2, 3]"#),
            "application/json"
        );

        // Test XML detection
        assert_eq!(
            comparator.detect_content_type("<root><item>value</item></root>"),
            "application/xml"
        );

        // Test plain text fallback
        assert_eq!(
            comparator.detect_content_type("This is plain text"),
            "text/plain"
        );
        assert_eq!(
            comparator.detect_content_type("{invalid json"),
            "text/plain"
        );
    }

    #[test]
    fn test_github_style_diff_generation() {
        let comparator = ResponseComparator::new();

        let text1 = "line1\nline2\nline3";
        let text2 = "line1\nmodified_line2\nline3";

        let diff_output = comparator.generate_unified_diff(text1, text2, "test", "prod");

        assert!(diff_output.contains("line2"));
        assert!(diff_output.contains("modified_line2"));
        // prettydiff uses colored - and + markers (we can't easily test ANSI colors in unit tests)
    }

    #[test]
    fn test_large_response_handling() {
        let comparator = ResponseComparator::new();

        // Create large JSON responses
        let mut large_json1 = serde_json::Map::new();
        let mut large_json2 = serde_json::Map::new();

        for i in 0..1000 {
            large_json1.insert(
                format!("key_{}", i),
                serde_json::Value::String(format!("value_{}", i)),
            );
            large_json2.insert(
                format!("key_{}", i),
                serde_json::Value::String(if i == 500 {
                    "different_value".to_string()
                } else {
                    format!("value_{}", i)
                }),
            );
        }

        let json1 = serde_json::to_string(&large_json1).unwrap();
        let json2 = serde_json::to_string(&large_json2).unwrap();

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), create_test_response(200, &json1));
        responses.insert("prod".to_string(), create_test_response(200, &json2));

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

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
        response1.headers.insert(
            "date".to_string(),
            "Mon, 01 Jan 2024 00:00:00 GMT".to_string(),
        );
        response2.headers.insert(
            "date".to_string(),
            "Tue, 02 Jan 2024 00:00:00 GMT".to_string(),
        );
        response1
            .headers
            .insert("x-request-id".to_string(), "req-123".to_string());
        response2
            .headers
            .insert("x-request-id".to_string(), "req-456".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Should be identical because ignored headers are not compared
        assert!(result.is_identical);
    }

    #[test]
    fn test_custom_comparator_settings() {
        let ignore_headers = vec!["custom-header".to_string()];
        let comparator =
            ResponseComparator::with_full_settings(ignore_headers, false, true, 50_000);

        let mut response1 = create_test_response(200, "  line1  \n  line2  ");
        let mut response2 = create_test_response(200, "line1\nline2");

        response1
            .headers
            .insert("custom-header".to_string(), "value1".to_string());
        response2
            .headers
            .insert("custom-header".to_string(), "value2".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Should not be identical because whitespace normalization is disabled
        assert!(!result.is_identical);

        // Custom header should be ignored (even with headers comparison enabled)
        let header_diffs: Vec<_> = result
            .differences
            .iter()
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
        response1
            .headers
            .insert("X-Version".to_string(), "1.0".to_string());
        response2
            .headers
            .insert("X-Version".to_string(), "2.0".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Should be identical because headers comparison is disabled by default
        assert!(result.is_identical);

        // No header differences should be reported
        let header_diffs: Vec<_> = result
            .differences
            .iter()
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
        response1
            .headers
            .insert("X-Version".to_string(), "1.0".to_string());
        response2
            .headers
            .insert("X-Version".to_string(), "2.0".to_string());

        let mut responses = HashMap::new();
        responses.insert("test".to_string(), response1);
        responses.insert("prod".to_string(), response2);

        let result = comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Should not be identical due to status and body differences
        assert!(!result.is_identical);

        // Should have status and body differences, but no header differences
        let status_diffs: Vec<_> = result
            .differences
            .iter()
            .filter(|d| d.category == DifferenceCategory::Status)
            .collect();
        assert!(!status_diffs.is_empty());

        let body_diffs: Vec<_> = result
            .differences
            .iter()
            .filter(|d| d.category == DifferenceCategory::Body)
            .collect();
        assert!(!body_diffs.is_empty());

        let header_diffs: Vec<_> = result
            .differences
            .iter()
            .filter(|d| d.category == DifferenceCategory::Headers)
            .collect();
        assert!(header_diffs.is_empty()); // Headers not compared by default
    }

    #[test]
    fn test_diff_view_style_configuration() {
        // Test default is unified
        let default_comparator = ResponseComparator::new();
        assert_eq!(default_comparator.diff_view_style, DiffViewStyle::Unified);

        // Test explicit unified setting
        let unified_comparator =
            ResponseComparator::new().with_diff_view_style(DiffViewStyle::Unified);
        assert_eq!(unified_comparator.diff_view_style, DiffViewStyle::Unified);

        // Test side-by-side setting
        let side_by_side_comparator =
            ResponseComparator::new().with_diff_view_style(DiffViewStyle::SideBySide);
        assert_eq!(
            side_by_side_comparator.diff_view_style,
            DiffViewStyle::SideBySide
        );

        // Test convenience method
        let convenient_comparator = ResponseComparator::new().with_side_by_side_diff();
        assert_eq!(
            convenient_comparator.diff_view_style,
            DiffViewStyle::SideBySide
        );
    }

    #[test]
    fn test_unified_diff_generation() {
        let comparator = ResponseComparator::new().with_diff_view_style(DiffViewStyle::Unified);

        let text1 = "line1\nline2\nline3";
        let text2 = "line1\nmodified_line2\nline3";

        let diff_output = comparator.generate_unified_diff(text1, text2, "test", "prod");

        // Should contain unified diff markers (prettydiff format)
        assert!(diff_output.contains("line2"));
        assert!(diff_output.contains("modified_line2"));
        // prettydiff uses colored - and + markers (we can't easily test ANSI colors in unit tests)
    }

    #[test]
    fn test_side_by_side_diff_generation() {
        let comparator = ResponseComparator::new().with_diff_view_style(DiffViewStyle::SideBySide);

        let text1 = "line1\nline2\nline3";
        let text2 = "line1\nmodified_line2\nline3";

        let diff_output = comparator.generate_side_by_side_diff(text1, text2, "test", "prod");

        // Should have proper table formatting with borders and uppercase environments
        assert!(diff_output.contains("┌") || diff_output.contains("╭")); // Top border
        assert!(diff_output.contains("└") || diff_output.contains("╰")); // Bottom border  
        assert!(diff_output.contains("TEST")); // Uppercase environment labels
        assert!(diff_output.contains("PROD")); 
        assert!(diff_output.contains("line1")); // Content
        assert!(diff_output.contains("modified_line2"));
    }

    #[test]
    fn test_diff_view_style_affects_comparison_result() {
        let unified_comparator =
            ResponseComparator::new().with_diff_view_style(DiffViewStyle::Unified);
        let side_by_side_comparator =
            ResponseComparator::new().with_diff_view_style(DiffViewStyle::SideBySide);

        let mut responses = HashMap::new();
        responses.insert(
            "test".to_string(),
            create_test_response(200, "line1\nline2\nline3"),
        );
        responses.insert(
            "prod".to_string(),
            create_test_response(200, "line1\nmodified_line2\nline3"),
        );

        let unified_result = unified_comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses.clone())
            .unwrap();

        let side_by_side_result = side_by_side_comparator
            .compare_responses("test-route".to_string(), HashMap::new(), responses)
            .unwrap();

        // Both should detect the difference
        assert!(!unified_result.is_identical);
        assert!(!side_by_side_result.is_identical);
        assert_eq!(
            unified_result.differences.len(),
            side_by_side_result.differences.len()
        );

        // But should have different diff output formats
        let unified_diff = &unified_result.differences[0].diff_output;
        let side_by_side_diff = &side_by_side_result.differences[0].diff_output;

        assert!(unified_diff.is_some());
        assert!(side_by_side_diff.is_some());

        // Both diffs should be properly formatted and contain content
        assert!(unified_diff.is_some());
        assert!(side_by_side_diff.is_some());
        // prettydiff uses colored - and + markers (we can't easily test ANSI colors in unit tests)
    }

    #[test]
    fn test_backward_compatibility_with_existing_constructors() {
        // Test that all existing constructors default to unified diff
        let default_comparator = ResponseComparator::new();
        assert_eq!(default_comparator.diff_view_style, DiffViewStyle::Unified);

        let with_settings_comparator = ResponseComparator::with_settings(vec![], true);
        assert_eq!(
            with_settings_comparator.diff_view_style,
            DiffViewStyle::Unified
        );

        let full_settings_comparator =
            ResponseComparator::with_full_settings(vec![], true, false, 50_000);
        assert_eq!(
            full_settings_comparator.diff_view_style,
            DiffViewStyle::Unified
        );
    }

    #[test]
    fn test_large_response_handling_both_diff_styles() {
        // Test that large responses are handled properly with both diff styles
        let large_content = "x".repeat(60_000); // Larger than threshold
        let large_content_modified = "y".repeat(60_000);

        let unified_comparator =
            ResponseComparator::new().with_diff_view_style(DiffViewStyle::Unified);
        let side_by_side_comparator =
            ResponseComparator::new().with_diff_view_style(DiffViewStyle::SideBySide);

        let unified_output = unified_comparator.generate_unified_diff(
            &large_content,
            &large_content_modified,
            "test",
            "prod",
        );
        let side_by_side_output = side_by_side_comparator.generate_side_by_side_diff(
            &large_content,
            &large_content_modified,
            "test",
            "prod",
        );

        // Both should provide large response summaries
        assert!(unified_output.contains("Large Response Comparison Summary"));
        assert!(side_by_side_output.contains("Large Response Comparison Summary"));
        assert!(unified_output.contains("Environment"));
        assert!(side_by_side_output.contains("Environment"));
        assert!(unified_output.contains("Response Size"));
        assert!(side_by_side_output.contains("Response Size"));
    }
}
