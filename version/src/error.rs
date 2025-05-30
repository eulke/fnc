use std::result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VersionError {
    #[error("Failed to parse version: {0}")]
    ParseError(#[from] semver::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse file: {0}")]
    ParseFileError(String),

    #[error("Version not found in file")]
    VersionNotFound,

    #[error("Unsupported ecosystem")]
    UnsupportedEcosystem,

    #[error("No ecosystem detected")]
    NoEcosystemDetected,

    #[error("Other error: {0}")]
    Other(String),

    #[error("{0}: {1}")]
    WithContext(String, Box<VersionError>),
}

impl VersionError {
    /// Add context to an error
    #[must_use]
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        Self::WithContext(context.into(), Box::new(self))
    }

    /// Get a user-friendly message for command line display
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::ParseError(e) => format!("Invalid version format: {e}"),
            Self::NoEcosystemDetected => {
                "Could not detect project type. Supported project types: JavaScript, Rust, Python"
                    .to_string()
            }
            Self::VersionNotFound => "Could not find version in project files".to_string(),
            Self::WithContext(ctx, err) => format!("{}: {}", ctx, err.user_message()),
            _ => format!("{self}"),
        }
    }
}

pub type Result<T> = result::Result<T, VersionError>;

// Helper trait for adding context to results
pub trait ResultExt<T, E> {
    /// Add context to a Result
    ///
    /// # Errors
    ///
    /// Returns the original error wrapped with additional context
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T, E> ResultExt<T, E> for result::Result<T, E>
where
    E: Into<VersionError>,
{
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|err| {
            let version_err: VersionError = err.into();
            version_err.with_context(context())
        })
    }
}
