use std::fs;
use std::path::Path;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use http_diff::{
    config::{HttpDiffConfig, ensure_config_files_exist, load_user_data},
    run_http_diff,
    output::CurlGenerator,
};

/// Helper function to create test configuration files for CLI testing
fn create_cli_test_config_files(dir: &Path) -> std::io::Result<()> {
    let config_content = r#"[environments.test]
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
"#;

    let users_content = r#"userId,siteId,userName
12345,MCO,test_user
"#;

    let config_path = dir.join("http-diff.toml");
    let users_path = dir.join("users.csv");

    fs::write(config_path, config_content)?;
    fs::write(users_path, users_content)?;

    Ok(())
}

#[cfg(test)]
mod cli_error_analysis_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_displays_error_analysis_when_errors_present() {
        // Test that CLI integration properly displays error analysis section when errors are present
        
        let temp_dir = TempDir::new().unwrap();
        create_cli_test_config_files(temp_dir.path()).unwrap();

        // Setup mock servers - one returns 500, one returns 404
        let test_server = MockServer::start().await;
        let prod_server = MockServer::start().await;

        // Mock test environment - health endpoint returns 500 
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_string("Internal Server Error")
            )
            .mount(&test_server)
            .await;

        // Mock prod environment - health endpoint returns 500 (identical failure)
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_string("Internal Server Error")
            )
            .mount(&prod_server)
            .await;

        // Mock test environment - users endpoint returns 404
        Mock::given(method("GET"))
            .and(path("/api/users"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_string("Users not found")
            )
            .mount(&test_server)
            .await;

        // Mock prod environment - users endpoint returns 200 (mixed response)
        Mock::given(method("GET"))
            .and(path("/api/users"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{\"users\": []}")
            )
            .mount(&prod_server)
            .await;

        // Update config with actual server URLs
        let config_content = format!(r#"[environments.test]
base_url = "{}"

[environments.prod]  
base_url = "{}"

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "api/users"
method = "GET"
path = "/api/users"
"#, test_server.uri(), prod_server.uri());

        let config_path = temp_dir.path().join("http-diff.toml");
        fs::write(&config_path, config_content).unwrap();

        // Load configuration and run HTTP diff
        let config = HttpDiffConfig::load_from_file(&config_path).unwrap();
        let user_data = load_user_data(&temp_dir.path().join("users.csv").to_string_lossy()).unwrap();

        let results = timeout(Duration::from_secs(30), async {
            run_http_diff(config, user_data, Some(vec!["test".to_string(), "prod".to_string()]), false, None).await
        }).await.unwrap().unwrap();

        // Generate CLI output using the same function the CLI uses
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify error analysis section appears in CLI output
        assert!(cli_output.contains("==== Error Analysis ===="));
        assert!(cli_output.contains("🚨 2 requests failed (non-2xx status codes)"));
        assert!(cli_output.contains("⚠️  1 requests failed identically across environments"));
        assert!(cli_output.contains("🔄 1 requests had different status codes across environments"));

        // Verify specific route error details are shown
        assert!(cli_output.contains("📍 Route 'health'"));
        assert!(cli_output.contains("📍 Route 'api/users'"));
        assert!(cli_output.contains("Status codes:"));
        assert!(cli_output.contains("test: 500"));
        assert!(cli_output.contains("prod: 500"));
        assert!(cli_output.contains("test: 404"));
        assert!(cli_output.contains("prod: 200"));

        // Verify response bodies are shown for error responses
        assert!(cli_output.contains("Response bodies:"));
        assert!(cli_output.contains("Internal Server Error"));
        assert!(cli_output.contains("Users not found"));
    }

    #[tokio::test]
    async fn test_cli_no_error_analysis_when_all_successful() {
        // Test that CLI integration does NOT display error analysis when all requests are successful
        
        let temp_dir = TempDir::new().unwrap();
        create_cli_test_config_files(temp_dir.path()).unwrap();

        // Setup mock servers - both return 200
        let test_server = MockServer::start().await;
        let prod_server = MockServer::start().await;

        // Mock both environments with successful responses
        for server in [&test_server, &prod_server] {
            Mock::given(method("GET"))
                .and(path("/health"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string("OK")
                )
                .mount(server)
                .await;

            Mock::given(method("GET"))
                .and(path("/api/users"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string("{\"users\": []}")
                )
                .mount(server)
                .await;
        }

        // Update config with actual server URLs
        let config_content = format!(r#"[environments.test]
base_url = "{}"

[environments.prod]
base_url = "{}"

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "api/users"
method = "GET"
path = "/api/users"
"#, test_server.uri(), prod_server.uri());

        let config_path = temp_dir.path().join("http-diff.toml");
        fs::write(&config_path, config_content).unwrap();

        // Load configuration and run HTTP diff
        let config = HttpDiffConfig::load_from_file(&config_path).unwrap();
        let user_data = load_user_data(&temp_dir.path().join("users.csv").to_string_lossy()).unwrap();

        let results = timeout(Duration::from_secs(30), async {
            run_http_diff(config, user_data, Some(vec!["test".to_string(), "prod".to_string()]), false, None).await
        }).await.unwrap().unwrap();

        // Generate CLI output using the same function the CLI uses
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify NO error analysis section appears
        assert!(!cli_output.contains("==== Error Analysis ===="));
        assert!(!cli_output.contains("🚨"));
        assert!(!cli_output.contains("⚠️"));
        assert!(!cli_output.contains("🔄"));

        // Verify successful completion message
        assert!(cli_output.contains("✅ All responses are identical across environments!"));
        
        // Verify traditional success rate is displayed
        assert!(cli_output.contains("🔍 Test Results Summary"));
        assert!(cli_output.contains("Success Rate: 100.0%"));
    }

    #[tokio::test]
    async fn test_cli_backward_compatibility_with_existing_flags() {
        // Test that CLI integration maintains backward compatibility with existing functionality
        
        let temp_dir = TempDir::new().unwrap();
        create_cli_test_config_files(temp_dir.path()).unwrap();

        // Setup mock servers with different response bodies but same status codes
        let test_server = MockServer::start().await;
        let prod_server = MockServer::start().await;

        // Mock test environment
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("Test Environment OK")
            )
            .mount(&test_server)
            .await;

        // Mock prod environment with different body
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("Production Environment OK")
            )
            .mount(&prod_server)
            .await;

        // Update config
        let config_content = format!(r#"[environments.test]
base_url = "{}"

[environments.prod]
base_url = "{}"

[[routes]]
name = "health"
method = "GET"
path = "/health"
"#, test_server.uri(), prod_server.uri());

        let config_path = temp_dir.path().join("http-diff.toml");
        fs::write(&config_path, config_content).unwrap();

        // Load configuration and run HTTP diff
        let config = HttpDiffConfig::load_from_file(&config_path).unwrap();
        let user_data = load_user_data(&temp_dir.path().join("users.csv").to_string_lossy()).unwrap();

        let results = timeout(Duration::from_secs(30), async {
            run_http_diff(config, user_data, Some(vec!["test".to_string(), "prod".to_string()]), false, None).await
        }).await.unwrap().unwrap();

        // Generate CLI output
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify traditional success rate calculation is maintained (body differences = not identical)
        assert!(cli_output.contains("🔍 Test Results Summary: 1 total, 0 identical, 1 different - Success Rate: 0.0%"));
        
        // Verify traditional differences section is still shown for body differences
        assert!(cli_output.contains("❌ Differences Found:"));
        assert!(cli_output.contains("📍 Route 'health'"));
        
        // Verify NO error analysis section (both are 2xx status codes)
        assert!(!cli_output.contains("==== Error Analysis ===="));
        
        // This proves backward compatibility: differences in body/headers are still detected 
        // and reported as before, while error analysis only appears for status code issues
    }

    #[tokio::test]
    async fn test_cli_success_rate_calculation_unchanged() {
        // Test that the existing success rate calculation logic remains exactly the same
        
        let temp_dir = TempDir::new().unwrap();
        create_cli_test_config_files(temp_dir.path()).unwrap();

        // Setup servers with mixed identical and different responses
        let test_server = MockServer::start().await;
        let prod_server = MockServer::start().await;

        // Route 1: Identical responses (2xx)
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("OK")
            )
            .mount(&test_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("OK")
            )
            .mount(&prod_server)
            .await;

        // Route 2: Different response bodies but same status (2xx)
        Mock::given(method("GET"))
            .and(path("/api/users"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{\"users\": [\"test\"]}")
            )
            .mount(&test_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/users"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{\"users\": [\"prod\"]}")
            )
            .mount(&prod_server)
            .await;

        // Update config
        let config_content = format!(r#"[environments.test]
base_url = "{}"

[environments.prod]
base_url = "{}"

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "api/users"
method = "GET"
path = "/api/users"
"#, test_server.uri(), prod_server.uri());

        let config_path = temp_dir.path().join("http-diff.toml");
        fs::write(&config_path, config_content).unwrap();

        // Run test
        let config = HttpDiffConfig::load_from_file(&config_path).unwrap();
        let user_data = load_user_data(&temp_dir.path().join("users.csv").to_string_lossy()).unwrap();

        let results = timeout(Duration::from_secs(30), async {
            run_http_diff(config, user_data, Some(vec!["test".to_string(), "prod".to_string()]), false, None).await
        }).await.unwrap().unwrap();

        // Generate CLI output
        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify traditional success rate: 1 identical, 1 different = 50% success rate
        assert!(cli_output.contains("🔍 Test Results Summary: 2 total, 1 identical, 1 different - Success Rate: 50.0%"));
        
        // Verify differences section for body differences
        assert!(cli_output.contains("❌ Differences Found:"));
        
        // Verify NO error analysis (all 2xx responses)
        assert!(!cli_output.contains("==== Error Analysis ===="));
        
        // This confirms the existing success rate calculation is based on response similarity,
        // not HTTP status codes, and remains unchanged
    }

    #[tokio::test]
    async fn test_cli_enhanced_output_in_mixed_scenarios() {
        // Test CLI output with both traditional differences AND error analysis
        
        let temp_dir = TempDir::new().unwrap();
        
        // Add a third route for more comprehensive testing
        let config_content = r#"[environments.test]
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

[[routes]]
name = "api/status"
method = "GET"
path = "/api/status"
"#;

        let users_content = "userId,siteId,userName\n12345,MCO,test_user\n";
        
        let config_path = temp_dir.path().join("http-diff.toml");
        let users_path = temp_dir.path().join("users.csv");
        fs::write(&config_path, config_content).unwrap();
        fs::write(&users_path, users_content).unwrap();

        // Setup complex scenario
        let test_server = MockServer::start().await;
        let prod_server = MockServer::start().await;

        // Route 1: Successful but different bodies (traditional difference)
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Test OK"))
            .mount(&test_server).await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Prod OK"))
            .mount(&prod_server).await;

        // Route 2: Error responses - different status codes (error analysis)
        Mock::given(method("GET"))
            .and(path("/api/users"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&test_server).await;
        Mock::given(method("GET"))
            .and(path("/api/users"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Error"))
            .mount(&prod_server).await;

        // Route 3: Identical successful responses
        Mock::given(method("GET"))
            .and(path("/api/status"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Available"))
            .mount(&test_server).await;
        Mock::given(method("GET"))
            .and(path("/api/status"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Available"))
            .mount(&prod_server).await;

        // Update config with server URLs
        let config_content = format!(r#"[environments.test]
base_url = "{}"

[environments.prod]
base_url = "{}"

[[routes]]
name = "health"
method = "GET"
path = "/health"

[[routes]]
name = "api/users"
method = "GET"
path = "/api/users"

[[routes]]
name = "api/status"
method = "GET"
path = "/api/status"
"#, test_server.uri(), prod_server.uri());

        fs::write(&config_path, config_content).unwrap();

        // Run test
        let config = HttpDiffConfig::load_from_file(&config_path).unwrap();
        let user_data = load_user_data(&users_path.to_string_lossy()).unwrap();

        let results = timeout(Duration::from_secs(30), async {
            run_http_diff(config, user_data, Some(vec!["test".to_string(), "prod".to_string()]), false, None).await
        }).await.unwrap().unwrap();

        let cli_output = CurlGenerator::format_comparison_results(&results);

        // Verify comprehensive output includes both traditional and error analysis
        assert!(cli_output.contains("🔍 Test Results Summary: 3 total, 1 identical, 2 different"));
        
        // Verify error analysis section appears
        assert!(cli_output.contains("==== Error Analysis ===="));
        assert!(cli_output.contains("🚨 1 requests failed (non-2xx status codes)"));
        assert!(cli_output.contains("🔄 1 requests had different status codes across environments"));
        
        // Verify traditional differences section also appears
        assert!(cli_output.contains("❌ Differences Found:"));
        
        // Verify both types of issues are reported properly
        assert!(cli_output.contains("📍 Route 'health'")); // Body difference
        assert!(cli_output.contains("📍 Route 'api/users'")); // Status code difference
        
        // This demonstrates that both traditional diff reporting and new error analysis
        // work together seamlessly in the CLI
    }
} 