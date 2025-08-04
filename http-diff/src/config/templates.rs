use crate::error::{HttpDiffError, Result};

/// Generate default http-diff.toml template with examples
pub fn generate_default_config_template() -> String {
    r#"# HTTP Diff Configuration
# This file defines environments, routes, and global settings for HTTP diff testing

# Environment definitions - you can add as many as needed
[environments.test]
base_url = "https://api-test.example.com"
headers."X-Scope" = "test"
headers."X-Environment" = "testing"

[environments.prod]
base_url = "https://api.example.com"
headers."X-Scope" = "prod"

[environments.staging]
base_url = "https://api-staging.example.com"
headers."X-Scope" = "staging"

# Global configuration settings (optional)
[global]
# Request timeout in seconds
timeout_seconds = 30
# Whether to follow HTTP redirects
follow_redirects = true

# Global headers applied to all requests
[global.headers]
"User-Agent" = "fnc-http-diff/1.0"
"Accept" = "application/json"
"Content-Type" = "application/json"

# Global query parameters applied to all requests
[global.params]
version = "v1"

# Route definitions - define your API endpoints here
[[routes]]
name = "user-profile"
method = "GET"
path = "/api/users/{userId}"

# Route-specific headers (optional)
[routes.headers]
"Accept" = "application/json"

# Route-specific query parameters (optional)
[routes.params]
include_metadata = "true"

# Per-environment base URL overrides (optional)
# Useful for legacy services or microservices with different domains
# [routes.base_urls]
# test = "https://legacy-test.example.com"
# prod = "https://legacy.example.com"

[[routes]]
name = "site-info"
method = "GET"
path = "/api/sites/{siteId}"

[[routes]]
name = "health-check"
method = "GET"
path = "/health"

# Example POST route with body
[[routes]]
name = "create-user"
method = "POST"
path = "/api/users"
body = '{"name": "Test User", "email": "test@example.com"}'

[routes.headers]
"Content-Type" = "application/json"
"#.to_string()
}

/// Generate default users.csv template
pub fn generate_default_users_csv() -> String {
    r"userId,siteId
745741037,MCO
85264518,MLA
123456789,MLB
987654321,MCO
555666777,MLA
".to_string()
}

/// Check if configuration files exist and optionally generate them
pub fn ensure_config_files_exist(
    config_path: &str,
    csv_path: &str,
    force_generate: bool,
) -> Result<(bool, bool)> {
    use std::path::Path;
    use std::fs;

    let config_exists = Path::new(config_path).exists();
    let csv_exists = Path::new(csv_path).exists();

    let mut config_generated = false;
    let mut csv_generated = false;

    // Generate config file if it doesn't exist
    if !config_exists && force_generate {
        let template = generate_default_config_template();
        fs::write(config_path, template)
            .map_err(HttpDiffError::Io)?;
        config_generated = true;
    }

    // Generate CSV file if it doesn't exist
    if !csv_exists && force_generate {
        let template = generate_default_users_csv();
        fs::write(csv_path, template)
            .map_err(HttpDiffError::Io)?;
        csv_generated = true;
    }

    Ok((config_generated, csv_generated))
}