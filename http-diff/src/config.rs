use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use crate::error::{HttpDiffError, Result};

/// Main configuration structure for HTTP diff testing
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpDiffConfig {
    /// Environment configurations
    pub environments: HashMap<String, Environment>,
    /// Global configuration settings
    pub global: Option<GlobalConfig>,
    /// Route definitions
    pub routes: Vec<Route>,
}

/// Environment configuration with base URL and headers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Environment {
    /// Base URL for this environment
    pub base_url: String,
    /// Environment-specific headers
    pub headers: Option<HashMap<String, String>>,
}

/// Global configuration settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Whether to follow redirects
    pub follow_redirects: Option<bool>,
    /// Global headers applied to all requests
    pub headers: Option<HashMap<String, String>>,
    /// Global query parameters applied to all requests
    pub params: Option<HashMap<String, String>>,
}

/// Route definition for HTTP requests
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    /// Unique name for this route
    pub name: String,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Path with optional parameters like /api/users/{userId}
    pub path: String,
    /// Route-specific headers
    pub headers: Option<HashMap<String, String>>,
    /// Route-specific query parameters
    pub params: Option<HashMap<String, String>>,
    /// Per-environment base URL overrides
    pub base_urls: Option<HashMap<String, String>>,
    /// Request body for POST/PUT requests
    pub body: Option<String>,
}

/// User data loaded from CSV for parameter substitution
#[derive(Debug, Clone)]
pub struct UserData {
    /// CSV column data
    pub data: HashMap<String, String>,
}

impl UserData {
    /// Substitute placeholders like {userId} with actual values from CSV data
    /// 
    /// # Arguments
    /// * `text` - The text containing placeholders in {param_name} format
    /// * `url_encode` - Whether to URL encode the substituted values (true for paths, false for headers/body)
    /// * `strict` - If true, error on missing parameters; if false, leave unmatched placeholders unchanged
    pub fn substitute_placeholders(&self, text: &str, url_encode: bool, strict: bool) -> Result<String> {
        let mut result = text.to_string();
        let mut position = 0;
        
        // Find and replace all valid {parameter} placeholders
        while let Some(start) = result[position..].find('{') {
            let start = position + start;
            
            if let Some(relative_end) = result[start..].find('}') {
                let end = start + relative_end;
                let param_name = &result[start + 1..end];
                

                
                // Check if this looks like a valid parameter placeholder
                if is_valid_param_name(param_name) {
                    if let Some(value) = self.data.get(param_name) {
                        // We have the parameter value, substitute it
                        let final_value = if url_encode {
                            urlencoding::encode(value).to_string()
                        } else {
                            value.clone()
                        };
                        

                        result.replace_range(start..end + 1, &final_value);
                        // Continue from after the replacement
                        position = start + final_value.len();
                    } else if strict {
                        // Strict mode: error if parameter is missing
                        return Err(HttpDiffError::MissingPathParameter {
                            param: param_name.to_string(),
                        });
                    } else {
                        // Non-strict mode: skip this placeholder and continue searching after it

                        position = end + 1;
                    }
                } else {
                    // Not a valid parameter name, skip this placeholder

                    position = end + 1;
                }
            } else {
                // No closing brace found, stop searching
                break;
            }
        }
        
        Ok(result)
    }
}

