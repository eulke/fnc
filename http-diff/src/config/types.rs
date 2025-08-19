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
    /// Whether this environment should be treated as the base for comparisons
    #[serde(default)]
    pub is_base: bool,
}

/// Global configuration settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Whether to follow redirects
    pub follow_redirects: Option<bool>,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: Option<usize>,
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
        // Use single-pass algorithm to avoid multiple reallocations
        let mut result = String::with_capacity(text.len() + 50); // Pre-allocate with some extra space
        let mut chars = text.char_indices().peekable();

        while let Some((_pos, ch)) = chars.next() {
            if ch == '{' {
                // Found potential parameter start, collect parameter name
                let mut param_name = String::new();
                let mut found_end = false;

                while let Some((_, next_ch)) = chars.peek() {
                    if *next_ch == '}' {
                        chars.next(); // consume the '}'
                        found_end = true;
                        break;
                    } else if *next_ch == '{' {
                        // Nested braces, not a valid parameter
                        break;
                    } else {
                        param_name.push(chars.next().unwrap().1);
                    }
                }

                if found_end && is_valid_param_name(&param_name) {
                    if let Some(value) = self.data.get(&param_name) {
                        // Substitute the parameter
                        if url_encode {
                            result.push_str(&urlencoding::encode(value));
                        } else {
                            result.push_str(value);
                        }
                    } else if strict {
                        // Strict mode: error if parameter is missing
                        let available = self
                            .data
                            .keys()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(HttpDiffError::MissingPathParameter {
                            param: param_name,
                            available_params: if available.is_empty() {
                                "none".to_string()
                            } else {
                                available
                            },
                        });
                    } else {
                        // Non-strict mode: preserve the original placeholder
                        result.push('{');
                        result.push_str(&param_name);
                        result.push('}');
                    }
                } else {
                    // Invalid parameter format, preserve the original text
                    result.push(ch);
                    result.push_str(&param_name);
                    if found_end {
                        result.push('}');
                    }
                }
            } else {
                result.push(ch);
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
            max_concurrent_requests: Some(10),
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
