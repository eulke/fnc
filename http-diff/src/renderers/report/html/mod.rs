//! HTML report renderer for executive summary reports

use super::{ReportMetadata, ReportRenderer};
use crate::types::ComparisonResult;

mod components;
mod diff_renderer;
mod json_diff_renderer;
mod templates;

pub use components::HtmlComponents;
pub use diff_renderer::HtmlDiffRenderer;
pub use json_diff_renderer::JsonDiffRenderer;
pub use templates::HtmlTemplate;

/// Detail level for diff rendering in HTML reports
#[derive(Debug, Clone, PartialEq)]
pub enum DiffDetailLevel {
    /// Executive summary only (no diffs)
    Executive,
    /// Basic diffs with summary (limited routes)
    Basic,
    /// Detailed diffs with full content
    Detailed,
}

/// HTML report renderer that generates professional, self-contained HTML reports
pub struct HtmlReportRenderer {
    /// Whether to include detailed technical information
    pub include_technical_details: bool,
    /// Level of diff detail to include
    pub diff_detail_level: DiffDetailLevel,
    /// Maximum number of routes to show diffs for (None = all)
    pub max_diff_routes: Option<usize>,
    /// Whether to show unchanged lines in detailed diffs
    pub show_unchanged_lines: bool,
}

impl HtmlReportRenderer {
    /// Create a new HTML report renderer with default settings
    pub fn new() -> Self {
        Self {
            include_technical_details: true,
            diff_detail_level: DiffDetailLevel::Basic,
            max_diff_routes: None, // Show all diffs - no artificial limits
            show_unchanged_lines: false,
        }
    }

    /// Create an HTML renderer for executive summary (minimal technical details)
    pub fn executive_summary() -> Self {
        Self {
            include_technical_details: false,
            diff_detail_level: DiffDetailLevel::Executive,
            max_diff_routes: None,
            show_unchanged_lines: false,
        }
    }

    /// Create an HTML renderer with detailed diffs
    pub fn with_detailed_diffs() -> Self {
        Self {
            include_technical_details: true,
            diff_detail_level: DiffDetailLevel::Detailed,
            max_diff_routes: None,
            show_unchanged_lines: true,
        }
    }

    /// Set the diff detail level
    pub fn with_diff_level(mut self, level: DiffDetailLevel) -> Self {
        self.diff_detail_level = level;
        self
    }

    /// Set the maximum number of routes to show diffs for
    pub fn with_max_diff_routes(mut self, max: Option<usize>) -> Self {
        self.max_diff_routes = max;
        self
    }

    /// Set whether to show unchanged lines in diffs
    pub fn with_show_unchanged_lines(mut self, show: bool) -> Self {
        self.show_unchanged_lines = show;
        self
    }
}

impl Default for HtmlReportRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportRenderer for HtmlReportRenderer {
    fn render_report(&self, results: &[ComparisonResult], metadata: &ReportMetadata) -> String {
        let template = HtmlTemplate::new();
        template.render(
            results,
            metadata,
            self.include_technical_details,
            &self.diff_detail_level,
            self.max_diff_routes,
            self.show_unchanged_lines,
        )
    }

    fn supported_extension(&self) -> &'static str {
        "html"
    }

    fn mime_type(&self) -> &'static str {
        "text/html"
    }
}
