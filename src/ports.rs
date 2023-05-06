use std::error::Error;

use crate::semver::Language;

pub trait VCSOperations {
    fn create_branch(&self, branch_name: &str);
    fn checkout_branch(&self, branch_name: &str) -> Result<(), Box<dyn Error>>;
}

pub trait PackageOperations {
    fn increment_version(&self, version: &str, language: &Language);
}
