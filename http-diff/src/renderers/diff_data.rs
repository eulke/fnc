//! Generic diff data structures that separate data from presentation
//!
//! This module provides pure data structures for representing diff information
//! without any formatting concerns. These structures can be used by different
//! renderers (CLI, TUI, etc.) to apply their own presentation logic.

/// Type of operation in a diff row
#[derive(Debug, Clone, PartialEq)]
pub enum DiffOperation {
    /// Content is unchanged between environments
    Unchanged,
    /// Content exists only in the left environment (removed)
    Removed,
    /// Content exists only in the right environment (added)
    Added,
    /// Content differs between environments (modified)
    Changed,
}

/// A single row of diff data representing a comparison
#[derive(Debug, Clone)]
pub struct DiffRow {
    /// The type of operation this row represents
    pub operation: DiffOperation,
    /// Content from the left side (first environment)
    pub left_content: Option<String>,
    /// Content from the right side (second environment)
    pub right_content: Option<String>,
    /// Additional context information (header name, line number, etc.)
    pub context: Option<String>,
}

impl DiffRow {
    /// Create a new unchanged diff row
    pub fn unchanged(content: String) -> Self {
        Self {
            operation: DiffOperation::Unchanged,
            left_content: Some(content.clone()),
            right_content: Some(content),
            context: None,
        }
    }

    /// Create a new removed diff row (only exists in left environment)
    pub fn removed(content: String) -> Self {
        Self {
            operation: DiffOperation::Removed,
            left_content: Some(content),
            right_content: None,
            context: None,
        }
    }

    /// Create a new added diff row (only exists in right environment)
    pub fn added(content: String) -> Self {
        Self {
            operation: DiffOperation::Added,
            left_content: None,
            right_content: Some(content),
            context: None,
        }
    }

    /// Create a new changed diff row (different content in both environments)
    pub fn changed(left_content: String, right_content: String) -> Self {
        Self {
            operation: DiffOperation::Changed,
            left_content: Some(left_content),
            right_content: Some(right_content),
            context: None,
        }
    }

    /// Add context information to this diff row
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }
}

/// Header diff data representing differences between HTTP headers
#[derive(Debug, Clone)]
pub struct HeaderDiffData {
    /// List of diff rows for headers
    pub rows: Vec<DiffRow>,
    /// Name of the first environment
    pub env1: String,
    /// Name of the second environment
    pub env2: String,
    /// Whether there are any actual differences
    pub has_differences: bool,
}

impl HeaderDiffData {
    /// Create new header diff data
    pub fn new(env1: String, env2: String) -> Self {
        Self {
            rows: Vec::new(),
            env1,
            env2,
            has_differences: false,
        }
    }

    /// Add a diff row and update has_differences flag
    pub fn add_row(&mut self, row: DiffRow) {
        if row.operation != DiffOperation::Unchanged {
            self.has_differences = true;
        }
        self.rows.push(row);
    }

    /// Check if this header diff data is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Body diff data representing differences between HTTP response bodies
#[derive(Debug, Clone)]
pub struct BodyDiffData {
    /// List of diff rows for body content
    pub rows: Vec<DiffRow>,
    /// Name of the first environment
    pub env1: String,
    /// Name of the second environment
    pub env2: String,
    /// Whether the response is too large for detailed diff
    pub is_large_response: bool,
    /// Total size of both responses combined
    pub total_size: usize,
    /// Whether there are any actual differences
    pub has_differences: bool,
    /// Summary information for large responses
    pub summary: Option<BodyDiffSummary>,
}

/// Summary information for large response body diffs
#[derive(Debug, Clone)]
pub struct BodyDiffSummary {
    /// Size of first response in bytes
    pub size1: usize,
    /// Size of second response in bytes
    pub size2: usize,
    /// Number of lines in first response
    pub lines1: usize,
    /// Number of lines in second response
    pub lines2: usize,
    /// Sample differences from the beginning of responses
    pub sample_differences: Vec<String>,
}

impl BodyDiffData {
    /// Create new body diff data
    pub fn new(env1: String, env2: String) -> Self {
        Self {
            rows: Vec::new(),
            env1,
            env2,
            is_large_response: false,
            total_size: 0,
            has_differences: false,
            summary: None,
        }
    }

    /// Create new body diff data for large responses
    pub fn new_large_response(
        env1: String,
        env2: String,
        total_size: usize,
        summary: BodyDiffSummary,
    ) -> Self {
        Self {
            rows: Vec::new(),
            env1,
            env2,
            is_large_response: true,
            total_size,
            has_differences: true, // Large responses always have differences if we're showing a summary
            summary: Some(summary),
        }
    }

    /// Add a diff row and update has_differences flag
    pub fn add_row(&mut self, row: DiffRow) {
        if row.operation != DiffOperation::Unchanged {
            self.has_differences = true;
        }
        self.rows.push(row);
    }

    /// Check if this body diff data is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.summary.is_none()
    }

    /// Set the total size of both responses
    pub fn set_total_size(&mut self, size: usize) {
        self.total_size = size;
    }
}

/// Combined diff data for a complete comparison result
#[derive(Debug, Clone)]
pub struct DiffData {
    /// Header differences (if headers comparison is enabled)
    pub headers: Option<HeaderDiffData>,
    /// Body differences
    pub body: Option<BodyDiffData>,
    /// Route name being compared
    pub route_name: String,
    /// Whether the overall comparison has any differences
    pub has_differences: bool,
}

impl DiffData {
    /// Create new diff data
    pub fn new(route_name: String) -> Self {
        Self {
            headers: None,
            body: None,
            route_name,
            has_differences: false,
        }
    }

    /// Set header diff data
    pub fn set_headers(&mut self, headers: HeaderDiffData) {
        if headers.has_differences {
            self.has_differences = true;
        }
        self.headers = Some(headers);
    }

    /// Set body diff data
    pub fn set_body(&mut self, body: BodyDiffData) {
        if body.has_differences {
            self.has_differences = true;
        }
        self.body = Some(body);
    }

    /// Check if this diff data has any content to display
    pub fn is_empty(&self) -> bool {
        self.headers.as_ref().map_or(true, |h| h.is_empty())
            && self.body.as_ref().map_or(true, |b| b.is_empty())
    }
}
