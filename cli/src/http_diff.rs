use crate::error::{CliError, Result};

use crate::progress::ProgressTracker as CliProgressTracker;
use crate::ui;
use dialoguer::{Confirm, theme::ColorfulTheme};
use http_diff::{
    config::{load_user_data, HttpDiffConfig, ensure_config_files_exist},
    curl::CurlGenerator,
    DefaultTestRunner, DefaultHttpClient, DefaultResponseComparator,
    CliRenderer,
    OutputRenderer,
    TestRunner,
    ProgressTracker as HttpProgressTracker,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
pub struct HttpDiffArgs {
    pub environments: Option<String>,
    pub routes: Option<String>,
    pub include_headers: bool,
    pub include_errors: bool,
    pub diff_view: crate::cli::DiffViewType,
    pub config_path: String,
    pub users_file: String,
    pub init: bool,
    pub verbose: bool,
    pub output_file: String,
}

pub fn execute(args: HttpDiffArgs) -> Result<()> {
    // Create async runtime for HTTP operations
    let rt = Runtime::new().map_err(|e| {
        CliError::Other(format!("Failed to create async runtime: {}", e))
    })?;

    rt.block_on(execute_async(args.clone()))?;
    Ok(())
}

async fn execute_async(args: HttpDiffArgs) -> Result<()> {
    let config_path = Path::new(&args.config_path);
    let users_path = Path::new(&args.users_file);

    // Check if configuration files exist, create if needed
    if args.init || !config_path.exists() || !users_path.exists() {
        if args.init {
            ui::section_header("HTTP Diff Configuration Setup");
        } else {
            ui::warning_message("Configuration files not found");
        }

        let should_create = if args.init {
            true
        } else {
            Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Would you like to create default configuration files?")
                .interact()
                .map_err(|e| CliError::Other(format!("Failed to get user confirmation: {}", e)))?
        };

        if should_create {
            ui::status_message("Creating configuration files...");
            ensure_config_files_exist(
                &config_path.to_string_lossy(),
                &users_path.to_string_lossy(),
                true
            ).map_err(|e| {
                CliError::Other(format!("Failed to create configuration files: {}", e))
            })?;
            
            ui::success_message("Configuration files created successfully!");
            ui::info_message(&format!("Edit {} to configure your environments and routes", config_path.display()));
            ui::info_message(&format!("Edit {} to add test user data", users_path.display()));
            
            if !args.init {
                return Ok(());
            }
        } else {
            return Err(CliError::Other(
                "Configuration files are required to run HTTP diff tests".to_string()
            ));
        }
    }

    // Load configuration
    ui::status_message("Loading configuration...");
    let config = HttpDiffConfig::load_from_file(config_path).map_err(|e| {
        CliError::Other(format!("Failed to load configuration: {}", e))
    })?;

    // Validate that we have environments and routes
    if config.environments.is_empty() {
        return Err(CliError::Other(
            "No environments configured. Please add environments to your configuration file".to_string()
        ));
    }

    if config.routes.is_empty() {
        return Err(CliError::Other(
            "No routes configured. Please add routes to your configuration file".to_string()
        ));
    }

    ui::success_message(&format!(
        "Loaded configuration: {} environments, {} routes",
        config.environments.len(),
        config.routes.len()
    ));

    // Parse environment list
    let env_list = args.environments.as_ref().map(|env_str| {
        env_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>()
    });

    // Parse route list
    let route_list = args.routes.as_ref().map(|route_str| {
        route_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>()
    });

    // Validate environment names if specified
    if let Some(ref envs) = env_list {
        for env in envs {
            if !config.environments.contains_key(env) {
                return Err(CliError::Other(format!(
                    "Environment '{}' not found in configuration. Available environments: {}",
                    env,
                    config.environments.keys().cloned().collect::<Vec<_>>().join(", ")
                )));
            }
        }
    }

    // Validate route names if specified
    if let Some(ref routes) = route_list {
        let available_routes: Vec<String> = config.routes.iter().map(|r| r.name.clone()).collect();
        for route in routes {
            if !available_routes.contains(route) {
                return Err(CliError::Other(format!(
                    "Route '{}' not found in configuration. Available routes: {}",
                    route,
                    available_routes.join(", ")
                )));
            }
        }
    }

    // Load user data
    ui::status_message("Loading user test data...");
    let user_data = load_user_data(users_path).map_err(|e| {
        CliError::Other(format!("Failed to load user data: {}", e))
    })?;

    if user_data.is_empty() {
        ui::warning_message("No user data found. Tests will run without parameter substitution.");
    } else {
        ui::success_message(&format!("Loaded {} user records", user_data.len()));
    }

    // Setup progress tracking
    let env_count = env_list.as_ref().map(|e| e.len()).unwrap_or(config.environments.len());
    let route_count = route_list.as_ref().map(|r| r.len()).unwrap_or(config.routes.len());
    let total_tests = route_count * user_data.len().max(1) * env_count;
    
    let mut progress = CliProgressTracker::new("HTTP Diff Testing").with_steps(vec![
        "Initializing test runner".to_string(),
        format!("Executing {} HTTP tests", total_tests),
        "Comparing responses".to_string(),
        "Generating output files".to_string(),
    ]);

    // Initialize test runner with headers comparison and diff view settings
    progress.start_step();
    
    // Convert CLI DiffViewType to http-diff DiffViewStyle
    let diff_view_style = match args.diff_view {
        crate::cli::DiffViewType::Unified => http_diff::DiffViewStyle::Unified,
        crate::cli::DiffViewType::SideBySide => http_diff::DiffViewStyle::SideBySide,
    };
    
    // Create test runner with custom comparator settings
    let client = DefaultHttpClient::new(config.clone()).map_err(|e| {
        CliError::Other(format!("Failed to create HTTP client: {}", e))
    })?;
    
    let mut comparator = DefaultResponseComparator::new().with_diff_view_style(diff_view_style.clone());
    if args.include_headers {
        comparator = comparator.with_headers_comparison();
    }
    
    let runner = DefaultTestRunner::new(config.clone(), client, comparator).map_err(|e| {
        CliError::Other(format!("Failed to initialize test runner: {}", e))
    })?;
    progress.complete_step();

    // Execute HTTP diff tests with visual progress bar
    progress.start_step();
    if args.verbose {
        let env_names = env_list.as_ref()
            .map(|envs| envs.join(", "))
            .unwrap_or_else(|| config.environments.keys().cloned().collect::<Vec<_>>().join(", "));
        ui::info_message(&format!("Testing environments: {}", env_names));
        
        let route_names = route_list.as_ref()
            .map(|routes| routes.join(", "))
            .unwrap_or_else(|| config.routes.iter().map(|r| r.name.clone()).collect::<Vec<_>>().join(", "));
        ui::info_message(&format!("Testing routes: {}", route_names));
        
        ui::info_message(&format!("Headers comparison: {}", if args.include_headers { "enabled" } else { "disabled" }));
        let diff_view_name = match args.diff_view {
            crate::cli::DiffViewType::Unified => "unified",
            crate::cli::DiffViewType::SideBySide => "side-by-side",
        };
        ui::info_message(&format!("Diff view style: {}", diff_view_name));
    }

    // Create progress bar for HTTP requests
    let pb = Arc::new(ProgressBar::new(total_tests as u64));
    let style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg} (ETA: {eta})"
    ).unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("█▉▊▋▌▍▎▏  ");
    pb.set_style(style);
    pb.set_message("Executing HTTP requests...");

    // Execute with progress callback
    let pb_clone = Arc::clone(&pb);
    let (results, _final_progress) = runner.execute_with_progress(
        env_list,
        route_list,
        Some(Box::new(move |p: &HttpProgressTracker| {
            pb_clone.set_position(p.completed_requests as u64);
        }))
    ).await.map_err(|e| {
        CliError::Other(format!("HTTP diff execution failed: {}", e))
    })?;

    pb.finish_with_message("✅ All HTTP requests completed!");
    progress.complete_step();

    // Analyze and display results
    progress.start_step();
    let total_results = results.len();
    let identical_count = results.iter().filter(|r| r.is_identical).count();
    let different_count = total_results - identical_count;

    if args.verbose {
        ui::info_message(&format!("Test results: {} total, {} identical, {} different", 
                                 total_results, identical_count, different_count));
    }
    progress.complete_step();

    // Generate output files
    progress.start_step();
    let _curl_generator = CurlGenerator::new(config);
    
    // Generate curl commands file
    let mut curl_commands = Vec::new();
    for result in &results {
        for (env_name, response) in &result.responses {
            curl_commands.push(format!(
                "# Route: {} | Environment: {} | User: {:?}\n{}",
                result.route_name,
                env_name,
                result.user_context,
                response.curl_command
            ));
        }
    }

    let curl_content = format!(
        "# HTTP Diff Test - Curl Commands\n# Generated: {}\n\n{}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        curl_commands.join("\n\n")
    );

    fs::write(&args.output_file, curl_content).map_err(|e| {
        CliError::Other(format!("Failed to write curl commands file: {}", e))
    })?;

    ui::success_message(&format!("Curl commands saved to {}", args.output_file));
    progress.complete_step();

    progress.complete();

    // Display summary
    ui::section_header("Test Results Summary");
    let renderer = if args.include_errors {
        CliRenderer::new().with_diff_style(diff_view_style.clone())
    } else {
        CliRenderer::without_errors().with_diff_style(diff_view_style)
    };
    println!("{}", renderer.render(&results));

    // Show next steps if there are differences
    if different_count > 0 {
        ui::section_header("Next Steps");
        ui::step_message(1, "Review differences above");
        ui::step_message(2, &format!("Use curl commands from {} to reproduce issues", args.output_file));
        if !args.include_headers {
            ui::step_message(3, "Re-run with --include-headers to compare headers if needed");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_execute_with_missing_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("http-diff.toml");
        let users_path = temp_dir.path().join("users.csv");

        // Ensure files don't exist
        assert!(!config_path.exists());
        assert!(!users_path.exists());
        
        // When files don't exist and init=false, the function should prompt the user
        // This test just verifies the initial condition check
        assert!(!config_path.exists() || !users_path.exists());
    }

    #[tokio::test]
    async fn test_execute_with_init() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("http-diff.toml");
        let users_path = temp_dir.path().join("users.csv");

        // Just test that configuration files are created with init=true
        // Don't actually run HTTP tests since they would fail without real servers
        let result = http_diff::config::ensure_config_files_exist(
            &config_path.to_string_lossy(),
            &users_path.to_string_lossy(),
            true
        );

        // Should succeed and create configuration files
        assert!(result.is_ok());
        assert!(config_path.exists());
        assert!(users_path.exists());
        
        // Verify config content is valid
        let config = http_diff::config::HttpDiffConfig::load_from_file(&config_path);
        assert!(config.is_ok());
        
        let config = config.unwrap();
        assert!(!config.environments.is_empty());
        assert!(!config.routes.is_empty());
    }

    #[tokio::test]
    async fn test_environment_validation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("http-diff.toml");
        let users_path = temp_dir.path().join("users.csv");

        // Create minimal config
        fs::write(&config_path, r#"
[environments.test]
base_url = "https://api-test.example.com"

[[routes]]
name = "test-route"
method = "GET"
path = "/api/test"
"#).unwrap();

        fs::write(&users_path, "userId,siteId\n123,MCO\n").unwrap();

        // Test with invalid environment
        let result = execute_async(HttpDiffArgs {
            environments: Some("invalid_env".to_string()),
            routes: None,
            include_headers: false,
            include_errors: false,
            diff_view: crate::cli::DiffViewType::Unified,
            config_path: config_path.to_string_lossy().to_string(),
            users_file: users_path.to_string_lossy().to_string(),
            init: false,
            verbose: false,
            output_file: "curl_commands.txt".to_string(),
        }).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Environment 'invalid_env' not found"));
    }

    #[test]
    fn test_cli_argument_parsing() {
        use clap::Parser;
        use crate::cli::{Cli, Commands};

        // Test basic http-diff command
        let cli = Cli::try_parse_from(["fnc", "http-diff"]).unwrap();
        
        if let Commands::HttpDiff { environments, routes, include_headers, .. } = cli.command {
            assert_eq!(environments, None);
            assert_eq!(routes, None);
            assert!(!include_headers);
        } else {
            panic!("Expected HttpDiff command");
        }

        // Test with all flags
        let cli = Cli::try_parse_from([
            "fnc", "http-diff", 
            "--environments", "test,prod",
            "--include-headers",
            "--config", "custom.toml",
            "--users-file", "custom.csv",
            "--init",
            "--verbose",
            "--output-file", "output.txt"
        ]).unwrap();
        
        if let Commands::HttpDiff { 
            environments, 
            routes: _,
            include_headers,
            include_errors: _,
            diff_view: _,
            config,
            users_file,
            init,
            verbose,
            output_file,
        } = cli.command {
            assert_eq!(environments, Some("test,prod".to_string()));
            assert!(include_headers);
            assert_eq!(config, "custom.toml");
            assert_eq!(users_file, "custom.csv");
            assert!(init);
            assert!(verbose);
            assert_eq!(output_file, "output.txt");
        } else {
            panic!("Expected HttpDiff command");
        }
    }
}
