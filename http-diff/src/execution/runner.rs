use crate::config::HttpDiffConfig;
use crate::error::{HttpDiffError, Result};
use crate::execution::progress::{ProgressCallback, ProgressTracker};
use crate::execution::dependency::DependencyResolver;
use crate::execution::context::ContextManager;
use crate::extraction::ValueExtractionEngine;
use crate::traits::{ConditionEvaluator, HttpClient, ResponseComparator, TestRunner};
use crate::types::{ExecutionError, ExecutionResult, ExtractionResult, ExtractionRule, ExtractionType};
use crate::utils::environment_utils::EnvironmentOrderResolver;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

// Simplify complex task result types for readability and to satisfy clippy::type_complexity
type RequestTaskOutput = (
    usize,
    usize,
    String,
    String,
    crate::config::types::Route,
    Option<crate::types::HttpResponse>,
    bool,
    Option<ExecutionError>,
);
type RequestJoinHandle = tokio::task::JoinHandle<Result<RequestTaskOutput>>;

// Alias for executable route-user combination
type ExecutableCombination<'a> = (
    usize,
    usize,
    &'a crate::config::Route,
    &'a crate::config::UserData,
);

// Aliases for collected responses keyed by (route_name, user_idx)
type RouteUserKey = (String, usize);
type EnvHttpResponses = HashMap<String, crate::types::HttpResponse>;
type RouteUserResponses = HashMap<RouteUserKey, EnvHttpResponses>;

/// Type alias for the most common concrete TestRunner implementation
pub type DefaultTestRunner = TestRunnerImpl<
    crate::http::HttpClientImpl,
    crate::comparison::ResponseComparator,
    crate::conditions::ConditionEvaluatorImpl,
>;

/// Test runner implementation with chain execution support
pub struct TestRunnerImpl<C, R, E>
where
    C: HttpClient + 'static,
    R: ResponseComparator + 'static,
    E: ConditionEvaluator + 'static,
{
    config: HttpDiffConfig,
    client: Arc<C>,
    comparator: Arc<R>,
    condition_evaluator: Arc<E>,
    max_concurrent_requests: usize,
    /// Value extraction engine for chained requests
    extraction_engine: ValueExtractionEngine,
    /// Context manager for dynamic variable resolution
    context_manager: Arc<ContextManager>,
}

