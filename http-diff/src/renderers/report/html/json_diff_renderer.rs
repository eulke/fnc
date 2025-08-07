//! JSON-formatted diff renderer for HTML reports

use crate::renderers::diff_data::{BodyDiffData, DiffOperation, DiffRow};
use serde_json::Value;

/// JSON-specific diff renderer for HTML code blocks
pub struct JsonDiffRenderer;

impl JsonDiffRenderer {
    /// Create a new JSON diff renderer
    pub fn new() -> Self {
        Self
    }

    /// Render body diff data as formatted JSON code blocks
    pub fn render_body_diff(&self, body_diff: &BodyDiffData, show_unchanged: bool) -> String {
        if body_diff.is_empty() {
            return String::new();
        }

        if body_diff.is_large_response {
            return self.render_large_response_summary(body_diff);
        }

        // Try to parse as JSON first, fallback to plain text
        let (left_content, right_content) = self.prepare_content_for_diff(body_diff);
        let diff_blocks = self.render_side_by_side_diff(&left_content, &right_content, &body_diff.rows, show_unchanged);
        
        format!(
            r#"
        <div class="json-diff-container" role="region" aria-labelledby="diff-header-{}" tabindex="0">
            <div class="json-diff-header" id="diff-header-{}">
                <div class="json-diff-environments">
                    <div class="json-env json-env-left">
                        <span class="json-env-label" aria-label="First environment">{}</span>
                    </div>
                    <div class="json-env-vs" aria-hidden="true">vs</div>
                    <div class="json-env json-env-right">
                        <span class="json-env-label" aria-label="Second environment">{}</span>
                    </div>
                </div>
                <div class="json-diff-stats">
                    <span class="json-stat" aria-label="Total response size">Total Size: {} bytes</span>
                    <span class="json-stat" aria-label="Number of differences">{} differences</span>
                </div>
            </div>
            <div class="json-diff-content" role="main" aria-label="JSON diff comparison">
                {}
            </div>
        </div>
        "#,
            Self::escape_html(&body_diff.env1).replace(" ", "-"),
            Self::escape_html(&body_diff.env1).replace(" ", "-"),
            Self::escape_html(&body_diff.env1),
            Self::escape_html(&body_diff.env2),
            body_diff.total_size,
            body_diff.rows.iter().filter(|r| r.operation != DiffOperation::Unchanged).count(),
            diff_blocks
        )
    }

    /// Prepare content for diff rendering (JSON formatting or plain text)
    fn prepare_content_for_diff(&self, body_diff: &BodyDiffData) -> (String, String) {
        // Reconstruct the full content from diff rows
        let mut left_content = String::new();
        let mut right_content = String::new();

        for row in &body_diff.rows {
            match &row.operation {
                DiffOperation::Unchanged => {
                    if let Some(content) = &row.left_content {
                        left_content.push_str(content);
                        left_content.push('\n');
                    }
                    if let Some(content) = &row.right_content {
                        right_content.push_str(content);
                        right_content.push('\n');
                    }
                }
                DiffOperation::Added => {
                    if let Some(content) = &row.right_content {
                        right_content.push_str(content);
                        right_content.push('\n');
                    }
                }
                DiffOperation::Removed => {
                    if let Some(content) = &row.left_content {
                        left_content.push_str(content);
                        left_content.push('\n');
                    }
                }
                DiffOperation::Changed => {
                    if let Some(content) = &row.left_content {
                        left_content.push_str(content);
                        left_content.push('\n');
                    }
                    if let Some(content) = &row.right_content {
                        right_content.push_str(content);
                        right_content.push('\n');
                    }
                }
            }
        }

        // Try to format as JSON, fallback to original if parsing fails
        let formatted_left = Self::try_format_json(&left_content).unwrap_or(left_content);
        let formatted_right = Self::try_format_json(&right_content).unwrap_or(right_content);

        (formatted_left, formatted_right)
    }

    /// Try to parse and format JSON with proper indentation
    fn try_format_json(content: &str) -> Option<String> {
        if let Ok(parsed) = serde_json::from_str::<Value>(content.trim()) {
            if let Ok(formatted) = serde_json::to_string_pretty(&parsed) {
                return Some(formatted);
            }
        }
        None
    }

