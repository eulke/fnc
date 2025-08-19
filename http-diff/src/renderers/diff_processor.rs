//! Pure diff processing logic without formatting concerns
//!
//! This module contains the core business logic for processing diff data
//! from raw comparison results into generic data structures that can be
//! rendered by different presentation layers.

use super::diff_data::{BodyDiffData, BodyDiffSummary, DiffData, DiffRow, HeaderDiffData};
use crate::comparison::analyzer::{BodyDiff, HeaderDiff};
use crate::types::{ComparisonResult, DifferenceCategory};
use crate::utils::environment_utils::EnvironmentValidator;
use crate::error::{HttpDiffError, Result};

/// Processor for extracting and organizing diff data
pub struct DiffProcessor {
    /// Maximum response size before switching to summary mode
    large_response_threshold: usize,
}

impl DiffProcessor {
    /// Create a new diff processor with default settings
    pub fn new() -> Self {
        Self {
            large_response_threshold: crate::types::DEFAULT_LARGE_RESPONSE_THRESHOLD,
        }
    }

    /// Create a diff processor with custom large response threshold
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            large_response_threshold: threshold,
        }
    }

    /// Process a complete comparison result into generic diff data
    pub fn process_comparison_result(
        &self,
        result: &ComparisonResult,
        compare_headers: bool,
    ) -> Result<DiffData> {
        let mut diff_data = DiffData::new(result.route_name.clone());

        // Process each difference in the comparison result
        for difference in &result.differences {
            match difference.category {
                DifferenceCategory::Headers if compare_headers => {
                    // Use structured data directly instead of JSON deserialization
                    if let Some(ref header_diffs) = difference.header_diff {
                        let envs = self.extract_environment_names(result)?;
                        let header_diff_data =
                            self.process_header_diffs(header_diffs, &envs.0, &envs.1);
                        diff_data.set_headers(header_diff_data);
                    } else if let Some(ref diff_output) = difference.diff_output {
                        // Fallback for backward compatibility
                        let header_diffs: Vec<HeaderDiff> = serde_json::from_str(diff_output)
                            .map_err(|e| HttpDiffError::general(format!("Failed to parse header diff data: {}", e)))?;

                        let envs = self.extract_environment_names(result)?;
                        let header_diff_data =
                            self.process_header_diffs(&header_diffs, &envs.0, &envs.1);
                        diff_data.set_headers(header_diff_data);
                    }
                }
                DifferenceCategory::Body => {
                    // Use structured data directly instead of JSON deserialization
                    if let Some(ref body_diff) = difference.body_diff {
                        let envs = self.extract_environment_names(result)?;
                        let body_diff_data = self.process_body_diff(body_diff, &envs.0, &envs.1);
                        diff_data.set_body(body_diff_data);
                    } else if let Some(ref diff_output) = difference.diff_output {
                        // Fallback for backward compatibility
                        let body_diff: BodyDiff = serde_json::from_str(diff_output)
                            .map_err(|e| HttpDiffError::general(format!("Failed to parse body diff data: {}", e)))?;

                        let envs = self.extract_environment_names(result)?;
                        let body_diff_data = self.process_body_diff(&body_diff, &envs.0, &envs.1);
                        diff_data.set_body(body_diff_data);
                    }
                }
                DifferenceCategory::Status => {
                    // Status differences are already captured in the description
                    // We don't need separate diff rows for status codes
                }
                _ => {
                    // Skip headers if header comparison is disabled
                }
            }
        }

        Ok(diff_data)
    }

    /// Process header differences into generic diff data
    pub fn process_header_diffs(
        &self,
        header_diffs: &[HeaderDiff],
        env1: &str,
        env2: &str,
    ) -> HeaderDiffData {
        let mut data = HeaderDiffData::new(env1.to_string(), env2.to_string());

        for diff in header_diffs {
            let row = match (&diff.value1, &diff.value2) {
                (Some(val1), Some(val2)) if val1 != val2 => {
                    // Header exists in both but with different values
                    DiffRow::changed(val1.clone(), val2.clone()).with_context(diff.name.clone())
                }
                (Some(val1), None) => {
                    // Header only exists in first environment
                    DiffRow::removed(val1.clone()).with_context(diff.name.clone())
                }
                (None, Some(val2)) => {
                    // Header only exists in second environment
                    DiffRow::added(val2.clone()).with_context(diff.name.clone())
                }
                (Some(val), Some(_)) => {
                    // Headers are identical (shouldn't happen in diff data, but handle gracefully)
                    DiffRow::unchanged(val.clone()).with_context(diff.name.clone())
                }
                (None, None) => {
                    // This shouldn't happen in real diff data
                    continue;
                }
            };

            data.add_row(row);
        }

        data
    }

    /// Process body differences into generic diff data
    pub fn process_body_diff(&self, body_diff: &BodyDiff, env1: &str, env2: &str) -> BodyDiffData {
        // Check if this is a large response that should be summarized
        if body_diff.is_large_response || body_diff.total_size > self.large_response_threshold {
            let summary = self.create_body_summary(body_diff);
            return BodyDiffData::new_large_response(
                env1.to_string(),
                env2.to_string(),
                body_diff.total_size,
                summary,
            );
        }

        // Process detailed diff for normal-sized responses
        let mut data = BodyDiffData::new(env1.to_string(), env2.to_string());
        data.set_total_size(body_diff.total_size);

        // Use prettydiff to generate line-by-line diff
        let lines1: Vec<&str> = body_diff.normalized_body1.lines().collect();
        let lines2: Vec<&str> = body_diff.normalized_body2.lines().collect();

        // Generate diff using prettydiff
        use prettydiff::{basic::DiffOp, diff_slice};
        let diff = diff_slice(&lines1, &lines2);

        // Convert prettydiff operations to our generic diff rows
        for op in diff.diff {
            match op {
                DiffOp::Equal(lines) => {
                    // Lines are identical
                    for line in lines {
                        data.add_row(DiffRow::unchanged(line.to_string()));
                    }
                }
                DiffOp::Remove(lines) => {
                    // Lines only exist in first environment
                    for line in lines {
                        data.add_row(DiffRow::removed(line.to_string()));
                    }
                }
                DiffOp::Insert(lines) => {
                    // Lines only exist in second environment
                    for line in lines {
                        data.add_row(DiffRow::added(line.to_string()));
                    }
                }
                DiffOp::Replace(old_lines, new_lines) => {
                    // Lines were replaced
                    let max_lines = old_lines.len().max(new_lines.len());

                    for i in 0..max_lines {
                        match (old_lines.get(i), new_lines.get(i)) {
                            (Some(old), Some(new)) => {
                                data.add_row(DiffRow::changed(old.to_string(), new.to_string()));
                            }
                            (Some(old), None) => {
                                data.add_row(DiffRow::removed(old.to_string()));
                            }
                            (None, Some(new)) => {
                                data.add_row(DiffRow::added(new.to_string()));
                            }
                            (None, None) => {
                                // This shouldn't happen
                            }
                        }
                    }
                }
            }
        }

        data
    }

    /// Create a summary for large response body diffs using shared utility
    fn create_body_summary(&self, body_diff: &BodyDiff) -> BodyDiffSummary {
        let builder = crate::utils::response_summary::LargeResponseSummaryBuilder::new();
        builder.build_structured_summary(&body_diff.normalized_body1, &body_diff.normalized_body2)
    }

    /// Extract environment names from comparison result with consistent ordering
    fn extract_environment_names(
        &self,
        result: &ComparisonResult,
    ) -> Result<(String, String)> {
        // Use the environment resolver for consistent ordering
        let resolver = result.create_environment_resolver();
        let ordered_environments = result.get_ordered_environment_names(&resolver);

        // Validate minimum environment count
        EnvironmentValidator::validate_minimum_environments(&ordered_environments)?;

        // For diff processing, we typically compare the first environment (base) against the second
        // This ensures consistent environment ordering in diffs
        Ok((ordered_environments[0].clone(), ordered_environments[1].clone()))
    }
}

