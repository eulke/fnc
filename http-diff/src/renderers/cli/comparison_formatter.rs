use super::table::{cells, TableBuilder, TableStyle};
use super::text_formatter::TextFormatter;
/// Comparison-specific formatting for diff output - handles presentation of differences
use crate::comparison::analyzer::{BodyDiff, HeaderDiff};
use crate::types::DiffViewStyle;

/// Formatter for comparison differences - handles all presentation logic
pub struct ComparisonFormatter {
    text_formatter: TextFormatter,
}

impl ComparisonFormatter {
    /// Create a new comparison formatter
    pub fn new() -> Self {
        Self {
            text_formatter: TextFormatter::new(),
        }
    }

    /// Format header differences based on view style
    pub fn format_header_differences(
        &self,
        header_diffs: &[HeaderDiff],
        env1: &str,
        env2: &str,
        diff_style: DiffViewStyle,
    ) -> String {
        match diff_style {
            DiffViewStyle::Unified => self.format_headers_unified(header_diffs, env1, env2),
            DiffViewStyle::SideBySide => self.format_headers_side_by_side(header_diffs, env1, env2),
        }
    }

    /// Format unified diff table for headers
    fn format_headers_unified(
        &self,
        header_diffs: &[HeaderDiff],
        env1: &str,
        env2: &str,
    ) -> String {
        let mut table = TableBuilder::new();
        table.apply_style(TableStyle::Diff); // Remove horizontal lines
        table.headers(vec!["Header", "Environment", "Value"]);

        for diff in header_diffs {
            if let Some(value1) = &diff.value1 {
                table.styled_row(vec![
                    cells::normal(&diff.name),
                    cells::removed(format!("- {}", env1.to_uppercase())),
                    cells::removed(value1),
                ]);
            }
            if let Some(value2) = &diff.value2 {
                table.styled_row(vec![
                    cells::normal(&diff.name),
                    cells::added(format!("+ {}", env2.to_uppercase())),
                    cells::added(value2),
                ]);
            }
        }

        table.build()
    }

    /// Format side-by-side diff table for headers
    fn format_headers_side_by_side(
        &self,
        header_diffs: &[HeaderDiff],
        env1: &str,
        env2: &str,
    ) -> String {
        let mut table = TableBuilder::new();
        table.apply_style(TableStyle::Diff);
        table.headers(vec!["Header", &env1.to_uppercase(), &env2.to_uppercase()]);

        for diff in header_diffs {
            let left_value = match &diff.value1 {
                Some(value) => cells::normal(value),
                None => cells::muted("(missing)"),
            };

            let right_value = match &diff.value2 {
                Some(value) => cells::normal(value),
                None => cells::muted("(missing)"),
            };

            // Color the header name if values are different
            let header_cell = if diff.value1 != diff.value2 {
                cells::bold(&diff.name)
            } else {
                cells::normal(&diff.name)
            };

            table.styled_row(vec![header_cell, left_value, right_value]);
        }

        table.build()
    }

    /// Format body differences based on view style
    pub fn format_body_difference(
        &self,
        body_diff: &BodyDiff,
        env1: &str,
        env2: &str,
        diff_style: DiffViewStyle,
    ) -> String {
        if body_diff.is_large_response {
            return self.format_large_response_summary(body_diff, env1, env2);
        }

        match diff_style {
            DiffViewStyle::Unified => self.format_unified_body_diff(
                &body_diff.normalized_body1,
                &body_diff.normalized_body2,
                env1,
                env2,
            ),
            DiffViewStyle::SideBySide => self.format_side_by_side_body_diff(
                &body_diff.normalized_body1,
                &body_diff.normalized_body2,
                env1,
                env2,
            ),
        }
    }

    /// Format unified diff for body content
    fn format_unified_body_diff(&self, text1: &str, text2: &str, env1: &str, env2: &str) -> String {
        let diff_output = self.text_formatter.unified_diff(text1, text2, env1, env2);

        // Create table using diff table style
        let mut table = TableBuilder::with_style(TableStyle::Diff);
        for line in diff_output.lines() {
            table.line(line);
        }
        let table_output = table.build();

        format!("\n{}\n", table_output)
    }

    /// Format side-by-side diff for body content
    fn format_side_by_side_body_diff(
        &self,
        text1: &str,
        text2: &str,
        env1: &str,
        env2: &str,
    ) -> String {
        self.text_formatter
            .side_by_side_diff(text1, text2, env1, env2)
    }

