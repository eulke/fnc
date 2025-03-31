use thiserror::Error;
use std::path::PathBuf;
use semver::Error as SemverError;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Version error: {0}")]
    Version(#[from] version::VersionError),

    #[error("Git error: {0}")]
    Git(#[from] git::error::GitError),

    #[error("Package not found at path: {0}")]
    PackageNotFound(PathBuf),

    #[error("Failed to parse package.json: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Glob pattern error: {0}")]
    GlobError(#[from] glob::PatternError),

    #[error("No workspaces found in package.json")]
    NoWorkspaces,
    
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("Semver parse error: {0}")]
    SemverError(#[from] SemverError),
    
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),

    #[error("{0}")]
    Other(String),

    #[error("{0}: {1}")]
    WithContext(String, Box<CliError>),
}

impl CliError {
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        CliError::WithContext(context.into(), Box::new(self))
    }
}

pub type Result<T> = std::result::Result<T, CliError>;