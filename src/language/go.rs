use crate::ports::{AuthorInfo, ChangelogOperations, PackageOperations};
use std::error::Error;

use super::common::Changelog;

pub struct Go;

impl PackageOperations for Go {
    fn increment_pkg_version(
        &self,
        version: &str,
        author: &AuthorInfo,
    ) -> Result<(), Box<dyn Error>> {
        Changelog::write_version(version, author)
    }

    fn current_pkg_version(&self) -> String {
        Changelog::read_version()
    }
}
