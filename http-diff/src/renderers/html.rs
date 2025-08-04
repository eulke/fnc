//! HTML renderer for web reports

use crate::types::ComparisonResult;
use super::OutputRenderer;

/// HTML renderer that produces a styled HTML report
pub struct HtmlRenderer {
    /// Whether to include CSS styling
    pub include_styles: bool,
}

impl HtmlRenderer {
    /// Create a new HTML renderer with CSS styles
    pub fn new() -> Self {
        Self {
            include_styles: true,
        }
    }

    /// Create an HTML renderer without CSS styles
    pub fn without_styles() -> Self {
        Self {
            include_styles: false,
        }
    }
}

impl Default for HtmlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for HtmlRenderer {
    fn render(&self, results: &[ComparisonResult]) -> String {
        let mut html = String::new();
        
        if self.include_styles {
            html.push_str(include_str!("../../templates/html_styles.css"));
            html.push_str("\n");
        }
        
        html.push_str("<div class=\"http-diff-report\">\n");
        html.push_str(&format_html_summary(results));
        html.push_str(&format_html_details(results));
        html.push_str("</div>\n");
        
        html
    }
}

/// Generate HTML summary section
fn format_html_summary(results: &[ComparisonResult]) -> String {
    let total_tests = results.len();
    let failed_count = results.iter().filter(|r| r.has_errors).count();
    let identical_count = results.iter().filter(|r| r.is_identical && !r.has_errors).count();
    let different_count = results.iter().filter(|r| !r.is_identical && !r.has_errors).count();

    format!(
        r#"<div class="summary">
    <h2>Test Summary</h2>
    <div class="stats">
        <div class="stat identical">
            <span class="count">{}</span>
            <span class="label">Identical</span>
        </div>
        <div class="stat different">
            <span class="count">{}</span>
            <span class="label">Different</span>
        </div>
        <div class="stat failed">
            <span class="count">{}</span>
            <span class="label">Failed</span>
        </div>
        <div class="stat total">
            <span class="count">{}</span>
            <span class="label">Total</span>
        </div>
    </div>
</div>
"#,
        identical_count, different_count, failed_count, total_tests
    )
}

/// Generate HTML details section
fn format_html_details(results: &[ComparisonResult]) -> String {
    let mut html = String::new();
    
    html.push_str("<div class=\"details\">\n");
    html.push_str("<h2>Test Results</h2>\n");
    
    for result in results {
        let status_class = if result.has_errors {
            "failed"
        } else if result.is_identical {
            "identical"
        } else {
            "different"
        };
        
        html.push_str(&format!(
            r#"<div class="test-result {}">
    <h3>{}</h3>
    <div class="environments">"#,
            status_class, result.route_name
        ));
        
        for (env, response) in &result.responses {
            html.push_str(&format!(
                r#"<div class="environment">
        <strong>{}:</strong> {} ({} bytes)
    </div>"#,
                env, response.status, response.body.len()
            ));
        }
        
        html.push_str("</div>\n");
        
        if !result.differences.is_empty() {
            html.push_str("<div class=\"differences\">\n");
            for diff in &result.differences {
                html.push_str(&format!(
                    "<div class=\"difference {}\">{}</div>\n",
                    format!("{:?}", diff.category).to_lowercase(),
                    html_escape(&diff.description)
                ));
            }
            html.push_str("</div>\n");
        }
        
        html.push_str("</div>\n");
    }
    
    html.push_str("</div>\n");
    html
}

/// Simple HTML escaping
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
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
    fn test_html_renderer() {
        let renderer = HtmlRenderer::without_styles();
        let results = vec![create_test_result()];
        let output = renderer.render(&results);
        
        assert!(output.contains("<div class=\"http-diff-report\">"));
        assert!(output.contains("test-route"));
        assert!(output.contains("Test Summary"));
    }
}