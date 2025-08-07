//! HTML diff renderer for converting diff data into styled HTML

use crate::renderers::diff_data::{BodyDiffData, DiffOperation, DiffRow};

/// HTML renderer for body diffs
pub struct HtmlDiffRenderer;

impl HtmlDiffRenderer {
    /// Create a new HTML diff renderer
    pub fn new() -> Self {
        Self
    }

    /// Render body diff data as HTML
    pub fn render_body_diff(&self, body_diff: &BodyDiffData, show_unchanged: bool) -> String {
        if body_diff.is_empty() {
            return String::new();
        }

        if body_diff.is_large_response {
            return self.render_large_response_summary(body_diff);
        }

        let diff_rows = self.render_diff_rows(&body_diff.rows, show_unchanged);
        
        format!(
            r#"
        <div class="diff-container">
            <div class="diff-header">
                <div class="diff-environments">
                    <div class="diff-env diff-env-left">
                        <span class="diff-env-label">{}</span>
                    </div>
                    <div class="diff-env diff-env-right">
                        <span class="diff-env-label">{}</span>
                    </div>
                </div>
                <div class="diff-stats">
                    <span class="diff-stat">Total Size: {} bytes</span>
                    <span class="diff-stat">{} lines</span>
                </div>
            </div>
            <div class="diff-content">
                <div class="diff-table-container">
                    <table class="diff-table">
                        {}
                    </table>
                </div>
            </div>
        </div>
        "#,
            Self::escape_html(&body_diff.env1),
            Self::escape_html(&body_diff.env2),
            body_diff.total_size,
            body_diff.rows.len(),
            diff_rows
        )
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
                .map(|diff| format!("<li>{}</li>", Self::escape_html(diff)))
                .collect::<Vec<_>>()
                .join("")
        };

        format!(
            r#"
        <div class="diff-container large-response">
            <div class="diff-header">
                <div class="diff-environments">
                    <div class="diff-env diff-env-left">
                        <span class="diff-env-label">{}</span>
                    </div>
                    <div class="diff-env diff-env-right">
                        <span class="diff-env-label">{}</span>
                    </div>
                </div>
            </div>
            <div class="diff-summary">
                <div class="summary-alert">
                    <div class="summary-icon"></div>
                    <div class="summary-content">
                        <h4>Large Response Summary</h4>
                        <p>Response too large for detailed diff. Showing summary instead.</p>
                    </div>
                </div>
                <div class="summary-stats-grid">
                    <div class="summary-stat">
                        <span class="stat-label">{} Size:</span>
                        <span class="stat-value">{} bytes ({} lines)</span>
                    </div>
                    <div class="summary-stat">
                        <span class="stat-label">{} Size:</span>
                        <span class="stat-value">{} bytes ({} lines)</span>
                    </div>
                    <div class="summary-stat full-width">
                        <span class="stat-label">Total Size:</span>
                        <span class="stat-value">{} bytes</span>
                    </div>
                </div>
                <div class="sample-differences">
                    <h5>Sample Differences:</h5>
                    <ul>
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

    /// Render individual diff rows as table rows
    fn render_diff_rows(&self, rows: &[DiffRow], show_unchanged: bool) -> String {
        let mut html = String::new();
        let mut line_number = 1;

        for row in rows {
            // Skip unchanged lines if not showing them
            if !show_unchanged && row.operation == DiffOperation::Unchanged {
                line_number += 1;
                continue;
            }

            let empty_string = String::new();
            let (row_class, left_content, right_content) = match &row.operation {
                DiffOperation::Unchanged => (
                    "diff-row-unchanged",
                    row.left_content.as_ref().unwrap_or(&empty_string),
                    row.right_content.as_ref().unwrap_or(&empty_string),
                ),
                DiffOperation::Added => (
                    "diff-row-added",
                    &empty_string,
                    row.right_content.as_ref().unwrap_or(&empty_string),
                ),
                DiffOperation::Removed => (
                    "diff-row-removed",
                    row.left_content.as_ref().unwrap_or(&empty_string),
                    &empty_string,
                ),
                DiffOperation::Changed => (
                    "diff-row-changed",
                    row.left_content.as_ref().unwrap_or(&empty_string),
                    row.right_content.as_ref().unwrap_or(&empty_string),
                ),
            };

            html.push_str(&format!(
                r#"
            <tr class="diff-row {}">
                <td class="diff-line-number">{}</td>
                <td class="diff-content diff-left">{}</td>
                <td class="diff-content diff-right">{}</td>
            </tr>
            "#,
                row_class,
                line_number,
                Self::escape_html(left_content),
                Self::escape_html(right_content)
            ));

            line_number += 1;
        }

        html
    }

    /// Generate a diff summary for the header
    pub fn render_diff_summary(&self, body_diff: &BodyDiffData) -> String {
        if body_diff.is_empty() || !body_diff.has_differences {
            return String::new();
        }

        let stats = self.calculate_diff_stats(&body_diff.rows);
        
        format!(
            r#"
        <div class="diff-summary-badge">
            <div class="summary-item">
                <span class="summary-count added">{}</span>
                <span class="summary-label">added</span>
            </div>
            <div class="summary-item">
                <span class="summary-count removed">{}</span>
                <span class="summary-label">removed</span>
            </div>
            <div class="summary-item">
                <span class="summary-count changed">{}</span>
                <span class="summary-label">changed</span>
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

impl Default for HtmlDiffRenderer {
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
        let renderer = HtmlDiffRenderer::new();
        let diff = BodyDiffData::new("env1".to_string(), "env2".to_string());
        
        let html = renderer.render_body_diff(&diff, true);
        assert!(html.is_empty());
    }

    #[test]
    fn test_render_basic_diff() {
        let renderer = HtmlDiffRenderer::new();
        let mut diff = BodyDiffData::new("test".to_string(), "prod".to_string());
        diff.add_row(DiffRow::unchanged("same line".to_string()));
        diff.add_row(DiffRow::added("added line".to_string()));
        diff.add_row(DiffRow::removed("removed line".to_string()));
        diff.set_total_size(100);

        let html = renderer.render_body_diff(&diff, true);
        
        assert!(html.contains("diff-container"));
        assert!(html.contains("test"));
        assert!(html.contains("prod"));
        assert!(html.contains("100 bytes"));
        assert!(html.contains("same line"));
        assert!(html.contains("added line"));
        assert!(html.contains("removed line"));
    }

    #[test]
    fn test_render_large_response_summary() {
        let renderer = HtmlDiffRenderer::new();
        let summary = BodyDiffSummary {
            size1: 1000,
            size2: 1200,
            lines1: 50,
            lines2: 60,
            sample_differences: vec!["Line 1: content differs".to_string()],
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
        assert!(html.contains("Line 1: content differs"));
    }

    #[test]
    fn test_html_escaping() {
        let text = "<script>alert('xss')</script>";
        let escaped = HtmlDiffRenderer::escape_html(text);
        
        assert_eq!(escaped, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert!(!escaped.contains("<script>"));
    }
}