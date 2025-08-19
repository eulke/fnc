use crate::config::HttpDiffConfig;
use crate::error::{HttpDiffError, Result};
use crate::execution::progress::{ProgressCallback, ProgressTracker};
use crate::traits::{HttpClient, ResponseComparator, TestRunner};
use crate::types::{ExecutionError, ExecutionResult};
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Type alias for the most common concrete TestRunner implementation
pub type DefaultTestRunner =
    TestRunnerImpl<crate::http::HttpClientImpl, crate::comparison::ResponseComparator>;

/// Test runner implementation
pub struct TestRunnerImpl<C, R>
where
    C: HttpClient + 'static,
    R: ResponseComparator + 'static,
{
    config: HttpDiffConfig,
    client: Arc<C>,
    comparator: Arc<R>,
    max_concurrent_requests: usize,
}

impl<C, R> TestRunnerImpl<C, R>
where
    C: HttpClient + 'static,
    R: ResponseComparator + 'static,
{
    /// Create a new test runner
    pub fn new(config: HttpDiffConfig, client: C, comparator: R) -> Result<Self> {
        // Extract max_concurrent_requests from config, defaulting to 10
        let max_concurrent_requests = config
            .global
            .as_ref()
            .and_then(|g| g.max_concurrent_requests)
            .unwrap_or(10);

        Ok(Self {
            config,
            client: Arc::new(client),
            comparator: Arc::new(comparator),
            max_concurrent_requests,
        })
    }

    /// Configure maximum concurrent requests
    pub fn with_max_concurrent_requests(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent_requests = max_concurrent.max(1);
        self
    }

    /// Execute tests concurrently with controlled parallelism and streaming progress
    async fn execute_concurrent(
        &self,
        user_data: &[crate::config::UserData],
        environments: &[String],
        routes: &[&crate::config::Route],
        progress_callback: Option<Box<ProgressCallback>>,
    ) -> Result<ExecutionResult> {
        // Calculate total requests and set up progress tracking
        let total_requests = routes.len() * user_data.len() * environments.len();
        let mut progress = ProgressTracker::new(total_requests);

        if let Some(ref callback) = progress_callback {
            callback(&progress);
        }

        // Use semaphore for concurrency limiting
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));

        // Data structures to collect responses and create comparisons
        let mut route_user_responses: HashMap<
            (String, usize),
            HashMap<String, crate::types::HttpResponse>,
        > = HashMap::new();
        let mut results = Vec::new();
        let mut all_errors = Vec::new();

        // Create individual request tasks (one per request, not per route-user combination)
        let mut request_tasks: FuturesUnordered<
            tokio::task::JoinHandle<
                Result<(
                    usize,
                    usize,
                    String,
                    Option<crate::types::HttpResponse>,
                    bool,
                    Option<ExecutionError>,
                )>,
            >,
        > = FuturesUnordered::new();

        for (route_idx, route) in routes.iter().enumerate() {
            for (user_idx, user) in user_data.iter().enumerate() {
                for env in environments {
                    let route_arc = Arc::new((*route).clone());
                    let user_arc = Arc::new(user.clone());
                    let env_name = env.clone();
                    let client = self.client.clone();
                    let semaphore_clone = semaphore.clone();

                    let task = tokio::spawn(async move {
                        let _permit = semaphore_clone.acquire().await.map_err(|e| {
                            HttpDiffError::general(format!("Failed to acquire semaphore: {}", e))
                        })?;

                        match client
                            .execute_request(&route_arc, &env_name, &user_arc)
                            .await
                        {
                            Ok(response) => {
                                let success = response.is_success();
                                Ok((route_idx, user_idx, env_name, Some(response), success, None))
                            }
                            Err(e) => {
                                let error = ExecutionError::request_error(
                                    route_arc.name.clone(),
                                    env_name.clone(),
                                    e.to_string(),
                                );
                                Ok((route_idx, user_idx, env_name, None, false, Some(error)))
                            }
                        }
                    });

                    request_tasks.push(task);
                }
            }
        }

        // Process requests as they complete for streaming progress updates
        while let Some(task_result) = request_tasks.next().await {
            match task_result {
                Ok(Ok((route_idx, user_idx, env_name, response_opt, success, error_opt))) => {
                    // Update progress immediately for each completed request
                    progress.request_completed(success);

                    if let Some(ref callback) = progress_callback {
                        callback(&progress);
                    }

                    // Collect response for later comparison
                    if let Some(response) = response_opt {
                        let key = (routes[route_idx].name.clone(), user_idx);
                        route_user_responses
                            .entry(key)
                            .or_insert_with(HashMap::new)
                            .insert(env_name, response);
                    }

                    // Collect errors
                    if let Some(error) = error_opt {
                        all_errors.push(error);
                    }
                }
                Ok(Err(e)) => {
                    // Task completed but returned an error
                    progress.request_completed(false);
                    if let Some(ref callback) = progress_callback {
                        callback(&progress);
                    }

                    let error = ExecutionError::general_execution_error(e.to_string());
                    all_errors.push(error);
                }
                Err(e) => {
                    // Task itself failed (JoinError)
                    progress.request_completed(false);
                    if let Some(ref callback) = progress_callback {
                        callback(&progress);
                    }

                    let error =
                        ExecutionError::general_execution_error(format!("Task panicked: {}", e));
                    all_errors.push(error);
                }
            }
        }

        // Now create comparisons from collected responses
        for ((route_name, user_idx), responses) in route_user_responses {
            if responses.len() >= 2 {
                let user = &user_data[user_idx];

                // Determine base env from config (if any)
                let base_env_opt = self
                    .config
                    .environments
                    .iter()
                    .find(|(_k, v)| v.is_base)
                    .map(|(k, _)| k.clone());

                if let Some(base_env) = base_env_opt.clone() {
                    if responses.contains_key(&base_env) {
                        // Create one comparison per (base, other)
                        for (env, resp) in responses.iter() {
                            if env == &base_env {
                                continue;
                            }
                            let mut pair_map = HashMap::new();
                            if let Some(base_resp) = responses.get(&base_env) {
                                pair_map.insert(base_env.clone(), base_resp.clone());
                                pair_map.insert(env.clone(), resp.clone());
                            }

                            match self.comparator.compare_responses(
                                route_name.clone(),
                                user.data.clone(),
                                pair_map,
                            ) {
                                Ok(mut comparison_result) => {
                                    comparison_result.base_environment = Some(base_env.clone());
                                    results.push(comparison_result);
                                }
                                Err(e) => {
                                    let error = ExecutionError::comparison_error(
                                        route_name.clone(),
                                        e.to_string(),
                                    );
                                    all_errors.push(error);
                                }
                            }
                        }
                    } else {
                        // Fallback to comparing all responses if base not present
                        match self.comparator.compare_responses(
                            route_name.clone(),
                            user.data.clone(),
                            responses,
                        ) {
                            Ok(mut comparison_result) => {
                                comparison_result.base_environment = base_env_opt.clone();
                                results.push(comparison_result);
                            }
                            Err(e) => {
                                let error = ExecutionError::comparison_error(
                                    route_name.clone(),
                                    e.to_string(),
                                );
                                all_errors.push(error);
                            }
                        }
                    }
                } else {
                    // No base configured: compare all responses together
                    match self.comparator.compare_responses(
                        route_name.clone(),
                        user.data.clone(),
                        responses,
                    ) {
                        Ok(comparison_result) => {
                            results.push(comparison_result);
                        }
                        Err(e) => {
                            let error =
                                ExecutionError::comparison_error(route_name.clone(), e.to_string());
                            all_errors.push(error);
                        }
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
                        return Err(HttpDiffError::InvalidEnvironment {
                            environment: env.clone(),
                        });
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
                        let available_routes: Vec<String> =
                            self.config.routes.iter().map(|r| r.name.clone()).collect();
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
        progress_callback: Option<Box<ProgressCallback>>,
    ) -> Result<ExecutionResult> {
        let environments = self.resolve_environments(environments)?;
        let routes = self.resolve_routes(routes)?;

        self.execute_concurrent(user_data, &environments, &routes, progress_callback)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{Environment, HttpDiffConfig, Route};
    use crate::testing::mocks::{test_helpers::*, MockHttpClient, MockResponseComparator};

    #[tokio::test]
    async fn test_base_environment_pairing() {
        // Build config with base environment
        let mut environments = HashMap::new();
        environments.insert(
            "base".to_string(),
            Environment {
                base_url: "https://base.example.com".to_string(),
                headers: None,
                is_base: true,
            },
        );
        environments.insert(
            "other".to_string(),
            Environment {
                base_url: "https://other.example.com".to_string(),
                headers: None,
                is_base: false,
            },
        );

        let route = Route {
            name: "health".to_string(),
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
        };

        let config = HttpDiffConfig {
            environments,
            global: None,
            routes: vec![route.clone()],
        };

        // Prepare mock client responses
        let base_response = create_mock_response(200, "base ok");
        let other_response = create_mock_response(200, "other ok");
        let client = MockHttpClient::new()
            .with_response("health:base".to_string(), base_response)
            .with_response("health:other".to_string(), other_response);

        let comparator = MockResponseComparator::new();
        let runner = TestRunnerImpl::new(config, client, comparator).unwrap();

        let user_data = vec![create_mock_user_data(vec![])];
        let result = runner
            .execute_with_data(&user_data, None, None, None)
            .await
            .unwrap();

        // Expect a single comparison between base and other
        assert_eq!(result.comparisons.len(), 1);
        let cmp = &result.comparisons[0];
        assert_eq!(cmp.base_environment.as_deref(), Some("base"));
        assert!(cmp.responses.contains_key("base"));
        assert!(cmp.responses.contains_key("other"));
    }
}
