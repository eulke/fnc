/// Configuration options for changelog formatting and behavior
#[derive(Debug, Clone)]
pub struct ChangelogConfig {
    pub date_format: String,
    pub version_header_format: String,
    pub category_order: Vec<String>,
    pub default_categories: Vec<String>,
    pub ignore_duplicates: bool,
    pub verbose: bool,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            date_format: "%Y-%m-%d".to_string(),
            version_header_format: "## [{0}] {1} _{2}_".to_string(),
            category_order: vec![
                "Added".to_string(),
                "Changed".to_string(),
                "Fixed".to_string(),
                "Deprecated".to_string(),
                "Removed".to_string(),
                "Security".to_string(),
            ],
            default_categories: vec!["Added".to_string()],
            ignore_duplicates: false,
            verbose: false,
        }
    }
}
