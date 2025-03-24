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
}

pub type Result<T> = std::result::Result<T, GitError>;
