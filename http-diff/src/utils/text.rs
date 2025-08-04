//! Text processing utilities
//!
//! This module provides basic text processing functions that are reusable
//! across different parts of the application.

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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_utilities() {
        let text = "line1\nline2\nline3\nline4";
        
        assert_eq!(line_count(text), 4);
        assert_eq!(byte_size(text), text.len());
        assert_eq!(truncate_lines(text, 2), "line1\nline2\n... (truncated)");
        assert_eq!(preview(text, 3), "line1\nline2\nline3\n... (truncated)");
    }

}