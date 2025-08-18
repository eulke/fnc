//! Shared response summary utilities
//!
//! This module provides common functionality for generating large response summaries
//! that can be reused across different renderers and components.

use crate::renderers::cli::TableBuilder;

/// Statistics for a text response
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseStats {
    pub size_bytes: usize,
    pub line_count: usize,
    pub text: String,
}

/// Summary data for large response comparison (matches BodyDiffSummary from diff_data.rs)
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseDiffSummary {
    pub size1: usize,
    pub size2: usize,
    pub lines1: usize,
    pub lines2: usize,
    pub sample_differences: Vec<String>,
}

impl ResponseStats {
    /// Create response statistics from text (optimized to avoid duplicate calculations)
    pub fn from_text(text: &str) -> Self {
        let line_count = count_lines_efficient(text);
        Self {
            size_bytes: text.len(),
            line_count,
            text: text.to_string(),
        }
    }
}

/// Efficient line counting utility to avoid duplicate calculations across the codebase
#[inline]
pub fn count_lines_efficient(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        // Count newlines and add 1 (more efficient than .lines().count())
        text.bytes().filter(|&b| b == b'\n').count() + 1
    }
}

/// Builder for large response comparison summaries
pub struct LargeResponseSummaryBuilder {
    use_emojis: bool,
    include_differences: bool,
    include_tips: bool,
}

impl LargeResponseSummaryBuilder {
    /// Create a new summary builder with default settings
    pub fn new() -> Self {
        Self {
            use_emojis: true,
            include_differences: true,
            include_tips: true,
        }
    }

    /// Configure whether to use emoji icons
    pub fn with_emojis(mut self, use_emojis: bool) -> Self {
        self.use_emojis = use_emojis;
        self
    }

    /// Configure whether to include difference analysis
    pub fn with_differences(mut self, include_differences: bool) -> Self {
        self.include_differences = include_differences;
        self
    }

    /// Configure whether to include usage tips
    pub fn with_tips(mut self, include_tips: bool) -> Self {
        self.include_tips = include_tips;
        self
    }

    /// Generate a summary for two response texts
    pub fn build_summary(&self, text1: &str, text2: &str, label1: &str, label2: &str) -> String {
        let stats1 = ResponseStats::from_text(text1);
        let stats2 = ResponseStats::from_text(text2);
        self.build_summary_from_stats(&stats1, &stats2, label1, label2)
    }

    /// Generate structured diff summary data  
    pub fn build_structured_summary(&self, text1: &str, text2: &str) -> ResponseDiffSummary {
        let stats1 = ResponseStats::from_text(text1);
        let stats2 = ResponseStats::from_text(text2);

        let mut sample_differences = Vec::new();

        // Try to detect sample differences from the beginning of responses
        let first_lines1: Vec<_> = text1.lines().take(10).collect();
        let first_lines2: Vec<_> = text2.lines().take(10).collect();

        for (i, (line1, line2)) in first_lines1.iter().zip(first_lines2.iter()).enumerate() {
            if line1 != line2 {
                sample_differences.push(format!("Line {}: content differs", i + 1));
                if sample_differences.len() >= 5 {
                    // Limit sample differences to avoid overwhelming output
                    sample_differences.push("... (additional differences truncated)".to_string());
                    break;
                }
            }
        }

        // Check if one response has more lines than the other
        if stats1.line_count != stats2.line_count {
            let diff = (stats1.line_count as i64 - stats2.line_count as i64).abs();
            sample_differences.push(format!("Line count differs by {} lines", diff));
        }

        ResponseDiffSummary {
            size1: stats1.size_bytes,
            size2: stats2.size_bytes,
            lines1: stats1.line_count,
            lines2: stats2.line_count,
            sample_differences,
        }
    }

    /// Generate a summary from pre-calculated statistics
    pub fn build_summary_from_stats(
        &self,
        stats1: &ResponseStats,
        stats2: &ResponseStats,
        label1: &str,
        label2: &str,
    ) -> String {
        let mut output = String::new();

        // Add header
        let title_icon = if self.use_emojis { "🔍 " } else { "" };
        let warning_icon = if self.use_emojis {
            "⚠️  "
        } else {
            "WARNING: "
        };

        output.push_str(&format!(
            "{}Large Response Comparison Summary\n",
            title_icon
        ));
        output.push_str(&format!(
            "{}Responses are too large for detailed diff - showing summary only\n\n",
            warning_icon
        ));

        // Build comparison table
        let table = self.build_comparison_table(stats1, stats2, label1, label2);
        output.push_str(&table);

        // Add difference analysis if enabled
        if self.include_differences {
            output.push_str(&self.build_difference_analysis(stats1, stats2));
        }

        // Add usage tips if enabled
        if self.include_tips {
            output.push_str(&self.build_usage_tips());
        }

        output
    }

