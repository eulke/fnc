//! HTML report renderer for executive summary reports

use super::{ReportMetadata, ReportRenderer};
use crate::types::ComparisonResult;

mod components;
mod templates;

pub use components::HtmlComponents;
pub use templates::HtmlTemplate;

/// HTML report renderer that generates professional, self-contained HTML reports
pub struct HtmlReportRenderer {
    /// Whether to include detailed technical information
    pub include_technical_details: bool,
}

impl HtmlReportRenderer {
    /// Create a new HTML report renderer with default settings
    pub fn new() -> Self {
        Self {
            include_technical_details: true,
        }
    }

    /// Create an HTML renderer for executive summary (minimal technical details)
    pub fn executive_summary() -> Self {
        Self {
            include_technical_details: false,
        }
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
        template.render(results, metadata, self.include_technical_details)
    }

    fn supported_extension(&self) -> &'static str {
        "html"
    }

    fn mime_type(&self) -> &'static str {
        "text/html"
    }
}
