//! TUI-specific diff rendering logic
//!
//! This module handles rendering of generic diff data structures into
//! TUI-compatible text format with proper styling and layout.

use super::theme::UiSymbols;
use crate::renderers::diff_data::{BodyDiffData, DiffData, DiffOperation, HeaderDiffData};
use crate::types::DiffViewStyle;
use std::fmt::Write;

/// TUI-specific diff renderer
pub struct TuiDiffRenderer {
    /// Maximum width for content before truncation
    max_width: usize,
}

impl TuiDiffRenderer {
    /// Create a new TUI diff renderer
    pub fn new() -> Self {
        Self {
            max_width: 120, // Reasonable default for terminal width
        }
    }

    /// Create a TUI diff renderer with custom max width
    pub fn with_max_width(max_width: usize) -> Self {
        Self { max_width }
    }

    /// Render complete diff data into TUI-compatible text
    pub fn render_diff_data(&self, diff_data: &DiffData, style: &DiffViewStyle) -> String {
        let mut output = String::new();

        // Add route information
        writeln!(
            output,
            "{} Route: {}",
            UiSymbols::ROUTE,
            diff_data.route_name
        )
        .unwrap();
        writeln!(
            output,
            "{} Diff Style: {}",
            UiSymbols::SETTINGS,
            match style {
                DiffViewStyle::Unified => "Unified",
                DiffViewStyle::SideBySide => "Side-by-Side",
            }
        )
        .unwrap();
        writeln!(output).unwrap();

        if !diff_data.has_differences {
            writeln!(
                output,
                "{} No differences found between environments",
                UiSymbols::SUCCESS
            )
            .unwrap();
            return output;
        }

        // Render header differences if present
        if let Some(ref headers) = diff_data.headers {
            if !headers.is_empty() {
                writeln!(output, "{} Header Differences:", UiSymbols::HEADERS).unwrap();
                output.push_str(&self.render_header_diff(headers, style));
                writeln!(output).unwrap();
            }
        }

        // Render body differences if present
        if let Some(ref body) = diff_data.body {
            if !body.is_empty() {
                writeln!(output, "{} Body Differences:", UiSymbols::BODY).unwrap();
                output.push_str(&self.render_body_diff(body, style));
            }
        }

        output
    }

    /// Render header differences
    pub fn render_header_diff(&self, headers: &HeaderDiffData, style: &DiffViewStyle) -> String {
        match style {
            DiffViewStyle::Unified => self.render_headers_unified(headers),
            DiffViewStyle::SideBySide => self.render_headers_side_by_side(headers),
        }
    }

    /// Render body differences
    pub fn render_body_diff(&self, body: &BodyDiffData, style: &DiffViewStyle) -> String {
        if body.is_large_response {
            self.render_large_response_summary(body)
        } else {
            match style {
                DiffViewStyle::Unified => self.render_body_unified(body),
                DiffViewStyle::SideBySide => self.render_body_side_by_side(body),
            }
        }
    }

    /// Render headers in unified diff format
    fn render_headers_unified(&self, headers: &HeaderDiffData) -> String {
        let mut output = String::new();

        // Simple table format for unified view
        writeln!(output, "┌─────────────────────┬─────────────────────┬─────────────────────────────────────────┐").unwrap();
        writeln!(output, "│ Header              │ Environment         │ Value                                   │").unwrap();
        writeln!(output, "├─────────────────────┼─────────────────────┼─────────────────────────────────────────┤").unwrap();

        for row in &headers.rows {
            let header_name = row.context.as_deref().unwrap_or("Unknown");
            let truncated_header = self.truncate_text(header_name, 19);

            match row.operation {
                DiffOperation::Removed => {
                    if let Some(ref content) = row.left_content {
                        let env_label = format!("- {}", headers.env1.to_uppercase());
                        let truncated_env = self.truncate_text(&env_label, 19);
                        let truncated_value = self.truncate_text(content, 39);
                        writeln!(
                            output,
                            "│ {} │ {} │ {} │",
                            truncated_header, truncated_env, truncated_value
                        )
                        .unwrap();
                    }
                }
                DiffOperation::Added => {
                    if let Some(ref content) = row.right_content {
                        let env_label = format!("+ {}", headers.env2.to_uppercase());
                        let truncated_env = self.truncate_text(&env_label, 19);
                        let truncated_value = self.truncate_text(content, 39);
                        writeln!(
                            output,
                            "│ {} │ {} │ {} │",
                            truncated_header, truncated_env, truncated_value
                        )
                        .unwrap();
                    }
                }
                DiffOperation::Changed => {
                    // Show both values for changed headers
                    if let Some(ref content1) = row.left_content {
                        let env_label = format!("- {}", headers.env1.to_uppercase());
                        let truncated_env = self.truncate_text(&env_label, 19);
                        let truncated_value = self.truncate_text(content1, 39);
                        writeln!(
                            output,
                            "│ {} │ {} │ {} │",
                            truncated_header, truncated_env, truncated_value
                        )
                        .unwrap();
                    }
                    if let Some(ref content2) = row.right_content {
                        let env_label = format!("+ {}", headers.env2.to_uppercase());
                        let truncated_env = self.truncate_text(&env_label, 19);
                        let truncated_value = self.truncate_text(content2, 39);
                        writeln!(
                            output,
                            "│ {} │ {} │ {} │",
                            truncated_header, truncated_env, truncated_value
                        )
                        .unwrap();
                    }
                }
                DiffOperation::Unchanged => {
                    // This shouldn't appear in diff data, but handle gracefully
                    if let Some(ref content) = row.left_content {
                        let env_label = format!("  {}", headers.env1.to_uppercase());
                        let truncated_env = self.truncate_text(&env_label, 19);
                        let truncated_value = self.truncate_text(content, 39);
                        writeln!(
                            output,
                            "│ {} │ {} │ {} │",
                            truncated_header, truncated_env, truncated_value
                        )
                        .unwrap();
                    }
                }
            }
        }

        writeln!(output, "└─────────────────────┴─────────────────────┴─────────────────────────────────────────┘").unwrap();
        output
    }

