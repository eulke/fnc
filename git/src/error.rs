use thiserror::Error;

/// Git operation error type that provides detailed context about the error
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git2 error: {0}")]
    Git2Error(#[from] git2::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Branch error: {0}")]
    BranchError(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Failed to execute git command: {0}")]
    CommandError(String),

    #[error("UTF-8 encoding error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Other error: {0}")]
    Other(String),

    #[error("{0}: {1}")]
    WithContext(String, Box<GitError>),
}

impl GitError {
    /// Add context to an error
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        GitError::WithContext(context.into(), Box::new(self))
    }

    /// Get a user-friendly message for command line display
    pub fn user_message(&self) -> String {
        match self {
            GitError::Git2Error(e) => {
                let msg = format!("{}", e);
                // Extract just the message without the class and code details
                msg.split(';').next().map_or_else(
                    || format!("Git error: {}", msg),
                    |main_msg| format!("Git error: {}", main_msg.trim()),
                )
            }
            GitError::IoError(e) => format!("I/O error: {}", e),
            GitError::BranchNotFound(branch) => format!("Branch '{}' not found", branch),
            GitError::BranchError(msg) => format!("Branch operation failed: {}", msg),
            GitError::RepositoryError(msg) => format!("Repository error: {}", msg),
            GitError::ConfigError(msg) => format!("Git config error: {}", msg),
            GitError::CommandError(msg) => format!("Git command failed: {}", msg),
            GitError::Utf8Error(e) => format!("Text encoding error: {}", e),
            GitError::Other(msg) => msg.clone(),
            GitError::WithContext(ctx, err) => format!("{}: {}", ctx, err.user_message()),
        }
    }
}

pub type Result<T> = std::result::Result<T, GitError>;

/// Helper trait for adding context to results
pub trait ResultExt<T, E> {
    /// Add context to an error result with a string or string-producing closure
    fn with_context<C, F>(self, context: F) -> std::result::Result<T, GitError>
    where
        C: Into<String>,
        F: FnOnce() -> C;

    /// Add context directly from a string
    fn context<C: Into<String>>(self, context: C) -> std::result::Result<T, GitError>;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: Into<GitError>,
{
    fn with_context<C, F>(self, context: F) -> std::result::Result<T, GitError>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|err| {
            let git_err: GitError = err.into();
            git_err.with_context(context())
        })
    }

    fn context<C: Into<String>>(self, context: C) -> std::result::Result<T, GitError> {
        self.map_err(|err| {
            let git_err: GitError = err.into();
            git_err.with_context(context)
        })
    }
}
