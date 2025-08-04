//! Pure presentation logic for error analysis rendering
//! Contains all formatting, emojis, and terminal-specific display logic

use crate::analysis::{ErrorAnalysis, ErrorGroup};
use crate::types::ErrorSeverity;

/// Renderer for error analysis with configurable presentation
pub struct ErrorRenderer {
    /// Whether to include emoji icons in output
    pub use_emojis: bool,
    /// Whether to use Unicode box drawing characters
    pub use_unicode_symbols: bool,
}

impl ErrorRenderer {
    /// Create a new error renderer with default settings (emojis enabled)
    pub fn new() -> Self {
        Self {
            use_emojis: true,
            use_unicode_symbols: true,
        }
    }
    
    /// Create an error renderer without emojis (for logs, plain text)
    pub fn without_emojis() -> Self {
        Self {
            use_emojis: false,
            use_unicode_symbols: false,
        }
    }
    
    /// Render complete error analysis to formatted string
    pub fn render_error_analysis(&self, analysis: &ErrorAnalysis) -> String {
        let mut output = String::new();
        
        // Header
        output.push_str(&self.format_header());
        output.push_str(&self.format_separator());
        
        // Summary statistics
        output.push_str(&self.format_summary_stats(analysis));
        
        // Detailed error groups if there are failures
        if !analysis.error_groups.is_empty() {
            output.push_str(&self.format_error_groups(&analysis.error_groups));
        }
        
        output
    }
    
    /// Format the main header
    fn format_header(&self) -> String {
        if self.use_emojis {
            "\n🚨 ERROR ANALYSIS\n".to_string()
        } else {
            "\nERROR ANALYSIS\n".to_string()
        }
    }
    
    /// Format separator line
    fn format_separator(&self) -> String {
        if self.use_unicode_symbols {
            "═══════════════\n".to_string()
        } else {
            "===============\n".to_string()
        }
    }
    
    /// Format summary statistics
    fn format_summary_stats(&self, analysis: &ErrorAnalysis) -> String {
        format!(
            "Critical Issues:     {} (different failures across envs)\n\
             Consistent Failures: {} (same errors in all environments)\n\
             Total Failed:        {}/{} requests ({:.1}%)\n",
            analysis.critical_issues,
            analysis.consistent_failures,
            analysis.total_failed,
            analysis.total_requests,
            analysis.failure_percentage
        )
    }
    
    /// Format detailed error groups
    fn format_error_groups(&self, error_groups: &[ErrorGroup]) -> String {
        let mut output = String::new();
        
        // Section header
        output.push_str("\nFAILED REQUESTS BY ERROR TYPE\n");
        if self.use_unicode_symbols {
            output.push_str("═══════════════════════════════\n");
        } else {
            output.push_str("===============================\n");
        }
        
        for group in error_groups {
            output.push_str(&self.format_error_group(group));
        }
        
        output
    }
    