    /// Render headers in side-by-side format
    fn render_headers_side_by_side(&self, headers: &HeaderDiffData) -> String {
        let mut output = String::new();

        writeln!(output, "┌─────────────────────┬─────────────────────────────────┬─────────────────────────────────┐").unwrap();
        writeln!(
            output,
            "│ Header              │ {} │ {} │",
            self.truncate_text(&headers.env1.to_uppercase(), 31),
            self.truncate_text(&headers.env2.to_uppercase(), 31)
        )
        .unwrap();
        writeln!(output, "├─────────────────────┼─────────────────────────────────┼─────────────────────────────────┤").unwrap();

        for row in &headers.rows {
            let header_name = row.context.as_deref().unwrap_or("Unknown");
            let truncated_header = self.truncate_text(header_name, 19);

            let left_value = match &row.left_content {
                Some(content) => self.truncate_text(content, 31),
                None => self.truncate_text("(missing)", 31),
            };

            let right_value = match &row.right_content {
                Some(content) => self.truncate_text(content, 31),
                None => self.truncate_text("(missing)", 31),
            };

            writeln!(
                output,
                "│ {} │ {} │ {} │",
                truncated_header, left_value, right_value
            )
            .unwrap();
        }

        writeln!(output, "└─────────────────────┴─────────────────────────────────┴─────────────────────────────────┘").unwrap();
        output
    }

    /// Render body in unified diff format
    fn render_body_unified(&self, body: &BodyDiffData) -> String {
        let mut output = String::new();

        for row in &body.rows {
            let line = match row.operation {
                DiffOperation::Unchanged => {
                    if let Some(ref content) = row.left_content {
                        format!("  {}", self.truncate_text(content, self.max_width - 3))
                    } else {
                        continue;
                    }
                }
                DiffOperation::Removed => {
                    if let Some(ref content) = row.left_content {
                        format!("- {}", self.truncate_text(content, self.max_width - 3))
                    } else {
                        continue;
                    }
                }
                DiffOperation::Added => {
                    if let Some(ref content) = row.right_content {
                        format!("+ {}", self.truncate_text(content, self.max_width - 3))
                    } else {
                        continue;
                    }
                }
                DiffOperation::Changed => {
                    // For changed lines, show both with - and +
                    let mut lines = String::new();
                    if let Some(ref content1) = row.left_content {
                        writeln!(
                            lines,
                            "- {}",
                            self.truncate_text(content1, self.max_width - 3)
                        )
                        .unwrap();
                    }
                    if let Some(ref content2) = row.right_content {
                        writeln!(
                            lines,
                            "+ {}",
                            self.truncate_text(content2, self.max_width - 3)
                        )
                        .unwrap();
                    }
                    lines.trim_end().to_string()
                }
            };

            writeln!(output, "{}", line).unwrap();
        }

        output
    }

    /// Render body in side-by-side format
    fn render_body_side_by_side(&self, body: &BodyDiffData) -> String {
        let mut output = String::new();
        let col_width = (self.max_width - 7) / 2; // Account for borders and separators

        // Header
        writeln!(
            output,
            "┌{}┬{}┐",
            "─".repeat(col_width + 2),
            "─".repeat(col_width + 2)
        )
        .unwrap();
        writeln!(
            output,
            "│ {} │ {} │",
            self.pad_text(&body.env1.to_uppercase(), col_width),
            self.pad_text(&body.env2.to_uppercase(), col_width)
        )
        .unwrap();
        writeln!(
            output,
            "├{}┼{}┤",
            "─".repeat(col_width + 2),
            "─".repeat(col_width + 2)
        )
        .unwrap();

        for row in &body.rows {
            let left_content = match row.operation {
                DiffOperation::Unchanged | DiffOperation::Removed | DiffOperation::Changed => row
                    .left_content
                    .as_ref()
                    .map(|c| self.truncate_text(c, col_width))
                    .unwrap_or_else(|| " ".repeat(col_width)),
                DiffOperation::Added => " ".repeat(col_width),
            };

            let right_content = match row.operation {
                DiffOperation::Unchanged | DiffOperation::Added | DiffOperation::Changed => row
                    .right_content
                    .as_ref()
                    .map(|c| self.truncate_text(c, col_width))
                    .unwrap_or_else(|| " ".repeat(col_width)),
                DiffOperation::Removed => " ".repeat(col_width),
            };

            writeln!(
                output,
                "│ {} │ {} │",
                self.pad_text(&left_content, col_width),
                self.pad_text(&right_content, col_width)
            )
            .unwrap();
        }

        writeln!(
            output,
            "└{}┴{}┘",
            "─".repeat(col_width + 2),
            "─".repeat(col_width + 2)
        )
        .unwrap();

        output
    }

