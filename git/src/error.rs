use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git2 error: {0}")]
    Git2Error(#[from] git2::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
    
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
            GitError::Git2Error(e) => format!("Git operation failed: {}", e),
            GitError::BranchNotFound(branch) => format!("Git branch '{}' not found", branch),
            GitError::WithContext(ctx, err) => format!("{}: {}", ctx, err.user_message()),
            _ => format!("{}", self),
        }
    }
}

pub type Result<T> = std::result::Result<T, GitError>;

// Helper trait for adding context to results
pub trait ResultExt<T, E> {
    fn with_context<C, F>(self, context: F) -> std::result::Result<T, GitError>
    where
        C: Into<String>,
        F: FnOnce() -> C;
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
}
