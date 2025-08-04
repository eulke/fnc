use crate::config::types::{HttpDiffConfig, Environment, GlobalConfig, Route};
use crate::error::Result;
use std::collections::HashMap;

/// Builder for HttpDiffConfig to improve API ergonomics
pub struct HttpDiffConfigBuilder {
    environments: HashMap<String, Environment>,
    global: Option<GlobalConfig>,
    routes: Vec<Route>,
}

impl HttpDiffConfigBuilder {
    /// Create a new config builder
    pub fn new() -> Self {
        Self {
            environments: HashMap::new(),
            global: None,
            routes: Vec::new(),
        }
    }

    /// Add an environment
    #[must_use]
    pub fn environment<S: Into<String>>(mut self, name: S, base_url: S) -> Self {
        self.environments.insert(
            name.into(),
            Environment {
                base_url: base_url.into(),
                headers: None,
            },
        );
        self
    }

    /// Add an environment with headers
    pub fn environment_with_headers<S: Into<String>>(
        mut self,
        name: S,
        base_url: S,
        headers: HashMap<String, String>,
    ) -> Self {
        self.environments.insert(
            name.into(),
            Environment {
                base_url: base_url.into(),
                headers: Some(headers),
            },
        );
        self
    }

    /// Set global configuration
    pub fn global_config(mut self, global: GlobalConfig) -> Self {
        self.global = Some(global);
        self
    }

    /// Set timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        let mut global = self.global.unwrap_or_default();
        global.timeout_seconds = Some(seconds);
        self.global = Some(global);
        self
    }

    /// Set whether to follow redirects
    pub fn follow_redirects(mut self, follow: bool) -> Self {
        let mut global = self.global.unwrap_or_default();
        global.follow_redirects = Some(follow);
        self.global = Some(global);
        self
    }

    /// Add global headers
    pub fn global_headers(mut self, headers: HashMap<String, String>) -> Self {
        let mut global = self.global.unwrap_or_default();
        global.headers = Some(headers);
        self.global = Some(global);
        self
    }

    /// Add a global header
    pub fn global_header<S: Into<String>>(mut self, key: S, value: S) -> Self {
        let mut global = self.global.unwrap_or_default();
        if global.headers.is_none() {
            global.headers = Some(HashMap::new());
        }
        global.headers.as_mut().unwrap().insert(key.into(), value.into());
        self.global = Some(global);
        self
    }

    /// Add a route
    pub fn route(mut self, route: Route) -> Self {
        self.routes.push(route);
        self
    }

    /// Add a simple GET route
    #[must_use]
    pub fn get_route<S: Into<String>>(mut self, name: S, path: S) -> Self {
        self.routes.push(Route {
            name: name.into(),
            method: "GET".to_string(),
            path: path.into(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
        });
        self
    }

    /// Add a POST route with body
    pub fn post_route<S: Into<String>>(mut self, name: S, path: S, body: S) -> Self {
        self.routes.push(Route {
            name: name.into(),
            method: "POST".to_string(),
            path: path.into(),
            headers: None,
            params: None,
            base_urls: None,
            body: Some(body.into()),
        });
        self
    }

    /// Build the configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid (e.g., no environments or routes defined)
    pub fn build(self) -> Result<HttpDiffConfig> {
        let config = HttpDiffConfig {
            environments: self.environments,
            global: self.global,
            routes: self.routes,
        };
        
        // Basic validation
        use crate::config::validator::ConfigValidatorImpl;
        use crate::traits::ConfigValidator;
        let validator = ConfigValidatorImpl;
        validator.validate(&config)?;
        
        Ok(config)
    }
}

impl Default for HttpDiffConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}