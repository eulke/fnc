use std::error::Error;

use git2::Config as GitConfig;


pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

pub trait Config {
    fn read_config(&self) -> Result<AuthorInfo, Box<dyn Error>>;
}

pub struct RealGitConfig {}
impl Config for RealGitConfig {
    fn read_config(&self) -> Result<AuthorInfo, Box<dyn Error>> {
        let config = GitConfig::open_default()?;
        let name = config.get_string("user.name")?;
        let email = config.get_string("user.email")?;

        Ok(AuthorInfo { name, email })
    }
}