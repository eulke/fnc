//! CLI-specific rendering utilities
//!
//! This module contains rendering utilities that are specifically designed
//! for command-line interface output and terminal display.

pub mod renderer;
pub mod text_formatter;

// Re-exports for CLI-specific functionality
pub use renderer::CliRenderer;
pub use text_formatter::{TextFormatter, FormatterConfig, DiffStyle};