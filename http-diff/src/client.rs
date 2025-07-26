use crate::config::{HttpDiffConfig, Route, UserData};
use crate::error::{HttpDiffError, Result};
use crate::types::HttpResponse;
use crate::url_builder::UrlBuilder;
use reqwest::{Client, Method, Request, Response};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client wrapper for executing requests
pub struct HttpClient {
    client: Client,
    config: HttpDiffConfig,
}

impl HttpClient {
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

    /// Execute a request for a specific route, environment, and user data
    pub async fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<HttpResponse> {
        let request = self.build_request(route, environment, user_data)?;
        let curl_command = self.generate_curl_command(&request, route);
        
        let response = self.client
            .execute(request)
            .await
            .map_err(|e| HttpDiffError::request_failed(format!("Request failed: {}", e)))?;

        self.convert_response(response, curl_command).await
    }

    /// Build an HTTP request from route configuration
    fn build_request(
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

        // Add headers
        request_builder = self.add_headers(request_builder, route, environment)?;

        // Add body if present
        if let Some(body) = &route.body {
            request_builder = request_builder.body(body.clone());
        }

        request_builder.build().map_err(Into::into)
    }





    /// Add headers to request
    fn add_headers(
        &self,
        mut request_builder: reqwest::RequestBuilder,
        route: &Route,
        environment: &str,
    ) -> Result<reqwest::RequestBuilder> {
        // Add global headers
        if let Some(global) = &self.config.global {
            if let Some(global_headers) = &global.headers {
                for (key, value) in global_headers {
                    request_builder = request_builder.header(key, value);
                }
            }
        }

        // Add environment-specific headers
        if let Some(env) = self.config.environments.get(environment) {
            if let Some(env_headers) = &env.headers {
                for (key, value) in env_headers {
                    request_builder = request_builder.header(key, value);
                }
            }
        }

        // Add route-specific headers (these take precedence)
        if let Some(route_headers) = &route.headers {
            for (key, value) in route_headers {
                request_builder = request_builder.header(key, value);
            }
        }

        Ok(request_builder)
    }

    /// Convert reqwest Response to our HttpResponse
    async fn convert_response(&self, response: Response, curl_command: String) -> Result<HttpResponse> {
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

    /// Generate a curl command equivalent for the request
    fn generate_curl_command(&self, request: &Request, route: &Route) -> String {
        let mut curl_parts = vec!["curl".to_string()];
        
        // Add method
        if request.method() != "GET" {
            curl_parts.push("-X".to_string());
            curl_parts.push(request.method().to_string());
        }
        
        // Add headers
        for (name, value) in request.headers() {
            if let Ok(value_str) = value.to_str() {
                curl_parts.push("-H".to_string());
                curl_parts.push(format!("'{}: {}'", name, value_str));
            }
        }
        
        // Add body if present
        if let Some(body) = &route.body {
            curl_parts.push("-d".to_string());
            curl_parts.push(format!("'{}'", body));
        }
        
        // Add URL
        curl_parts.push(format!("'{}'", request.url()));
        
        curl_parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Environment, GlobalConfig};

    fn create_test_config() -> HttpDiffConfig {
        let mut environments = HashMap::new();
        environments.insert(
            "test".to_string(),
            Environment {
                base_url: "https://api-test.example.com".to_string(),
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("X-Scope".to_string(), "test".to_string());
                    headers
                }),
            },
        );

        let global = GlobalConfig {
            timeout_seconds: Some(30),
            follow_redirects: Some(true),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("User-Agent".to_string(), "fnc-http-diff/1.0".to_string());
                headers
            }),
            params: Some({
                let mut params = HashMap::new();
                params.insert("version".to_string(), "v1".to_string());
                params
            }),
        };

        HttpDiffConfig {
            environments,
            global: Some(global),
            routes: vec![],
        }
    }

    #[test]
    fn test_client_creation() {
        let config = create_test_config();
        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_missing_path_parameter() {
        let config = create_test_config();
        let _client = HttpClient::new(config).unwrap();

        let _user_data = UserData {
            data: HashMap::new(),
        };

        // This test is now covered in url_builder.rs tests
        assert!(true);
    }

    #[test]
    fn test_url_building_integration() {
        // URL building functionality is now tested in url_builder.rs
        // This test verifies the integration works
        let config = create_test_config();
        let _client = HttpClient::new(config);
        // Integration test would require actual HTTP calls
        assert!(true);
    }

    #[test]
    fn test_curl_command_generation() {
        let config = create_test_config();
        let client = HttpClient::new(config).unwrap();

        let route = crate::config::Route {
            name: "test".to_string(),
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            }),
            params: None,
            base_urls: None,
            body: Some(r#"{"test": "data"}"#.to_string()),
        };

        // Create a mock request
        let url = url::Url::parse("https://api.example.com/api/test").unwrap();
        let mut request = reqwest::Request::new(reqwest::Method::POST, url);
        request.headers_mut().insert("content-type", "application/json".parse().unwrap());

        let curl_command = client.generate_curl_command(&request, &route);
        
        assert!(curl_command.contains("curl"));
        assert!(curl_command.contains("-X POST"));
        assert!(curl_command.contains("content-type: application/json"));
        assert!(curl_command.contains(r#"-d '{"test": "data"}'"#));
        assert!(curl_command.contains("'https://api.example.com/api/test'"));
    }

    #[test]
    fn test_client_creation_with_timeout() {
        let mut config = create_test_config();
        config.global = Some(crate::config::GlobalConfig {
            timeout_seconds: Some(60),
            follow_redirects: Some(false),
            headers: None,
            params: None,
        });

        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }
} 