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

    #[error("Dialoguer error: {0}")]
    DialoguerError(#[from] dialoguer::Error),

    #[error("{0}")]
    Other(String),

    #[error("{0}: {1}")]
    WithContext(String, Box<CliError>),
}

impl CliError {
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        CliError::WithContext(context.into(), Box::new(self))
    }
    
    pub fn user_message(&self) -> String {
        match self {
            CliError::Io(err) => format!("I/O operation failed: {}", err),
            CliError::Version(err) => err.user_message(),
            CliError::Git(err) => err.user_message(),
            CliError::PackageNotFound(path) => format!("Package not found at: {}", path.display()),
            CliError::JsonParseError(err) => format!("Failed to parse JSON: {}", err),
            CliError::GlobError(err) => format!("Invalid glob pattern: {}", err),
            CliError::NoWorkspaces => "No workspaces found in package.json".to_string(),
            CliError::RegexError(err) => format!("Invalid regular expression: {}", err),
            CliError::SemverError(err) => format!("Invalid semantic version: {}", err),
            CliError::AnyhowError(err) => format!("Error: {}", err),
            CliError::DialoguerError(err) => format!("UI interaction error: {}", err),
            CliError::Other(msg) => msg.clone(),
            CliError::WithContext(ctx, err) => format!("{}: {}", ctx, err.user_message()),
        }
    }
}

pub type Result<T> = std::result::Result<T, CliError>;