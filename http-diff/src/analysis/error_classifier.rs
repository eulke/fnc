//! Pure business logic for error classification and analysis
//! No presentation concerns - returns structured data only

use crate::types::{ComparisonResult, ErrorSeverity};
use std::collections::{HashMap, HashSet};

/// Structured analysis result containing only business data
#[derive(Debug, Clone)]
pub struct ErrorAnalysis {
    /// Number of routes with different failures across environments
    pub critical_issues: usize,
    /// Number of routes with same errors in all environments
    pub consistent_failures: usize,
    /// Total number of failed requests
    pub total_failed: usize,
    /// Total number of requests analyzed
    pub total_requests: usize,
    /// Percentage of failed requests
    pub failure_percentage: f32,
    /// Grouped errors by type and severity
    pub error_groups: Vec<ErrorGroup>,
}

/// A group of errors of the same type and severity
#[derive(Debug, Clone)]
pub struct ErrorGroup {
    /// Type of error (e.g., "ValidationError", "DependencyError")
    pub error_type: String,
    /// Severity level of this error group
    pub severity: ErrorSeverity,
    /// Routes affected by this error type
    pub affected_routes: Vec<RouteError>,
    /// Unique error messages found in this group
    pub unique_error_messages: Vec<String>,
    /// Debugging suggestion for this error type (business logic)
    pub debugging_suggestion: Option<String>,
}

/// Information about a specific route error
#[derive(Debug, Clone)]
pub struct RouteError {
    /// Name of the route that failed
    pub route_name: String,
    /// User context data
    pub user_context: HashMap<String, String>,
    /// Status codes across environments
    pub status_codes: Vec<u16>,
    /// Whether status codes are consistent across environments
    pub has_consistent_status: bool,
}

/// Trait for error analysis - pure business logic
pub trait ErrorAnalyzer: Send + Sync {
    /// Analyze comparison results and return structured error data
    fn analyze_errors(&self, results: &[ComparisonResult]) -> ErrorAnalysis;
    
    /// Extract error type from response body
    fn extract_error_type(&self, body: &str) -> String;
    
    /// Determine severity based on error type and status codes
    fn determine_severity(&self, error_type: &str, status_codes: &[u16]) -> ErrorSeverity;
    
    /// Get debugging suggestion for error type and status
    fn get_debugging_suggestion(&self, error_type: &str, status: u16) -> Option<String>;
    
    /// Format error body for analysis (business logic only)
    fn format_error_message(&self, body: &str, status_code: Option<u16>) -> String;
}

/// Default implementation of error analyzer
pub struct ErrorClassifierImpl;

impl ErrorClassifierImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ErrorClassifierImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorAnalyzer for ErrorClassifierImpl {
    fn analyze_errors(&self, results: &[ComparisonResult]) -> ErrorAnalysis {
        let total_requests = results.len();
        let failed_results: Vec<&ComparisonResult> = results.iter().filter(|r| r.has_errors).collect();
        let total_failed = failed_results.len();
        
        // Calculate critical issues (different failures across environments)
        let critical_issues = results.iter()
            .filter(|r| r.has_errors && !r.has_consistent_status())
            .count();
        
        // Calculate consistent failures (same errors in all environments)
        let consistent_failures = results.iter()
            .filter(|r| r.has_errors && r.has_consistent_status())
            .count();
        
        let failure_percentage = if total_requests > 0 {
            (total_failed as f32 / total_requests as f32) * 100.0
        } else {
            0.0
        };
        
        // Group errors by type
        let error_groups = self.group_errors_by_type(&failed_results);
        
        ErrorAnalysis {
            critical_issues,
            consistent_failures,
            total_failed,
            total_requests,
            failure_percentage,
            error_groups,
        }
    }
    
    fn extract_error_type(&self, body: &str) -> String {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(error) = json_value.get("error").and_then(|v| v.as_str()) {
                return error.to_string();
            }
        }
        
