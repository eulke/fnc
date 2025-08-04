//! Shared utility modules
//! 
//! This module contains reusable utilities that are used across different
//! parts of the application.

pub mod text;

// Re-export commonly used functions
pub use text::{line_count, byte_size, truncate_lines, preview, are_identical};