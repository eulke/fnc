use http_diff::{output::CurlGenerator, types::*};
use std::collections::HashMap;

fn main() {
    let mut responses = HashMap::new();
    responses.insert("prod".to_string(), HttpResponse {
        status: 200,
        headers: HashMap::new(),
        body: "{}".to_string(),
        url: "https://prod.example.com".to_string\(\),
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

    let output = CurlGenerator::format_comparison_results(&[result], true);
    println!("Output with include_errors=true:");
    println!("{}", output);
    
    let output = CurlGenerator::format_comparison_results(&[result], false);
    println!("\nOutput with include_errors=false:");
    println!("{}", output);
}
