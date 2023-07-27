use std::error::Error;

pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

pub trait VCSOperations {
    fn new() -> Self;
    fn create_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>>;
    fn checkout_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>>;
    fn read_config(&self) -> Result<AuthorInfo, Box<dyn Error>>;
    fn validate_status(&self) -> Result<bool, Box<dyn Error>>;
    fn get_default_branch(&self) -> Result<String, Box<dyn Error>>;
    fn pull(&self) -> Result<(), Box<dyn Error>>;
}

pub trait PackageOperations {
    fn increment_pkg_version(
        &self,
        version: &str,
        author: &AuthorInfo,
    ) -> Result<(), Box<dyn Error>>;

    fn current_pkg_version(&self) -> String;
}

pub trait ChangelogOperations {
    fn write_version(version: &str, author: &AuthorInfo) -> Result<(), Box<dyn Error>>;
    fn read_version() -> String;
}
