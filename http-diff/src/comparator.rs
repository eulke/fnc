use crate::client::HttpResponse;
use crate::error::{HttpDiffError, Result};
use colored::*;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement,
    Table, TableComponent,
};
use prettydiff::basic::DiffOp;
use std::collections::HashMap;

/// Result of comparing two HTTP responses
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub route_name: String,
    pub user_context: HashMap<String, String>,
    pub responses: HashMap<String, HttpResponse>,
    pub differences: Vec<Difference>,
    pub is_identical: bool,
    // New fields for error tracking
    pub status_codes: HashMap<String, u16>, // env_name -> status_code
    pub has_errors: bool,                   // true if any non-2xx status
    pub error_bodies: Option<HashMap<String, String>>, // env_name -> response_body (only for errors)
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

/// Diff view style configuration for text differences
#[derive(Debug, Clone, PartialEq)]
pub enum DiffViewStyle {
    /// Traditional unified diff (up/down) - default, backward compatible
    Unified,
    /// Side-by-side diff view for easier comparison
    SideBySide,
}

/// Summary of error statistics across all comparison results
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorSummary {
    pub total_requests: usize,
    pub successful_requests: usize, // 2xx status codes
    pub failed_requests: usize,     // non-2xx status codes
    pub identical_successes: usize, // identical 2xx responses
    pub identical_failures: usize,  // identical non-2xx responses
    pub mixed_responses: usize,     // different status codes across envs
}

