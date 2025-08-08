//! JSON diff renderer for the HTML report
//!
//! Produces a side-by-side, GitHub-like diff using the `.json-diff-*` CSS classes
//! referenced by the embedded stylesheet and JavaScript in `templates.rs`.

use crate::renderers::diff_data::{BodyDiffData, BodyDiffSummary, DiffOperation, DiffRow};

/// Renderer that converts `BodyDiffData` into HTML blocks styled specifically
/// for JSON diffs in the report.
pub struct JsonDiffRenderer;

impl JsonDiffRenderer {
    /// Create a new renderer
    pub fn new() -> Self {
        Self
    }

    /// Render a compact summary badge for the diff
    pub fn render_diff_summary(&self, body_diff: &BodyDiffData) -> String {
        if body_diff.is_empty() || !body_diff.has_differences {
            return String::new();
        }

        let stats = self.calculate_stats(&body_diff.rows);
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
            stats.added, stats.removed, stats.changed
        )
    }

    /// Render the full JSON body diff as side-by-side columns
    pub fn render_body_diff(&self, body_diff: &BodyDiffData, show_unchanged: bool) -> String {
        if body_diff.is_empty() {
            return String::new();
        }

        if body_diff.is_large_response {
            return self.render_large_response_summary(&body_diff.env1, &body_diff.env2, &body_diff.summary);
        }

        let rows_html = self.render_rows(&body_diff.rows, show_unchanged);

        format!(
            r#"
        <div class="json-diff-container">
            <div class="json-diff-header">
                <div class="json-env json-env-left">{}</div>
                <div class="json-env json-env-right">{}</div>
            </div>
            <div class="json-diff-content">
                <table class="json-diff-table">
                    {}
                </table>
            </div>
        </div>
        "#,
            Self::escape_html(&body_diff.env1),
            Self::escape_html(&body_diff.env2),
            rows_html
        )
    }

    fn render_large_response_summary(
        &self,
        env1: &str,
        env2: &str,
        summary: &Option<BodyDiffSummary>,
    ) -> String {
        let Some(summary) = summary else { return String::new(); };
        format!(
            r#"
        <div class="json-diff-container large-response">
            <div class="diff-summary">
                <div class="summary-alert">
                    <div class="summary-icon"></div>
                    <div class="summary-content">
                        <h4>Large JSON Body Summary</h4>
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
                </div>
            </div>
        </div>
        "#,
            Self::escape_html(env1),
            summary.size1,
            summary.lines1,
            Self::escape_html(env2),
            summary.size2,
            summary.lines2
        )
    }

    fn render_rows(&self, rows: &[DiffRow], show_unchanged: bool) -> String {
        let mut html = String::new();
        let mut line_number: usize = 1;

        for row in rows {
            if !show_unchanged && row.operation == DiffOperation::Unchanged {
                line_number += 1;
                continue;
            }

            let empty = String::new();
            let (class, left, right) = match &row.operation {
                DiffOperation::Unchanged => (
                    "json-row-unchanged",
                    row.left_content.as_ref().unwrap_or(&empty),
                    row.right_content.as_ref().unwrap_or(&empty),
                ),
                DiffOperation::Added => (
                    "json-row-added",
                    &empty,
                    row.right_content.as_ref().unwrap_or(&empty),
                ),
                DiffOperation::Removed => (
                    "json-row-removed",
                    row.left_content.as_ref().unwrap_or(&empty),
                    &empty,
                ),
                DiffOperation::Changed => (
                    "json-row-changed",
                    row.left_content.as_ref().unwrap_or(&empty),
                    row.right_content.as_ref().unwrap_or(&empty),
                ),
            };

            html.push_str(&format!(
                r#"
            <tr class="json-diff-row {}">
                <td class="json-line-number">{}</td>
                <td class="json-code-block json-left"><pre><code>{}</code></pre></td>
                <td class="json-code-block json-right"><pre><code>{}</code></pre></td>
            </tr>
            "#,
                class,
                line_number,
                Self::escape_html(left),
                Self::escape_html(right)
            ));

            line_number += 1;
        }

        html
    }

    fn calculate_stats(&self, rows: &[DiffRow]) -> DiffStats {
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

    #[test]
    fn renders_empty_when_no_rows() {
        let r = JsonDiffRenderer::new();
        let diff = BodyDiffData::new("a".into(), "b".into());
        assert!(r.render_body_diff(&diff, true).is_empty());
        assert!(r.render_diff_summary(&diff).is_empty());
    }

    #[test]
    fn renders_side_by_side_rows() {
        let r = JsonDiffRenderer::new();
        let mut diff = BodyDiffData::new("env1".into(), "env2".into());
        diff.add_row(DiffRow::unchanged("same".into()));
        diff.add_row(DiffRow::added("added".into()));
        diff.add_row(DiffRow::removed("removed".into()));
        diff.add_row(DiffRow::changed("l".into(), "r".into()));

        let html = r.render_body_diff(&diff, true);
        assert!(html.contains("json-diff-container"));
        assert!(html.contains("json-diff-table"));
        assert!(html.contains("json-row-added"));
        assert!(html.contains("json-row-removed"));
        assert!(html.contains("json-row-changed"));
    }
}


