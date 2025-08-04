//! Result renderers for different output formats
//! 
//! This module provides a clean way to render comparison results in different formats
//! while keeping the core business logic separate from presentation concerns.

use crate::types::ComparisonResult;

/// Simple trait for rendering comparison results in different formats
pub trait OutputRenderer {
    /// Render comparison results to a string in the specific format
    fn render(&self, results: &[ComparisonResult]) -> String;
}

// Sub-modules
pub mod cli;
pub mod comparison_formatter;
pub mod table;

// Re-exports for convenience
pub use cli::CliRenderer;
pub use comparison_formatter::ComparisonFormatter;
pub use table::{TableBuilder, TableStyle};