    /// Render side-by-side diff with JSON code blocks
    fn render_side_by_side_diff(&self, left_content: &str, right_content: &str, rows: &[DiffRow], show_unchanged: bool) -> String {
        let _left_lines: Vec<&str> = left_content.lines().collect();
        let _right_lines: Vec<&str> = right_content.lines().collect();
        
        let mut html = String::new();
        html.push_str(r#"<div class="json-diff-viewer">"#);
        
        // Left side (first environment)
        html.push_str(r#"<div class="json-diff-side json-diff-left">"#);
        html.push_str(r#"<pre class="json-code-block"><code class="language-json">"#);
        
        let mut left_line_index = 0;
        for row in rows {
            if !show_unchanged && row.operation == DiffOperation::Unchanged {
                if row.left_content.is_some() {
                    left_line_index += 1;
                }
                continue;
            }

            let line_class = match row.operation {
                DiffOperation::Unchanged => "json-line-unchanged",
                DiffOperation::Added => "json-line-empty",
                DiffOperation::Removed => "json-line-removed",
                DiffOperation::Changed => "json-line-changed",
            };

            if let Some(content) = &row.left_content {
                html.push_str(&format!(
                    r#"<span class="json-line {} {}" data-line="{}">{}</span>
"#,
                    line_class,
                    if row.operation != DiffOperation::Unchanged { "json-highlight" } else { "" },
                    left_line_index + 1,
                    Self::escape_html(content)
                ));
                left_line_index += 1;
            } else if row.operation == DiffOperation::Added {
                // Empty line to maintain alignment
                html.push_str(&format!(
                    r#"<span class="json-line json-line-empty" data-line="{}">&nbsp;</span>
"#,
                    left_line_index + 1
                ));
            }
        }
        
        html.push_str(r#"</code></pre></div>"#);

        // Right side (second environment)
        html.push_str(r#"<div class="json-diff-side json-diff-right">"#);
        html.push_str(r#"<pre class="json-code-block"><code class="language-json">"#);
        
        let mut right_line_index = 0;
        for row in rows {
            if !show_unchanged && row.operation == DiffOperation::Unchanged {
                if row.right_content.is_some() {
                    right_line_index += 1;
                }
                continue;
            }

            let line_class = match row.operation {
                DiffOperation::Unchanged => "json-line-unchanged",
                DiffOperation::Added => "json-line-added",
                DiffOperation::Removed => "json-line-empty",
                DiffOperation::Changed => "json-line-changed",
            };

            if let Some(content) = &row.right_content {
                html.push_str(&format!(
                    r#"<span class="json-line {} {}" data-line="{}">{}</span>
"#,
                    line_class,
                    if row.operation != DiffOperation::Unchanged { "json-highlight" } else { "" },
                    right_line_index + 1,
                    Self::escape_html(content)
                ));
                right_line_index += 1;
            } else if row.operation == DiffOperation::Removed {
                // Empty line to maintain alignment
                html.push_str(&format!(
                    r#"<span class="json-line json-line-empty" data-line="{}">&nbsp;</span>
"#,
                    right_line_index + 1
                ));
            }
        }
        
        html.push_str(r#"</code></pre></div>"#);
        html.push_str(r#"</div>"#);

        html
    }

    /// Render summary for large response bodies
    fn render_large_response_summary(&self, body_diff: &BodyDiffData) -> String {
        let summary = match &body_diff.summary {
            Some(summary) => summary,
            None => return String::new(),
        };

        let sample_diffs = if summary.sample_differences.is_empty() {
            "<li>No specific differences detected</li>".to_string()
        } else {
            summary
                .sample_differences
                .iter()
                .map(|diff| format!("<li><code>{}</code></li>", Self::escape_html(diff)))
                .collect::<Vec<_>>()
                .join("")
        };

        format!(
            r#"
        <div class="json-diff-container large-response">
            <div class="json-diff-header">
                <div class="json-diff-environments">
                    <div class="json-env json-env-left">
                        <span class="json-env-label">{}</span>
                    </div>
                    <div class="json-env-vs">vs</div>
                    <div class="json-env json-env-right">
                        <span class="json-env-label">{}</span>
                    </div>
                </div>
            </div>
            <div class="json-large-summary">
                <div class="large-response-alert">
                    <div class="large-response-icon"></div>
                    <div class="large-response-content">
                        <h4>Large Response Summary</h4>
                        <p>Response too large for detailed JSON diff. Showing summary instead.</p>
                    </div>
                </div>
                <div class="large-response-stats">
                    <div class="response-stat">
                        <span class="stat-label">{} Size:</span>
                        <span class="stat-value">{} bytes ({} lines)</span>
                    </div>
                    <div class="response-stat">
                        <span class="stat-label">{} Size:</span>
                        <span class="stat-value">{} bytes ({} lines)</span>
                    </div>
                    <div class="response-stat full-width">
                        <span class="stat-label">Total Combined:</span>
                        <span class="stat-value">{} bytes</span>
                    </div>
                </div>
                <div class="large-response-differences">
                    <h5>Sample Differences:</h5>
                    <ul class="sample-diff-list">
                        {}
                    </ul>
                </div>
            </div>
        </div>
        "#,
            Self::escape_html(&body_diff.env1),
            Self::escape_html(&body_diff.env2),
            Self::escape_html(&body_diff.env1),
            summary.size1,
            summary.lines1,
            Self::escape_html(&body_diff.env2),
            summary.size2,
            summary.lines2,
            body_diff.total_size,
            sample_diffs
        )
    }

    /// Generate a diff summary for the route header
    pub fn render_diff_summary(&self, body_diff: &BodyDiffData) -> String {
        if body_diff.is_empty() || !body_diff.has_differences {
            return String::new();
        }

        let stats = self.calculate_diff_stats(&body_diff.rows);
        
        format!(
            r#"
        <div class="json-diff-summary-badge">
            <div class="json-summary-item">
                <span class="json-summary-count added">+{}</span>
                <span class="json-summary-label">added</span>
            </div>
            <div class="json-summary-item">
                <span class="json-summary-count removed">-{}</span>
                <span class="json-summary-label">removed</span>
            </div>
            <div class="json-summary-item">
                <span class="json-summary-count changed">~{}</span>
                <span class="json-summary-label">changed</span>
            </div>
            <div class="json-summary-actions">
                <button class="json-action-btn copy-diff" title="Copy diff to clipboard" aria-label="Copy diff to clipboard" onclick="copyDiff(this)">Copy</button>
                <button class="json-action-btn expand-collapse" title="Toggle expand/collapse" aria-label="Toggle diff section" onclick="toggleDiffSection(this)">🔍</button>
            </div>
        </div>
        "#,
            stats.added,
            stats.removed,
            stats.changed
        )
    }

    /// Calculate statistics for diff rows
    fn calculate_diff_stats(&self, rows: &[DiffRow]) -> DiffStats {
        let mut stats = DiffStats::default();

        for row in rows {
            match row.operation {
                DiffOperation::Added => stats.added += 1,
                DiffOperation::Removed => stats.removed += 1,
                DiffOperation::Changed => stats.changed += 1,
                DiffOperation::Unchanged => stats.unchanged += 1,
            }
        }

        stats
    }

    /// Escape HTML special characters
    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

impl Default for JsonDiffRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for diff operations
#[derive(Default)]
struct DiffStats {
    added: usize,
    removed: usize,
    changed: usize,
    unchanged: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::diff_data::{BodyDiffData, BodyDiffSummary, DiffRow};

    #[test]
    fn test_render_empty_diff() {
        let renderer = JsonDiffRenderer::new();
        let diff = BodyDiffData::new("env1".to_string(), "env2".to_string());
        
        let html = renderer.render_body_diff(&diff, true);
        assert!(html.is_empty());
    }

    #[test]
    fn test_json_formatting() {
        let json_str = r#"{"name":"test","value":123}"#;
        let formatted = JsonDiffRenderer::try_format_json(json_str);
        
        assert!(formatted.is_some());
        let formatted = formatted.unwrap();
        assert!(formatted.contains("\"name\": \"test\""));
        assert!(formatted.contains("\"value\": 123"));
    }

    #[test]
    fn test_render_basic_json_diff() {
        let renderer = JsonDiffRenderer::new();
        let mut diff = BodyDiffData::new("test".to_string(), "prod".to_string());
        diff.add_row(DiffRow::unchanged(r#"  "name": "test""#.to_string()));
        diff.add_row(DiffRow::added(r#"  "new_field": "value""#.to_string()));
        diff.add_row(DiffRow::removed(r#"  "old_field": "removed""#.to_string()));
        diff.set_total_size(100);

        let html = renderer.render_body_diff(&diff, true);
        
        assert!(html.contains("json-diff-container"));
        assert!(html.contains("test"));
        assert!(html.contains("prod"));
        assert!(html.contains("100 bytes"));
        assert!(html.contains("json-line-unchanged"));
        assert!(html.contains("json-line-added"));
        assert!(html.contains("json-line-removed"));
    }

    #[test]
    fn test_render_large_response_summary() {
        let renderer = JsonDiffRenderer::new();
        let summary = BodyDiffSummary {
            size1: 1000,
            size2: 1200,
            lines1: 50,
            lines2: 60,
            sample_differences: vec!["Line 1: JSON structure differs".to_string()],
        };
        
        let diff = BodyDiffData::new_large_response(
            "test".to_string(),
            "prod".to_string(),
            2200,
            summary,
        );

        let html = renderer.render_body_diff(&diff, true);
        
        assert!(html.contains("large-response"));
        assert!(html.contains("Large Response Summary"));
        assert!(html.contains("1000 bytes"));
        assert!(html.contains("1200 bytes"));
        assert!(html.contains("JSON structure differs"));
    }

    #[test]
    fn test_html_escaping() {
        let text = r#"<script>alert('xss')</script>"#;
        let escaped = JsonDiffRenderer::escape_html(text);
        
        assert_eq!(escaped, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert!(!escaped.contains("<script>"));
    }

    #[test]
    fn test_diff_stats_calculation() {
        let renderer = JsonDiffRenderer::new();
        let rows = vec![
            DiffRow::unchanged("line1".to_string()),
            DiffRow::added("line2".to_string()),
            DiffRow::removed("line3".to_string()),
            DiffRow::changed("old".to_string(), "new".to_string()),
        ];
        
        let stats = renderer.calculate_diff_stats(&rows);
        assert_eq!(stats.unchanged, 1);
        assert_eq!(stats.added, 1);
        assert_eq!(stats.removed, 1);
        assert_eq!(stats.changed, 1);
    }
}