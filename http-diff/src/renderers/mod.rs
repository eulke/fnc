//! Result renderers organized by output format
//! 
//! This module provides a clean way to render comparison results in different formats
//! while keeping the core business logic separate from presentation concerns.

use crate::types::ComparisonResult;

/// General trait for rendering comparison results in different formats
pub trait OutputRenderer {
    /// Render comparison results to a string in the specific format
    fn render(&self, results: &[ComparisonResult]) -> String;
}

// Output format modules
pub mod cli;

// Re-export main CLI renderer for convenience
pub use cli::CliRenderer;