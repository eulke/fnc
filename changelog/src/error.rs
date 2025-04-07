use std::fmt::{self, Display, Formatter};
use thiserror::Error;

/// Error context to enrich error messages
#[derive(Debug)]
pub struct ErrorContext {
    pub operation: String,
    pub source: Option<String>,
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Operation: {}", self.operation)?;
        if let Some(source) = &self.source {
            write!(f, " (source: {})", source)?;
        }
        Ok(())
    }
}

/// Errors that can occur when working with changelogs
#[derive(Error, Debug)]
pub enum ChangelogError {
    #[error("Failed to read or write changelog file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse changelog: {0}")]
    ParseError(String),

    #[error("Failed to find version section in changelog")]
    MissingVersionSection,

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Git operation failed: {0}")]
    Git(String),

    #[error("Invalid changelog format at line {0}: {1}")]
    InvalidFormat(usize, String),

    #[error("Duplicate category {0} in version {1}")]
    DuplicateCategory(String, String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("{0}")]
    Other(String),

    #[error("{0}: {1}")]
    WithContext(String, Box<ChangelogError>),

    #[error("{context}: {source}")]
    ContextualError {
        context: ErrorContext,
        source: Box<ChangelogError>,
    },
}

impl ChangelogError {
    #[must_use]
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        Self::WithContext(context.into(), Box::new(self))
    }

    #[must_use]
    pub fn with_operation_context(
        self,
        operation: impl Into<String>,
        source: Option<impl Into<String>>,
    ) -> Self {
        Self::ContextualError {
            context: ErrorContext {
                operation: operation.into(),
                source: source.map(Into::into),
            },
            source: Box::new(self),
        }
    }

    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::ReadError(e) => format!("File operation failed: {e}"),
            Self::ParseError(msg) => format!("Failed to parse changelog: {msg}"),
            Self::MissingVersionSection => {
                "Failed to find version section in changelog".to_string()
            }
            Self::InvalidVersion(ver) => format!("Invalid version format: {ver}"),
            Self::Git(msg) => format!("Git operation failed: {msg}"),
            Self::InvalidFormat(line, msg) => {
                format!("Invalid changelog format at line {line}: {msg}")
            }
            Self::DuplicateCategory(cat, ver) => {
                format!("Duplicate category {cat} in version {ver}")
            }
            Self::RegexError(e) => format!("Regular expression error: {e}"),
            Self::Other(msg) => msg.clone(),
            Self::WithContext(ctx, err) => format!("{ctx}: {}", err.user_message()),
            Self::ContextualError { context, source } => {
                format!("{}: {}", context, source.user_message())
            }
        }
    }
}
