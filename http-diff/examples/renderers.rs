#!/usr/bin/env rust-script
//! Example demonstrating the new multi-format output renderers
//! 
//! Usage: cargo run --example renderers

use http_diff::{
    ComparisonResult, HttpResponse, Difference, DifferenceCategory,
    OutputRenderer, CliRenderer, JsonRenderer, HtmlRenderer
};
use std::collections::HashMap;

fn main() {
    // Create sample comparison results
    let results = create_sample_results();
    
    println!("=== CLI Output ===");
    let cli_renderer = CliRenderer::new();
    let cli_output = cli_renderer.render(&results);
    println!("{}", cli_output);
    
    println!("\n=== JSON Output ===");
    let json_renderer = JsonRenderer::new();
    let json_output = json_renderer.render(&results);
    println!("{}", json_output);
    
    println!("\n=== HTML Output ===");
    let html_renderer = HtmlRenderer::new();
    let html_output = html_renderer.render(&results);
    println!("{}", html_output);
}

fn create_sample_results() -> Vec<ComparisonResult> {
    let mut results = Vec::new();
    
    // Create an identical result
    let mut identical_result = ComparisonResult::new(
        "get_user".to_string(),
        {
            let mut ctx = HashMap::new();
            ctx.insert("userId".to_string(), "123".to_string());
            ctx
        }
    );
    
    let dev_response = HttpResponse::new(
        200,
        {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
        r#"{"id": 123, "name": "John Doe"}"#.to_string(),
        "https://api-dev.example.com/users/123".to_string(),
        "curl -X GET 'https://api-dev.example.com/users/123'".to_string(),
    );
    
    let prod_response = HttpResponse::new(
        200,
        {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
        r#"{"id": 123, "name": "John Doe"}"#.to_string(),
        "https://api.example.com/users/123".to_string(),
        "curl -X GET 'https://api.example.com/users/123'".to_string(),
    );
    
    identical_result.add_response("dev".to_string(), dev_response);
    identical_result.add_response("prod".to_string(), prod_response);
    results.push(identical_result);
    
    // Create a different result
    let mut different_result = ComparisonResult::new(
        "get_profile".to_string(),
        {
            let mut ctx = HashMap::new();
            ctx.insert("userId".to_string(), "456".to_string());
            ctx
        }
    );
    
    let dev_profile = HttpResponse::new(
        200,
        {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
        r#"{"id": 456, "name": "Jane Smith", "email": "jane@dev.com"}"#.to_string(),
        "https://api-dev.example.com/users/456/profile".to_string(),
        "curl -X GET 'https://api-dev.example.com/users/456/profile'".to_string(),
    );
    
    let prod_profile = HttpResponse::new(
        200,
        {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
        r#"{"id": 456, "name": "Jane Smith", "email": "jane@example.com"}"#.to_string(),
        "https://api.example.com/users/456/profile".to_string(),
        "curl -X GET 'https://api.example.com/users/456/profile'".to_string(),
    );
    
    different_result.add_response("dev".to_string(), dev_profile);
    different_result.add_response("prod".to_string(), prod_profile);
    different_result.add_difference(Difference::new(
        DifferenceCategory::Body,
        "Email domain differs between environments".to_string(),
    ));
    
    results.push(different_result);
    
    // Create a failed result to test error analysis
    let mut failed_result = ComparisonResult::new(
        "get_orders".to_string(),
        {
            let mut ctx = HashMap::new();
            ctx.insert("userId".to_string(), "789".to_string());
            ctx
        }
    );
    
    let dev_error = HttpResponse::new(
        500,
        {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
        r#"{"error": "UnhandledError", "message": "Database connection failed"}"#.to_string(),
        "https://api-dev.example.com/users/789/orders".to_string(),
        "curl -X GET 'https://api-dev.example.com/users/789/orders'".to_string(),
    );
    
    let prod_error = HttpResponse::new(
        500,
        {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers
        },
        r#"{"error": "UnhandledError", "message": "Database connection failed"}"#.to_string(),
        "https://api.example.com/users/789/orders".to_string(),
        "curl -X GET 'https://api.example.com/users/789/orders'".to_string(),
    );
    
    failed_result.add_response("dev".to_string(), dev_error);
    failed_result.add_response("prod".to_string(), prod_error);
    
    results.push(failed_result);
    
    results
}