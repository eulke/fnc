//! Curl command generation and file operations
//!
//! This module handles the generation of curl commands from HTTP diff configurations
//! and provides utilities for writing curl commands to shell script files.

use crate::config::{HttpDiffConfig, Route, UserData};
use crate::error::Result;
use crate::url_builder::UrlBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Generator for curl commands and file operations
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

impl CurlCommand {
    /// Create a new curl command
    pub fn new(
        route_name: String,
        environment: String,
        user_context: HashMap<String, String>,
        command: String,
    ) -> Self {
        Self {
            route_name,
            environment,
            user_context,
            command,
        }
    }
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
        let url = UrlBuilder::new(&self.config, route, environment, user_data).build()?;
        let url_str = url.as_str();
        let mut command = format!("curl -X {} '{}'", route.method, escape_argument(url_str));

        // Add headers with CSV substitution and proper escaping
        let headers =
            crate::url_builder::resolve_headers(&self.config, route, environment, user_data)?;
        for (key, value) in headers {
            command.push_str(&format!(
                " \\\n  -H '{}: {}'",
                escape_argument(&key),
                escape_argument(&value)
            ));
        }

        // Add body with CSV substitution and proper escaping
        if let Some(body) = &route.body {
            let substituted_body = user_data.substitute_placeholders(body, false, false)?;
            let escaped_body = escape_argument(&substituted_body);
            command.push_str(&format!(" \\\n  -d '{}'", escaped_body));
        }

        Ok(CurlCommand {
            route_name: route.name.clone(),
            environment: environment.to_string(),
            user_context: user_data.data.clone(),
            command,
        })
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

    /// Write curl commands to a file using OutputManager for structured output
    pub fn write_curl_commands_file_managed<P: AsRef<Path>>(
        commands: &[CurlCommand],
        file_path: P,
        output_manager: &crate::output_manager::OutputManager,
    ) -> Result<std::path::PathBuf> {
        output_manager.ensure_structure()?;
        let resolved_path = output_manager
            .resolve_output_path(&file_path, crate::output_manager::OutputCategory::Scripts);

        Self::write_curl_commands_file(commands, &resolved_path)?;
        Ok(resolved_path)
    }