impl ErrorSummary {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            identical_successes: 0,
            identical_failures: 0,
            mixed_responses: 0,
        }
    }

    pub fn from_comparison_results(results: &[ComparisonResult]) -> Self {
        let mut summary = Self::new();
        summary.total_requests = results.len();

        for result in results {
            let statuses: Vec<u16> = result.status_codes.values().cloned().collect();
            let all_successful = statuses.iter().all(|&status| status >= 200 && status < 300);
            let all_same_status = statuses.windows(2).all(|w| w[0] == w[1]);

            // First check if all status codes are the same
            if all_same_status {
                if all_successful {
                    summary.successful_requests += 1;
                    if result.is_identical {
                        summary.identical_successes += 1;
                    }
                } else {
                    summary.failed_requests += 1;
                    if result.is_identical {
                        summary.identical_failures += 1;
                    }
                }
            } else {
                // Different status codes across environments = mixed responses
                summary.failed_requests += 1;
                summary.mixed_responses += 1;
            }
        }

        summary
    }
}

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
                    let diff_output =
                        self.generate_header_diff(original_key1, value1, value2, env1, env2);
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
    fn generate_header_diff(
        &self,
        header_name: &str,
        value1: &str,
        value2: &str,
        env1: &str,
        env2: &str,
    ) -> String {
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
            description: format!("Response body differs between {} and {}", env1, env2),
            diff_output: Some(diff_output),
        }))
    }

    /// Create a clean diff table with optimal sizing and ANSI support
    fn create_diff_table(&self, headers: Vec<Cell>) -> Table {
        let mut table = Table::new();

        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .remove_style(TableComponent::HorizontalLines)
            .remove_style(TableComponent::LeftBorderIntersections)
            .remove_style(TableComponent::RightBorderIntersections)
            .remove_style(TableComponent::MiddleIntersections)
            .set_header(headers);

        table
    }

    /// Generate unified diff using prettydiff's native styling with proper table handling
    fn generate_unified_diff(&self, text1: &str, text2: &str, env1: &str, env2: &str) -> String {
        let total_size = text1.len() + text2.len();

        // For very large responses, provide a summary instead of full diff
        if total_size > self.large_response_threshold {
            return self.generate_large_response_summary(text1, text2, env1, env2);
        }

        // Use prettydiff's native unified diff output with colors
        let diff = prettydiff::diff_lines(text1, text2);
        let prettydiff_output = diff.to_string();

        // Create table using shared implementation
        let mut table = self.create_diff_table(vec![Cell::new(format!(
            "{} vs {} - Unified Response Body Comparison",
            env1.to_uppercase(),
            env2.to_uppercase()
        ))
        .add_attribute(Attribute::Bold)]);

        // Add prettydiff's native output line by line, preserving ANSI codes
        for line in prettydiff_output.lines() {
            // Create cell that preserves prettydiff's native styling
            let cell = Cell::new(line);
            table.add_row(vec![cell]);
        }

        let mut output = String::new();
        output.push_str(&format!("\n{}\n", table.to_string()));

        // Add summary
        let lines1 = text1.lines().count();
        let lines2 = text2.lines().count();
        output.push_str(&format!("\n📊 Comparison Summary:\n"));
        output.push_str(&format!("   {} response: {} lines\n", env1, lines1));
        output.push_str(&format!("   {} response: {} lines\n", env2, lines2));

        if lines1 != lines2 {
            output.push_str(&format!(
                "   Line count difference: {}\n",
                (lines1 as i32 - lines2 as i32).abs()
            ));
        }

        output
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

        // Use prettydiff to get structured diff data
        let diff = prettydiff::diff_lines(text1, text2);
        let diff_ops = diff.diff();

        // Create table using shared implementation
        let mut table = self.create_diff_table(vec![
            Cell::new(format!("{}", env1.to_uppercase())).add_attribute(Attribute::Bold),
            Cell::new(format!("{}", env2.to_uppercase())).add_attribute(Attribute::Bold),
        ]);

        // Process diff operations and add rows to table
        for op in diff_ops {
            match op {
                DiffOp::Equal(lines) => {
                    // Both sides have the same content - no special styling
                    for line in lines {
                        table.add_row(vec![Cell::new(line), Cell::new(line)]);
                    }
                }
                DiffOp::Replace(old_lines, new_lines) => {
                    // Handle replacements with proper color highlighting
                    let max_lines = old_lines.len().max(new_lines.len());
                    for i in 0..max_lines {
                        let left = old_lines.get(i).unwrap_or(&"");
                        let right = new_lines.get(i).unwrap_or(&"");

                        table.add_row(vec![
                            if !left.is_empty() {
                                Cell::new(left).fg(Color::Red)
                            } else {
                                Cell::new("")
                            },
                            if !right.is_empty() {
                                Cell::new(right).fg(Color::Green)
                            } else {
                                Cell::new("")
                            },
                        ]);
                    }
                }
                DiffOp::Remove(lines) => {
                    // Lines only in left side - red for removed
                    for line in lines {
                        table.add_row(vec![Cell::new(line).fg(Color::Red), Cell::new("")]);
                    }
                }
                DiffOp::Insert(lines) => {
                    // Lines only in right side - green for added
                    for line in lines {
                        table.add_row(vec![Cell::new(""), Cell::new(line).fg(Color::Green)]);
                    }
                }
            }
        }

        let mut output = String::new();

        // Add descriptive header
        output.push_str(&format!(
            "\n📊 {} vs {} - Side-by-Side Response Body Comparison\n",
            env1.to_uppercase(),
            env2.to_uppercase()
        ));

        // Add the properly formatted table
        output.push_str(&table.to_string());

        // Add informative legend
        output.push_str("\n🎨 Color Legend:\n");
        output.push_str(&format!("   {} Lines removed from {}\n", "Red".red(), env1));
        output.push_str(&format!("   {} Lines added in {}\n", "Green".green(), env2));

        // Add comparison summary
        let lines1 = text1.lines().count();
        let lines2 = text2.lines().count();
        output.push_str(&format!("\n📈 Comparison Summary:\n"));
        output.push_str(&format!("   {} response: {} lines\n", env1, lines1));
        output.push_str(&format!("   {} response: {} lines\n", env2, lines2));

        if lines1 != lines2 {
            let diff_count = (lines1 as i32 - lines2 as i32).abs();
            output.push_str(&format!("   Line count difference: {} lines\n", diff_count));
        }

        output
    }

    /// Generate summary for very large responses using proper table formatting
    fn generate_large_response_summary(
        &self,
        text1: &str,
        text2: &str,
        env1: &str,
        env2: &str,
    ) -> String {
        let lines1 = text1.lines().count();
        let lines2 = text2.lines().count();
        let size1 = text1.len();
        let size2 = text2.len();

        // Create a summary table for large responses
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Environment").add_attribute(Attribute::Bold),
                Cell::new("Response Size").add_attribute(Attribute::Bold),
                Cell::new("Line Count").add_attribute(Attribute::Bold),
            ]);

        table.add_row(vec![
            Cell::new(env1),
            Cell::new(format!("{} bytes", size1)),
            Cell::new(lines1.to_string()),
        ]);

        table.add_row(vec![
            Cell::new(env2),
            Cell::new(format!("{} bytes", size2)),
            Cell::new(lines2.to_string()),
        ]);

        let mut output = String::new();
        output.push_str("\n🔍 Large Response Comparison Summary\n");
        output.push_str("⚠️  Responses are too large for detailed diff - showing summary only\n\n");
        output.push_str(&table.to_string());

        // Add size difference analysis
        output.push_str("\n📈 Differences:\n");
        let size_diff = (size1 as i64 - size2 as i64).abs();
        output.push_str(&format!("   Size difference: {} bytes\n", size_diff));

        if lines1 != lines2 {
            let line_diff = (lines1 as i64 - lines2 as i64).abs();
            output.push_str(&format!("   Line count difference: {} lines\n", line_diff));
        }

        // Try to detect what kind of differences exist
        let first_lines1: Vec<_> = text1.lines().take(10).collect();
        let first_lines2: Vec<_> = text2.lines().take(10).collect();

        if first_lines1 != first_lines2 {
            output.push_str("\n🔍 Sample Differences (first 10 lines):\n");
            for (i, (line1, line2)) in first_lines1.iter().zip(first_lines2.iter()).enumerate() {
                if line1 != line2 {
                    output.push_str(&format!("   Line {}: content differs\n", i + 1));
                    if output.lines().count() > 20 {
                        // Limit output
                        output.push_str("   ... (more differences)\n");
                        break;
                    }
                }
            }
        }

        output.push_str(
            "\n💡 Tip: Use curl commands or reduce response size for detailed comparison\n",
        );
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

        assert!(diff_output.contains("TEST vs PROD"));
        assert!(diff_output.contains("Unified Response Body Comparison"));
        assert!(diff_output.contains("line2"));
        assert!(diff_output.contains("modified_line2"));
        assert!(diff_output.contains("📊 Comparison Summary"));
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
        assert!(diff_output.contains("TEST vs PROD"));
        assert!(diff_output.contains("Unified Response Body Comparison"));
        assert!(diff_output.contains("line2"));
        assert!(diff_output.contains("modified_line2"));
        assert!(diff_output.contains("📊 Comparison Summary"));
        // prettydiff uses colored - and + markers (we can't easily test ANSI colors in unit tests)
    }

    #[test]
    fn test_side_by_side_diff_generation() {
        let comparator = ResponseComparator::new().with_diff_view_style(DiffViewStyle::SideBySide);

        let text1 = "line1\nline2\nline3";
        let text2 = "line1\nmodified_line2\nline3";

        let diff_output = comparator.generate_side_by_side_diff(text1, text2, "test", "prod");

        // Should contain side-by-side diff formatting
        assert!(diff_output.contains("TEST vs PROD"));
        assert!(diff_output.contains("Side-by-Side"));
        assert!(diff_output.contains("📈 Comparison Summary"));
        assert!(diff_output.contains("test response: 3 lines"));
        assert!(diff_output.contains("prod response: 3 lines"));

        // Should have proper table formatting with UTF8 borders and rounded corners
        assert!(diff_output.contains("│")); // Vertical separators
        assert!(diff_output.contains("╭")); // Rounded top corners
        assert!(diff_output.contains("╰")); // Rounded bottom corners
        assert!(diff_output.contains("┆")); // Vertical column separator
                                            // Note: Horizontal lines between content rows are removed for cleaner appearance
        assert!(!diff_output.contains("├╌╌")); // Should NOT have horizontal row separators
        assert!(!diff_output.contains("├ ")); // Should NOT have left border intersections

        // Should show proper color legend
        assert!(diff_output.contains("🎨 Color Legend"));
        assert!(diff_output.contains("Lines removed from"));
        assert!(diff_output.contains("Lines added in"));

        // Should contain the header with environment names
        assert!(diff_output.contains("TEST"));
        assert!(diff_output.contains("PROD"));
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

        // Unified should contain prettydiff-style formatting
        assert!(unified_diff
            .as_ref()
            .unwrap()
            .contains("Unified Response Body Comparison"));
        // prettydiff uses colored - and + markers (we can't easily test ANSI colors in unit tests)

        // Side-by-side should contain its specific formatting
        assert!(side_by_side_diff.as_ref().unwrap().contains("Side-by-Side"));
        assert!(side_by_side_diff
            .as_ref()
            .unwrap()
            .contains("📈 Comparison Summary"));
        assert!(side_by_side_diff
            .as_ref()
            .unwrap()
            .contains("🎨 Color Legend"));
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