    /// Render large response summary
    fn render_large_response_summary(&self, body: &BodyDiffData) -> String {
        let mut output = String::new();

        if let Some(ref summary) = body.summary {
            writeln!(
                output,
                "{} Large Response Comparison Summary",
                UiSymbols::INFO
            )
            .unwrap();
            writeln!(
                output,
                "{} Responses are too large for detailed diff - showing summary only",
                UiSymbols::WARNING
            )
            .unwrap();
            writeln!(output).unwrap();

            // Size comparison table
            writeln!(
                output,
                "┌─────────────────────┬─────────────────────┬─────────────────────┐"
            )
            .unwrap();
            writeln!(
                output,
                "│ Environment         │ Size (bytes)        │ Lines               │"
            )
            .unwrap();
            writeln!(
                output,
                "├─────────────────────┼─────────────────────┼─────────────────────┤"
            )
            .unwrap();
            writeln!(
                output,
                "│ {} │ {} │ {} │",
                self.pad_text(&body.env1.to_uppercase(), 19),
                self.pad_text(&summary.size1.to_string(), 19),
                self.pad_text(&summary.lines1.to_string(), 19)
            )
            .unwrap();
            writeln!(
                output,
                "│ {} │ {} │ {} │",
                self.pad_text(&body.env2.to_uppercase(), 19),
                self.pad_text(&summary.size2.to_string(), 19),
                self.pad_text(&summary.lines2.to_string(), 19)
            )
            .unwrap();
            writeln!(
                output,
                "└─────────────────────┴─────────────────────┴─────────────────────┘"
            )
            .unwrap();
            writeln!(output).unwrap();

            // Differences summary
            writeln!(output, "{} Differences:", UiSymbols::COMPARE).unwrap();
            let size_diff = (summary.size1 as i64 - summary.size2 as i64).abs();
            writeln!(output, "   Size difference: {} bytes", size_diff).unwrap();

            if summary.lines1 != summary.lines2 {
                let line_diff = (summary.lines1 as i64 - summary.lines2 as i64).abs();
                writeln!(output, "   Line count difference: {} lines", line_diff).unwrap();
            }

            if !summary.sample_differences.is_empty() {
                writeln!(output, "\n{} Sample Differences:", UiSymbols::DIFF).unwrap();
                for diff in &summary.sample_differences {
                    writeln!(output, "   {}", diff).unwrap();
                }
            }

            writeln!(
                output,
                "\n{} Tip: Use curl commands or reduce response size for detailed comparison",
                UiSymbols::TIP
            )
            .unwrap();
        }

        output
    }

    /// Truncate text to specified width, adding ellipsis if needed
    fn truncate_text(&self, text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            text.to_string()
        } else if max_width >= 3 {
            format!("{}...", &text[..max_width - 3])
        } else {
            text.chars().take(max_width).collect()
        }
    }

    /// Pad text to specified width with spaces
    fn pad_text(&self, text: &str, width: usize) -> String {
        if text.len() >= width {
            self.truncate_text(text, width)
        } else {
            format!("{}{}", text, " ".repeat(width - text.len()))
        }
    }
}

impl Default for TuiDiffRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::diff_data::{DiffOperation, DiffRow};

    #[test]
    fn test_truncate_text() {
        let renderer = TuiDiffRenderer::new();

        assert_eq!(renderer.truncate_text("short", 10), "short");
        assert_eq!(
            renderer.truncate_text("this is a very long text", 10),
            "this is..."
        );
        assert_eq!(renderer.truncate_text("ab", 2), "ab");
        assert_eq!(renderer.truncate_text("abc", 2), "ab");
    }

    #[test]
    fn test_pad_text() {
        let renderer = TuiDiffRenderer::new();

        assert_eq!(renderer.pad_text("short", 10), "short     ");
        assert_eq!(renderer.pad_text("exact", 5), "exact");
        assert_eq!(renderer.pad_text("toolong", 5), "to...");
    }

    #[test]
    fn test_render_header_diff_unified() {
        let renderer = TuiDiffRenderer::new();

        let mut headers = HeaderDiffData::new("test".to_string(), "prod".to_string());
        headers.add_row(
            DiffRow::changed("1.0".to_string(), "2.0".to_string())
                .with_context("X-Version".to_string()),
        );

        let output = renderer.render_headers_unified(&headers);

        assert!(output.contains("X-Version"));
        assert!(output.contains("1.0"));
        assert!(output.contains("2.0"));
        assert!(output.contains("TEST"));
        assert!(output.contains("PROD"));
    }
}