        // Fallback: try to extract from common patterns
        if body.contains("DependencyError") || body.contains("dependency") {
            "DependencyError".to_string()
        } else if body.contains("UnhandledError") || body.contains("unhandled") {
            "UnhandledError".to_string()
        } else if body.contains("ValidationError") || body.contains("validation") {
            "ValidationError".to_string()
        } else {
            "Unknown".to_string()
        }
    }
    
    fn determine_severity(&self, error_type: &str, status_codes: &[u16]) -> ErrorSeverity {
        let max_status = status_codes.iter().max().unwrap_or(&0);
        
        match (error_type, max_status) {
            (_, status) if *status >= 500 => ErrorSeverity::Critical,
            ("DependencyError", _) | (_, 424 | 502 | 503) => ErrorSeverity::Dependency,
            (_, status) if *status >= 400 => ErrorSeverity::Client,
            _ => ErrorSeverity::Client, // Default for all other cases
        }
    }
    
    fn get_debugging_suggestion(&self, error_type: &str, status: u16) -> Option<String> {
        match (error_type, status) {
            ("DependencyError", 424) => Some("Check dependent service health and connectivity".to_string()),
            ("UnhandledError", 500) => Some("Review request payload structure and required fields".to_string()),
            ("ValidationError", 400) => Some("Verify request parameters and data format".to_string()),
            (_, 500) => Some("Check application logs for internal errors".to_string()),
            (_, 424) => Some("Verify dependent services are operational".to_string()),
            _ => None,
        }
    }
    
    fn format_error_message(&self, body: &str, status_code: Option<u16>) -> String {
        // Try to parse as JSON for better structure
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(obj) = json_value.as_object() {
                let mut parts = Vec::new();
                
                if let Some(error) = obj.get("error").and_then(|v| v.as_str()) {
                    parts.push(format!("Type: {}", error));
                }
                
                if let Some(message) = obj.get("message").and_then(|v| v.as_str()) {
                    let truncated = if message.len() > 100 {
                        format!("{}...", &message[..100])
                    } else {
                        message.to_string()
                    };
                    parts.push(format!("Message: {}", truncated));
                }
                
                if let Some(status_code) = obj.get("statusCode").and_then(|v| v.as_u64()) {
                    parts.push(format!("Code: {}", status_code));
                }
                
                if !parts.is_empty() {
                    return parts.join(" | ");
                }
            }
        }
        
        // Fallback to truncated original body
        let formatted_body = self.truncate_response_body(body, 150);
        
        // If the body is empty or whitespace-only, use status code description
        if formatted_body.trim().is_empty() {
            if let Some(status) = status_code {
                self.get_friendly_error_message(status)
            } else {
                "No error details provided in response body".to_string()
            }
        } else {
            formatted_body
        }
    }
}

