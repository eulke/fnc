use crate::config::{HttpDiffConfig, Route, UserData};
use crate::error::{HttpDiffError, Result};
use reqwest::{Client, Method, Request, Response};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;
use urlencoding::encode;

/// HTTP client wrapper for executing requests
pub struct HttpClient {
    client: Client,
    config: HttpDiffConfig,
}

/// Response data with metadata
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub url: String,
    pub curl_command: String,
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
        // Get base URL for this route and environment
        let base_url = self.config.get_base_url(route, environment)?;
        
        // Substitute path parameters
        let path = self.substitute_path_parameters(&route.path, user_data)?;
        
        // Build full URL
        let full_url = format!("{}{}", base_url.trim_end_matches('/'), path);
        let mut url = Url::parse(&full_url)?;

        // Add query parameters
        self.add_query_parameters(&mut url, route, user_data)?;

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

    /// Substitute path parameters like {userId} with actual values
    fn substitute_path_parameters(&self, path: &str, user_data: &UserData) -> Result<String> {
        let mut result = path.to_string();
        
        // Find all parameters in the format {param_name}
        while let Some(start) = result.find('{') {
            if let Some(end) = result[start..].find('}') {
                let param_name = &result[start + 1..start + end];
                
                let value = user_data.data.get(param_name)
                    .ok_or_else(|| HttpDiffError::MissingPathParameter {
                        param: param_name.to_string(),
                    })?;
                
                // URL encode the parameter value for safety
                let encoded_value = encode(value);
                result.replace_range(start..start + end + 1, &encoded_value);
            } else {
                break;
            }
        }
        
        Ok(result)
    }

    /// Add query parameters to URL
    fn add_query_parameters(
        &self,
        url: &mut Url,
        route: &Route,
        _user_data: &UserData,
    ) -> Result<()> {
        // Add global parameters
        if let Some(global) = &self.config.global {
            if let Some(global_params) = &global.params {
                for (key, value) in global_params {
                    url.query_pairs_mut().append_pair(key, value);
                }
            }
        }

        // Add route-specific parameters
        if let Some(route_params) = &route.params {
            for (key, value) in route_params {
                url.query_pairs_mut().append_pair(key, value);
            }
        }

        Ok(())
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
    fn test_path_parameter_substitution() {
        let config = create_test_config();
        let client = HttpClient::new(config).unwrap();

        let mut user_data = HashMap::new();
        user_data.insert("userId".to_string(), "12345".to_string());
        user_data.insert("siteId".to_string(), "MCO".to_string());
        let user_data = UserData { data: user_data };

        let result = client.substitute_path_parameters("/api/users/{userId}/sites/{siteId}", &user_data).unwrap();
        assert_eq!(result, "/api/users/12345/sites/MCO");
    }

    #[test]
    fn test_missing_path_parameter() {
        let config = create_test_config();
        let client = HttpClient::new(config).unwrap();

        let user_data = UserData {
            data: HashMap::new(),
        };

        let result = client.substitute_path_parameters("/api/users/{userId}", &user_data);
        assert!(result.is_err());
        
        if let Err(HttpDiffError::MissingPathParameter { param }) = result {
            assert_eq!(param, "userId");
        } else {
            panic!("Expected MissingPathParameter error");
        }
    }

    #[test]
    fn test_url_encoding_in_path_parameters() {
        let config = create_test_config();
        let client = HttpClient::new(config).unwrap();

        let mut user_data = HashMap::new();
        user_data.insert("userId".to_string(), "user@example.com".to_string());
        user_data.insert("query".to_string(), "hello world".to_string());
        let user_data = UserData { data: user_data };

        let result = client.substitute_path_parameters("/api/users/{userId}/search/{query}", &user_data).unwrap();
        assert_eq!(result, "/api/users/user%40example.com/search/hello%20world");
    }

    #[test]
    fn test_multiple_same_parameters() {
        let config = create_test_config();
        let client = HttpClient::new(config).unwrap();

        let mut user_data = HashMap::new();
        user_data.insert("id".to_string(), "123".to_string());
        let user_data = UserData { data: user_data };

        let result = client.substitute_path_parameters("/api/{id}/items/{id}", &user_data).unwrap();
        assert_eq!(result, "/api/123/items/123");
    }

    #[test]
    fn test_malformed_path_parameters() {
        let config = create_test_config();
        let client = HttpClient::new(config).unwrap();

        let user_data = UserData {
            data: HashMap::new(),
        };

        // Test with unclosed parameter
        let result = client.substitute_path_parameters("/api/users/{userId", &user_data).unwrap();
        assert_eq!(result, "/api/users/{userId");
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
        let url = Url::parse("https://api.example.com/api/test").unwrap();
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