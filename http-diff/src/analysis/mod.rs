//! Error analysis and classification - pure business logic only
//!
//! This module provides functionality to analyze and classify error information
//! from failed HTTP requests without any presentation concerns.

pub mod error_classifier;

pub use error_classifier::{
    ErrorAnalysis, ErrorAnalyzer, ErrorClassifierImpl, ErrorGroup, RouteError,
};
