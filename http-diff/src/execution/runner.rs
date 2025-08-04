use crate::config::HttpDiffConfig;
use crate::error::{Result, HttpDiffError};
use crate::traits::{HttpClient, ResponseComparator, TestRunner, ErrorCollector};
use crate::types::{ExecutionResult, ExecutionError};
use crate::execution::progress::{ProgressTracker, ProgressCallback};
use std::collections::HashMap;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;
use std::sync::Arc;

/// Test runner implementation
pub struct TestRunnerImpl<C, R> 
where 
    C: HttpClient,
    R: ResponseComparator,
{
    config: HttpDiffConfig,
    client: Arc<C>,
    comparator: Arc<R>,
    max_concurrent_requests: usize,
}

impl<C, R> TestRunnerImpl<C, R>
where
    C: HttpClient,
    R: ResponseComparator,
{
    /// Create a new test runner
    pub fn new(
        config: HttpDiffConfig,
        client: C,
        comparator: R,
    ) -> Result<Self> {
        Ok(Self {
            config,
            client: Arc::new(client),
            comparator: Arc::new(comparator),
            max_concurrent_requests: 10,
        })
    }

    /// Configure maximum concurrent requests
    pub fn with_max_concurrent_requests(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent_requests = max_concurrent.max(1);
        self
    }

    /// Execute tests concurrently with controlled parallelism and error collection
    async fn execute_concurrent_with_error_collection(
        &self,
        user_data: &[crate::config::UserData],
        environments: &[String],
        routes: &[&crate::config::Route],
        error_collector: Option<Box<dyn ErrorCollector>>,
        progress_callback: Option<Box<ProgressCallback>>,
    ) -> Result<ExecutionResult> {
        // Calculate total requests and set up progress tracking
        let total_requests = routes.len() * user_data.len() * environments.len();
        let mut progress = ProgressTracker::new(total_requests);
        
        if let Some(ref callback) = progress_callback {
            callback(&progress);
        }

        // Use semaphore to limit concurrent requests
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));
        let mut request_futures = FuturesUnordered::new();
        
        // Create all request tasks upfront
        for route in routes {
            for user in user_data {
                // For each route-user combination, execute requests to all environments
                let route_clone = (*route).clone();
                let user_clone = user.clone();
                let environments_clone = environments.to_vec();
                let client = self.client.clone();
                let semaphore_clone = semaphore.clone();
                
                let request_task = async move {
                    let mut responses = HashMap::new();
                    let mut request_results = Vec::new();
                    let mut errors = Vec::new();
                    
                    // Execute requests to all environments for this route-user combination
                    for env in &environments_clone {
                        let _permit = semaphore_clone.acquire().await.map_err(|e| {
                            HttpDiffError::general(format!("Failed to acquire semaphore: {}", e))
                        })?;
                        
                        match client.execute_request(&route_clone, env, &user_clone).await {
                            Ok(response) => {
                                let success = response.is_success();
                                responses.insert(env.clone(), response);
                                request_results.push(success);
                            }
                            Err(e) => {
                                // Collect error instead of printing to stderr
                                errors.push(ExecutionError::request_error(
                                    route_clone.name.clone(),
                                    env.clone(),
                                    e.to_string(),
                                ));
                                request_results.push(false);
                            }
                        }
                    }
                    
                    Result::<_>::Ok((route_clone, user_clone, responses, request_results, errors))
                };
                
                request_futures.push(request_task);
            }
        }

        let mut results = Vec::new();
        let mut all_errors = Vec::new();
        
        // Process completed requests as they finish
        while let Some(request_result) = request_futures.next().await {
            match request_result {
                Ok((route, user, responses, request_success_flags, mut errors)) => {
                    // Update progress for all requests in this batch
                    for success in &request_success_flags {
                        progress.request_completed(*success);
                    }
                    
                    if let Some(ref callback) = progress_callback {
                        callback(&progress);
                    }
                    
                    // Collect request errors
                    for error in &errors {
                        if let Some(ref collector) = error_collector {
                            collector.record_request_error(&error.route, error.environment.as_ref().unwrap_or(&"unknown".to_string()), error.message.clone());
                        }
                    }
                    all_errors.append(&mut errors);
                    
                    // Only create comparison result if we have at least 2 responses
                    if responses.len() >= 2 {
                        match self.comparator.compare_responses(
                            route.name.clone(),
                            user.data.clone(),
                            responses,
                        ) {
                            Ok(comparison_result) => {
                                results.push(comparison_result);
                            }
                            Err(e) => {
                                let error = ExecutionError::comparison_error(route.name.clone(), e.to_string());
                                if let Some(ref collector) = error_collector {
                                    collector.record_comparison_error(&error.route, error.message.clone());
                                }
                                all_errors.push(error);
                            }
                        }
                    }
                }
                Err(e) => {
                    let error = ExecutionError::general_execution_error(e.to_string());
                    if let Some(ref collector) = error_collector {
                        collector.record_execution_error(error.message.clone());
                    }
                    all_errors.push(error);
                    
                    // Still need to update progress for failed requests
                    for _ in 0..environments.len() {
                        progress.request_completed(false);
                    }
                    
                    if let Some(ref callback) = progress_callback {
                        callback(&progress);
                    }
                }
            }
        }

        Ok(ExecutionResult::new(results, progress, all_errors))
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

    /// Resolve route names and filter routes
    fn resolve_routes(&self, routes: Option<Vec<String>>) -> Result<Vec<&crate::config::Route>> {
        match routes {
            Some(route_names) => {
                let mut filtered_routes = Vec::new();
                
                // Validate that all requested routes exist and collect them
                for route_name in &route_names {
                    if let Some(route) = self.config.routes.iter().find(|r| r.name == *route_name) {
                        filtered_routes.push(route);
                    } else {
                        let available_routes: Vec<String> = self.config.routes.iter().map(|r| r.name.clone()).collect();
                        return Err(HttpDiffError::invalid_config(format!(
                            "Route '{}' not found in configuration. Available routes: {}",
                            route_name,
                            available_routes.join(", ")
                        )));
                    }
                }
                Ok(filtered_routes)
            }
            None => {
                // Use all available routes
                Ok(self.config.routes.iter().collect())
            }
        }
    }
}

impl<C, R> TestRunner for TestRunnerImpl<C, R>
where
    C: HttpClient,
    R: ResponseComparator,
{
    async fn execute_with_data(
        &self,
        user_data: &[crate::config::UserData],
        environments: Option<Vec<String>>,
        routes: Option<Vec<String>>,
        error_collector: Option<Box<dyn ErrorCollector>>,
        progress_callback: Option<Box<ProgressCallback>>,
    ) -> Result<ExecutionResult> {
        let environments = self.resolve_environments(environments)?;
        let routes = self.resolve_routes(routes)?;
        
        self.execute_concurrent_with_error_collection(user_data, &environments, &routes, error_collector, progress_callback).await
    }
}