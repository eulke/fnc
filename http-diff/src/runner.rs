use crate::client::HttpClient;
use crate::comparator::ResponseComparator;
use crate::types::ComparisonResult;
use crate::config::{HttpDiffConfig, load_user_data};
use crate::error::{Result, HttpDiffError};
use std::collections::HashMap;

use std::time::Instant;

/// Main test runner for HTTP diff operations
pub struct TestRunner {
    config: HttpDiffConfig,
    client: HttpClient,
    comparator: ResponseComparator,
}

/// Progress tracking for concurrent execution
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    pub total_requests: usize,
    pub completed_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub start_time: Instant,
}

impl ProgressTracker {
    pub fn new(total_requests: usize) -> Self {
        Self {
            total_requests,
            completed_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            start_time: Instant::now(),
        }
    }

    pub fn request_completed(&mut self, success: bool) {
        self.completed_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.completed_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    pub fn elapsed_time(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn estimated_remaining(&self) -> Option<std::time::Duration> {
        if self.completed_requests == 0 || self.completed_requests >= self.total_requests {
            return None;
        }

        let elapsed = self.elapsed_time();
        let avg_time_per_request = elapsed.as_secs_f64() / self.completed_requests as f64;
        let remaining_requests = self.total_requests - self.completed_requests;
        let estimated_seconds = avg_time_per_request * remaining_requests as f64;

        Some(std::time::Duration::from_secs_f64(estimated_seconds))
    }
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(config: HttpDiffConfig) -> Result<Self> {
        let client = HttpClient::new(config.clone())?;
        let comparator = ResponseComparator::new(); // Default: headers disabled
        
        Ok(Self {
            config,
            client,
            comparator,
        })
    }

    /// Create a new test runner with headers comparison enabled
    pub fn with_headers_comparison(config: HttpDiffConfig) -> Result<Self> {
        let client = HttpClient::new(config.clone())?;
        let comparator = ResponseComparator::new().with_headers_comparison();
        
        Ok(Self {
            config,
            client,
            comparator,
        })
    }

    /// Create a new test runner with custom comparator settings
    pub fn with_comparator_settings(
        config: HttpDiffConfig,
        include_headers: bool,
        diff_view_style: crate::types::DiffViewStyle,
    ) -> Result<Self> {
        let client = HttpClient::new(config.clone())?;
        let mut comparator = ResponseComparator::new().with_diff_view_style(diff_view_style);
        
        if include_headers {
            comparator = comparator.with_headers_comparison();
        }
        
        Ok(Self {
            config,
            client,
            comparator,
        })
    }

    /// Execute HTTP diff tests with concurrent request execution and progress tracking
    pub async fn execute(&self, environments: Option<Vec<String>>) -> Result<Vec<ComparisonResult>> {
        self.execute_with_progress(environments, None).await.map(|(results, _)| results)
    }

    /// Execute HTTP diff tests with progress tracking callback
    pub async fn execute_with_progress(
        &self, 
        environments: Option<Vec<String>>,
        progress_callback: Option<Box<dyn Fn(&ProgressTracker) + Send + Sync>>,
    ) -> Result<(Vec<ComparisonResult>, ProgressTracker)>
    {
        let user_data = load_user_data("users.csv")?;
        let environments = self.resolve_environments(environments)?;
        
        // Calculate total number of requests
        let total_requests = self.config.routes.len() * user_data.len() * environments.len();
        let mut progress = ProgressTracker::new(total_requests);
        
        if let Some(ref callback) = progress_callback {
            callback(&progress);
        }

        let mut results = Vec::new();

        // Execute tests for each route and user combination
        for route in &self.config.routes {
            for user in &user_data {
                // Execute requests for this route-user combination
                let mut responses = HashMap::new();
                
                for env in &environments {
                    let response = self.client.execute_request(route, env, user).await?;
                    responses.insert(env.clone(), response);
                    
                    // Update progress
                    progress.request_completed(true);
                    if let Some(ref callback) = progress_callback {
                        callback(&progress);
                    }
                }

                // Compare responses
                let comparison_result = self.comparator.compare_responses(
                    route.name.clone(),
                    user.data.clone(),
                    responses,
                )?;

                results.push(comparison_result);
            }
        }

        Ok((results, progress))
    }



    /// Resolve environment names
    fn resolve_environments(&self, environments: Option<Vec<String>>) -> Result<Vec<String>> {
        match environments {
            Some(envs) => {
                // Validate that all requested environments exist
                for env in &envs {
                    if !self.config.environments.contains_key(env) {
                        return Err(HttpDiffError::InvalidEnvironment { environment: env.clone() });
                    }
                }
                Ok(envs)
            }
            None => {
                // Use all available environments
                Ok(self.config.environments.keys().cloned().collect())
            }
        }
    }
}

/// Convenience function to run HTTP diff with default settings
pub async fn run_http_diff(
    config: HttpDiffConfig,
    environments: Option<Vec<String>>,
) -> Result<Vec<ComparisonResult>> {
    let runner = TestRunner::new(config)?;
    runner.execute(environments).await
}

/// Convenience function to run HTTP diff with headers comparison enabled
pub async fn run_http_diff_with_headers(
    config: HttpDiffConfig,
    environments: Option<Vec<String>>,
) -> Result<Vec<ComparisonResult>> {
    let runner = TestRunner::with_headers_comparison(config)?;
    runner.execute(environments).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;
    use std::collections::HashMap;

    #[test]
    fn test_environment_resolution() {
        let mut environments = HashMap::new();
        environments.insert("test".to_string(), Environment {
            base_url: "http://test.example.com".to_string(),
            headers: None,
        });
        environments.insert("prod".to_string(), Environment {
            base_url: "http://prod.example.com".to_string(),
            headers: None,
        });

                 let config = HttpDiffConfig {
             environments,
             routes: vec![],
             global: None,
         };

        let runner = TestRunner::new(config).unwrap();

        // Test with specific environments
        let result = runner.resolve_environments(Some(vec!["test".to_string()]));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["test"]);

        // Test with non-existent environment
        let result = runner.resolve_environments(Some(vec!["nonexistent".to_string()]));
        assert!(result.is_err());

        // Test with no environments specified (should return all)
        let result = runner.resolve_environments(None);
        assert!(result.is_ok());
        let envs = result.unwrap();
        assert_eq!(envs.len(), 2);
        assert!(envs.contains(&"test".to_string()));
        assert!(envs.contains(&"prod".to_string()));
    }

    #[test]
    fn test_progress_tracker() {
        let mut tracker = ProgressTracker::new(10);
        
        assert_eq!(tracker.progress_percentage(), 0.0);
        assert_eq!(tracker.completed_requests, 0);
        assert_eq!(tracker.successful_requests, 0);
        assert_eq!(tracker.failed_requests, 0);

        // Complete some successful requests
        tracker.request_completed(true);
        tracker.request_completed(true);
        tracker.request_completed(false);

        assert_eq!(tracker.progress_percentage(), 30.0); // 3/10 * 100
        assert_eq!(tracker.completed_requests, 3);
        assert_eq!(tracker.successful_requests, 2);
        assert_eq!(tracker.failed_requests, 1);

        // Complete all requests
        for _ in 0..7 {
            tracker.request_completed(true);
        }

        assert_eq!(tracker.progress_percentage(), 100.0);
        assert_eq!(tracker.completed_requests, 10);
        assert_eq!(tracker.successful_requests, 9);
        assert_eq!(tracker.failed_requests, 1);
    }
} 