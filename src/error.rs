use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeployError {
    #[error("VCS status check failed: {0}")]
    VCSStatusError(String),

    #[error("Branch operation failed: {0}")]
    BranchError(String),

    #[error("Remote operation failed: {0}")]
    RemoteError(String),

    #[error("Version update failed: {0}")]
    VersionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, DeployError>;
