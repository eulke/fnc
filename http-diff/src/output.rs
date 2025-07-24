// HttpResponse type is available in client module for future use
use crate::comparator::{ComparisonResult, Difference, DifferenceCategory, ErrorSummary};
use crate::config::{HttpDiffConfig, Route, UserData};
use crate::error::Result;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Generator for curl commands and output formatting
pub struct CurlGenerator {
    config: HttpDiffConfig,
}

/// Represents a curl command with metadata
#[derive(Debug, Clone)]
pub struct CurlCommand {
    pub route_name: String,
    pub environment: String,
    pub user_context: HashMap<String, String>,
    pub command: String,
}

impl CurlGenerator {
    /// Create a new curl generator
    pub fn new(config: HttpDiffConfig) -> Self {
        Self { config }
    }

    /// Generate curl command for a specific request with proper escaping
    pub fn generate_curl_command(
        &self,
        route: &Route,
        environment: &str,
        user_data: &UserData,
    ) -> Result<CurlCommand> {
        let base_url = self.config.get_base_url(route, environment)?;
        let path = self.substitute_path_parameters(&route.path, user_data)?;
        let url = format!("{}{}", base_url.trim_end_matches('/'), path);

        let mut command = format!("curl -X {} '{}'", route.method, self.escape_shell_argument(&url));

        // Add headers with proper escaping
        let headers = self.collect_headers(route, environment);
        for (key, value) in headers {
            command.push_str(&format!(" \\\n  -H '{}: {}'", 
                self.escape_shell_argument(&key), 
                self.escape_shell_argument(&value)
            ));
        }

        // Add query parameters with proper URL encoding
        let params = self.collect_query_parameters(route);
        if !params.is_empty() {
            let query_string = params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            
            command = command.replace(&format!("'{}'", self.escape_shell_argument(&url)), 
                                    &format!("'{}?{}'", self.escape_shell_argument(&url), query_string));
        }

        // Add body if present with proper escaping
        if let Some(body) = &route.body {
            let escaped_body = self.escape_shell_argument(body);
            command.push_str(&format!(" \\\n  -d '{}'", escaped_body));
        }

        Ok(CurlCommand {
            route_name: route.name.clone(),
            environment: environment.to_string(),
            user_context: user_data.data.clone(),
            command,
        })
    }

    /// Escape shell arguments to handle special characters properly
    fn escape_shell_argument(&self, arg: &str) -> String {
        // Handle single quotes by replacing them with '"'"'
        // This closes the current quote, adds an escaped quote, then opens a new quote
        arg.replace('\'', "'\"'\"'")
    }

    /// Generate curl commands for all test scenarios
    pub fn generate_all_curl_commands(
        &self,
        user_data: &[UserData],
        environments: &[String],
    ) -> Result<Vec<CurlCommand>> {
        let mut commands = Vec::new();

        for route in &self.config.routes {
            for env in environments {
                for user in user_data {
                    let command = self.generate_curl_command(route, env, user)?;
                    commands.push(command);
                }
            }
        }

        Ok(commands)
    }

