use crate::error::Result;
use git2::Config as GitConfig;

pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

pub trait Config {
    /// Reads the git configuration to retrieve author information
    ///
    /// # Errors
    ///
    /// Returns an error if the git configuration cannot be read or if required fields are missing
    fn read_config() -> Result<AuthorInfo>;
}

pub struct RealGitConfig {}
impl Config for RealGitConfig {
    fn read_config() -> Result<AuthorInfo> {
        let config = GitConfig::open_default()?;
        let name = config.get_string("user.name")?;
        let email = config.get_string("user.email")?;

        Ok(AuthorInfo { name, email })
    }
}
