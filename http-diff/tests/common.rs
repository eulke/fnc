// Shared test utilities for http-diff crate tests
use http_diff::{HttpResponse, ComparisonResult};
use std::collections::HashMap;

/// Helper function to create HttpResponse with specific status and content
pub fn create_response(status: u16, body: &str, url: Option<&str>) -> HttpResponse {
    HttpResponse {
        status,
        headers: HashMap::new(),
        body: body.to_string(),
        url: url.unwrap_or("http://test.com").to_string(),
        curl_command: format!("curl {}", url.unwrap_or("http://test.com")),
    }
}

/// Helper function to create ComparisonResult for testing
pub fn create_comparison_result(
    route_name: &str,
    env_responses: Vec<(&str, u16, &str)>, // (env, status, body)
    is_identical: bool,
) -> ComparisonResult {
    let mut responses = HashMap::new();
    let mut status_codes = HashMap::new();
    let mut error_bodies = HashMap::new();
    let mut has_errors = false;

    for (env, status, body) in env_responses {
        responses.insert(env.to_string(), create_response(status, body, None));
        status_codes.insert(env.to_string(), status);
        
        if status < 200 || status >= 300 {
            has_errors = true;
            error_bodies.insert(env.to_string(), body.to_string());
        }
    }

    ComparisonResult {
        route_name: route_name.to_string(),
        user_context: HashMap::new(),
        responses,
        differences: vec![],
        is_identical,
        status_codes,
        has_errors,
        error_bodies: if error_bodies.is_empty() { None } else { Some(error_bodies) },
    }
}

/// Create test configuration for integration tests
pub fn create_test_config() -> String {
    r#"[environments.test]
base_url = "http://127.0.0.1:8080"

[environments.prod]
base_url = "http://127.0.0.1:8081"

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "api/users"
method = "GET"
path = "/api/users"
"#.to_string()
}

/// Create test user data CSV content
pub fn create_test_users_csv() -> String {
    "userId,siteId,userName\n12345,MCO,test_user\n".to_string()
} 