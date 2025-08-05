//! Executive summary report renderers for multiple output formats
//! 
//! This module provides a generic interface for generating professional reports
//! that can be shared with executives and team members. It supports multiple
//! output formats through an extensible architecture.

use crate::types::ComparisonResult;
use std::path::Path;

/// Trait for rendering executive summary reports in different formats
pub trait ReportRenderer {
    /// Render comparison results to a formatted report string
    fn render_report(&self, results: &[ComparisonResult], metadata: &ReportMetadata) -> String;
    
    /// Get the file extension this renderer supports
    fn supported_extension(&self) -> &'static str;
    
    /// Get the MIME type for this format
    fn mime_type(&self) -> &'static str;
}

/// Metadata for report generation
#[derive(Debug, Clone)]
pub struct ReportMetadata {
    /// When the test was executed
    pub timestamp: chrono::DateTime<chrono::Local>,
    /// How long the test took to run
    pub execution_duration: std::time::Duration,
    /// Environments tested
    pub environments: Vec<String>,
    /// Total number of routes tested
    pub total_routes: usize,
    /// Any additional context
    pub context: std::collections::HashMap<String, String>,
}

impl ReportMetadata {
    /// Create new report metadata with current timestamp
    pub fn new(environments: Vec<String>, total_routes: usize) -> Self {
        Self {
            timestamp: chrono::Local::now(),
            execution_duration: std::time::Duration::from_secs(0),
            environments,
            total_routes,
            context: std::collections::HashMap::new(),
        }
    }
    
    /// Set execution duration
    pub fn with_duration(mut self, duration: std::time::Duration) -> Self {
        self.execution_duration = duration;
        self
    }
    
    /// Add context information
    pub fn with_context<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Main report renderer that auto-detects format from file extension
pub struct ReportRendererFactory;

impl ReportRendererFactory {
    /// Create appropriate renderer based on file extension
    pub fn create_renderer<P: AsRef<Path>>(file_path: P) -> Box<dyn ReportRenderer> {
        let path = file_path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("html") // Default to HTML
            .to_lowercase();
        
        match extension.as_str() {
            "html" | "htm" => Box::new(html::HtmlReportRenderer::new()),
            // Future formats can be added here:
            // "pdf" => Box::new(pdf::PdfReportRenderer::new()),
            // "json" => Box::new(json::JsonReportRenderer::new()),
            _ => {
                // Default to HTML for unknown extensions
                eprintln!("Warning: Unknown report format '{}', defaulting to HTML", extension);
                Box::new(html::HtmlReportRenderer::new())
            }
        }
    }
    
    /// Get all supported formats
    pub fn supported_formats() -> Vec<&'static str> {
        vec!["html", "htm"]
        // Future: vec!["html", "htm", "pdf", "json"]
    }
}

// Format-specific implementations
pub mod html;

// Re-export main types for convenience
pub use html::HtmlReportRenderer;