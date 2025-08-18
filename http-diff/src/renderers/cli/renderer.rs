//! CLI renderer for terminal output with colors and formatting

use super::{ComparisonFormatter, ErrorRenderer};
use crate::analysis::{ErrorAnalyzer, ErrorClassifierImpl};
use crate::comparison::analyzer::{BodyDiff, HeaderDiff};
use crate::renderers::OutputRenderer;
use crate::types::{
    ComparisonResult, DiffViewStyle, DifferenceCategory, ErrorSummary, ExecutionResult,
};
use std::collections::HashMap;

/// CLI renderer that produces the original colored terminal output
pub struct CliRenderer {
    /// Whether to include error analysis in output
    pub include_errors: bool,
    /// Comparison formatter for diff output
    formatter: ComparisonFormatter,
    /// Default diff view style to use
    diff_style: DiffViewStyle,
}

impl CliRenderer {
    /// Create a new CLI renderer
    pub fn new() -> Self {
        Self {
            include_errors: true,
            formatter: ComparisonFormatter::new(),
            diff_style: DiffViewStyle::Unified,
        }
    }

    /// Create a CLI renderer without error analysis
    pub fn without_errors() -> Self {
        Self {
            include_errors: false,
            formatter: ComparisonFormatter::new(),
            diff_style: DiffViewStyle::Unified,
        }
    }

    /// Create a CLI renderer with custom diff style
    pub fn with_diff_style(mut self, diff_style: DiffViewStyle) -> Self {
        self.diff_style = diff_style;
        self
    }

    /// Map execution error messages to appropriate HTTP status codes
    fn map_execution_error_to_status_code(&self, error_message: &str) -> u16 {
        if error_message.contains("error sending request")
            || error_message.contains("connection refused")
            || error_message.contains("Connection refused")
            || error_message.contains("connect refused")
        {
            503 // Service Unavailable - can't connect to service
        } else if error_message.contains("timeout")
            || error_message.contains("timed out")
            || error_message.contains("Timeout")
        {
            504 // Gateway Timeout
        } else if error_message.contains("dns")
            || error_message.contains("DNS")
            || error_message.contains("name resolution")
            || error_message.contains("Name resolution")
            || error_message.contains("failed to resolve")
        {
            502 // Bad Gateway - DNS resolution failed
        } else if error_message.contains("certificate")
            || error_message.contains("Certificate")
            || error_message.contains("SSL")
            || error_message.contains("TLS")
            || error_message.contains("ssl")
            || error_message.contains("tls")
        {
            503 // Service Unavailable - SSL/TLS issues
        } else {
            500 // Internal Server Error - generic execution failure
        }
    }
}

impl Default for CliRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for CliRenderer {
    fn render(&self, execution_result: &ExecutionResult) -> String {
        // Convert execution errors to comparison results for unified analysis
        let mut all_results = execution_result.comparisons.clone();

        // Convert execution errors to comparison results
        for exec_error in &execution_result.errors {
            let mut error_result = ComparisonResult::new(exec_error.route.clone(), HashMap::new());
            error_result.has_errors = true;

            // Smart status code mapping based on error type
            let status_code = self.map_execution_error_to_status_code(&exec_error.message);

            if let Some(env) = &exec_error.environment {
                error_result.status_codes.insert(env.clone(), status_code);
                let mut error_bodies = HashMap::new();
                error_bodies.insert(env.clone(), exec_error.message.clone());
                error_result.error_bodies = Some(error_bodies);
            }

            all_results.push(error_result);
        }

        format_comparison_results(
            &all_results,
            self.include_errors,
            &self.formatter,
            self.diff_style.clone(),
        )
    }
}

