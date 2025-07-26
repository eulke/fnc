
use std::fmt::Write;

/// Configuration for text formatting operations
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Maximum response size before switching to summary mode
    pub large_response_threshold: usize,
    /// Maximum width for side-by-side diffs
    pub max_column_width: usize,
    /// Whether to preserve ANSI color codes
    pub preserve_ansi: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            large_response_threshold: 50_000, // 50KB
            max_column_width: 80,
            preserve_ansi: true,
        }
    }
}

/// Diff view style for text comparison
#[derive(Debug, Clone, PartialEq)]
pub enum DiffStyle {
    /// Traditional unified diff (up/down)
    Unified,
    /// Side-by-side diff view
    SideBySide,
}

/// Text formatter with various utility functions
pub struct TextFormatter {
    config: FormatterConfig,
}

impl TextFormatter {
    /// Create a new text formatter with default configuration
    pub fn new() -> Self {
        Self {
            config: FormatterConfig::default(),
        }
    }

    /// Create a text formatter with custom configuration
    pub fn with_config(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// Generate a unified diff between two texts
    pub fn unified_diff(
        &self,
        text1: &str,
        text2: &str,
        label1: &str,
        label2: &str,
    ) -> String {
        let total_size = text1.len() + text2.len();

        // For very large responses, provide a summary instead of full diff
        if total_size > self.config.large_response_threshold {
            return self.large_response_summary(text1, text2, label1, label2);
        }

        // Use prettydiff's native unified diff output
        let diff = prettydiff::diff_lines(text1, text2);
        diff.to_string()
    }

    /// Generate a side-by-side diff between two texts
    pub fn side_by_side_diff(
        &self,
        text1: &str,
        text2: &str,
        label1: &str,
        label2: &str,
    ) -> String {
        let total_size = text1.len() + text2.len();

        // For very large responses, provide a summary instead of full diff
        if total_size > self.config.large_response_threshold {
            return self.large_response_summary(text1, text2, label1, label2);
        }

        // Use simplified side-by-side formatting 
        self.format_side_by_side(text1, text2, label1, label2)
    }

    /// Generate a summary for large responses
    fn large_response_summary(
        &self,
        text1: &str,
        text2: &str,
        label1: &str,
        label2: &str,
    ) -> String {
        let lines1 = text1.lines().count();
        let lines2 = text2.lines().count();
        let size1 = text1.len();
        let size2 = text2.len();

        // Use TableBuilder for consistent formatting
        use crate::table_builder::TableBuilder;
        
        let mut table = TableBuilder::new();
        table.headers(vec!["Environment", "Size (bytes)", "Lines"]);
        table.row(vec![&label1.to_uppercase(), &size1.to_string(), &lines1.to_string()]);
        table.row(vec![&label2.to_uppercase(), &size2.to_string(), &lines2.to_string()]);
        
        let mut output = String::new();
        writeln!(output, "🔍 Large Response Comparison Summary").unwrap();
        writeln!(output, "⚠️  Responses are too large for detailed diff - showing summary only\n").unwrap();
        output.push_str(&table.build());
        writeln!(output).unwrap();

        writeln!(output, "\n📈 Differences:").unwrap();
        let size_diff = (size1 as i64 - size2 as i64).abs();
        writeln!(output, "   Size difference: {} bytes", size_diff).unwrap();

        if lines1 != lines2 {
            let line_diff = (lines1 as i64 - lines2 as i64).abs();
            writeln!(output, "   Line count difference: {} lines", line_diff).unwrap();
        }

        // Try to detect what kind of differences exist
        let first_lines1: Vec<_> = text1.lines().take(10).collect();
        let first_lines2: Vec<_> = text2.lines().take(10).collect();

        if first_lines1 != first_lines2 {
            writeln!(output, "\n🔍 Sample Differences (first 10 lines):").unwrap();
            for (i, (line1, line2)) in first_lines1.iter().zip(first_lines2.iter()).enumerate() {
                if line1 != line2 {
                    writeln!(output, "   Line {}: content differs", i + 1).unwrap();
                    if output.lines().count() > 20 {
                        // Limit output
                        writeln!(output, "   ... (truncated)").unwrap();
                        break;
                    }
                }
            }
        }

        output
    }

    /// Format a side-by-side diff using proper table rendering with diff styling
    fn format_side_by_side(
        &self,
        text1: &str,
        text2: &str,
        label1: &str,
        label2: &str,
    ) -> String {
                use crate::table_builder::{TableBuilder, TableStyle, cells};
        use prettydiff::{diff_slice, basic::DiffOp};

        // Create table with diff styling (no horizontal lines between rows)
        let mut table = TableBuilder::new();
        table.apply_style(TableStyle::Diff);
        
        // Add headers with uppercase environment names
        table.headers(vec![&label1.to_uppercase(), &label2.to_uppercase()]);

        // Convert texts to line vectors for prettydiff
        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();
        
        // Generate diff using prettydiff's proper diff algorithm
        let diff = diff_slice(&lines1, &lines2);
        
        // Process diff operations to create side-by-side view
        for op in diff.diff {
            match op {
                DiffOp::Equal(lines) => {
                    // Lines are identical on both sides
                    for line in lines {
                        let truncated = self.truncate_line_simple(line);
                        table.styled_row(vec![
                            cells::normal(&format!("  {}", truncated)),
                            cells::normal(&format!("  {}", truncated))
                        ]);
                    }
                }
                DiffOp::Remove(lines) => {
                    // Lines only exist in left side (removed)
                    for line in lines {
                        let truncated = self.truncate_line_simple(line);
                        table.styled_row(vec![
                            cells::removed(&format!("- {}", truncated)),
                            cells::normal("")
                        ]);
                    }
                }
                DiffOp::Insert(lines) => {
                    // Lines only exist in right side (added)
                    for line in lines {
                        let truncated = self.truncate_line_simple(line);
                        table.styled_row(vec![
                            cells::normal(""),
                            cells::added(&format!("+ {}", truncated))
                        ]);
                    }
                }
                DiffOp::Replace(old_lines, new_lines) => {
                    // Lines were replaced - show old on left, new on right
                    let max_lines = old_lines.len().max(new_lines.len());
                    
                    for i in 0..max_lines {
                        let left_content = if let Some(line) = old_lines.get(i) {
                            let truncated = self.truncate_line_simple(line);
                            cells::removed(&format!("- {}", truncated))
                        } else {
                            cells::normal("")
                        };
                        
                        let right_content = if let Some(line) = new_lines.get(i) {
                            let truncated = self.truncate_line_simple(line);
                            cells::added(&format!("+ {}", truncated))
                        } else {
                            cells::normal("")
                        };
                        
                        table.styled_row(vec![left_content, right_content]);
                    }
                }
            }
        }

        table.build()
    }



    /// Truncate a line using the configured max column width
    fn truncate_line_simple(&self, line: &str) -> String {
        let max_width = self.config.max_column_width.saturating_sub(3); // Account for "- " or "+ " prefix
        if line.len() <= max_width {
            line.to_string()
        } else {
            format!("{}...", &line[..max_width.saturating_sub(3)])
        }
    }

    /// Check if the total response size exceeds the threshold
    pub fn is_large_response(&self, text1: &str, text2: &str) -> bool {
        text1.len() + text2.len() > self.config.large_response_threshold
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Shell escaping utilities for generating command-line safe strings
pub mod shell {
    /// Escape shell arguments to handle special characters properly
    pub fn escape_argument(arg: &str) -> String {
        // Handle single quotes by replacing them with '"'"'
        // This closes the current quote, adds an escaped quote, then opens a new quote
        arg.replace('\'', "'\"'\"'")
    }

    /// Escape and quote a shell argument if it contains special characters
    pub fn quote_if_needed(arg: &str) -> String {
        if needs_quoting(arg) {
            format!("'{}'", escape_argument(arg))
        } else {
            arg.to_string()
        }
    }

    /// Check if a string needs shell quoting
    pub fn needs_quoting(arg: &str) -> bool {
        arg.chars().any(|c| matches!(c, ' ' | '\t' | '\n' | '\'' | '"' | '\\' | '|' | '&' | ';' | '(' | ')' | '<' | '>' | '`' | '$'))
    }

    /// Format a curl command with proper escaping
    pub fn format_curl_command(method: &str, url: &str, headers: &[(String, String)], body: Option<&str>) -> String {
        let mut command = format!("curl -X {} {}", method, quote_if_needed(url));

        // Add headers with proper escaping
        for (key, value) in headers {
            command.push_str(&format!(" \\\n  -H {}: {}", 
                quote_if_needed(key), 
                quote_if_needed(value)
            ));
        }

        // Add body if present
        if let Some(body) = body {
            command.push_str(&format!(" \\\n  -d {}", quote_if_needed(body)));
        }

        command
    }
}

/// Utility functions for text processing
pub mod text {
    /// Count the number of lines in a text
    pub fn line_count(text: &str) -> usize {
        text.lines().count()
    }

    /// Get the size of text in bytes
    pub fn byte_size(text: &str) -> usize {
        text.len()
    }

    /// Truncate text to a maximum number of lines
    pub fn truncate_lines(text: &str, max_lines: usize) -> String {
        let lines: Vec<&str> = text.lines().take(max_lines).collect();
        let mut result = lines.join("\n");
        
        if text.lines().count() > max_lines {
            result.push_str("\n... (truncated)");
        }
        
        result
    }

    /// Get a preview of text (first few lines)
    pub fn preview(text: &str, max_lines: usize) -> String {
        truncate_lines(text, max_lines)
    }

    /// Check if two texts are identical
    pub fn are_identical(text1: &str, text2: &str) -> bool {
        text1 == text2
    }

    /// Calculate a simple similarity score between two texts
    pub fn similarity_score(text1: &str, text2: &str) -> f64 {
        if text1 == text2 {
            return 1.0;
        }
        
        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();
        
        if lines1.is_empty() && lines2.is_empty() {
            return 1.0;
        }
        
        let max_lines = lines1.len().max(lines2.len());
        if max_lines == 0 {
            return 1.0;
        }
        
        let matching_lines = lines1.iter()
            .zip(lines2.iter())
            .filter(|(l1, l2)| l1 == l2)
            .count();
            
        matching_lines as f64 / max_lines as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escaping() {
        assert_eq!(shell::escape_argument("simple"), "simple");
        assert_eq!(shell::escape_argument("with'quote"), "with'\"'\"'quote");
        assert_eq!(shell::escape_argument("multiple'single'quotes"), "multiple'\"'\"'single'\"'\"'quotes");
    }

    #[test]
    fn test_shell_quoting() {
        assert_eq!(shell::quote_if_needed("simple"), "simple");
        assert_eq!(shell::quote_if_needed("with space"), "'with space'");
        assert_eq!(shell::quote_if_needed("with'quote"), "'with'\"'\"'quote'");
    }

    #[test]
    fn test_text_utilities() {
        let text = "line1\nline2\nline3\nline4";
        
        assert_eq!(text::line_count(text), 4);
        assert_eq!(text::byte_size(text), text.len());
        assert_eq!(text::truncate_lines(text, 2), "line1\nline2\n... (truncated)");
        assert_eq!(text::preview(text, 3), "line1\nline2\nline3\n... (truncated)");
    }

    #[test]
    fn test_text_similarity() {
        assert_eq!(text::similarity_score("identical", "identical"), 1.0);
        assert_eq!(text::similarity_score("", ""), 1.0);
        
        let score = text::similarity_score("line1\nline2", "line1\nline3");
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn test_unified_diff() {
        let formatter = TextFormatter::new();
        let diff = formatter.unified_diff("hello\nworld", "hello\nrust", "old", "new");
        
        assert!(!diff.is_empty());
        // We can't easily test the exact content since prettydiff uses ANSI colors
    }

    #[test]
    fn test_large_response_detection() {
        let formatter = TextFormatter::with_config(FormatterConfig {
            large_response_threshold: 10,
            ..Default::default()
        });
        
        assert!(formatter.is_large_response("123456", "67890")); // 11 bytes total > 10 threshold
        assert!(!formatter.is_large_response("123", "456")); // 6 bytes total < 10 threshold
    }

    #[test]
    fn test_curl_command_formatting() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer token'with'quotes".to_string()),
        ];
        
        let command = shell::format_curl_command(
            "POST",
            "https://api.example.com/users",
            &headers,
            Some(r#"{"name": "test"}"#)
        );
        
        assert!(command.contains("curl -X POST"));
        assert!(command.contains("https://api.example.com/users"));
        assert!(command.contains("Content-Type"));
        assert!(command.contains("Bearer token"));
        assert!(command.contains(r#"{"name": "test"}"#));
    }
} 