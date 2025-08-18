//! Shared diff processing utilities
//!
//! This module provides common diff processing functions that can be reused
//! across different renderers and components.

use prettydiff::{basic::DiffOp, diff_slice};

/// Generic diff operation result
#[derive(Debug, Clone)]
pub enum DiffOperation {
    /// Lines that are identical in both texts
    Equal(Vec<String>),
    /// Lines that were removed (only in first text)
    Remove(Vec<String>),
    /// Lines that were added (only in second text)
    Insert(Vec<String>),
    /// Lines that were replaced (different in both texts)
    Replace(Vec<String>, Vec<String>),
}

/// Process two texts and return a sequence of diff operations
pub fn process_text_diff(text1: &str, text2: &str) -> Vec<DiffOperation> {
    let lines1: Vec<&str> = text1.lines().collect();
    let lines2: Vec<&str> = text2.lines().collect();
    
    let diff = diff_slice(&lines1, &lines2);
    let mut operations = Vec::new();
    
    for op in diff.diff {
        match op {
            DiffOp::Equal(lines) => {
                operations.push(DiffOperation::Equal(
                    lines.iter().map(|s| s.to_string()).collect()
                ));
            }
            DiffOp::Remove(lines) => {
                operations.push(DiffOperation::Remove(
                    lines.iter().map(|s| s.to_string()).collect()
                ));
            }
            DiffOp::Insert(lines) => {
                operations.push(DiffOperation::Insert(
                    lines.iter().map(|s| s.to_string()).collect()
                ));
            }
            DiffOp::Replace(removed_lines, inserted_lines) => {
                operations.push(DiffOperation::Replace(
                    removed_lines.iter().map(|s| s.to_string()).collect(),
                    inserted_lines.iter().map(|s| s.to_string()).collect()
                ));
            }
        }
    }
    
    operations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_texts() {
        let text1 = "line1\nline2\nline3";
        let text2 = "line1\nline2\nline3";
        
        let operations = process_text_diff(text1, text2);
        
        assert_eq!(operations.len(), 1);
        match &operations[0] {
            DiffOperation::Equal(lines) => {
                assert_eq!(lines, &vec!["line1", "line2", "line3"]);
            }
            _ => panic!("Expected Equal operation"),
        }
    }

    #[test]
    fn test_added_lines() {
        let text1 = "line1\nline2";
        let text2 = "line1\nline2\nline3";
        
        let operations = process_text_diff(text1, text2);
        
        assert!(operations.len() >= 1);
        // Should contain both equal and insert operations
        let has_equal = operations.iter().any(|op| matches!(op, DiffOperation::Equal(_)));
        let has_insert = operations.iter().any(|op| matches!(op, DiffOperation::Insert(_)));
        
        assert!(has_equal);
        assert!(has_insert);
    }

    #[test]
    fn test_removed_lines() {
        let text1 = "line1\nline2\nline3";
        let text2 = "line1\nline2";
        
        let operations = process_text_diff(text1, text2);
        
        assert!(operations.len() >= 1);
        // Should contain both equal and remove operations
        let has_equal = operations.iter().any(|op| matches!(op, DiffOperation::Equal(_)));
        let has_remove = operations.iter().any(|op| matches!(op, DiffOperation::Remove(_)));
        
        assert!(has_equal);
        assert!(has_remove);
    }
}