    /// Write curl commands to a file with comprehensive documentation
    pub fn write_curl_commands_file<P: AsRef<Path>>(
        commands: &[CurlCommand],
        file_path: P,
    ) -> Result<()> {
        let file_name = file_path.as_ref().file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut file = File::create(&file_path)?;
        
        // Write header with timestamp and metadata
        writeln!(file, "#!/bin/bash")?;
        writeln!(file, "# HTTP Diff Test - Generated Curl Commands")?;
        writeln!(file, "# Generated at: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
        writeln!(file, "# Total commands: {}", commands.len())?;
        writeln!(file, "# Usage: bash {} or copy individual commands", file_name)?;
        writeln!(file)?;

        // Group commands by route for better organization
        let mut commands_by_route: HashMap<String, Vec<&CurlCommand>> = HashMap::new();
        for command in commands {
            commands_by_route
                .entry(command.route_name.clone())
                .or_insert_with(Vec::new)
                .push(&command);
        }

        for (route_name, route_commands) in commands_by_route {
            writeln!(file, "# ========================================")?;
            writeln!(file, "# Route: {}", route_name)?;
            writeln!(file, "# ========================================")?;
            writeln!(file)?;

            for command in route_commands {
                writeln!(file, "# Environment: {} | User: {:?}", 
                         command.environment, 
                         command.user_context)?;
                writeln!(file, "{}", command.command)?;
                writeln!(file)?;
            }
        }

        // Add footer with usage instructions
        writeln!(file, "# ========================================")?;
        writeln!(file, "# Usage Instructions:")?;
        writeln!(file, "# 1. Make this file executable: chmod +x {}", file_name)?;
        writeln!(file, "# 2. Run all commands: bash {}", file_name)?;
        writeln!(file, "# 3. Or copy individual curl commands for manual testing")?;
        writeln!(file, "# ========================================")?;

        Ok(())
    }

    /// Generate comprehensive request documentation
    pub fn generate_request_documentation(
        &self,
        results: &[ComparisonResult],
    ) -> Result<String> {
        let mut doc = String::new();
        
        doc.push_str("# HTTP Diff Test Documentation\n");
        doc.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        // Summary statistics
        let total_tests = results.len();
        let identical_count = results.iter().filter(|r| r.is_identical).count();
        let different_count = total_tests - identical_count;
        
        doc.push_str("## Test Summary\n");
        doc.push_str(&format!("- Total test scenarios: {}\n", total_tests));
        doc.push_str(&format!("- Identical responses: {}\n", identical_count));
        doc.push_str(&format!("- Different responses: {}\n", different_count));
        doc.push_str(&format!("- Success rate: {:.1}%\n\n", 
                             (identical_count as f32 / total_tests as f32) * 100.0));
        
        // Environment information
        if let Some(first_result) = results.first() {
            let environments: Vec<String> = first_result.responses.keys().cloned().collect();
            doc.push_str("## Environments Tested\n");
            for env in &environments {
                doc.push_str(&format!("- {}\n", env));
            }
            doc.push_str("\n");
        }
        
        // Route analysis
        let mut routes_analysis: HashMap<String, (usize, usize)> = HashMap::new();
        for result in results {
            let entry = routes_analysis.entry(result.route_name.clone()).or_insert((0, 0));
            if result.is_identical {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
        
        doc.push_str("## Route Analysis\n");
        for (route_name, (identical, different)) in routes_analysis {
            let total = identical + different;
            let success_rate = (identical as f32 / total as f32) * 100.0;
            doc.push_str(&format!("### {}\n", route_name));
            doc.push_str(&format!("- Total tests: {}\n", total));
            doc.push_str(&format!("- Identical: {} ({:.1}%)\n", identical, success_rate));
            doc.push_str(&format!("- Different: {} ({:.1}%)\n\n", different, 100.0 - success_rate));
        }
        
        // Differences summary
        if different_count > 0 {
            doc.push_str("## Differences Found\n");
            for result in results.iter().filter(|r| !r.is_identical) {
                doc.push_str(&format!("### {} (User: {:?})\n", result.route_name, result.user_context));
                for diff in &result.differences {
                    doc.push_str(&format!("- {:?}: {}\n", diff.category, diff.description));
                }
                doc.push_str("\n");
            }
        }
        
        Ok(doc)
    }

    /// Format comparison results for display
    pub fn format_comparison_results(results: &[ComparisonResult]) -> String {
        let mut output = String::new();
        
        let total_tests = results.len();
        let identical_count = results.iter().filter(|r| r.is_identical).count();
        let different_count = total_tests - identical_count;
        let success_rate = if total_tests > 0 {
            (identical_count as f32 / total_tests as f32) * 100.0
        } else {
            0.0
        };
        
        output.push_str(&format!("🔍 Test Results Summary: {} total, {} identical, {} different - Success Rate: {:.1}%\n", 
                                total_tests, identical_count, different_count, success_rate));
        
        // Generate error summary and add error analysis if there are failures
        let error_summary = ErrorSummary::from_comparison_results(results);
        if error_summary.failed_requests > 0 {
            output.push_str(&Self::format_error_analysis(&error_summary, results));
        }
        
        if different_count > 0 {
            output.push_str("\n❌ Differences Found:\n");
            for result in results.iter().filter(|r| !r.is_identical) {
                output.push_str(&format!("\n📍 Route '{}' (User: {:?})\n", 
                                        result.route_name, result.user_context));
                for diff in &result.differences {
                    output.push_str(&Self::format_difference(diff));
                }
            }
        } else {
            output.push_str("\n✅ All responses are identical across environments!");
        }
        
        output
    }

    /// Format a single comparison result
    pub fn format_single_result(result: &ComparisonResult) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("Route: {} | User: {:?}\n", 
                                result.route_name, 
                                result.user_context));
        
        for difference in &result.differences {
            output.push_str(&Self::format_difference(difference));
        }

        output
    }

    /// Format a single difference
    pub fn format_difference(difference: &Difference) -> String {
        let mut output = String::new();
        
        let icon = match difference.category {
            DifferenceCategory::Status => "🔢",
            DifferenceCategory::Headers => "📋",
            DifferenceCategory::Body => "📄",
        };
        
        output.push_str(&format!("  {} {}\n", icon, difference.description));
        
        if let Some(diff_output) = &difference.diff_output {
            output.push_str("\n");
            // Display diff output without additional indentation since it's already formatted
            output.push_str(diff_output);
            output.push_str("\n");
        }

        output
    }

    /// Format error analysis section
    fn format_error_analysis(error_summary: &ErrorSummary, results: &[ComparisonResult]) -> String {
        let mut output = String::new();
        
        output.push_str("\n==== Error Analysis ====\n");
        output.push_str(&format!("🚨 {} requests failed (non-2xx status codes)\n", error_summary.failed_requests));
        
        if error_summary.identical_failures > 0 {
            output.push_str(&format!("⚠️  {} requests failed identically across environments\n", error_summary.identical_failures));
        }
        
        if error_summary.mixed_responses > 0 {
            output.push_str(&format!("🔄 {} requests had different status codes across environments\n", error_summary.mixed_responses));
        }
        
        // Show detailed error information for failed requests
        let failed_results: Vec<&ComparisonResult> = results.iter().filter(|r| r.has_errors).collect();
        for result in failed_results {
            output.push_str(&Self::format_failed_request_details(result));
        }
        
        output
    }

    /// Format detailed information for a failed request
    fn format_failed_request_details(result: &ComparisonResult) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("\n📍 Route '{}'\n", result.route_name));
        
