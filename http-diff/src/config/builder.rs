use crate::config::types::{Environment, GlobalConfig, HttpDiffConfig, Route};
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

    /// Add an environment with optional headers
    #[must_use]
    pub fn environment<S: Into<String>>(
        mut self,
        name: S,
        base_url: S,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        self.environments.insert(
            name.into(),
            Environment {
                base_url: base_url.into(),
                headers,
                is_base: false,
            },
        );
        self
    }

    /// Set global configuration directly
    pub fn global_config(mut self, global: GlobalConfig) -> Self {
        self.global = Some(global);
        self
    }

    /// Configure global settings using fluent builder
    pub fn configure_global<F>(mut self, configure_fn: F) -> Self
    where
        F: FnOnce(
            crate::config::global_builder::GlobalConfigBuilder,
        ) -> crate::config::global_builder::GlobalConfigBuilder,
    {
        let builder = if let Some(existing) = self.global.take() {
            crate::config::global_builder::GlobalConfigBuilder::from_config(existing)
        } else {
            crate::config::global_builder::GlobalConfigBuilder::new()
        };

        let global_config = configure_fn(builder).build();
        self.global = Some(global_config);
        self
    }

    /// Set timeout in seconds (convenience method)
    pub fn timeout(self, seconds: u64) -> Self {
        self.configure_global(|global| global.timeout(seconds))
    }

    /// Set whether to follow redirects (convenience method)
    pub fn follow_redirects(self, follow: bool) -> Self {
        self.configure_global(|global| global.follow_redirects(follow))
    }

    /// Add global headers (convenience method)
    pub fn global_headers(self, headers: HashMap<String, String>) -> Self {
        self.configure_global(|global| global.headers(headers))
    }

    /// Add a global header (convenience method)
    pub fn global_header<S: Into<String>>(self, key: S, value: S) -> Self {
        self.configure_global(|global| global.header(key, value))
    }

    /// Set maximum number of concurrent requests (convenience method)
    pub fn max_concurrent_requests(self, max_concurrent: usize) -> Self {
        self.configure_global(|global| global.max_concurrent_requests(max_concurrent))
    }

    /// Add a route
    pub fn route(mut self, route: Route) -> Self {
        self.routes.push(route);
        self
    }

    /// Add a route with specified HTTP method and optional body
    #[must_use]
    pub fn add_route<N, M, P, B>(mut self, name: N, method: M, path: P, body: Option<B>) -> Self
    where
        N: Into<String>,
        M: Into<String>,
        P: Into<String>,
        B: Into<String>,
    {
        self.routes.push(Route {
            name: name.into(),
            method: method.into(),
            path: path.into(),
            headers: None,
            params: None,
            base_urls: None,
            body: body.map(|b| b.into()),
        });
        self
    }

    /// Add a simple GET route (convenience method)
    #[must_use]
    pub fn get_route<S: Into<String>>(self, name: S, path: S) -> Self {
        self.add_route(name, "GET", path, None::<&str>)
    }

    /// Add a POST route with body (convenience method)  
    #[must_use]
    pub fn post_route<S: Into<String>>(self, name: S, path: S, body: S) -> Self {
        self.add_route(name, "POST", path, Some(body))
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
