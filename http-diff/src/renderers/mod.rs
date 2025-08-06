//! Result renderers organized by output format
//!
//! This module provides a clean way to render execution results in different formats
//! while keeping the core business logic separate from presentation concerns.

use crate::types::ExecutionResult;

/// General trait for rendering execution results in different formats
pub trait OutputRenderer {
    /// Render execution results to a string in the specific format
    fn render(&self, execution_result: &ExecutionResult) -> String;
}

// Core diff processing modules
pub mod diff_data;
pub mod diff_processor;

// Output format modules
pub mod cli;
pub mod report;
pub mod tui;

// Re-export main renderers for convenience
pub use cli::CliRenderer;
pub use report::{ReportMetadata, ReportRendererFactory};
pub use tui::{InteractiveRenderer, TuiRenderer};
