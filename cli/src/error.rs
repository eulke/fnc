use semver::Error as SemverError;
use std::path::PathBuf;
use thiserror::Error;

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
        Self::WithContext(context.into(), Box::new(self))
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Io(err) => format!("I/O operation failed: {err}"),
            Self::Version(err) => err.user_message(),
            Self::Git(err) => err.user_message(),
            Self::PackageNotFound(path) => format!("Package not found at: {}", path.display()),
            Self::JsonParseError(err) => format!("Failed to parse JSON: {err}"),
            Self::GlobError(err) => format!("Invalid glob pattern: {err}"),
            Self::NoWorkspaces => "No workspaces found in package.json".to_string(),
            Self::RegexError(err) => format!("Invalid regular expression: {err}"),
            Self::SemverError(err) => format!("Invalid semantic version: {err}"),
            Self::AnyhowError(err) => format!("Error: {err}"),
            Self::DialoguerError(err) => format!("UI interaction error: {err}"),
            Self::Other(msg) => msg.clone(),
            Self::WithContext(ctx, err) => format!("{ctx}: {}", err.user_message()),
        }
    }
}

pub type Result<T> = std::result::Result<T, CliError>;

pub trait ResultExt<T, E> {
    #[allow(dead_code)]
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: Into<CliError>,
{
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|err| {
            let cli_err: CliError = err.into();
            cli_err.with_context(context())
        })
    }
}