    /// Write curl commands to a file with comprehensive documentation
    pub fn write_curl_commands_file<P: AsRef<Path>>(
        commands: &[CurlCommand],
        file_path: P,
    ) -> Result<()> {
        let file_name = file_path
            .as_ref()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let mut file = File::create(&file_path)?;

        // Write header with timestamp and metadata
        writeln!(file, "#!/bin/bash")?;
        writeln!(file, "# HTTP Diff Test - Generated Curl Commands")?;
        writeln!(
            file,
            "# Generated at: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        writeln!(file, "# Total commands: {}", commands.len())?;
        writeln!(
            file,
            "# Usage: bash {} or copy individual commands",
            file_name
        )?;
        writeln!(file)?;

        // Group commands by route for better organization
        let mut commands_by_route: HashMap<String, Vec<&CurlCommand>> = HashMap::new();
        for command in commands {
            commands_by_route
                .entry(command.route_name.clone())
                .or_default()
                .push(command);
        }

        for (route_name, route_commands) in commands_by_route {
            writeln!(file, "# ========================================")?;
            writeln!(file, "# Route: {}", route_name)?;
            writeln!(file, "# ========================================")?;
            writeln!(file)?;

            for command in route_commands {
                writeln!(
                    file,
                    "# Environment: {} | User: {:?}",
                    command.environment, command.user_context
                )?;
                writeln!(file, "{}", command.command)?;
                writeln!(file)?;
            }
        }

        // Add footer with usage instructions
        writeln!(file, "# ========================================")?;
        writeln!(file, "# Usage Instructions:")?;
        writeln!(
            file,
            "# 1. Make this file executable: chmod +x {}",
            file_name
        )?;
        writeln!(file, "# 2. Run all commands: bash {}", file_name)?;
        writeln!(
            file,
            "# 3. Or copy individual curl commands for manual testing"
        )?;
        writeln!(file, "# ========================================")?;

        Ok(())
    }
}

/// Shell escaping utilities for curl command generation
mod shell_utils {
    /// Escape shell arguments to handle special characters properly
    pub(super) fn escape_argument(arg: &str) -> String {
        // Handle single quotes by replacing them with '"'"'
        // This closes the current quote, adds an escaped quote, then opens a new quote
        arg.replace('\'', "'\"'\"'")
    }
}

// Re-export shell utilities at module level for internal use
use shell_utils::escape_argument;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Environment, Route};
    use std::fs;
    use tempfile::TempDir;

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
                is_base: false,
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
        let command = generator
            .generate_curl_command(route, "test", &user_data)
            .unwrap();

        assert_eq!(command.route_name, "user-profile");
        assert_eq!(command.environment, "test");
        assert!(command.command.contains("GET"));
        assert!(command
            .command
            .contains("https://api-test.example.com/api/users/12345"));
        assert!(command.command.contains("X-Scope: test"));
        assert!(command.command.contains("Accept: application/json"));
    }

    #[test]
    fn test_curl_commands_file_generation() {
        let config = create_test_config();
        let generator = CurlGenerator::new(config);

        let user_data = vec![create_test_user_data()];
        let environments = vec!["test".to_string()];

        let commands = generator
            .generate_all_curl_commands(&user_data, &environments)
            .unwrap();
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
    fn test_curl_command_with_special_characters() {
        let mut config = create_test_config();

        // Add route with special characters in body and headers
        config.routes.push(Route {
            name: "special-chars".to_string(),
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert(
                    "Authorization".to_string(),
                    "Bearer token'with'quotes".to_string(),
                );
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            }),
            params: None,
            base_urls: None,
            body: Some(
                r#"{"message": "Hello 'world' with \"quotes\" and $special chars!"}"#.to_string(),
            ),
        });

        let generator = CurlGenerator::new(config);
        let user_data = create_test_user_data();
        let route = &generator.config.routes[1]; // Use the special chars route

        let command = generator
            .generate_curl_command(route, "test", &user_data)
            .unwrap();

        // Verify proper escaping of single quotes in headers and body
        // The actual escaping pattern is '"'"' (close quote, escaped quote, open quote)
        assert!(command
            .command
            .contains("Bearer token'\"'\"'with'\"'\"'quotes"));

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

        let command = generator
            .generate_curl_command(route, "test", &user_data)
            .unwrap();

        // Should contain URL-encoded userId in path
        // Note: The URL encoding happens in the path substitution, not in our curl generation
        assert!(command.command.contains("user%40example.com"));
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
                is_base: false,
            },
        );

        let generator = CurlGenerator::new(config);

        let user_data = vec![create_test_user_data(), {
            let mut data = HashMap::new();
            data.insert("userId".to_string(), "67890".to_string());
            UserData { data }
        }];
        let environments = vec!["test".to_string(), "prod".to_string()];

        let commands = generator
            .generate_all_curl_commands(&user_data, &environments)
            .unwrap();

        // Should generate: 1 route * 2 environments * 2 users = 4 commands
        assert_eq!(commands.len(), 4);

        // Verify different combinations exist
        let test_commands: Vec<_> = commands
            .iter()
            .filter(|c| c.environment == "test")
            .collect();
        let prod_commands: Vec<_> = commands
            .iter()
            .filter(|c| c.environment == "prod")
            .collect();

        assert_eq!(test_commands.len(), 2);
        assert_eq!(prod_commands.len(), 2);

        // Verify different users
        let user_12345_commands: Vec<_> = commands
            .iter()
            .filter(|c| c.user_context.get("userId") == Some(&"12345".to_string()))
            .collect();
        let user_67890_commands: Vec<_> = commands
            .iter()
            .filter(|c| c.user_context.get("userId") == Some(&"67890".to_string()))
            .collect();

        assert_eq!(user_12345_commands.len(), 2);
        assert_eq!(user_67890_commands.len(), 2);
    }
}