impl HttpDiffConfig {
    /// Load configuration from http-diff.toml file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .map_err(|_| HttpDiffError::ConfigNotFound {
                path: path.as_ref().to_path_buf(),
            })?;
        
        let config: HttpDiffConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.environments.is_empty() {
            return Err(HttpDiffError::NoEnvironments);
        }

        if self.routes.is_empty() {
            return Err(HttpDiffError::invalid_config("No routes configured"));
        }

        // Validate that route base_url overrides reference valid environments
        for route in &self.routes {
            if let Some(base_urls) = &route.base_urls {
                for env_name in base_urls.keys() {
                    if !self.environments.contains_key(env_name) {
                        return Err(HttpDiffError::InvalidEnvironment {
                            environment: env_name.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the base URL for a route in a specific environment
    pub fn get_base_url(&self, route: &Route, environment: &str) -> Result<String> {
        // First check if route has environment-specific override
        if let Some(base_urls) = &route.base_urls {
            if let Some(url) = base_urls.get(environment) {
                return Ok(url.clone());
            }
        }

        // Fall back to environment default
        self.environments
            .get(environment)
            .map(|env| env.base_url.clone())
            .ok_or_else(|| HttpDiffError::InvalidEnvironment {
                environment: environment.to_string(),
            })
    }
}

/// Load user data from CSV file
pub fn load_user_data<P: AsRef<Path>>(path: P) -> Result<Vec<UserData>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    
    let mut users = Vec::new();
    for result in reader.records() {
        let record = result?;
        let mut data = HashMap::new();
        
        for (i, header) in headers.iter().enumerate() {
            if let Some(value) = record.get(i) {
                data.insert(header.to_string(), value.to_string());
            }
        }
        
        users.push(UserData { data });
    }
    
    Ok(users)
}

/// Check if a parameter name is a valid identifier (letters, numbers, underscore)
fn is_valid_param_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

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
    r#"userId,siteId
745741037,MCO
85264518,MLA
123456789,MLB
987654321,MCO
555666777,MLA
"#.to_string()
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

/// Enhanced configuration loading with validation and helpful error messages
impl HttpDiffConfig {
    /// Load configuration with enhanced error context
    pub fn load_with_validation<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        
        // Check if file exists and provide helpful error message
        if !path_ref.exists() {
            return Err(HttpDiffError::ConfigNotFound {
                path: path_ref.to_path_buf(),
            });
        }

        let content = std::fs::read_to_string(path_ref)
            .map_err(HttpDiffError::Io)?;

        // Parse TOML with enhanced error context
        let config: HttpDiffConfig = toml::from_str(&content)
            .map_err(|e| {
                HttpDiffError::invalid_config(format!(
                    "Failed to parse TOML in {}: {}",
                    path_ref.display(),
                    e
                ))
            })?;

        // Validate configuration
        config.validate_with_context(path_ref)?;

        Ok(config)
    }

    /// Validation with enhanced error context
    fn validate_with_context<P: AsRef<Path>>(&self, config_path: P) -> Result<()> {
        if self.environments.is_empty() {
            return Err(HttpDiffError::invalid_config(format!(
                "No environments configured in {}. Add at least one environment to [environments] section.",
                config_path.as_ref().display()
            )));
        }

        if self.routes.is_empty() {
            return Err(HttpDiffError::invalid_config(format!(
                "No routes configured in {}. Add at least one [[routes]] entry.",
                config_path.as_ref().display()
            )));
        }

        // Validate HTTP methods
        for route in &self.routes {
            let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
            if !valid_methods.contains(&route.method.as_str()) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Invalid HTTP method '{}' in route '{}'. Valid methods: {}",
                    route.method,
                    route.name,
                    valid_methods.join(", ")
                )));
            }

            // Validate base URL overrides reference existing environments
            if let Some(base_urls) = &route.base_urls {
                for env_name in base_urls.keys() {
                    if !self.environments.contains_key(env_name) {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Route '{}' references unknown environment '{}' in base_urls. Available environments: {}",
                            route.name,
                            env_name,
                            self.environments.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                        )));
                    }
                }
            }

            // Validate URLs in environments and route overrides
            for (env_name, env) in &self.environments {
                if url::Url::parse(&env.base_url).is_err() {
                    return Err(HttpDiffError::invalid_config(format!(
                        "Invalid base_url '{}' in environment '{}'. Must be a valid URL.",
                        env.base_url,
                        env_name
                    )));
                }
            }
        }

        // Validate timeout if specified
        if let Some(global) = &self.global {
            if let Some(timeout) = global.timeout_seconds {
                if timeout == 0 || timeout > 300 {
                    return Err(HttpDiffError::invalid_config(
                        "timeout_seconds must be between 1 and 300 seconds".to_string()
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_validation() {
        let mut config = HttpDiffConfig {
            environments: HashMap::new(),
            global: None,
            routes: vec![],
        };

        // Should fail with no environments
        assert!(config.validate().is_err());

        // Add environment
        config.environments.insert(
            "test".to_string(),
            Environment {
                base_url: "https://test.example.com".to_string(),
                headers: None,
            },
        );

        // Should fail with no routes
        assert!(config.validate().is_err());

        // Add route
        config.routes.push(Route {
            name: "test-route".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
        });

        // Should pass validation
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_with_invalid_base_url_overrides() {
        let mut environments = HashMap::new();
        environments.insert(
            "test".to_string(),
            Environment {
                base_url: "https://test.example.com".to_string(),
                headers: None,
            },
        );

        // Route with base_url override referencing non-existent environment
        let mut base_urls = HashMap::new();
        base_urls.insert("nonexistent".to_string(), "https://legacy.example.com".to_string());

        let routes = vec![Route {
            name: "invalid-route".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            headers: None,
            params: None,
            base_urls: Some(base_urls),
            body: None,
        }];

        let config = HttpDiffConfig {
            environments,
            global: None,
            routes,
        };

        // Should fail validation due to invalid environment reference
        assert!(config.validate().is_err());
        
        if let Err(HttpDiffError::InvalidEnvironment { environment }) = config.validate() {
            assert_eq!(environment, "nonexistent");
        } else {
            panic!("Expected InvalidEnvironment error");
        }
    }

    #[test]
    fn test_base_url_resolution() {
        let mut environments = HashMap::new();
        environments.insert(
            "test".to_string(),
            Environment {
                base_url: "https://test.example.com".to_string(),
                headers: None,
            },
        );

        let config = HttpDiffConfig {
            environments,
            global: None,
            routes: vec![],
        };

        // Test route without override
        let route_no_override = Route {
            name: "normal".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
        };

        assert_eq!(
            config.get_base_url(&route_no_override, "test").unwrap(),
            "https://test.example.com"
        );

        // Test route with override
        let mut base_urls = HashMap::new();
        base_urls.insert("test".to_string(), "https://legacy-test.example.com".to_string());

        let route_with_override = Route {
            name: "legacy".to_string(),
            method: "GET".to_string(),
            path: "/api/legacy".to_string(),
            headers: None,
            params: None,
            base_urls: Some(base_urls),
            body: None,
        };

        assert_eq!(
            config.get_base_url(&route_with_override, "test").unwrap(),
            "https://legacy-test.example.com"
        );

        // Test with non-existent environment
        assert!(config.get_base_url(&route_no_override, "nonexistent").is_err());
    }

    #[test]
    fn test_toml_parsing_valid_minimal_config() {
        let toml_content = r#"
[environments.test]
base_url = "https://api-test.example.com"

[environments.prod]
base_url = "https://api.example.com"

[[routes]]
name = "health-check"
method = "GET"
path = "/health"
"#;

        let config: HttpDiffConfig = toml::from_str(toml_content).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.environments.len(), 2);
        assert_eq!(config.routes.len(), 1);
        assert!(config.global.is_none());
    }

    #[test]
    fn test_toml_parsing_full_config() {
        let toml_content = r#"
[environments.test]
base_url = "https://api-test.example.com"
headers."X-Scope" = "test"
headers."X-Environment" = "testing"

[environments.prod]
base_url = "https://api.example.com"
headers."X-Scope" = "prod"

[global]
timeout_seconds = 45
follow_redirects = false
headers."User-Agent" = "fnc-http-diff/1.0"
headers."Accept" = "application/json"
params.version = "v2"
params.format = "json"

[[routes]]
name = "user-profile"
method = "GET"
path = "/api/users/{userId}"
headers."Authorization" = "Bearer {token}"
params.include_metadata = "true"

[routes.base_urls]
test = "https://legacy-test.example.com"

[[routes]]
name = "create-user"
method = "POST"
path = "/api/users"
body = '{"name": "test", "email": "test@example.com"}'
"#;

        let config: HttpDiffConfig = toml::from_str(toml_content).unwrap();
        assert!(config.validate().is_ok());
        
        // Validate environments
        assert_eq!(config.environments.len(), 2);
        assert!(config.environments.get("test").unwrap().headers.is_some());
        assert_eq!(
            config.environments.get("test").unwrap().headers.as_ref().unwrap().get("X-Scope"),
            Some(&"test".to_string())
        );
        
        // Validate global config
        assert!(config.global.is_some());
        let global = config.global.as_ref().unwrap();
        assert_eq!(global.timeout_seconds, Some(45));
        assert_eq!(global.follow_redirects, Some(false));
        assert!(global.headers.is_some());
        assert!(global.params.is_some());
        
        // Validate routes
        assert_eq!(config.routes.len(), 2);
        let user_profile_route = &config.routes[0];
        assert_eq!(user_profile_route.name, "user-profile");
        assert_eq!(user_profile_route.method, "GET");
        assert!(user_profile_route.base_urls.is_some());
        
        let create_user_route = &config.routes[1];
        assert_eq!(create_user_route.method, "POST");
        assert!(create_user_route.body.is_some());
    }

    #[test]
    fn test_toml_parsing_invalid_config() {
        // Missing required fields
        let invalid_toml = r#"
[environments.test]
# Missing base_url

[[routes]]
name = "test"
# Missing method and path
"#;

        let result = toml::from_str::<HttpDiffConfig>(invalid_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("http-diff.toml");
        
        let toml_content = r#"
[environments.test]
base_url = "https://api-test.example.com"

[[routes]]
name = "test-route"
method = "GET"
path = "/api/test"
"#;
        
        fs::write(&config_path, toml_content).unwrap();
        
        let config = HttpDiffConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.environments.len(), 1);
        assert_eq!(config.routes.len(), 1);
    }

    #[test]
    fn test_config_load_from_nonexistent_file() {
        let result = HttpDiffConfig::load_from_file("nonexistent.toml");
        assert!(result.is_err());
        
        if let Err(HttpDiffError::ConfigNotFound { path }) = result {
            assert_eq!(path.to_string_lossy(), "nonexistent.toml");
        } else {
            panic!("Expected ConfigNotFound error");
        }
    }

    #[test]
    fn test_csv_parsing_valid_data() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("users.csv");
        
        let csv_content = "userId,siteId,country\n745741037,MCO,Colombia\n85264518,MLA,Argentina\n123456789,MLB,Brazil";
        fs::write(&csv_path, csv_content).unwrap();
        
        let users = load_user_data(&csv_path).unwrap();
        assert_eq!(users.len(), 3);
        
        let first_user = &users[0];
        assert_eq!(first_user.data.get("userId"), Some(&"745741037".to_string()));
        assert_eq!(first_user.data.get("siteId"), Some(&"MCO".to_string()));
        assert_eq!(first_user.data.get("country"), Some(&"Colombia".to_string()));
    }

    #[test]
    fn test_csv_parsing_different_columns() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");
        
        let csv_content = "id,name,email\n1,Alice,alice@example.com\n2,Bob,bob@example.com";
        fs::write(&csv_path, csv_content).unwrap();
        
        let users = load_user_data(&csv_path).unwrap();
        assert_eq!(users.len(), 2);
        
        let first_user = &users[0];
        assert_eq!(first_user.data.get("id"), Some(&"1".to_string()));
        assert_eq!(first_user.data.get("name"), Some(&"Alice".to_string()));
        assert_eq!(first_user.data.get("email"), Some(&"alice@example.com".to_string()));
    }

    #[test]
    fn test_csv_parsing_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("empty.csv");
        
        let csv_content = "userId,siteId\n"; // Header only, no data
        fs::write(&csv_path, csv_content).unwrap();
        
        let users = load_user_data(&csv_path).unwrap();
        assert_eq!(users.len(), 0);
    }

    #[test]
    fn test_csv_parsing_malformed_data() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("malformed.csv");
        
        // CSV with missing field in second row - this should cause an error
        let csv_content = "userId,siteId\n123,MCO\n456"; // Missing field in second row
        fs::write(&csv_path, csv_content).unwrap();
        
        // The CSV crate is strict about field count, so this should fail
        let result = load_user_data(&csv_path);
        assert!(result.is_err());
        
        // Test with properly padded but empty fields instead
        let csv_content_padded = "userId,siteId\n123,MCO\n456,"; // Empty field but correct count
        fs::write(&csv_path, csv_content_padded).unwrap();
        
        let users = load_user_data(&csv_path).unwrap();
        assert_eq!(users.len(), 2);
        
        let second_user = &users[1];
        assert_eq!(second_user.data.get("userId"), Some(&"456".to_string()));
        assert_eq!(second_user.data.get("siteId"), Some(&"".to_string())); // Empty string
    }

    #[test]
    fn test_csv_parsing_nonexistent_file() {
        let result = load_user_data("nonexistent.csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_template_generation() {
        let template = generate_default_config_template();
        
        // Should be valid TOML that can be parsed
        let config: HttpDiffConfig = toml::from_str(&template).unwrap();
        assert!(config.validate().is_ok());
        
        // Should contain expected sections
        assert!(template.contains("[environments.test]"));
        assert!(template.contains("[environments.prod]"));
        assert!(template.contains("[environments.staging]"));
        assert!(template.contains("[[routes]]"));
        assert!(template.contains("[global]"));
        
        // Should have helpful comments
        assert!(template.contains("# HTTP Diff Configuration"));
        assert!(template.contains("# Environment definitions"));
    }

    #[test]
    fn test_users_csv_template_generation() {
        let template = generate_default_users_csv();
        
        // Should be valid CSV with header
        assert!(template.starts_with("userId,siteId"));
        assert!(template.contains("745741037,MCO"));
        assert!(template.contains("85264518,MLA"));
        
        // Should be parseable by our CSV loader
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("template.csv");
        fs::write(&csv_path, template).unwrap();
        
        let users = load_user_data(&csv_path).unwrap();
        assert!(!users.is_empty());
        assert!(users[0].data.contains_key("userId"));
        assert!(users[0].data.contains_key("siteId"));
    }

    #[test]
    fn test_csv_parameter_substitution_comprehensive() {
        // Test comprehensive CSV parameter substitution across all request parts
        let mut user_data = HashMap::new();
        user_data.insert("userId".to_string(), "12345".to_string());
        user_data.insert("apiKey".to_string(), "secret-key-123".to_string());
        user_data.insert("region".to_string(), "us-east-1".to_string());
        let user_data = UserData { data: user_data };

        // Test path parameter substitution (strict mode)
        let path = "/api/users/{userId}/profile";
        let result = user_data.substitute_placeholders(path, true, true).unwrap();
        assert_eq!(result, "/api/users/12345/profile");

        // Test header substitution (non-strict mode)
        let header = "Bearer {apiKey}";
        let result = user_data.substitute_placeholders(header, false, false).unwrap();
        assert_eq!(result, "Bearer secret-key-123");

        // Test query parameter substitution (non-strict mode)
        let query_param = "region={region}&format=json";
        let result = user_data.substitute_placeholders(query_param, false, false).unwrap();
        assert_eq!(result, "region=us-east-1&format=json");

        // Test simple body substitution (non-strict mode) - should work for simple cases
        let simple_body = "userId={userId}&region={region}";
        let result = user_data.substitute_placeholders(simple_body, false, false).unwrap();
        assert_eq!(result, "userId=12345&region=us-east-1");

        // Test that missing parameters in non-strict mode are left unchanged
        let text_with_missing = "Hello {userId}, your {missingParam} is ready";
        let result = user_data.substitute_placeholders(text_with_missing, false, false).unwrap();
        assert_eq!(result, "Hello 12345, your {missingParam} is ready");

        // Test that missing parameters in strict mode cause an error
        let path_with_missing = "/api/users/{userId}/posts/{missingParam}";
        let result = user_data.substitute_placeholders(path_with_missing, true, true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpDiffError::MissingPathParameter { .. }));
    }

    #[test]
    fn test_ensure_config_files_exist() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("http-diff.toml").to_string_lossy().to_string();
        let csv_path = temp_dir.path().join("users.csv").to_string_lossy().to_string();
        
        // Files don't exist initially
        assert!(!std::path::Path::new(&config_path).exists());
        assert!(!std::path::Path::new(&csv_path).exists());
        
        // Generate files
        let (config_generated, csv_generated) = ensure_config_files_exist(
            &config_path, 
            &csv_path, 
            true
        ).unwrap();
        
        assert!(config_generated);
        assert!(csv_generated);
        
        // Files should now exist and be valid
        assert!(std::path::Path::new(&config_path).exists());
        assert!(std::path::Path::new(&csv_path).exists());
        
        let config = HttpDiffConfig::load_from_file(&config_path).unwrap();
        assert!(config.validate().is_ok());
        
        let users = load_user_data(&csv_path).unwrap();
        assert!(!users.is_empty());
    }

    #[test]
    fn test_enhanced_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");
        
        // Invalid HTTP method
        let invalid_method_toml = r#"