impl ErrorClassifierImpl {
    /// Group failed requests by error type - pure business logic
    fn group_errors_by_type(&self, failed_results: &[&ComparisonResult]) -> Vec<ErrorGroup> {
        let mut error_groups: HashMap<String, Vec<&ComparisonResult>> = HashMap::new();
        
        for result in failed_results {
            let error_type = if let Some(error_bodies) = &result.error_bodies {
                if let Some(first_body) = error_bodies.values().next() {
                    self.extract_error_type(first_body)
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            };
            
            error_groups.entry(error_type).or_default().push(result);
        }
        
        // Convert to structured error groups
        let mut groups = Vec::new();
        
        for (error_type, group_results) in error_groups {
            let status_codes: Vec<u16> = group_results.iter()
                .flat_map(|r| r.status_codes.values())
                .copied()
                .collect();
            
            let severity = self.determine_severity(&error_type, &status_codes);
            
            let affected_routes: Vec<RouteError> = group_results.iter()
                .map(|result| RouteError {
                    route_name: result.route_name.clone(),
                    user_context: result.user_context.clone(),
                    status_codes: result.status_codes.values().copied().collect(),
                    has_consistent_status: result.has_consistent_status(),
                })
                .collect();
            
            // Collect unique error messages
            let mut unique_errors = HashSet::new();
            for result in &group_results {
                if let Some(error_bodies) = &result.error_bodies {
                    for (env_name, body) in error_bodies {
                        let status_code = result.status_codes.get(env_name).copied();
                        let formatted_message = self.format_error_message(body, status_code);
                        unique_errors.insert(formatted_message);
                    }
                }
            }
            
            // Get debugging suggestion for this error type
            let debugging_suggestion = if let Some(sample_result) = group_results.first() {
                if let Some(status) = sample_result.status_codes.values().next() {
                    self.get_debugging_suggestion(&error_type, *status)
                } else {
                    None
                }
            } else {
                None
            };
            
            groups.push(ErrorGroup {
                error_type,
                severity,
                affected_routes,
                unique_error_messages: unique_errors.into_iter().collect(),
                debugging_suggestion,
            });
        }
        
        // Sort by severity: Critical > Dependency > Client
        groups.sort_by(|a, b| {
            let severity_order = |s: &ErrorSeverity| match s {
                ErrorSeverity::Critical => 1,
                ErrorSeverity::Dependency => 2,
                ErrorSeverity::Client => 3,
            };
            severity_order(&a.severity).cmp(&severity_order(&b.severity))
        });
        
        groups
    }
    
    /// Truncate response body - utility function
    fn truncate_response_body(&self, body: &str, max_length: usize) -> String {
        if body.len() <= max_length {
            body.to_string()
        } else {
            format!("{}... (truncated)", &body[..max_length])
        }
    }
    
    /// Get friendly error message based on status code - business logic only
    fn get_friendly_error_message(&self, status: u16) -> String {
        match status {
            400 => "Request contains invalid data or missing required fields".to_string(),
            401 => "Authentication failed - check credentials or authorization headers".to_string(),
            403 => "Access denied - insufficient permissions for this resource".to_string(),
            404 => "Requested resource or endpoint not found".to_string(),
            424 => "Dependent service is unavailable or failing".to_string(),
            500 => "Internal server error occurred - check application logs".to_string(),
            502 => "Gateway error - upstream service not responding correctly".to_string(),
            503 => "Service temporarily unavailable - likely overloaded or under maintenance".to_string(),
            504 => "Request timed out - service taking too long to respond".to_string(),
            _ => format!("Service returned error status {} with no additional details", status),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ComparisonResult;
    use std::collections::HashMap;

    fn create_test_result_with_error(route_name: &str, status: u16, error_body: &str) -> ComparisonResult {
        let mut result = ComparisonResult::new(route_name.to_string(), HashMap::new());
        
        let mut status_codes = HashMap::new();
        status_codes.insert("test".to_string(), status);
        result.status_codes = status_codes;
        
        if status >= 400 {
            result.has_errors = true;
            let mut error_bodies = HashMap::new();
            error_bodies.insert("test".to_string(), error_body.to_string());
            result.error_bodies = Some(error_bodies);
        }
        
        result
    }

    #[test]
    fn test_error_analysis_calculation() {
        let analyzer = ErrorClassifierImpl::new();
        
        let results = vec![
            create_test_result_with_error("route1", 500, r#"{"error": "UnhandledError"}"#),
            create_test_result_with_error("route2", 400, r#"{"error": "ValidationError"}"#),
        ];
        
        let analysis = analyzer.analyze_errors(&results);
        
        assert_eq!(analysis.total_requests, 2);
        assert_eq!(analysis.total_failed, 2);
        assert_eq!(analysis.failure_percentage, 100.0);
        assert_eq!(analysis.error_groups.len(), 2);
    }

    #[test]
    fn test_error_type_extraction() {
        let analyzer = ErrorClassifierImpl::new();
        
        assert_eq!(analyzer.extract_error_type(r#"{"error": "ValidationError"}"#), "ValidationError");
        assert_eq!(analyzer.extract_error_type("DependencyError occurred"), "DependencyError");
        assert_eq!(analyzer.extract_error_type("random text"), "Unknown");
    }

    #[test]
    fn test_severity_determination() {
        let analyzer = ErrorClassifierImpl::new();
        
        assert_eq!(analyzer.determine_severity("any", &[500]), ErrorSeverity::Critical);
        assert_eq!(analyzer.determine_severity("DependencyError", &[400]), ErrorSeverity::Dependency);
        assert_eq!(analyzer.determine_severity("ValidationError", &[400]), ErrorSeverity::Client);
    }
}