/// Format comparison results for CLI display
fn format_comparison_results(
    results: &[ComparisonResult],
    include_errors: bool,
    formatter: &ComparisonFormatter,
    diff_style: DiffViewStyle,
) -> String {
    let mut output = String::new();

    let total_tests = results.len();
    // Make categories mutually exclusive: Failed > Identical > Different
    let failed_count = results.iter().filter(|r| r.has_errors).count();
    let identical_count = results
        .iter()
        .filter(|r| r.is_identical && !r.has_errors)
        .count();
    let different_count = results
        .iter()
        .filter(|r| !r.is_identical && !r.has_errors)
        .count();

    // Priority 1: Enhanced Summary Format - structured multi-line format
    if total_tests > 0 {
        let identical_rate = (identical_count as f32 / total_tests as f32) * 100.0;
        output.push_str(&format!(
            "✅ Identical:     {}/{} ({:.1}%)\n",
            identical_count, total_tests, identical_rate
        ));
        if different_count > 0 {
            output.push_str(&format!(
                "❌ Different:     {}/{} ({:.1}%)\n",
                different_count,
                total_tests,
                (different_count as f32 / total_tests as f32) * 100.0
            ));
        }
        if failed_count > 0 {
            output.push_str(&format!(
                "🔥 Failed:        {}/{} ({:.1}%)\n",
                failed_count,
                total_tests,
                (failed_count as f32 / total_tests as f32) * 100.0
            ));
        }
    } else {
        output.push_str("No test scenarios found\n");
    }

    // Generate error analysis using clean business logic + presentation separation
    let error_summary = ErrorSummary::from_comparison_results(results);
    if error_summary.failed_requests > 0 && include_errors {
        let error_analyzer = ErrorClassifierImpl::new();
        let error_analysis = error_analyzer.analyze_errors(results);
        let error_renderer = ErrorRenderer::new();
        output.push_str(&error_renderer.render_error_analysis(&error_analysis));
    }

    if different_count > 0 {
        output.push_str("\nDIFFERENCES FOUND\n");
        output.push_str("═════════════════\n");
        for result in results.iter().filter(|r| !r.is_identical && !r.has_errors) {
            output.push_str(&format_route_group(result, formatter, diff_style.clone()));
        }
    } else if identical_count == total_tests {
        output.push_str("\n✅ All responses are identical across environments!");
    }

    output
}

/// Format route differences with improved visual grouping
fn format_route_group(
    result: &ComparisonResult,
    formatter: &ComparisonFormatter,
    diff_style: DiffViewStyle,
) -> String {
    let mut output = String::new();

    // Format user context more concisely
    let user_context = if result.user_context.is_empty() {
        "default".to_string()
    } else {
        result
            .user_context
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    };

    // Simple route header only
    let route_header = format!("📍 Route: {} | User: {}", result.route_name, user_context);
    output.push_str(&format!("{}\n", route_header));

    // Add detailed diff output - deserialize raw data and format it
    let env_names: Vec<String> = result.responses.keys().cloned().collect();
    if env_names.len() >= 2 {
        // Orient to base if present
        let (env1, env2) = if let Some(base) = &result.base_environment {
            let other = env_names
                .iter()
                .find(|e| *e != base)
                .cloned()
                .unwrap_or_else(|| env_names[1].clone());
            (base.clone(), other)
        } else {
            (env_names[0].clone(), env_names[1].clone())
        };

        for difference in &result.differences {
            let icon = match difference.category {
                DifferenceCategory::Status => "🚨",
                DifferenceCategory::Headers => "📝",
                DifferenceCategory::Body => "📄",
            };

            if let Some(diff_data) = &difference.diff_output {
                output.push('\n');

                match difference.category {
                    DifferenceCategory::Headers => {
                        // Deserialize header diff data and format it
                        if let Ok(header_diffs) = serde_json::from_str::<Vec<HeaderDiff>>(diff_data)
                        {
                            let formatted_diff = formatter.format_header_differences(
                                &header_diffs,
                                &env1,
                                &env2,
                                diff_style.clone(),
                            );
                            output.push_str(&format!(
                                "{} Header Differences:\n{}\n",
                                icon, formatted_diff
                            ));
                        } else {
                            output.push_str(&format!("  {} {}\n", icon, difference.description));
                        }
                    }
                    DifferenceCategory::Body => {
                        // Deserialize body diff data and format it
                        if let Ok(body_diff) = serde_json::from_str::<BodyDiff>(diff_data) {
                            let formatted_diff = formatter.format_body_difference(
                                &body_diff,
                                &env1,
                                &env2,
                                diff_style.clone(),
                            );
                            output.push_str(&format!(
                                "{} Body Differences:\n{}\n",
                                icon, formatted_diff
                            ));
                        } else {
                            output.push_str(&format!("  {} {}\n", icon, difference.description));
                        }
                    }
                    DifferenceCategory::Status => {
                        // Status differences don't need special formatting
                        output.push_str(&format!("  {} {}\n", icon, difference.description));
                    }
                }
            } else if !difference.description.is_empty() {
                // Show at least the description if no diff output is available
                output.push_str(&format!("  {} {}\n", icon, difference.description));
            }
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
        responses.insert(
            "test".to_string(),
            crate::types::HttpResponse {
                status: 200,
                headers: HashMap::new(),
                body: "test response".to_string(),
                url: "https://test.example.com".to_string(),
                curl_command: "curl test".to_string(),
            },
        );

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
            base_environment: None,
        }
    }

    #[test]
    fn test_cli_renderer() {
        use crate::types::ExecutionResult;
        
        let renderer = CliRenderer::new();
        let execution_result = ExecutionResult {
            comparisons: vec![create_test_result()],
            progress: crate::execution::progress::ProgressTracker::new(1),
            errors: Vec::new(),
        };
        let output = renderer.render(&execution_result);

        assert!(output.contains("✅ Identical:"));
        assert!(output.contains("1/1 (100.0%)"));
    }
}
