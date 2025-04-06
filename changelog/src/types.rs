use crate::error::ChangelogError;
use std::collections::HashMap;

/// Type alias for Result with `ChangelogError`
pub type Result<T> = std::result::Result<T, ChangelogError>;

/// Map of sections in changelog, organized by version and category
pub type ChangelogSections = HashMap<String, HashMap<String, Vec<String>>>;

/// An entry in the changelog to be moved
pub struct ChangelogEntry {
    pub content: String,
    pub category: String,
}
