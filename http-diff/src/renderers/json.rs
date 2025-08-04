//! JSON renderer for structured output

use crate::types::ComparisonResult;
use super::OutputRenderer;

/// JSON renderer that produces structured JSON output
pub struct JsonRenderer {
    /// Whether to pretty-print the JSON output
    pub pretty: bool,
}

impl JsonRenderer {
    /// Create a new JSON renderer with pretty printing
    pub fn new() -> Self {
        Self { pretty: true }
    }

    /// Create a JSON renderer with compact output
    pub fn compact() -> Self {
        Self { pretty: false }
    }
}

impl Default for JsonRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for JsonRenderer {
    fn render(&self, results: &[ComparisonResult]) -> String {
        if self.pretty {
            serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string())
        } else {
            serde_json::to_string(results).unwrap_or_else(|_| "[]".to_string())
        }
    }
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
    fn test_json_renderer() {
        let renderer = JsonRenderer::new();
        let results = vec![create_test_result()];
        let output = renderer.render(&results);
        
        assert!(output.contains("test-route"));
        assert!(output.starts_with('['));
        assert!(output.ends_with(']'));
    }
}