use git2::Config as GitConfig;
use crate::error::Result;


pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

pub trait Config {
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