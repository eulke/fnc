//! Error analysis and classification for HTTP response failures
//!
//! This module provides functionality to analyze, classify, and format
//! error information from failed HTTP requests in a user-friendly way.

use crate::types::{ComparisonResult, ErrorSummary, ErrorSeverity};
use std::collections::{HashMap, HashSet};

/// Format error analysis section for display
pub fn format_error_analysis(error_summary: &ErrorSummary, results: &[ComparisonResult]) -> String {
    let mut output = String::new();
    
    output.push_str("\n🚨 ERROR ANALYSIS\n");
    output.push_str("═══════════════\n");
    
    let total_requests = results.len();
    
    // Calculate critical issues (different failures across environments)
    let critical_issues = error_summary.mixed_responses;
    
    output.push_str(&format!("Critical Issues:     {} (different failures across envs)\n", critical_issues));
    output.push_str(&format!("Consistent Failures: {} (same errors in all environments)\n", error_summary.identical_failures));
    output.push_str(&format!("Total Failed:        {}/{} requests ({:.1}%)\n", 
                            error_summary.failed_requests, total_requests, 
                            (error_summary.failed_requests as f32 / total_requests as f32) * 100.0));
    
    // Group failed requests by error type and show them organized
    let failed_results: Vec<&ComparisonResult> = results.iter().filter(|r| r.has_errors).collect();
    if !failed_results.is_empty() {
        output.push_str(&format_grouped_failed_requests(&failed_results));
    }
    
    output
}

/// Group and format failed requests by error type
fn format_grouped_failed_requests(failed_results: &[&ComparisonResult]) -> String {
    let mut output = String::new();
    
    // Group by error type
    let mut error_groups: HashMap<String, Vec<&ComparisonResult>> = HashMap::new();
    
    for result in failed_results {
        let error_type = if let Some(error_bodies) = &result.error_bodies {
            if let Some(first_body) = error_bodies.values().next() {
                extract_error_type(first_body)
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };
        
        error_groups.entry(error_type).or_default().push(result);
    }
    
    output.push_str("\nFAILED REQUESTS BY ERROR TYPE\n");
    output.push_str("═══════════════════════════════\n");
    
    // Sort groups by severity: Critical > Dependency > Client > Unknown
    let mut sorted_groups: Vec<_> = error_groups.into_iter().collect();
    sorted_groups.sort_by(|(error_type_a, results_a), (error_type_b, results_b)| {
        let severity_a = get_error_type_severity(error_type_a, results_a);
        let severity_b = get_error_type_severity(error_type_b, results_b);
        severity_a.cmp(&severity_b)
    });
    
    for (error_type, group_results) in sorted_groups {
        let severity_score = get_error_type_severity(&error_type, &group_results);
        let severity = score_to_severity(severity_score);
        let icon = get_error_icon(&severity);
       
        output.push_str(&format!("\n{} {} errors ({} route{})\n", 
            icon, 
            error_type, 
            group_results.len(),
            if group_results.len() == 1 { "" } else { "s" }
        ));
        
        // Show affected routes
        for result in &group_results {
            let user_context = if result.user_context.is_empty() {
                "default".to_string()
            } else {
                result.user_context.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            
            let status_values: Vec<u16> = result.status_codes.values().cloned().collect();
            let status_display = if !status_values.is_empty() && (status_values.len() == 1 || status_values.windows(2).all(|w| w[0] == w[1])) {
                // All statuses are the same, show just one
                status_values[0].to_string()
            } else if !status_values.is_empty() {
                // Different statuses across environments
                format!("{:?}", status_values)
            } else {
                // Fallback case (shouldn't happen)
                "unknown".to_string()
            };
            
            output.push_str(&format!("  └─ {} [{}] | Status: {}\n", 
                result.route_name, user_context, status_display));
        }
        
        // Show error details for each unique error message in this group
        let mut unique_errors: HashSet<String> = HashSet::new();
        
        for result in &group_results {
            if let Some(error_bodies) = &result.error_bodies {
                for (env_name, body) in error_bodies {
                    // Get the status code for this environment
                    let status_code = result.status_codes.get(env_name).copied();
                    let formatted_body = format_error_body_with_status(body, status_code);
                    if unique_errors.insert(formatted_body.clone()) {
                        output.push_str(&format!("     Issue: {}\n", formatted_body));
                    }
                }
            }
        }
        
        // Add debugging suggestion for this error type
        if let Some(sample_result) = group_results.first() {
            if let Some(status) = sample_result.status_codes.values().next() {
                if let Some(suggestion) = get_debugging_suggestion(&error_type, status) {
                    output.push_str(&format!("     💡 Suggestion: {}\n", suggestion));
                }
            }
        }
    }
    
    output    
}

/// Get error type severity for sorting
fn get_error_type_severity(error_type: &str, results: &[&ComparisonResult]) -> u8 {
    // Get max status code from this error group
    let max_status = results.iter()
        .flat_map(|r| r.status_codes.values())
        .max()
        .unwrap_or(&0);
        
    match (error_type, max_status) {
        (_, status) if *status >= 500 => 1, // Critical
        ("DependencyError", _) | (_, 424 | 502 | 503) => 2, // Dependency
        (_, status) if *status >= 400 => 3, // Client
        _ => 4, // Unknown
    }
}

/// Convert severity score to ErrorSeverity enum
fn score_to_severity(score: u8) -> ErrorSeverity {
    match score {
        1 => ErrorSeverity::Critical,
        2 => ErrorSeverity::Dependency,
        _ => ErrorSeverity::Client,
    }
}

/// Get icon for error severity
fn get_error_icon(severity: &ErrorSeverity) -> &'static str {
    match severity {
        ErrorSeverity::Critical => "🔥",
        ErrorSeverity::Dependency => "🔗",
        ErrorSeverity::Client => "⚠️",
    }
}

/// Format error response body for better readability, with status code fallback
fn format_error_body_with_status(body: &str, status_code: Option<u16>) -> String {
    // Try to parse as JSON for better formatting
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(obj) = json_value.as_object() {
            let mut parts = Vec::new();
            
            if let Some(error) = obj.get("error").and_then(|v| v.as_str()) {
                parts.push(format!("Type: {}", error));
            }
            
            if let Some(message) = obj.get("message").and_then(|v| v.as_str()) {
                // Truncate long messages
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
    let formatted_body = truncate_response_body(body, 150);
    
    // If the body is empty or whitespace-only, use status code description
    if formatted_body.trim().is_empty() {
        if let Some(status) = status_code {
            get_friendly_error_message(status)
        } else {
            "No error details provided in response body".to_string()
        }
    } else {
        formatted_body
    }
}

/// Extract error type from response body
fn extract_error_type(body: &str) -> String {
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

/// Get debugging suggestion based on error type and status
fn get_debugging_suggestion(error_type: &str, status: &u16) -> Option<String> {
    match (error_type, status) {
        ("DependencyError", 424) => Some("Check dependent service health and connectivity".to_string()),
        ("UnhandledError", 500) => Some("Review request payload structure and required fields".to_string()),
        ("ValidationError", 400) => Some("Verify request parameters and data format".to_string()),
        (_, 500) => Some("Check application logs for internal errors".to_string()),
        (_, 424) => Some("Verify dependent services are operational".to_string()),
        _ => None,
    }
}

/// Get friendly error message based on status code when response body is empty
fn get_friendly_error_message(status: u16) -> String {
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

/// Truncate response body for display
fn truncate_response_body(body: &str, max_length: usize) -> String {
    if body.len() <= max_length {
        body.to_string()
    } else {
        format!("{}... (truncated)", &body[..max_length])
    }
}