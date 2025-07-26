//! HTTP Diff Integration tests
//! 
//! These tests cover complete user workflows from configuration generation
//! to request execution and output generation.

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};
use serde_json::json;

use http_diff::{
    config::{HttpDiffConfig, ensure_config_files_exist, load_user_data},
    run_http_diff,
    CurlGenerator,
};

/// Helper function to create test configuration files
fn create_test_config_files(dir: &Path) -> std::io::Result<()> {
    let config_content = r#"# HTTP Diff Integration Test Configuration
[environments.test]
base_url = "http://127.0.0.1:8080"
headers."X-Environment" = "test"
headers."X-API-Key" = "test-key-123"

[environments.prod]
base_url = "http://127.0.0.1:8081" 
headers."X-Environment" = "prod"
headers."X-API-Key" = "prod-key-456"

[global]
timeout_seconds = 10
follow_redirects = true
headers."User-Agent" = "fnc-http-diff-integration-test/1.0"
headers."Accept" = "application/json"

[[routes]]
name = "health-check"
method = "GET"
path = "/health"
"#;

    let users_content = r#"userId,siteId,userName
12345,MCO,john_doe
67890,MLA,jane_smith
99999,MLB,test_user
"#;

    fs::write(dir.join("http-diff.toml"), config_content)?;
    fs::write(dir.join("users.csv"), users_content)?;
    Ok(())
}

/// Helper function to setup mock servers for different environments
async fn setup_mock_servers() -> (MockServer, MockServer) {
    let test_server = MockServer::start().await;
    let prod_server = MockServer::start().await;

    // Setup common mocks for all servers
    for server in [&test_server, &prod_server] {
        // Health check endpoint
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "ok",
                "timestamp": "2023-01-01T00:00:00Z"
            })))
            .mount(server)
            .await;

        // User profile endpoint
        Mock::given(method("GET"))
            .and(path("/api/users/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 12345,
                "name": "John Doe",
                "email": "john@example.com",
                "environment": server.address().port()
            })))
            .mount(server)
            .await;
    }

    // Add environment-specific differences for testing
    Mock::given(method("GET"))
        .and(path("/api/users/67890"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 67890,
            "name": "Jane Smith",
            "email": "jane@example.com",
            "status": "active" // Only in test environment
        })))
        .mount(&test_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/users/67890"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 67890,
            "name": "Jane Smith",
            "email": "jane@example.com"
            // Missing status field in prod
        })))
        .mount(&prod_server)
        .await;

    (test_server, prod_server)
}

#[tokio::test]
async fn test_complete_user_workflow_with_identical_responses() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Step 1: Create configuration files
    create_test_config_files(config_dir).unwrap();

    // Step 2: Setup mock servers
    let (test_server, prod_server) = setup_mock_servers().await;

    // Step 3: Update configuration with actual server addresses
    let config_content = format!(r#"# HTTP Diff Integration Test Configuration
[environments.test]
base_url = "{}"
headers."X-Environment" = "test"

[environments.prod]
base_url = "{}"
headers."X-Environment" = "prod"

[global]
timeout_seconds = 10
follow_redirects = true
headers."User-Agent" = "fnc-http-diff-integration-test/1.0"
headers."Accept" = "application/json"

[[routes]]
name = "health-check"
method = "GET"
path = "/health"
"#, test_server.uri(), prod_server.uri());

    fs::write(config_dir.join("http-diff.toml"), config_content).unwrap();
    
    let users_content = "userId,siteId\n12345,MCO\n";
    fs::write(config_dir.join("users.csv"), users_content).unwrap();

    // Step 4: Load configuration and execute tests
    let config = HttpDiffConfig::load_from_file(config_dir.join("http-diff.toml")).unwrap();
    let user_data = load_user_data(config_dir.join("users.csv")).unwrap();

    assert_eq!(config.environments.len(), 2);
    assert_eq!(config.routes.len(), 1);
    assert_eq!(user_data.len(), 1);

    // Step 5: Execute HTTP diff tests
    let results = timeout(Duration::from_secs(30), 
        run_http_diff(config.clone(), Some(vec!["test".to_string(), "prod".to_string()]))
    ).await.unwrap().unwrap();

    // Step 6: Verify results
    assert_eq!(results.len(), 1); // 1 route * 1 user = 1 result
    let result = &results[0];
    assert_eq!(result.route_name, "health-check");
    assert!(result.is_identical); // Health check should be identical
    assert_eq!(result.responses.len(), 2);

    // Step 7: Generate and verify output files
    let curl_generator = CurlGenerator::new(config);
    let all_commands = curl_generator.generate_all_curl_commands(&user_data, &vec!["test".to_string(), "prod".to_string()]).unwrap();
    
    let curl_file_path = config_dir.join("curl_commands.sh");
    CurlGenerator::write_curl_commands_file(&all_commands, &curl_file_path).unwrap();
    
    let curl_content = fs::read_to_string(&curl_file_path).unwrap();
    assert!(curl_content.contains("#!/bin/bash"));
    assert!(curl_content.contains("health-check"));
    assert!(curl_content.contains(&test_server.uri()));
    assert!(curl_content.contains(&prod_server.uri()));

    // Step 8: Generate documentation
    let documentation = curl_generator.generate_request_documentation(&results).unwrap();
    assert!(documentation.contains("# HTTP Diff Test Documentation"));
    assert!(documentation.contains("- Total test scenarios: 1"));
    assert!(documentation.contains("- Identical responses: 1"));
    assert!(documentation.contains("- Different responses: 0"));
}

