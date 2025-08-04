//! CLI renderer for terminal output with colors and formatting

use crate::types::{ComparisonResult, ErrorSummary};
use super::OutputRenderer;

/// CLI renderer that produces the original colored terminal output
pub struct CliRenderer {
    /// Whether to include error analysis in output
    pub include_errors: bool,
}

impl CliRenderer {
    /// Create a new CLI renderer
    pub fn new() -> Self {
        Self {
            include_errors: true,
        }
    }

    /// Create a CLI renderer without error analysis
    pub fn without_errors() -> Self {
        Self {
            include_errors: false,
        }
    }
}

impl Default for CliRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for CliRenderer {
    fn render(&self, results: &[ComparisonResult]) -> String {
        format_comparison_results(results, self.include_errors)
    }
}

/// Format comparison results for CLI display
fn format_comparison_results(results: &[ComparisonResult], include_errors: bool) -> String {
    let mut output = String::new();
    
    let total_tests = results.len();
    // Make categories mutually exclusive: Failed > Identical > Different
    let failed_count = results.iter().filter(|r| r.has_errors).count();
    let identical_count = results.iter().filter(|r| r.is_identical && !r.has_errors).count();
    let different_count = results.iter().filter(|r| !r.is_identical && !r.has_errors).count();
    
    // Priority 1: Enhanced Summary Format - structured multi-line format
    if total_tests > 0 {
        let identical_rate = (identical_count as f32 / total_tests as f32) * 100.0;
        output.push_str(&format!("✅ Identical:     {}/{} ({:.1}%)\n", 
                                identical_count, total_tests, identical_rate));
        if different_count > 0 {
            output.push_str(&format!("❌ Different:     {}/{} ({:.1}%)\n", 
                                    different_count, total_tests, 
                                    (different_count as f32 / total_tests as f32) * 100.0));
        }
        if failed_count > 0 {
            output.push_str(&format!("🔥 Failed:        {}/{} ({:.1}%)\n", 
                                    failed_count, total_tests, 
                                    (failed_count as f32 / total_tests as f32) * 100.0));
        }
    } else {
        output.push_str("No test scenarios found\n");
    }
    
    // Generate error summary and add error analysis if there are failures AND include_errors is true
    let error_summary = ErrorSummary::from_comparison_results(results);
    if error_summary.failed_requests > 0 && include_errors {
        output.push_str(&crate::error_analysis::format_error_analysis(&error_summary, results));
    }

    if different_count > 0 {
        output.push_str("\nDIFFERENCES FOUND\n");
        output.push_str("═════════════════\n");
        for result in results.iter().filter(|r| !r.is_identical && !r.has_errors) {
            output.push_str(&format_route_group(result));
        }
    } else if identical_count == total_tests {
        output.push_str("\n✅ All responses are identical across environments!");
    }

    output
}

/// Format route differences with improved visual grouping
fn format_route_group(result: &ComparisonResult) -> String {
    let mut output = String::new();
    
    // Format user context more concisely
    let user_context = if result.user_context.is_empty() {
        "default".to_string()
    } else {
        result.user_context.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    };
    
    // Simple route header only
    let route_header = format!("📍 Route: {} | User: {}", result.route_name, user_context);
    output.push_str(&format!("{}\n", route_header));
    
    // Add detailed diff output if available
    for difference in &result.differences {
        if let Some(diff_output) = &difference.diff_output {
            output.push('\n');
            output.push_str(diff_output);
            output.push('\n');
        } else if !difference.description.is_empty() {
            // Show at least the description if no diff output is available
            let icon = match difference.category {
                crate::types::DifferenceCategory::Status => "🚨",
                crate::types::DifferenceCategory::Headers => "📝",
                crate::types::DifferenceCategory::Body => "📄",
            };
            output.push_str(&format!("  {} {}\n", icon, difference.description));
        }
    }
    
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_result() -> ComparisonResult {
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), crate::types::HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: "test response".to_string(),
            url: "https://test.example.com".to_string(),
            curl_command: "curl test".to_string(),
        });

        ComparisonResult {
            route_name: "test-route".to_string(),
            user_context: HashMap::new(),
            responses,
            differences: vec![],
            is_identical: true,
            status_codes: {
                let mut codes = HashMap::new();
                codes.insert("test".to_string(), 200);
                codes
            },
            has_errors: false,
            error_bodies: None,
        }
    }

    #[test]
    fn test_cli_renderer() {
        let renderer = CliRenderer::new();
        let results = vec![create_test_result()];
        let output = renderer.render(&results);
        
        assert!(output.contains("✅ Identical:"));
        assert!(output.contains("1/1 (100.0%)"));
    }
}