    /// Format large response summary
    fn format_large_response_summary(
        &self,
        body_diff: &BodyDiff,
        env1: &str,
        env2: &str,
    ) -> String {
        let lines1 = body_diff.normalized_body1.lines().count();
        let lines2 = body_diff.normalized_body2.lines().count();
        let size1 = body_diff.normalized_body1.len();
        let size2 = body_diff.normalized_body2.len();

        let mut table = TableBuilder::new();
        table.headers(vec!["Environment", "Response Size", "Line Count"]);
        table.row(vec![env1, &format!("{} bytes", size1), &lines1.to_string()]);
        table.row(vec![env2, &format!("{} bytes", size2), &lines2.to_string()]);
        let table_output = table.build();

        let mut output = String::new();
        output.push_str("\n🔍 Large Response Comparison Summary\n");
        output.push_str("⚠️  Responses are too large for detailed diff - showing summary only\n\n");
        output.push_str(&table_output);

        // Add size difference analysis
        output.push_str("\n📈 Differences:\n");
        let size_diff = (size1 as i64 - size2 as i64).abs();
        output.push_str(&format!("   Size difference: {} bytes\n", size_diff));

        if lines1 != lines2 {
            let line_diff = (lines1 as i64 - lines2 as i64).abs();
            output.push_str(&format!("   Line count difference: {} lines\n", line_diff));
        }

        output.push_str(
            "\n💡 Tip: Use curl commands or reduce response size for detailed comparison\n",
        );
        output
    }
}

impl Default for ComparisonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_unified_formatting() {
        let formatter = ComparisonFormatter::new();

        let header_diffs = vec![
            HeaderDiff {
                name: "X-Version".to_string(),
                value1: Some("1.0".to_string()),
                value2: Some("2.0".to_string()),
            },
            HeaderDiff {
                name: "X-New-Header".to_string(),
                value1: None,
                value2: Some("new-value".to_string()),
            },
        ];

        let output = formatter.format_header_differences(
            &header_diffs,
            "test",
            "prod",
            DiffViewStyle::Unified,
        );

        assert!(output.contains("X-Version"));
        assert!(output.contains("X-New-Header"));
        assert!(output.contains("1.0"));
        assert!(output.contains("2.0"));
        assert!(output.contains("new-value"));
        assert!(output.contains("TEST"));
        assert!(output.contains("PROD"));
    }

    #[test]
    fn test_header_side_by_side_formatting() {
        let formatter = ComparisonFormatter::new();

        let header_diffs = vec![HeaderDiff {
            name: "X-Version".to_string(),
            value1: Some("1.0".to_string()),
            value2: Some("2.0".to_string()),
        }];

        let output = formatter.format_header_differences(
            &header_diffs,
            "test",
            "prod",
            DiffViewStyle::SideBySide,
        );

        assert!(output.contains("X-Version"));
        assert!(output.contains("1.0"));
        assert!(output.contains("2.0"));
        assert!(output.contains("TEST"));
        assert!(output.contains("PROD"));
    }

    #[test]
    fn test_body_diff_formatting() {
        let formatter = ComparisonFormatter::new();

        let body_diff = BodyDiff {
            normalized_body1: "line1\nline2\nline3".to_string(),
            normalized_body2: "line1\nmodified_line2\nline3".to_string(),
            is_large_response: false,
            total_size: 100,
        };

        let unified_output =
            formatter.format_body_difference(&body_diff, "test", "prod", DiffViewStyle::Unified);
        assert!(!unified_output.is_empty());

        let side_by_side_output =
            formatter.format_body_difference(&body_diff, "test", "prod", DiffViewStyle::SideBySide);
        assert!(!side_by_side_output.is_empty());
        assert!(side_by_side_output.contains("TEST"));
        assert!(side_by_side_output.contains("PROD"));
    }

    #[test]
    fn test_large_response_summary() {
        let formatter = ComparisonFormatter::new();

        let body_diff = BodyDiff {
            normalized_body1: "x".repeat(60_000),
            normalized_body2: "y".repeat(60_000),
            is_large_response: true,
            total_size: 120_000,
        };

        let output =
            formatter.format_body_difference(&body_diff, "test", "prod", DiffViewStyle::Unified);

        assert!(output.contains("Large Response Comparison Summary"));
        assert!(output.contains("Environment"));
        assert!(output.contains("Response Size"));
        assert!(output.contains("60000 bytes"));
        assert!(output.contains("Size difference:"));
        assert!(output.contains("💡 Tip:"));
    }

    #[test]
    fn test_missing_header_formatting() {
        let formatter = ComparisonFormatter::new();

        let header_diffs = vec![HeaderDiff {
            name: "X-Missing".to_string(),
            value1: Some("exists".to_string()),
            value2: None,
        }];

        let output = formatter.format_header_differences(
            &header_diffs,
            "test",
            "prod",
            DiffViewStyle::SideBySide,
        );

        assert!(output.contains("X-Missing"));
        assert!(output.contains("exists"));
        assert!(output.contains("(missing)"));
    }
}
