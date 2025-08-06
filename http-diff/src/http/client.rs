use crate::config::{HttpDiffConfig, Route, UserData};
use crate::error::{HttpDiffError, Result};
use crate::http::{RequestBuilderImpl, ResponseConverterImpl};
use crate::traits::{HttpClient, RequestBuilder, ResponseConverter};
use crate::types::HttpResponse;
use reqwest::Client;
use std::time::Duration;

/// HTTP client implementation
#[derive(Clone)]
pub struct HttpClientImpl {
    client: Client,
    request_builder: RequestBuilderImpl,
    response_converter: ResponseConverterImpl,
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

        let request_builder = RequestBuilderImpl::new(client.clone(), config);
        let response_converter = ResponseConverterImpl::new();

        Ok(Self {
            client,
            request_builder,
            response_converter,
        })
    }

    /// Generate a curl command equivalent for the request
    fn generate_curl_command(&self, request: &reqwest::Request, route: &Route) -> String {
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

impl HttpClient for HttpClientImpl {
    async fn execute_request(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<HttpResponse> {
        let request = self
            .request_builder
            .build_request(route, environment, user_data)
            .await?;
        let curl_command = self.generate_curl_command(&request, route);

        let response = self.client.execute(request).await.map_err(|e| {
            HttpDiffError::request_failed(
                route.name.clone(),
                environment.to_string(),
                format!("Request failed: {}", e),
            )
        })?;

        self.response_converter
            .convert_response(response, curl_command)
            .await
    }
}
