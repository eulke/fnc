use crate::config::{HttpDiffConfig, Route, UserData};
use crate::error::{HttpDiffError, Result};
use crate::traits::RequestBuilder;
use crate::url_builder::UrlBuilder;
use reqwest::{Client, Method, Request};

/// Implementation of RequestBuilder trait
#[derive(Clone)]
pub struct RequestBuilderImpl {
    client: Client,
    config: HttpDiffConfig,
}

impl RequestBuilderImpl {
    /// Create a new request builder
    pub fn new(client: Client, config: HttpDiffConfig) -> Self {
        Self { client, config }
    }

    /// Add headers to request with CSV parameter substitution
    fn add_headers(
        &self,
        request_builder: reqwest::RequestBuilder,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<reqwest::RequestBuilder> {
        let headers = crate::url_builder::resolve_headers(&self.config, route, environment, user_data)?;
        let mut rb = request_builder;
        for (key, value) in headers {
            rb = rb.header(key, value);
        }
        Ok(rb)
    }
}

impl RequestBuilder for RequestBuilderImpl {
    async fn build_request(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<Request> {
        // Use UrlBuilder to construct the URL
        let url_builder = UrlBuilder::new(&self.config, route, environment, user_data);
        let url = url_builder.build()?;

        // Parse HTTP method
        let method = Method::from_bytes(route.method.as_bytes())
            .map_err(|_| HttpDiffError::invalid_config(format!("Invalid HTTP method: {}", route.method)))?;

        // Start building request
        let mut request_builder = self.client.request(method, url);

        // Add headers with CSV parameter substitution
        request_builder = self.add_headers(request_builder, route, environment, user_data)?;

        // Add body with CSV parameter substitution if present
        if let Some(body) = &route.body {
            let substituted_body = user_data.substitute_placeholders(body, false, false)?;
            request_builder = request_builder.body(substituted_body);
        }

        request_builder.build().map_err(Into::into)
    }
}