    /// Build the comparison table
    fn build_comparison_table(
        &self,
        stats1: &ResponseStats,
        stats2: &ResponseStats,
        label1: &str,
        label2: &str,
    ) -> String {
        let mut table = TableBuilder::new();
        table.headers(vec!["Environment", "Size (bytes)", "Line Count"]);

        table.row(vec![
            label1,
            &stats1.size_bytes.to_string(),
            &stats1.line_count.to_string(),
        ]);

        table.row(vec![
            label2,
            &stats2.size_bytes.to_string(),
            &stats2.line_count.to_string(),
        ]);

        format!("{}\n", table.build())
    }

    /// Build the difference analysis section
    fn build_difference_analysis(&self, stats1: &ResponseStats, stats2: &ResponseStats) -> String {
        let mut output = String::new();

        let diff_icon = if self.use_emojis { "📈 " } else { "" };
        output.push_str(&format!("{}Differences:\n", diff_icon));

        // Size difference
        let size_diff = (stats1.size_bytes as i64 - stats2.size_bytes as i64).abs();
        output.push_str(&format!("   Size difference: {} bytes\n", size_diff));

        // Line count difference
        if stats1.line_count != stats2.line_count {
            let line_diff = (stats1.line_count as i64 - stats2.line_count as i64).abs();
            output.push_str(&format!("   Line count difference: {} lines\n", line_diff));
        }

        output.push('\n');
        output
    }

    /// Build the usage tips section
    fn build_usage_tips(&self) -> String {
        let tip_icon = if self.use_emojis { "💡 " } else { "TIP: " };
        format!(
            "{}Use curl commands or reduce response size for detailed comparison\n",
            tip_icon
        )
    }
}

impl Default for LargeResponseSummaryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_stats_from_text() {
        let text = "line1\nline2\nline3";
        let stats = ResponseStats::from_text(text);

        assert_eq!(stats.size_bytes, 17); // "line1\nline2\nline3".len()
        assert_eq!(stats.line_count, 3);
        assert_eq!(stats.text, text);
    }

    #[test]
    fn test_large_response_summary_builder() {
        let builder = LargeResponseSummaryBuilder::new();
        let text1 = "line1\nline2\nline3\nline4";
        let text2 = "line1\nline2\nDIFF\nline4\nline5";

        let summary = builder.build_summary(text1, text2, "env1", "env2");

        assert!(summary.contains("Large Response Comparison Summary"));
        assert!(summary.contains("env1"));
        assert!(summary.contains("env2"));
        assert!(summary.contains("Size difference"));
        assert!(summary.contains("Line count difference"));
        assert!(summary.contains("🔍")); // emoji enabled by default
        assert!(summary.contains("💡")); // tips enabled by default
    }

    #[test]
    fn test_builder_with_no_emojis() {
        let builder = LargeResponseSummaryBuilder::new().with_emojis(false);
        let text1 = "short";
        let text2 = "longer text";

        let summary = builder.build_summary(text1, text2, "test1", "test2");

        assert!(!summary.contains("🔍"));
        assert!(!summary.contains("💡"));
        assert!(summary.contains("WARNING:"));
        assert!(summary.contains("TIP:"));
    }

    #[test]
    fn test_builder_minimal_output() {
        let builder = LargeResponseSummaryBuilder::new()
            .with_emojis(false)
            .with_differences(false)
            .with_tips(false);

        let text1 = "test1";
        let text2 = "test2";

        let summary = builder.build_summary(text1, text2, "env1", "env2");

        assert!(summary.contains("Large Response Comparison Summary"));
        assert!(summary.contains("env1"));
        assert!(summary.contains("env2"));
        assert!(!summary.contains("Differences:"));
        assert!(!summary.contains("TIP:"));
        assert!(!summary.contains("💡"));
    }

    #[test]
    fn test_identical_responses() {
        let builder = LargeResponseSummaryBuilder::new();
        let text = "identical\ntext\nfor\nboth";

        let summary = builder.build_summary(text, text, "env1", "env2");

        assert!(summary.contains("Size difference: 0 bytes"));
        assert!(!summary.contains("Line count difference")); // Should not show when identical
    }
}