#[tokio::test]
async fn test_error_scenarios_and_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test 1: Missing configuration file
    let result = HttpDiffConfig::load_from_file(config_dir.join("nonexistent.toml"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), http_diff::error::HttpDiffError::ConfigNotFound { .. }));

    // Test 2: Invalid configuration file
    fs::write(config_dir.join("invalid.toml"), "invalid toml content [[[").unwrap();
    let result = HttpDiffConfig::load_from_file(config_dir.join("invalid.toml"));
    assert!(result.is_err());

    // Test 3: Configuration with no environments
    let empty_config = r#"[[routes]]
name = "test"
method = "GET"
path = "/test"
"#;
    fs::write(config_dir.join("empty.toml"), empty_config).unwrap();
    let result = HttpDiffConfig::load_from_file(config_dir.join("empty.toml"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), http_diff::error::HttpDiffError::NoEnvironments));

    // Test 4: Missing users.csv file
    create_test_config_files(config_dir).unwrap();
    fs::remove_file(config_dir.join("users.csv")).unwrap();
    
    let config = HttpDiffConfig::load_from_file(config_dir.join("http-diff.toml")).unwrap();
    let result = load_user_data(config_dir.join("users.csv"));
    assert!(result.is_err());

    // Test 5: Empty users.csv file
    fs::write(config_dir.join("users.csv"), "").unwrap();
    let result = load_user_data(config_dir.join("users.csv"));
    assert!(result.is_ok());
    let users = result.unwrap();
    assert!(users.is_empty());
}

#[tokio::test]
async fn test_configuration_template_generation() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test ensure_config_files_exist function
    let (config_generated, csv_generated) = ensure_config_files_exist(
        &config_dir.join("http-diff.toml").to_string_lossy(),
        &config_dir.join("users.csv").to_string_lossy(),
        true
    ).unwrap();

    assert!(config_generated);
    assert!(csv_generated);

    // Verify files were created
    assert!(config_dir.join("http-diff.toml").exists());
    assert!(config_dir.join("users.csv").exists());

    // Verify generated config is valid
    let config = HttpDiffConfig::load_from_file(config_dir.join("http-diff.toml")).unwrap();
    assert!(!config.environments.is_empty());
    assert!(!config.routes.is_empty());

    // Verify generated CSV is valid
    let users = load_user_data(config_dir.join("users.csv")).unwrap();
    assert!(!users.is_empty());
    assert!(users[0].data.contains_key("userId"));
    assert!(users[0].data.contains_key("siteId"));

    // Test that files won't be overwritten
    let original_config = fs::read_to_string(config_dir.join("http-diff.toml")).unwrap();
    let (config_generated2, csv_generated2) = ensure_config_files_exist(
        &config_dir.join("http-diff.toml").to_string_lossy(),
        &config_dir.join("users.csv").to_string_lossy(),
        true
    ).unwrap();

    assert!(!config_generated2); // Should not regenerate existing files
    assert!(!csv_generated2);
    
    let unchanged_config = fs::read_to_string(config_dir.join("http-diff.toml")).unwrap();
    assert_eq!(original_config, unchanged_config);
}