        // Display status codes
        output.push_str("  Status codes:\n");
        for (env, status) in &result.status_codes {
            output.push_str(&format!("    {}: {}\n", env, status));
        }
        
        // Display error response bodies if available
        if let Some(error_bodies) = &result.error_bodies {
            output.push_str("  Response bodies:\n");
            for (env, body) in error_bodies {
                let truncated_body = Self::truncate_response_body(body, 500);
                output.push_str(&format!("    {}: {}\n", env, truncated_body));
            }
        }
        
        output
    }

    /// Truncate response body for display
    fn truncate_response_body(body: &str, max_length: usize) -> String {
        if body.len() <= max_length {
            body.to_string()
        } else {
            format!("{}... (truncated)", &body[..max_length])
        }
    }

    /// Substitute path parameters like {userId} with actual values (same as HttpClient)
    fn substitute_path_parameters(&self, path: &str, user_data: &UserData) -> Result<String> {
        let mut result = path.to_string();
        
        // Find all parameters in the format {param_name}
        while let Some(start) = result.find('{') {
            if let Some(end) = result[start..].find('}') {
                let param_name = &result[start + 1..start + end];
                
                let value = user_data.data.get(param_name)
                    .ok_or_else(|| crate::error::HttpDiffError::MissingPathParameter {
                        param: param_name.to_string(),
                    })?;
                
                // URL encode the parameter value for safety
                let encoded_value = urlencoding::encode(value);
                result.replace_range(start..start + end + 1, &encoded_value);
            } else {
                break;
            }
        }
        
        Ok(result)
    }

    /// Collect all headers for a request
    fn collect_headers(&self, route: &Route, environment: &str) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        // Add global headers
        if let Some(global) = &self.config.global {
            if let Some(global_headers) = &global.headers {
                headers.extend(global_headers.clone());
            }
        }

        // Add environment-specific headers
        if let Some(env) = self.config.environments.get(environment) {
            if let Some(env_headers) = &env.headers {
                headers.extend(env_headers.clone());
            }
        }

        // Add route-specific headers (these take precedence)
        if let Some(route_headers) = &route.headers {
            headers.extend(route_headers.clone());
        }

        headers
    }

    /// Collect all query parameters for a request
    fn collect_query_parameters(&self, route: &Route) -> HashMap<String, String> {
        let mut params = HashMap::new();

        // Add global parameters
        if let Some(global) = &self.config.global {
            if let Some(global_params) = &global.params {
                params.extend(global_params.clone());
            }
        }

        // Add route-specific parameters
        if let Some(route_params) = &route.params {
            params.extend(route_params.clone());
        }

        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Environment, Route};
    use tempfile::TempDir;
    use std::fs;

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

        let routes = vec![Route {
            name: "user-profile".to_string(),
            method: "GET".to_string(),
            path: "/api/users/{userId}".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Accept".to_string(), "application/json".to_string());
                headers
            }),
            params: None,
            base_urls: None,
            body: None,
        }];

        HttpDiffConfig {
            environments,
            global: None,
            routes,
        }
    }

    fn create_test_user_data() -> UserData {
        let mut data = HashMap::new();
        data.insert("userId".to_string(), "12345".to_string());
        UserData { data }
    }

    #[test]
    fn test_curl_command_generation() {
        let config = create_test_config();
        let generator = CurlGenerator::new(config);

        let user_data = create_test_user_data();
        let route = &generator.config.routes[0];
        let command = generator.generate_curl_command(route, "test", &user_data).unwrap();

        assert_eq!(command.route_name, "user-profile");
        assert_eq!(command.environment, "test");
        assert!(command.command.contains("GET"));
        assert!(command.command.contains("https://api-test.example.com/api/users/12345"));
        assert!(command.command.contains("X-Scope: test"));
        assert!(command.command.contains("Accept: application/json"));
    }

    #[test]
    fn test_curl_command_with_special_characters() {
        let mut config = create_test_config();
        
        // Add route with special characters in body and headers
        config.routes.push(Route {
            name: "special-chars".to_string(),
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer token'with'quotes".to_string());
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            }),
            params: None,
            base_urls: None,
            body: Some(r#"{"message": "Hello 'world' with \"quotes\" and $special chars!"}"#.to_string()),
        });

        let generator = CurlGenerator::new(config);
        let user_data = create_test_user_data();
        let route = &generator.config.routes[1]; // Use the special chars route
        
        let command = generator.generate_curl_command(route, "test", &user_data).unwrap();
        
        // Verify proper escaping of single quotes in headers and body
        // The actual escaping pattern is '"'"' (close quote, escaped quote, open quote)
        assert!(command.command.contains("Bearer token'\"'\"'with'\"'\"'quotes"));
        
        // Update the expected body pattern to match actual escaping
        // Single quotes become '"'"' in shell escaping
        let expected_body_pattern = r#"Hello '"'"'world'"'"' with \"quotes\""#;
        assert!(command.command.contains(expected_body_pattern));
        
        // Verify the command structure is valid
        assert!(command.command.starts_with("curl -X POST"));
        assert!(command.command.contains("-H 'Authorization:"));
        assert!(command.command.contains("-d '{"));
    }

    #[test]
    fn test_curl_command_with_url_encoding() {
        let config = create_test_config();
        
        // Modify the user data to include special characters that need URL encoding
        let mut user_data = HashMap::new();
        user_data.insert("userId".to_string(), "user@example.com".to_string());
        let user_data = UserData { data: user_data };

        let generator = CurlGenerator::new(config);
        let route = &generator.config.routes[0];
        
        let command = generator.generate_curl_command(route, "test", &user_data).unwrap();
        
        // Should contain URL-encoded userId in path
        // Note: The URL encoding happens in the path substitution, not in our curl generation
        assert!(command.command.contains("user%40example.com"));
    }

    #[test]
    fn test_escape_shell_argument() {
        let config = create_test_config();
        let generator = CurlGenerator::new(config);

        // Test various special characters
        assert_eq!(generator.escape_shell_argument("simple"), "simple");
        assert_eq!(generator.escape_shell_argument("with'quote"), "with'\"'\"'quote");
        assert_eq!(generator.escape_shell_argument("multiple'single'quotes"), "multiple'\"'\"'single'\"'\"'quotes");
        assert_eq!(generator.escape_shell_argument("no'quotes'here"), "no'\"'\"'quotes'\"'\"'here");
    }

    #[test]
    fn test_curl_commands_file_generation() {
        let config = create_test_config();
        let generator = CurlGenerator::new(config);

        let user_data = vec![create_test_user_data()];
        let environments = vec!["test".to_string()];
        
        let commands = generator.generate_all_curl_commands(&user_data, &environments).unwrap();
        assert_eq!(commands.len(), 1);

        // Test file generation with timestamp
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("curl_commands.sh");
        
        CurlGenerator::write_curl_commands_file(&commands, &file_path).unwrap();
        
        let content = fs::read_to_string(&file_path).unwrap();
        
        // Verify file structure and content
        assert!(content.contains("#!/bin/bash"));
        assert!(content.contains("# HTTP Diff Test - Generated Curl Commands"));
        assert!(content.contains("# Generated at:"));
        assert!(content.contains("# Total commands: 1"));
        assert!(content.contains("# Usage: bash curl_commands.sh"));
        assert!(content.contains("# Route: user-profile"));
        assert!(content.contains("curl -X GET"));
        assert!(content.contains("# Usage Instructions:"));
        assert!(content.contains("chmod +x curl_commands.sh"));
    }

    #[test]
    fn test_curl_commands_file_with_multiple_routes() {
        let mut config = create_test_config();
        
        // Add another route
        config.routes.push(Route {
            name: "health-check".to_string(),
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
        });

        let generator = CurlGenerator::new(config);
        let user_data = vec![create_test_user_data()];
        let environments = vec!["test".to_string()];
        
        let commands = generator.generate_all_curl_commands(&user_data, &environments).unwrap();
        assert_eq!(commands.len(), 2); // 2 routes * 1 environment * 1 user

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_commands.sh");
        
        CurlGenerator::write_curl_commands_file(&commands, &file_path).unwrap();
        
        let content = fs::read_to_string(&file_path).unwrap();
        
        // Verify both routes are properly grouped
        assert!(content.contains("# Route: user-profile"));
        assert!(content.contains("# Route: health-check"));
        assert!(content.contains("# Total commands: 2"));
    }

    #[test]
    fn test_request_documentation_generation() {
        let config = create_test_config();
        let generator = CurlGenerator::new(config);

        // Create mock comparison results
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), crate::client::HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: "{}".to_string(),
            url: "https://test.example.com".to_string(),
            curl_command: "curl 'https://test.example.com'".to_string(),
        });

        let mut status_codes1 = HashMap::new();
        status_codes1.insert("prod".to_string(), 200u16);
        status_codes1.insert("staging".to_string(), 200u16);

        let result1 = ComparisonResult {
            route_name: "user-profile".to_string(),
            user_context: create_test_user_data().data,
            responses: responses.clone(),
            differences: vec![], // Identical
            is_identical: true,
            status_codes: status_codes1,
            has_errors: false,
            error_bodies: None,
        };

        let mut different_responses = responses.clone();
        different_responses.insert("prod".to_string(), crate::client::HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: "Not found".to_string(),
            url: "https://prod.example.com".to_string(),
            curl_command: "curl 'https://prod.example.com'".to_string(),
        });

        let mut status_codes2 = HashMap::new();
        status_codes2.insert("prod".to_string(), 404u16);
        status_codes2.insert("staging".to_string(), 200u16);

        let mut error_bodies2 = HashMap::new();
        error_bodies2.insert("prod".to_string(), "Not found".to_string());

        let result2 = ComparisonResult {
            route_name: "health-check".to_string(),
            user_context: create_test_user_data().data,
            responses: different_responses,
            differences: vec![Difference {
                category: DifferenceCategory::Status,
                description: "Status differs".to_string(),
                diff_output: None,
            }],
            is_identical: false,
            status_codes: status_codes2,
            has_errors: true,
            error_bodies: Some(error_bodies2),
        };

        let results = vec![result1, result2];
        let documentation = generator.generate_request_documentation(&results).unwrap();
        
        // Verify documentation content
        assert!(documentation.contains("# HTTP Diff Test Documentation"));
        assert!(documentation.contains("Generated:"));
        assert!(documentation.contains("## Test Summary"));
        assert!(documentation.contains("- Total test scenarios: 2"));
        assert!(documentation.contains("- Identical responses: 1"));
        assert!(documentation.contains("- Different responses: 1"));
        assert!(documentation.contains("## Environments Tested"));
        assert!(documentation.contains("## Route Analysis"));
        assert!(documentation.contains("### user-profile"));
        assert!(documentation.contains("### health-check"));
        assert!(documentation.contains("## Differences Found"));
    }

    #[test]
    fn test_format_comparison_results() {
        // Create test results with mixed outcomes
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), crate::client::HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: "{}".to_string(),
            url: "https://test.example.com".to_string(),
            curl_command: "curl 'https://test.example.com'".to_string(),
        });

        let mut status_codes_identical = HashMap::new();
        status_codes_identical.insert("test".to_string(), 200u16);

        let identical_result = ComparisonResult {
            route_name: "route1".to_string(),
            user_context: HashMap::new(),
            responses: responses.clone(),
            differences: vec![],
            is_identical: true,
            status_codes: status_codes_identical,
            has_errors: false,
            error_bodies: None,
        };

        let mut status_codes_different = HashMap::new();
        status_codes_different.insert("test".to_string(), 200u16);

        let different_result = ComparisonResult {
            route_name: "route2".to_string(),
            user_context: {
                let mut ctx = HashMap::new();
                ctx.insert("userId".to_string(), "123".to_string());
                ctx
            },
            responses: responses,
            differences: vec![Difference {
                category: DifferenceCategory::Body,
                description: "Body content differs".to_string(),
                diff_output: None,
            }],
            is_identical: false,
            status_codes: status_codes_different,
            has_errors: false,
            error_bodies: None,
        };

        let results = vec![identical_result, different_result];
        let output = CurlGenerator::format_comparison_results(&results);
        
        assert!(output.contains("🔍 Test Results Summary: 2 total, 1 identical, 1 different - Success Rate: 50.0%"));
        assert!(output.contains("❌ Differences Found:"));
        assert!(output.contains("📍 Route 'route2'"));
        assert!(output.contains("📄 Body content differs"));
    }

    #[test]
    fn test_format_comparison_results_all_identical() {
        let mut responses = HashMap::new();
        responses.insert("test".to_string(), crate::client::HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: "{}".to_string(),
            url: "https://test.example.com".to_string(),
            curl_command: "curl 'https://test.example.com'".to_string(),
        });

        let mut status_codes_all_identical = HashMap::new();
        status_codes_all_identical.insert("test".to_string(), 200u16);

        let result = ComparisonResult {
            route_name: "route1".to_string(),
            user_context: HashMap::new(),
            responses,
            differences: vec![],
            is_identical: true,
            status_codes: status_codes_all_identical,
            has_errors: false,
            error_bodies: None,
        };

        let results = vec![result];
        let output = CurlGenerator::format_comparison_results(&results);
        
        assert!(output.contains("🔍 Test Results Summary: 1 total, 1 identical, 0 different - Success Rate: 100.0%"));
        assert!(output.contains("✅ All responses are identical across environments!"));
        assert!(!output.contains("❌ Differences Found:"));
    }

    #[test]
    fn test_generate_all_curl_commands() {
        let mut config = create_test_config();
        
        // Add another environment
        config.environments.insert(
            "prod".to_string(),
            Environment {
                base_url: "https://api.example.com".to_string(),
                headers: None,
            },
        );

        let generator = CurlGenerator::new(config);
        
        let user_data = vec![
            create_test_user_data(),
            {
                let mut data = HashMap::new();
                data.insert("userId".to_string(), "67890".to_string());
                UserData { data }
            }
        ];
        let environments = vec!["test".to_string(), "prod".to_string()];
        
        let commands = generator.generate_all_curl_commands(&user_data, &environments).unwrap();
        
        // Should generate: 1 route * 2 environments * 2 users = 4 commands
        assert_eq!(commands.len(), 4);
        
        // Verify different combinations exist
        let test_commands: Vec<_> = commands.iter().filter(|c| c.environment == "test").collect();
        let prod_commands: Vec<_> = commands.iter().filter(|c| c.environment == "prod").collect();
        
        assert_eq!(test_commands.len(), 2);
        assert_eq!(prod_commands.len(), 2);
        
        // Verify different users
        let user_12345_commands: Vec<_> = commands.iter()
            .filter(|c| c.user_context.get("userId") == Some(&"12345".to_string()))
            .collect();
        let user_67890_commands: Vec<_> = commands.iter()
            .filter(|c| c.user_context.get("userId") == Some(&"67890".to_string()))
            .collect();
        
        assert_eq!(user_12345_commands.len(), 2);
        assert_eq!(user_67890_commands.len(), 2);
    }
} 