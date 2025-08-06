use crate::error::{HttpDiffError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub fn substitute_placeholders(
        &self,
        text: &str,
        url_encode: bool,
        strict: bool,
    ) -> Result<String> {
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

                        result.replace_range(start..=end, &final_value);
                        // Continue from after the replacement
                        position = start + final_value.len();
                    } else if strict {
                        // Strict mode: error if parameter is missing
                        let available = self
                            .data
                            .keys()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(HttpDiffError::MissingPathParameter {
                            param: param_name.to_string(),
                            available_params: if available.is_empty() {
                                "none".to_string()
                            } else {
                                available
                            },
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

/// Check if a parameter name is a valid identifier (letters, numbers, underscore)
fn is_valid_param_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Default implementation for GlobalConfig
impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(30),
            follow_redirects: Some(true),
            headers: None,
            params: None,
        }
    }
}

impl HttpDiffConfig {
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