impl Default for DiffProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::diff_data::DiffOperation;

    #[test]
    fn test_process_header_diffs() {
        let processor = DiffProcessor::new();

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

        let result = processor.process_header_diffs(&header_diffs, "test", "prod");

        assert_eq!(result.env1, "test");
        assert_eq!(result.env2, "prod");
        assert_eq!(result.rows.len(), 2);
        assert!(result.has_differences);

        // Check first row (changed header)
        assert_eq!(result.rows[0].operation, DiffOperation::Changed);
        assert_eq!(result.rows[0].left_content, Some("1.0".to_string()));
        assert_eq!(result.rows[0].right_content, Some("2.0".to_string()));
        assert_eq!(result.rows[0].context, Some("X-Version".to_string()));

        // Check second row (added header)
        assert_eq!(result.rows[1].operation, DiffOperation::Added);
        assert_eq!(result.rows[1].left_content, None);
        assert_eq!(result.rows[1].right_content, Some("new-value".to_string()));
        assert_eq!(result.rows[1].context, Some("X-New-Header".to_string()));
    }

    #[test]
    fn test_body_diff_processing() {
        let processor = DiffProcessor::new();

        let body_diff = BodyDiff {
            normalized_body1: "line1\nline2\nline3".to_string(),
            normalized_body2: "line1\nmodified_line2\nline3".to_string(),
            is_large_response: false,
            total_size: 100,
        };

        let result = processor.process_body_diff(&body_diff, "test", "prod");

        assert_eq!(result.env1, "test");
        assert_eq!(result.env2, "prod");
        assert!(!result.is_large_response);
        assert_eq!(result.total_size, 100);
        assert!(result.has_differences);
        assert!(!result.rows.is_empty());
    }

    #[test]
    fn test_large_response_summary() {
        let processor = DiffProcessor::with_threshold(10); // Very small threshold for testing

        let body_diff = BodyDiff {
            normalized_body1: "This is a long response body".to_string(),
            normalized_body2: "This is a different long response body".to_string(),
            is_large_response: true,
            total_size: 100,
        };

        let result = processor.process_body_diff(&body_diff, "test", "prod");

        assert!(result.is_large_response);
        assert!(result.summary.is_some());
        assert!(result.rows.is_empty()); // No detailed rows for large responses

        let summary = result.summary.unwrap();
        assert_eq!(summary.size1, 28); // "This is a long response body".len()
        assert_eq!(summary.size2, 38); // "This is a different long response body".len()
    }
}
