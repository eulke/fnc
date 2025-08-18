use crate::config::{HttpDiffConfig, Route, UserData};
use crate::error::{HttpDiffError, Result};
use crate::traits::HttpClient;
use crate::types::HttpResponse;
use crate::url_builder::UrlBuilder;
use reqwest::{Client, Method, Request};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client implementation
#[derive(Clone)]
pub struct HttpClientImpl {
    client: Client,
    config: HttpDiffConfig,
}

impl HttpClientImpl {
    /// Create a new HTTP client with configuration
    pub fn new(config: HttpDiffConfig) -> Result<Self> {
        let timeout = config
            .global
            .as_ref()
            .and_then(|g| g.timeout_seconds)
            .unwrap_or(30);

        let follow_redirects = config
            .global
            .as_ref()
            .and_then(|g| g.follow_redirects)
            .unwrap_or(true);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .redirect(if follow_redirects {
                reqwest::redirect::Policy::default()
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()?;

        Ok(Self { client, config })
    }

    /// Build an HTTP request from route configuration
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
        let method = Method::from_bytes(route.method.as_bytes()).map_err(|_| {
            HttpDiffError::invalid_config(format!("Invalid HTTP method: {}", route.method))
        })?;

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

    /// Add headers to request with CSV parameter substitution
    fn add_headers(
        &self,
        request_builder: reqwest::RequestBuilder,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<reqwest::RequestBuilder> {
        let headers =
            crate::url_builder::resolve_headers(&self.config, route, environment, user_data)?;
        let mut rb = request_builder;
        for (key, value) in headers {
            rb = rb.header(key, value);
        }
        Ok(rb)
    }

    /// Convert reqwest Response to our HttpResponse
    async fn convert_response(
        &self,
        response: reqwest::Response,
        curl_command: String,
    ) -> Result<HttpResponse> {
        let status = response.status().as_u16();
        let url = response.url().to_string();

        // Extract headers
        let mut headers = HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        // Extract body
        let body = response.text().await?;

        Ok(HttpResponse {
            status,
            headers,
            body,
            url,
            curl_command,
        })
    }

    /// Generate a curl command equivalent for the request (optimized for memory efficiency)
    fn generate_curl_command(&self, request: &reqwest::Request, route: &Route) -> String {
        // More accurate capacity estimation to minimize reallocations
        let mut estimated_capacity = 4; // "curl"
        
        // Method (if not GET)
        if request.method() != "GET" {
            estimated_capacity += 4 + request.method().as_str().len(); // " -X " + method
        }
        
        // Headers with more accurate estimation
        for (name, value) in request.headers() {
            if value.to_str().is_ok() {
                estimated_capacity += 6 + name.as_str().len() + value.len(); // " -H '" + name + ": " + value + "'"
            }
        }
        
        // Body
        if let Some(body) = &route.body {
            estimated_capacity += 5 + body.len(); // " -d '" + body + "'"
        }
        
        // URL
        estimated_capacity += 3 + request.url().as_str().len(); // " '" + url + "'"
        
        // Add 10% buffer to handle estimation errors
        estimated_capacity = (estimated_capacity as f32 * 1.1) as usize;
        
        let mut result = String::with_capacity(estimated_capacity);

        result.push_str("curl");

        // Add method
        if request.method() != "GET" {
            result.push_str(" -X ");
            result.push_str(request.method().as_str());
        }

        // Add headers with optimized string building
        for (name, value) in request.headers() {
            if let Ok(value_str) = value.to_str() {
                // Build header string efficiently without temporary allocations
                result.push_str(" -H '");
                result.push_str(name.as_str());
                result.push_str(": ");
                result.push_str(value_str);
                result.push('\'');
            }
        }

        // Add body if present
        if let Some(body) = &route.body {
            result.reserve(5 + body.len()); // Ensure capacity before appending
            result.push_str(" -d '");
            result.push_str(body);
            result.push('\'');
        }

        // Add URL
        let url = request.url().as_str();
        result.reserve(3 + url.len()); // Ensure capacity before appending
        result.push_str(" '");
        result.push_str(url);
        result.push('\'');

        result
    }
}

impl HttpClient for HttpClientImpl {
    async fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<HttpResponse> {
        let request = self.build_request(route, environment, user_data).await?;
        let curl_command = self.generate_curl_command(&request, route);

        let response = self.client.execute(request).await.map_err(|e| {
            HttpDiffError::request_failed(
                route.name.clone(),
                environment.to_string(),
                format!("Request failed: {}", e),
            )
        })?;

        self.convert_response(response, curl_command).await
    }
}
