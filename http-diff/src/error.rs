use thiserror::Error;
use std::path::PathBuf;

/// Result type alias for http-diff operations
pub type Result<T> = std::result::Result<T, HttpDiffError>;

/// Comprehensive error types for HTTP diff operations
#[derive(Debug, Error)]
pub enum HttpDiffError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration parse error: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("CSV parse error: {0}")]
    CsvParse(#[from] csv::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("Invalid environment: {environment}")]
    InvalidEnvironment { environment: String },

    #[error("No environments configured")]
    NoEnvironments,

    #[error("Route '{route}' not found")]
    RouteNotFound { route: String },

    #[error("Path parameter '{param}' not found in user data")]
    MissingPathParameter { param: String },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Request execution failed: {message}")]
    RequestFailed { message: String },

    #[error("Response comparison failed: {message}")]
    ComparisonFailed { message: String },

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl HttpDiffError {
    /// Create a new invalid configuration error
    pub fn invalid_config<S: Into<String>>(message: S) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create a new request failed error
    pub fn request_failed<S: Into<String>>(message: S) -> Self {
        Self::RequestFailed {
            message: message.into(),
        }
    }

    /// Create a new comparison failed error
    pub fn comparison_failed<S: Into<String>>(message: S) -> Self {
        Self::ComparisonFailed {
            message: message.into(),
        }
    }
} 