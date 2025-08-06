use crate::error::Result;
use crate::traits::ResponseConverter;
use crate::types::HttpResponse;
use reqwest::Response;
use std::collections::HashMap;

/// Implementation of ResponseConverter trait
#[derive(Clone)]
pub struct ResponseConverterImpl;

impl ResponseConverterImpl {
    /// Create a new response converter
    pub fn new() -> Self {
        Self
    }
}

impl Default for ResponseConverterImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseConverter for ResponseConverterImpl {
    async fn convert_response(
        &self,
        response: Response,
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
}