#[tokio::test]
async fn test_github_style_diff_output() {
    // Setup mock servers with different JSON responses
    let staging_server = MockServer::start().await;
    let prod_server = MockServer::start().await;

    // Create different JSON responses to generate a meaningful diff
    let staging_response = json!({
        "user_id": "123",
        "balance": {
            "available": 1000.50,
            "pending": 25.00,
            "currency": "USD"
        },
        "account_status": "active",
        "last_updated": "2024-01-15T10:30:00Z"
    });

    let prod_response = json!({
        "user_id": "123", 
        "balance": {
            "available": 850.75,
            "pending": 45.00,
            "currency": "USD"
        },
        "account_status": "active",
        "last_updated": "2024-01-15T11:15:00Z"
    });

    // Configure different responses for each environment
    Mock::given(method("GET"))
        .and(path("/api/users/123/balance"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(&staging_response)
            .insert_header("content-type", "application/json"))
        .mount(&staging_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/users/123/balance"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(&prod_response)
            .insert_header("content-type", "application/json"))
        .mount(&prod_server)
        .await;

    // Create test configuration
    let temp_dir = TempDir::new().unwrap();
    create_test_config_files(temp_dir.path()).unwrap();

    // Update config with mock server URLs and balance route
    let config_content = format!(r#"# HTTP Diff GitHub-Style Diff Test Configuration
[environments.staging]
base_url = "{}"

[environments.prod]
base_url = "{}"

[[routes]]
name = "user-balance"
method = "GET"
path = "/api/users/{{userId}}/balance"
"#, staging_server.uri(), prod_server.uri());

    fs::write(temp_dir.path().join("http-diff.toml"), config_content).unwrap();

    // Create user data
    let users_content = r#"userId
123"#;
    fs::write(temp_dir.path().join("users.csv"), users_content).unwrap();

    // Change to the temp directory so TestRunner can find users.csv
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Load the configuration and run the test
    let config = HttpDiffConfig::load_from_file("http-diff.toml").unwrap();
    let environments = Some(vec!["staging".to_string(), "prod".to_string()]);

    let result = run_http_diff(config, environments).await;
    
    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    if let Err(e) = &result {
        println!("Error running http_diff: {:?}", e);
    }
    assert!(result.is_ok());
    let results = result.unwrap();

    // Verify we have differences detected
    assert!(!results.is_empty());
    let comparison_result = &results[0];
    assert!(!comparison_result.is_identical);
    assert!(!comparison_result.differences.is_empty());

    // Test the formatted output contains GitHub-style diff
    let formatted_output = CurlGenerator::format_comparison_results(&results);
    println!("\n🎯 GitHub-Style Diff Output:\n{}", formatted_output);

    // Verify the output contains expected diff formatting
    assert!(formatted_output.contains("📍 Route 'user-balance'"));
    assert!(formatted_output.contains("📄 Response body differs"));
    assert!(formatted_output.contains("STAGING vs PROD"));
    assert!(formatted_output.contains("🔴")); // Delete lines  
    assert!(formatted_output.contains("🟢")); // Insert lines
    assert!(formatted_output.contains("1000.5")); // Original value
    assert!(formatted_output.contains("850.75")); // Changed value

    // Also test individual result formatting
    let single_result_output = CurlGenerator::format_single_result(&comparison_result);
    println!("\n🔍 Single Result Output:\n{}", single_result_output);
    
    assert!(single_result_output.contains("Route: user-balance"));
    assert!(single_result_output.contains("📄"));

    println!("✅ GitHub-style diff output test completed successfully!");
    
    // Verify that the diff shows the actual content differences
    let body_diff = comparison_result.differences.iter()
        .find(|d| d.category == http_diff::comparator::DifferenceCategory::Body)
        .expect("Should have a body difference");
    
    assert!(body_diff.diff_output.is_some());
    let diff_content = body_diff.diff_output.as_ref().unwrap();
    
    // Verify the diff contains the expected change indicators and values
    assert!(diff_content.contains("25.0"));  // pending amount changed
    assert!(diff_content.contains("45.0"));  // pending amount changed
    assert!(diff_content.contains("10:30:00")); // timestamp changed 
    assert!(diff_content.contains("11:15:00")); // timestamp changed
    
    println!("\n📊 Diff content verification passed!");
} 