    /// Format a single error group
    fn format_error_group(&self, group: &ErrorGroup) -> String {
        let mut output = String::new();
        
        let icon = self.get_severity_icon(&group.severity);
        let route_count = group.affected_routes.len();
        let route_suffix = if route_count == 1 { "" } else { "s" };
        
        output.push_str(&format!(
            "\n{} {} errors ({} route{})\n",
            icon, group.error_type, route_count, route_suffix
        ));
        
        // Show affected routes
        for route_error in &group.affected_routes {
            let user_context = if route_error.user_context.is_empty() {
                "default".to_string()
            } else {
                route_error.user_context.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            
            let status_display = if route_error.has_consistent_status && !route_error.status_codes.is_empty() {
                // All statuses are the same, show just one
                route_error.status_codes[0].to_string()
            } else if !route_error.status_codes.is_empty() {
                // Different statuses across environments
                format!("{:?}", route_error.status_codes)
            } else {
                "unknown".to_string()
            };
            
            let tree_symbol = if self.use_unicode_symbols { "└─" } else { "+-" };
            output.push_str(&format!(
                "  {} {} [{}] | Status: {}\n",
                tree_symbol, route_error.route_name, user_context, status_display
            ));
        }
        
        // Show unique error messages
        for error_message in &group.unique_error_messages {
            output.push_str(&format!("     Issue: {}\n", error_message));
        }
        
        // Add debugging suggestion if available
        if let Some(suggestion) = &group.debugging_suggestion {
            let suggestion_icon = if self.use_emojis { "💡" } else { "TIP:" };
            output.push_str(&format!("     {} Suggestion: {}\n", suggestion_icon, suggestion));
        }
        
        output
    }
    
    /// Get icon for error severity
    fn get_severity_icon(&self, severity: &ErrorSeverity) -> &str {
        if self.use_emojis {
            match severity {
                ErrorSeverity::Critical => "🔥",
                ErrorSeverity::Dependency => "🔗",
                ErrorSeverity::Client => "⚠️",
            }
        } else {
            match severity {
                ErrorSeverity::Critical => "[CRITICAL]",
                ErrorSeverity::Dependency => "[DEPENDENCY]",
                ErrorSeverity::Client => "[CLIENT]",
            }
        }
    }
}

impl Default for ErrorRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{RouteError};
    use std::collections::HashMap;

    fn create_test_error_group() -> ErrorGroup {
        ErrorGroup {
            error_type: "ValidationError".to_string(),
            severity: ErrorSeverity::Client,
            affected_routes: vec![
                RouteError {
                    route_name: "test-route".to_string(),
                    user_context: HashMap::new(),
                    status_codes: vec![400],
                    has_consistent_status: true,
                }
            ],
            unique_error_messages: vec!["Invalid request format".to_string()],
            debugging_suggestion: Some("Check request parameters".to_string()),
        }
    }

    #[test]
    fn test_error_renderer_with_emojis() {
        let renderer = ErrorRenderer::new();
        let analysis = ErrorAnalysis {
            critical_issues: 1,
            consistent_failures: 0,
            total_failed: 1,
            total_requests: 2,
            failure_percentage: 50.0,
            error_groups: vec![create_test_error_group()],
        };
        
        let output = renderer.render_error_analysis(&analysis);
        
        assert!(output.contains("🚨 ERROR ANALYSIS"));
        assert!(output.contains("⚠️ ValidationError"));
        assert!(output.contains("💡 Suggestion"));
        assert!(output.contains("═"));
    }

    #[test]
    fn test_error_renderer_without_emojis() {
        let renderer = ErrorRenderer::without_emojis();
        let analysis = ErrorAnalysis {
            critical_issues: 1,
            consistent_failures: 0,
            total_failed: 1,
            total_requests: 2,
            failure_percentage: 50.0,
            error_groups: vec![create_test_error_group()],
        };
        
        let output = renderer.render_error_analysis(&analysis);
        
        assert!(output.contains("ERROR ANALYSIS"));
        assert!(output.contains("[CLIENT] ValidationError"));
        assert!(output.contains("TIP: Suggestion"));
        assert!(!output.contains("🚨"));
        assert!(!output.contains("⚠️"));
        assert!(!output.contains("💡"));
    }

    #[test]
    fn test_severity_icons() {
        let renderer_with_emojis = ErrorRenderer::new();
        let renderer_without_emojis = ErrorRenderer::without_emojis();
        
        assert_eq!(renderer_with_emojis.get_severity_icon(&ErrorSeverity::Critical), "🔥");
        assert_eq!(renderer_without_emojis.get_severity_icon(&ErrorSeverity::Critical), "[CRITICAL]");
        
        assert_eq!(renderer_with_emojis.get_severity_icon(&ErrorSeverity::Client), "⚠️");
        assert_eq!(renderer_without_emojis.get_severity_icon(&ErrorSeverity::Client), "[CLIENT]");
    }
}