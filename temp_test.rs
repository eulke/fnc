use http_diff::{curl::CurlGenerator, types::*};
use std::collections::HashMap;

fn main() {
    let mut responses = HashMap::new();
    responses.insert("prod".to_string(), HttpResponse {
        status: 200,
        headers: HashMap::new(),
        body: "{}".to_string(),
        url: "https://prod.example.com".to_string(),
        curl_command: "curl".to_string(),
    });

    let result = ComparisonResult {
        route_name: "test".to_string(),
        user_context: HashMap::new(),
        responses,
        differences: vec![],
        is_identical: true,
        status_codes: {
            let mut sc = HashMap::new();
            sc.insert("prod".to_string(), 200);
            sc
        },
        has_errors: false,
        error_bodies: None,
    };

    // Create a basic HTTP diff config for the CurlGenerator
    let config = http_diff::HttpDiffConfig::builder()
        .environment("prod", "https://prod.example.com")
        .get_route("test", "/api/test")
        .build()
        .unwrap();
    
    let generator = CurlGenerator::new(config.clone());
    
    // Create test user data
    let user_data = http_diff::UserData {
        data: HashMap::new(),
    };
    
    // Generate a curl command for the test route
    match generator.generate_curl_command(&config.routes[0], "prod", &user_data) {
        Ok(command) => {
            println!("Generated curl command:");
            println!("{}", command);
        }
        Err(e) => {
            println!("Error generating curl command: {}", e);
        }
    }
}
