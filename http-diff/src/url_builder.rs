use crate::config::{Route, UserData, HttpDiffConfig};
use crate::error::{HttpDiffError, Result};
use url::Url;
use urlencoding::encode;
use std::collections::HashMap;

/// Builder for constructing HTTP URLs with parameter substitution
pub struct UrlBuilder<'a> {
    config: &'a HttpDiffConfig,
    route: &'a Route,
    environment: &'a str,
    user_data: &'a UserData,
}

impl<'a> UrlBuilder<'a> {
    /// Create a new URL builder
    pub fn new(
        config: &'a HttpDiffConfig,
        route: &'a Route,
        environment: &'a str,
        user_data: &'a UserData,
    ) -> Self {
        Self {
            config,
            route,
            environment,
            user_data,
        }
    }

    /// Build the complete URL with all substitutions and parameters
    pub fn build(&self) -> Result<Url> {
        let base_url = self.get_base_url()?;
        let path = self.substitute_path_parameters()?;
        let full_url = format!("{}{}", base_url.trim_end_matches('/'), path);
        
        let mut url = Url::parse(&full_url)?;
        self.add_query_parameters(&mut url)?;
        
        Ok(url)
    }

    /// Get the base URL for this route and environment
    fn get_base_url(&self) -> Result<String> {
        // Check for route-specific base URL override first
        if let Some(base_urls) = &self.route.base_urls {
            if let Some(override_url) = base_urls.get(self.environment) {
                return Ok(override_url.clone());
            }
        }

        // Fall back to environment base URL
        self.config
            .environments
            .get(self.environment)
            .map(|env| env.base_url.clone())
            .ok_or_else(|| HttpDiffError::InvalidEnvironment {
                environment: self.environment.to_string(),
            })
    }

    /// Substitute path parameters like {userId} with actual values
    fn substitute_path_parameters(&self) -> Result<String> {
        let mut path = self.route.path.clone();
        
        // Find and replace all {parameter} placeholders
        while let Some(start) = path.find('{') {
            let end = path[start..]
                .find('}')
                .ok_or_else(|| HttpDiffError::invalid_config("Unclosed path parameter"))?;
            
            let param_name = &path[start + 1..start + end];
            let value = self.user_data
                .data
                .get(param_name)
                .ok_or_else(|| HttpDiffError::MissingPathParameter {
                    param: param_name.to_string(),
                })?;
            
            // URL encode the parameter value
            let encoded_value = encode(value);
            path.replace_range(start..start + end + 1, &encoded_value);
        }
        
        Ok(path)
    }

    /// Add query parameters to the URL
    fn add_query_parameters(&self, url: &mut Url) -> Result<()> {
        let params = self.collect_query_parameters();
        
        for (key, value) in params {
            url.query_pairs_mut().append_pair(&key, &value);
        }
        
        Ok(())
    }

    /// Collect all query parameters from global config and route
    fn collect_query_parameters(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();

        // Add global parameters first
        if let Some(global_params) = self.config
            .global
            .as_ref()
            .and_then(|g| g.params.as_ref())
        {
            params.extend(global_params.clone());
        }

        // Add route-specific parameters (override global ones)
        if let Some(route_params) = &self.route.params {
            params.extend(route_params.clone());
        }

        params
    }

    /// Get just the path with substituted parameters (for display purposes)
    pub fn get_substituted_path(&self) -> Result<String> {
        self.substitute_path_parameters()
    }

    /// Get the base URL without path (for display purposes)
    pub fn get_base_url_only(&self) -> Result<String> {
        self.get_base_url()
    }

    /// Get all query parameters as a formatted string
    pub fn get_query_string(&self) -> String {
        let params = self.collect_query_parameters();
        if params.is_empty() {
            return String::new();
        }

        params
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }
}

/// Utility functions for URL manipulation
pub mod utils {
    use super::*;

    /// Check if a URL path contains parameter placeholders
    pub fn has_path_parameters(path: &str) -> bool {
        path.contains('{') && path.contains('}')
    }

    /// Extract parameter names from a path
    pub fn extract_parameter_names(path: &str) -> Vec<String> {
        let mut params = Vec::new();
        let mut chars = path.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut param = String::new();
                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        break;
                    }
                    param.push(ch);
                }
                if !param.is_empty() {
                    params.push(param);
                }
            }
        }
        
        params
    }

    /// Validate that all path parameters have corresponding user data
    pub fn validate_path_parameters(path: &str, user_data: &UserData) -> Result<()> {
        let required_params = extract_parameter_names(path);
        
        for param in required_params {
            if !user_data.data.contains_key(&param) {
                return Err(HttpDiffError::MissingPathParameter { param });
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> HttpDiffConfig {
        let mut environments = HashMap::new();
        environments.insert(
            "dev".to_string(),
            crate::config::Environment {
                base_url: "https://api-dev.example.com".to_string(),
                headers: None,
            },
        );

        HttpDiffConfig {
            environments,
            global: Some(crate::config::GlobalConfig {
                timeout_seconds: None,
                follow_redirects: None,
                headers: None,
                params: Some({
                    let mut params = HashMap::new();
                    params.insert("api_version".to_string(), "v1".to_string());
                    params
                }),
            }),
            routes: vec![],
        }
    }

    fn create_test_route() -> Route {
        Route {
            name: "get_user".to_string(),
            method: "GET".to_string(),
            path: "/api/users/{userId}".to_string(),
            headers: None,
            params: Some({
                let mut params = HashMap::new();
                params.insert("include".to_string(), "profile".to_string());
                params
            }),
            base_urls: None,
            body: None,
        }
    }

    fn create_test_user_data() -> UserData {
        let mut data = HashMap::new();
        data.insert("userId".to_string(), "123".to_string());
        data.insert("siteId".to_string(), "MCO".to_string());
        
        UserData { data }
    }

    #[test]
    fn test_url_building() {
        let config = create_test_config();
        let route = create_test_route();
        let user_data = create_test_user_data();

        let builder = UrlBuilder::new(&config, &route, "dev", &user_data);
        let url = builder.build().unwrap();

        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("api-dev.example.com"));
        assert_eq!(url.path(), "/api/users/123");
        assert!(url.query().unwrap().contains("api_version=v1"));
        assert!(url.query().unwrap().contains("include=profile"));
    }

    #[test]
    fn test_parameter_extraction() {
        let path = "/api/users/{userId}/posts/{postId}";
        let params = utils::extract_parameter_names(path);
        
        assert_eq!(params, vec!["userId", "postId"]);
    }

    #[test]
    fn test_missing_parameter() {
        let config = create_test_config();
        let mut route = create_test_route();
        route.path = "/api/users/{userId}/posts/{postId}".to_string();
        let user_data = create_test_user_data();

        let builder = UrlBuilder::new(&config, &route, "dev", &user_data);
        let result = builder.build();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpDiffError::MissingPathParameter { .. }));
    }
} 