impl<C, R, E> TestRunnerImpl<C, R, E>
where
    C: HttpClient + 'static,
    R: ResponseComparator + 'static,
    E: ConditionEvaluator + 'static,
{
    /// Create a new test runner
    pub fn new(
        config: HttpDiffConfig,
        client: C,
        comparator: R,
        condition_evaluator: E,
    ) -> Result<Self> {
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
            condition_evaluator: Arc::new(condition_evaluator),
            max_concurrent_requests,
            extraction_engine: ValueExtractionEngine::new(),
            context_manager: Arc::new(ContextManager::new()),
        })
    }

    /// Configure maximum concurrent requests
    pub fn with_max_concurrent_requests(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent_requests = max_concurrent.max(1);
        self
    }

    /// Configure with custom value extraction engine
    pub fn with_extraction_engine(mut self, engine: ValueExtractionEngine) -> Self {
        self.extraction_engine = engine;
        self
    }

    /// Configure with custom context manager
    pub fn with_context_manager(mut self, manager: ContextManager) -> Self {
        self.context_manager = Arc::new(manager);
        self
    }

    /// Filter route-user combinations based on conditions for performance optimization
    fn filter_executable_combinations<'a>(
        &self,
        user_data: &'a [crate::config::UserData],
        routes: &'a [&'a crate::config::Route],
    ) -> Result<(Vec<ExecutableCombination<'a>>, usize)> {
        let mut executable_combinations = Vec::new();
        let mut skipped_count = 0;

        for (route_idx, route) in routes.iter().enumerate() {
            for (user_idx, user) in user_data.iter().enumerate() {
                match self.condition_evaluator.should_execute_route(route, user) {
                    Ok(should_execute) => {
                        if should_execute {
                            executable_combinations.push((route_idx, user_idx, *route, user));
                        } else {
                            skipped_count += 1;
                            // Note: Route skipped due to condition evaluation
                        }
                    }
                    Err(_e) => {
                        // Note: Condition evaluation failed, skipping route
                        skipped_count += 1;
                    }
                }
            }
        }

        Ok((executable_combinations, skipped_count))
    }

    /// Execute tests concurrently with controlled parallelism and streaming progress
    async fn execute_concurrent(
        &self,
        user_data: &[crate::config::UserData],
        environments: &[String],
        routes: &[&crate::config::Route],
        progress_callback: Option<Box<ProgressCallback>>,
    ) -> Result<ExecutionResult> {
        // Early filtering: Filter route-user combinations based on conditions
        let (executable_combinations, skipped_route_user_count) =
            self.filter_executable_combinations(user_data, routes)?;

        // Calculate total requests based on combinations that will actually execute
        let total_requests = executable_combinations.len() * environments.len();
        let mut progress = ProgressTracker::new(total_requests);

        // Track skipped routes in progress
        for _ in 0..skipped_route_user_count {
            progress.route_skipped();
        }

        if let Some(ref callback) = progress_callback {
            callback(&progress);
        }

        // Use semaphore for concurrency limiting
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));

        // Data structures to collect responses and create comparisons
        let mut route_user_responses: RouteUserResponses = HashMap::new();
        let mut results = Vec::new();
        let mut all_errors = Vec::new();

        // Create individual request tasks (one per request, only for executable combinations)
        let mut request_tasks: FuturesUnordered<RequestJoinHandle> = FuturesUnordered::new();

        for (route_idx, user_idx, route, user) in executable_combinations {
            for env in environments {
                let route_arc = Arc::new(route.clone());
                let user_arc = Arc::new(user.clone());
                let env_name = env.clone();
                let route_name = route.name.clone();
                let route_for_extraction = route.clone();
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
                            Ok((route_idx, user_idx, env_name, route_name, route_for_extraction, Some(response), success, None))
                        }
                        Err(e) => {
                            let error = ExecutionError::request_error(
                                route_arc.name.clone(),
                                env_name.clone(),
                                e.to_string(),
                            );
                            Ok((route_idx, user_idx, env_name, route_name, route_for_extraction, None, false, Some(error)))
                        }
                    }
                });

                request_tasks.push(task);
            }
        }

        // Process requests as they complete for streaming progress updates
        while let Some(task_result) = request_tasks.next().await {
            match task_result {
                Ok(Ok((_route_idx, user_idx, env_name, route_name, _route, response_opt, success, error_opt))) => {
                    // Update progress immediately for each completed request
                    progress.request_completed(success);

                    if let Some(ref callback) = progress_callback {
                        callback(&progress);
                    }

                    // Collect response for later comparison
                    if let Some(response) = response_opt {
                        let key = (route_name, user_idx);
                        route_user_responses
                            .entry(key)
                            .or_default()
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

                // Determine base environment from config (if any)
                let base_env_opt = self
                    .config
                    .environments
                    .iter()
                    .find(|(_k, v)| v.is_base)
                    .map(|(k, _)| k.clone());

                // Create environment resolver with consistent ordering
                let config_env_order: Vec<String> = self.config.environments.keys().cloned().collect();
                let _resolver = EnvironmentOrderResolver::new(&config_env_order, base_env_opt.clone());

                // Create unified comparison result with proper base environment
                match self.comparator.compare_responses(
                    route_name.clone(),
                    user.data.clone(),
                    responses,
                ) {
                    Ok(mut comparison_result) => {
                        // Ensure base environment is properly set from configuration
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
        }

        Ok(ExecutionResult::new(results, progress, all_errors, None))
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

    /// Check if routes have chaining requirements (dependencies or extraction rules)
    fn has_chaining_requirements(&self, routes: &[&crate::config::Route]) -> bool {
        routes.iter().any(|route| {
            route.depends_on.is_some() || 
            route.extract.is_some() || 
            route.wait_for_extraction.unwrap_or(false)
        })
    }

    /// Execute tests with dependency-aware chaining support
    async fn execute_with_chaining(
        &self,
        user_data: &[crate::config::UserData],
        environments: &[String],
        routes: &[&crate::config::Route],
        progress_callback: Option<Box<ProgressCallback>>,
    ) -> Result<ExecutionResult> {
        // Create dependency resolver and execution plan
        let owned_routes: Vec<_> = routes.iter().map(|&route| route.clone()).collect();
        let dependency_resolver = DependencyResolver::from_routes(&owned_routes)?;
        let execution_plan = dependency_resolver.compute_execution_plan()?;
        
        // Reset context manager for new execution
        self.context_manager.reset_all_scopes()?;
        
        // Early filtering: Filter route-user combinations based on conditions
        let (executable_combinations, skipped_route_user_count) =
            self.filter_executable_combinations(user_data, routes)?;
            

        // Calculate total requests based on combinations that will actually execute
        let total_requests = executable_combinations.len() * environments.len();
        let mut progress = ProgressTracker::new(total_requests);

        // Track skipped routes in progress
        for _ in 0..skipped_route_user_count {
            progress.route_skipped();
        }

        if let Some(ref callback) = progress_callback {
            callback(&progress);
        }

        // Data structures to collect responses and create comparisons
        let mut route_user_responses: RouteUserResponses = HashMap::new();
        let mut results = Vec::new();
        let mut all_errors = Vec::new();
        
        // Initialize chain execution metadata
        let mut chain_metadata = crate::types::ChainExecutionMetadata::new(execution_plan.batch_count());
        
        // Count routes with dependencies and extraction rules for metadata
        for route in routes {
            if route.depends_on.is_some() {
                chain_metadata.dependent_routes += 1;
            }
            if route.extract.is_some() {
                chain_metadata.extraction_routes += 1;
            }
        }
        
        // Execute routes in batches according to dependency plan
        for batch in &execution_plan.batches {
            let batch_result = self.execute_batch_with_extraction(
                &batch.routes,
                user_data,
                environments,
                &executable_combinations,
                &mut progress,
                &progress_callback,
            ).await?;
            
            // Collect responses and errors from batch execution
            for (key, responses) in batch_result.route_user_responses {
                route_user_responses.insert(key, responses);
            }
            all_errors.extend(batch_result.errors);
        }

        // Create comparisons from collected responses
        for ((route_name, user_idx), responses) in route_user_responses {
            if responses.len() >= 2 {
                let user = &user_data[user_idx];

                // Determine base environment from config (if any)
                let base_env_opt = self
                    .config
                    .environments
                    .iter()
                    .find(|(_k, v)| v.is_base)
                    .map(|(k, _)| k.clone());

                // Create unified comparison result with proper base environment
                match self.comparator.compare_responses(
                    route_name.clone(),
                    user.data.clone(),
                    responses,
                ) {
                    Ok(mut comparison_result) => {
                        // Ensure base environment is properly set from configuration
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
        }

        // Finalize chain metadata
        chain_metadata.total_extracted_values = self.context_manager
            .get_all_scope_stats(routes.len())?
            .iter()
            .map(|stats| stats.context_values)
            .sum();
            
        // Count extraction errors
        chain_metadata.extraction_errors = all_errors.iter()
            .filter(|err| matches!(err.error_type, crate::types::ExecutionErrorType::ExecutionError))
            .filter(|err| err.message.contains("extraction"))
            .count();
            
        // Check if there were dependency waits
        chain_metadata.had_dependency_waits = progress.dependency_wait_count > 0;
        
        Ok(crate::types::ExecutionResult::new(
            results, 
            progress, 
            all_errors, 
            Some(chain_metadata)
        ))
    }

    /// Execute a batch of routes with value extraction support
    async fn execute_batch_with_extraction(
        &self,
        batch_routes: &[String],
        user_data: &[crate::config::UserData],
        environments: &[String],
        executable_combinations: &[(usize, usize, &crate::config::Route, &crate::config::UserData)],
        progress: &mut ProgressTracker,
        progress_callback: &Option<Box<ProgressCallback>>,
    ) -> Result<BatchExecutionResult> {
        // Use semaphore for concurrency limiting within batch
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));
        
        // Filter combinations for routes in this batch
        let batch_combinations: Vec<_> = executable_combinations
            .iter()
            .filter(|(_, _, route, _)| batch_routes.contains(&route.name))
            .cloned()
            .collect();
        
        // Data structures for batch results
        let mut route_user_responses: HashMap<
            (String, usize),
            HashMap<String, crate::types::HttpResponse>,
        > = HashMap::new();
        let mut errors = Vec::new();
        let mut extraction_results: HashMap<(String, usize, String), ExtractionResult> = HashMap::new();
        
        // Create request tasks for this batch
        let mut request_tasks: FuturesUnordered<RequestJoinHandle> = FuturesUnordered::new();

        for (route_idx, user_idx, route, user) in batch_combinations {
            for env in environments {
                let route_arc = Arc::new(route.clone());
                let user_arc = Arc::new(user.clone());
                let env_name = env.clone();
                let route_name = route.name.clone();
                let route_for_extraction = route.clone();
                let client = self.client.clone();
                let context_manager = self.context_manager.clone();
                let semaphore_clone = semaphore.clone();

                let task = tokio::spawn(async move {
                    let _permit = semaphore_clone.acquire().await.map_err(|e| {
                        HttpDiffError::general(format!("Failed to acquire semaphore: {}", e))
                    })?;

                    // Create enhanced user data with context for parameter substitution
                    let enhanced_user_data = if context_manager.scope_count()? > 0 {
                        let context = context_manager.get_or_create_scope(user_idx)?.get_context().clone();
                        user_arc.with_context(&context)
                    } else {
                        user_arc.with_context(&crate::execution::context::DynamicContext::new())
                    };

                    let merged_user_data = enhanced_user_data.to_merged_user_data();
                    match client
                        .execute_request(&route_arc, &env_name, &merged_user_data)
                        .await
                    {
                        Ok(response) => {
                            let success = response.is_success();
                            Ok((route_idx, user_idx, env_name, route_name, route_for_extraction, Some(response), success, None))
                        }
                        Err(e) => {
                            let error = ExecutionError::request_error(
                                route_arc.name.clone(),
                                env_name.clone(),
                                e.to_string(),
                            );
                            Ok((route_idx, user_idx, env_name, route_name, route_for_extraction, None, false, Some(error)))
                        }
                    }
                });

                request_tasks.push(task);
            }
        }

        // Process batch requests as they complete
        while let Some(task_result) = request_tasks.next().await {
            match task_result {
                Ok(Ok((_route_idx, user_idx, env_name, route_name, route, response_opt, success, error_opt))) => {
                    // Update progress immediately for each completed request
                    progress.request_completed(success);

                    if let Some(ref callback) = progress_callback {
                        callback(progress);
                    }

                    // Collect response for later comparison
                    if let Some(response) = response_opt {
                        let key = (route_name.clone(), user_idx);
                        route_user_responses
                            .entry(key)
                            .or_default()
                            .insert(env_name.clone(), response.clone());
                        
                        // Perform value extraction if route has extraction rules
                        if let Some(extraction_rules) = &route.extract {
                            // Convert ValueExtractionRule to ExtractionRule
                            let converted_rules: Vec<ExtractionRule> = extraction_rules.iter().map(|rule| {
                                let extraction_type = match rule.extractor_type {
                                    crate::config::types::ExtractorType::JsonPath => ExtractionType::JsonPath,
                                    crate::config::types::ExtractorType::Regex => ExtractionType::Regex,
                                    crate::config::types::ExtractorType::Header => ExtractionType::Header,
                                    crate::config::types::ExtractorType::StatusCode => ExtractionType::StatusCode,
                                };
                                ExtractionRule {
                                    key: rule.name.clone(),
                                    extraction_type,
                                    pattern: rule.source.clone(),
                                    default_value: rule.default_value.clone(),
                                    required: rule.required,
                                }
                            }).collect();
                            
                            let extraction_result = self.extraction_engine.extract_values(
                                route.name.clone(),
                                env_name.clone(),
                                response,
                                user_data[user_idx].data.clone(),
                                &converted_rules,
                            );
                            
                            match extraction_result {
                                Ok(result) => {
                                    let key = (route.name.clone(), user_idx, env_name.clone());
                                    extraction_results.insert(key, result);
                                }
                                Err(e) => {
                                    let error = ExecutionError::general_execution_error(
                                        format!("Value extraction failed for route '{}': {}", route.name, e)
                                    );
                                    errors.push(error);
                                }
                            }
                        }
                    }

                    // Collect errors
                    if let Some(error) = error_opt {
                        errors.push(error);
                    }
                }
                Ok(Err(e)) => {
                    // Task completed but returned an error
                    progress.request_completed(false);
                    if let Some(ref callback) = progress_callback {
                        callback(progress);
                    }

                    let error = ExecutionError::general_execution_error(e.to_string());
                    errors.push(error);
                }
                Err(e) => {
                    // Task itself failed (JoinError)
                    progress.request_completed(false);
                    if let Some(ref callback) = progress_callback {
                        callback(progress);
                    }

                    let error = ExecutionError::general_execution_error(
                        format!("Task panicked: {}", e)
                    );
                    errors.push(error);
                }
            }
        }
        
        // Process extraction results and update context manager
        for ((route_name, user_idx, env_name), extraction_result) in extraction_results {
            if !extraction_result.errors.is_empty() {
                for extraction_error in &extraction_result.errors {
                    let error = ExecutionError::general_execution_error(
                        format!(
                            "Value extraction failed for route '{}' in environment '{}': {}",
                            route_name, env_name, extraction_error.message
                        )
                    );
                    errors.push(error);
                }
            }
            
            // Add extracted values to context manager
            if !extraction_result.extracted_values.is_empty() {
                if let Err(e) = self.context_manager.add_values_to_scope(
                    user_idx,
                    extraction_result.extracted_values.clone(),
                ) {
                    let error = ExecutionError::general_execution_error(
                        format!(
                            "Failed to add extracted values to context for route '{}': {}",
                            route_name, e
                        )
                    );
                    errors.push(error);
                }
            }
        }
        
        // Mark routes as completed and extraction completed in context manager
        for route_name in batch_routes {
            for user_idx in 0..user_data.len() {
                let _ = self.context_manager.mark_route_completed(user_idx, route_name);
                let _ = self.context_manager.mark_extraction_completed(user_idx, route_name);
            }
        }
        
        Ok(BatchExecutionResult {
            route_user_responses,
            errors,
        })
    }
}

/// Result of executing a batch of routes with extraction
struct BatchExecutionResult {
    route_user_responses: RouteUserResponses,
    errors: Vec<ExecutionError>,
}

impl<C, R, E> TestRunner for TestRunnerImpl<C, R, E>
where
    C: HttpClient,
    R: ResponseComparator,
    E: ConditionEvaluator,
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

        // Detect if any routes have dependencies or extraction rules
        if self.has_chaining_requirements(&routes) {
            self.execute_with_chaining(user_data, &environments, &routes, progress_callback)
                .await
        } else {
            // Use existing concurrent execution for backward compatibility
            self.execute_concurrent(user_data, &environments, &routes, progress_callback)
                .await
        }
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
            conditions: None,
            extract: None,
            depends_on: None,
            wait_for_extraction: None,
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
        let condition_evaluator = crate::conditions::ConditionEvaluatorImpl::new();
        let runner = TestRunnerImpl::new(config, client, comparator, condition_evaluator).unwrap();

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

    #[tokio::test]
    async fn test_chain_execution_detection() {
        let mut environments = HashMap::new();
        environments.insert(
            "dev".to_string(),
            Environment {
                base_url: "https://dev.example.com".to_string(),
                headers: None,
                is_base: false,
            },
        );

        // Route with dependencies should trigger chain execution
        let dependent_route = Route {
            name: "users".to_string(),
            method: "GET".to_string(),
            path: "/users".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: None,
            depends_on: Some(vec!["auth".to_string()]),
            wait_for_extraction: None,
        };

        let auth_route = Route {
            name: "auth".to_string(),
            method: "POST".to_string(),
            path: "/auth".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: Some(vec![crate::config::types::ValueExtractionRule {
                name: "token".to_string(),
                extractor_type: crate::config::types::ExtractorType::JsonPath,
                source: "$.token".to_string(),
                default_value: None,
                required: true,
            }]),
            depends_on: None,
            wait_for_extraction: Some(true),
        };

        let config = HttpDiffConfig {
            environments,
            global: None,
            routes: vec![auth_route.clone(), dependent_route.clone()],
        };

        let client = MockHttpClient::new()
            .with_response("auth:dev".to_string(), create_mock_response(200, r#"{"token": "abc123"}"#))
            .with_response("users:dev".to_string(), create_mock_response(200, "user data"));

        let comparator = MockResponseComparator::new();
        let condition_evaluator = crate::conditions::ConditionEvaluatorImpl::new();
        let runner = TestRunnerImpl::new(config, client, comparator, condition_evaluator).unwrap();

        // Test chain requirement detection
        let routes = vec![&auth_route, &dependent_route];
        assert!(runner.has_chaining_requirements(&routes));

        // Test normal route doesn't trigger chaining
        let normal_route = Route {
            name: "health".to_string(),
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: None,
            depends_on: None,
            wait_for_extraction: None,
        };
        let normal_routes = vec![&normal_route];
        assert!(!runner.has_chaining_requirements(&normal_routes));
    }

    #[tokio::test]
    async fn test_chain_execution_with_extraction() {
        let mut environments = HashMap::new();
        environments.insert(
            "dev".to_string(),
            Environment {
                base_url: "https://dev.example.com".to_string(),
                headers: None,
                is_base: false,
            },
        );
        environments.insert(
            "staging".to_string(),
            Environment {
                base_url: "https://staging.example.com".to_string(),
                headers: None,
                is_base: false,
            },
        );

        let auth_route = Route {
            name: "auth".to_string(),
            method: "POST".to_string(),
            path: "/auth".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: Some(vec![crate::config::types::ValueExtractionRule {
                name: "token".to_string(),
                extractor_type: crate::config::types::ExtractorType::JsonPath,
                source: "$.token".to_string(),
                default_value: None,
                required: true,
            }]),
            depends_on: None,
            wait_for_extraction: None,
        };

        let config = HttpDiffConfig {
            environments,
            global: None,
            routes: vec![auth_route.clone()],
        };

        let client = MockHttpClient::new()
            .with_response("auth:dev".to_string(), create_mock_response(200, r#"{"token": "dev_token"}"#))
            .with_response("auth:staging".to_string(), create_mock_response(200, r#"{"token": "staging_token"}"#));

        let comparator = MockResponseComparator::new();
        let condition_evaluator = crate::conditions::ConditionEvaluatorImpl::new();
        let runner = TestRunnerImpl::new(config, client, comparator, condition_evaluator).unwrap();

        let user_data = vec![create_mock_user_data(vec![])];
        let result = runner
            .execute_with_data(&user_data, None, None, None)
            .await
            .unwrap();

        // Should have chain metadata since extraction was used
        assert!(result.is_chain_execution());
        assert!(result.get_chain_metadata().is_some());
        
        if let Some(metadata) = result.get_chain_metadata() {
            assert_eq!(metadata.total_batches, 1);
            assert_eq!(metadata.extraction_routes, 1);
            assert_eq!(metadata.dependent_routes, 0);
            assert!(metadata.used_extraction());
        }
    }
}
