//! CLI-specific rendering utilities
//!
//! This module contains all rendering utilities that are specifically designed
//! for command-line interface output and terminal display.

pub mod comparison_formatter;
pub mod error_renderer;
pub mod renderer;
pub mod table;
pub mod text_formatter;

// Re-exports for CLI-specific functionality
pub use comparison_formatter::ComparisonFormatter;
pub use error_renderer::ErrorRenderer;
pub use renderer::CliRenderer;
pub use table::{TableBuilder, TableStyle};
pub use text_formatter::{FormatterConfig, TextFormatter};