[environments.test]
base_url = "https://api-test.example.com"

[[routes]]
name = "invalid-method"
method = "INVALID"
path = "/api/test"
"#;
        
        fs::write(&config_path, invalid_method_toml).unwrap();
        let result = HttpDiffConfig::load_with_validation(&config_path);
        assert!(result.is_err());
        
        if let Err(HttpDiffError::InvalidConfig { message }) = result {
            assert!(message.contains("Invalid HTTP method"));
            assert!(message.contains("INVALID"));
        } else {
            panic!("Expected InvalidConfig error for invalid HTTP method");
        }
        
        // Invalid URL
        let invalid_url_toml = r#"
[environments.test]
base_url = "not-a-valid-url"

[[routes]]
name = "test-route"
method = "GET"
path = "/api/test"
"#;
        
        fs::write(&config_path, invalid_url_toml).unwrap();
        let result = HttpDiffConfig::load_with_validation(&config_path);
        assert!(result.is_err());
        
        // Invalid timeout
        let invalid_timeout_toml = r#"
[environments.test]
base_url = "https://api-test.example.com"

[global]
timeout_seconds = 0

[[routes]]
name = "test-route"
method = "GET"
path = "/api/test"
"#;
        
        fs::write(&config_path, invalid_timeout_toml).unwrap();
        let result = HttpDiffConfig::load_with_validation(&config_path);
        assert!(result.is_err());
        
        if let Err(HttpDiffError::InvalidConfig { message }) = result {
            assert!(message.contains("timeout_seconds must be between 1 and 300"));
        } else {
            panic!("Expected InvalidConfig error for invalid timeout");
        }
    }
} 