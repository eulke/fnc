use crate::conditions::ExecutionCondition;
use crate::error::{HttpDiffError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration structure for HTTP diff testing
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpDiffConfig {
    /// Environment configurations
    pub environments: HashMap<String, Environment>,
    /// Global configuration settings
    pub global: Option<GlobalConfig>,
    /// Route definitions
    pub routes: Vec<Route>,
}

/// Environment configuration with base URL and headers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Environment {
    /// Base URL for this environment
    pub base_url: String,
    /// Environment-specific headers
    pub headers: Option<HashMap<String, String>>,
    /// Whether this environment should be treated as the base for comparisons
    #[serde(default)]
    pub is_base: bool,
}

/// Global configuration settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Whether to follow redirects
    pub follow_redirects: Option<bool>,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: Option<usize>,
    /// Global headers applied to all requests
    pub headers: Option<HashMap<String, String>>,
    /// Global query parameters applied to all requests
    pub params: Option<HashMap<String, String>>,
}

/// Route definition for HTTP requests
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    /// Unique name for this route
    pub name: String,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Path with optional parameters like /api/users/{userId}
    pub path: String,
    /// Route-specific headers
    pub headers: Option<HashMap<String, String>>,
    /// Route-specific query parameters
    pub params: Option<HashMap<String, String>>,
    /// Per-environment base URL overrides
    pub base_urls: Option<HashMap<String, String>>,
    /// Request body for POST/PUT requests
    pub body: Option<String>,
    /// Conditional execution rules for this route
    pub conditions: Option<Vec<ExecutionCondition>>,
    /// Value extraction rules for chaining requests
    pub extract: Option<Vec<ValueExtractionRule>>,
    /// Names of routes this route depends on (must execute first)
    pub depends_on: Option<Vec<String>>,
    /// Whether to wait for value extraction from dependencies before executing
    #[serde(default)]
    pub wait_for_extraction: Option<bool>,
}

/// User data loaded from CSV for parameter substitution
#[derive(Debug, Clone)]
pub struct UserData {
    /// CSV column data
    pub data: HashMap<String, String>,
}

impl UserData {
    /// Create new UserData instance
    pub fn new(data: HashMap<String, String>) -> Self {
        Self { data }
    }

    /// Create UserData instance with context integration
    /// This method merges the CSV data with extracted values from the dynamic context
    /// Context values take precedence over CSV data by default
    pub fn with_context(
        &self, 
        context: &crate::execution::context::DynamicContext
    ) -> UserDataWithContext {
        UserDataWithContext::new(self.clone(), context.clone())
    }

    /// Substitute placeholders like {userId} with actual values from CSV data
    ///
    /// # Arguments
    /// * `text` - The text containing placeholders in {param_name} format
    /// * `url_encode` - Whether to URL encode the substituted values (true for paths, false for headers/body)
    /// * `strict` - If true, error on missing parameters; if false, leave unmatched placeholders unchanged
    pub fn substitute_placeholders(
        &self,
        text: &str,
        url_encode: bool,
        strict: bool,
    ) -> Result<String> {
        // Use single-pass algorithm to avoid multiple reallocations
        let mut result = String::with_capacity(text.len() + 50); // Pre-allocate with some extra space
        let mut chars = text.char_indices().peekable();

        while let Some((_pos, ch)) = chars.next() {
            if ch == '{' {
                // Found potential parameter start, collect parameter name
                let mut param_name = String::new();
                let mut found_end = false;

                while let Some((_, next_ch)) = chars.peek() {
                    if *next_ch == '}' {
                        chars.next(); // consume the '}'
                        found_end = true;
                        break;
                    } else if *next_ch == '{' {
                        // Nested braces, not a valid parameter
                        break;
                    } else {
                        param_name.push(chars.next().unwrap().1);
                    }
                }

                if found_end && is_valid_param_name(&param_name) {
                    if let Some(value) = self.data.get(&param_name) {
                        // Substitute the parameter
                        if url_encode {
                            result.push_str(&urlencoding::encode(value));
                        } else {
                            result.push_str(value);
                        }
                    } else if strict {
                        // Strict mode: error if parameter is missing
                        let available = self
                            .data
                            .keys()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(HttpDiffError::MissingPathParameter {
                            param: param_name,
                            available_params: if available.is_empty() {
                                "none".to_string()
                            } else {
                                available
                            },
                        });
                    } else {
                        // Non-strict mode: preserve the original placeholder
                        result.push('{');
                        result.push_str(&param_name);
                        result.push('}');
                    }
                } else {
                    // Invalid parameter format, preserve the original text
                    result.push(ch);
                    result.push_str(&param_name);
                    if found_end {
                        result.push('}');
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Get available parameters from this UserData instance
    pub fn get_available_parameters(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// Check if a parameter exists
    pub fn has_parameter(&self, param_name: &str) -> bool {
        self.data.contains_key(param_name)
    }

    /// Get parameter value
    pub fn get_parameter(&self, param_name: &str) -> Option<&str> {
        self.data.get(param_name).map(String::as_str)
    }

    /// Get the number of parameters
    pub fn parameter_count(&self) -> usize {
        self.data.len()
    }
}

/// UserData enhanced with dynamic context for parameter resolution
/// This struct combines CSV user data with extracted values from previous routes
#[derive(Debug, Clone)]
pub struct UserDataWithContext {
    /// The original user data from CSV
    user_data: UserData,
    /// The dynamic context containing extracted values
    context: crate::execution::context::DynamicContext,
    /// Variable resolver for handling precedence and resolution
    resolver: crate::execution::context::VariableResolver,
}

impl UserDataWithContext {
    /// Create a new UserDataWithContext instance
    pub fn new(user_data: UserData, context: crate::execution::context::DynamicContext) -> Self {
        Self {
            user_data,
            context,
            resolver: crate::execution::context::VariableResolver::new(),
        }
    }

    /// Create a new instance with custom resolver settings
    pub fn with_resolver(
        user_data: UserData, 
        context: crate::execution::context::DynamicContext,
        resolver: crate::execution::context::VariableResolver
    ) -> Self {
        Self {
            user_data,
            context,
            resolver,
        }
    }

    /// Get the underlying user data
    pub fn get_user_data(&self) -> &UserData {
        &self.user_data
    }

    /// Get the underlying context
    pub fn get_context(&self) -> &crate::execution::context::DynamicContext {
        &self.context
    }

    /// Substitute placeholders using both user data and context
    /// Context values take precedence over CSV data by default
    pub fn substitute_placeholders(
        &self,
        text: &str,
        url_encode: bool,
    ) -> Result<String> {
        self.resolver.substitute_placeholders(text, &self.context, &self.user_data, url_encode)
    }

    /// Get all available parameters from both user data and context
    pub fn get_available_parameters(&self) -> HashMap<String, String> {
        self.resolver.get_available_parameters(&self.context, &self.user_data)
    }

    /// Check if a parameter exists in either user data or context
    pub fn has_parameter(&self, param_name: &str) -> bool {
        self.context.has_value(param_name) || self.user_data.has_parameter(param_name)
    }

    /// Get parameter value, checking context first then user data (based on resolver priority)
    pub fn get_parameter(&self, param_name: &str) -> Result<Option<String>> {
        self.resolver.resolve_parameter(param_name, &self.context, &self.user_data)
    }

    /// Get the number of unique parameters across user data and context
    pub fn parameter_count(&self) -> usize {
        self.get_available_parameters().len()
    }

    /// Create a new UserData object with context values merged in
    /// Context values take precedence over CSV data
    pub fn to_merged_user_data(&self) -> UserData {
        let merged_params = self.get_available_parameters();
        UserData {
            data: merged_params,
        }
    }

    /// Get parameters that come from the context (extracted values)
    pub fn get_context_parameters(&self) -> Vec<String> {
        self.context.get_all_keys()
    }

    /// Get parameters that come from user data (CSV)
    pub fn get_user_data_parameters(&self) -> Vec<String> {
        self.user_data.get_available_parameters()
    }

    /// Get parameters that exist in both context and user data (potential conflicts)
    pub fn get_conflicting_parameters(&self) -> Vec<String> {
        let context_keys: std::collections::HashSet<String> = self.context.get_all_keys().into_iter().collect();
        let user_data_keys: std::collections::HashSet<String> = self.user_data.get_available_parameters().into_iter().collect();
        
        context_keys.intersection(&user_data_keys).cloned().collect()
    }

    /// Create a debug summary showing parameter sources
    pub fn create_parameter_summary(&self) -> UserDataParameterSummary {
        let context_params = self.get_context_parameters();
        let user_data_params = self.get_user_data_parameters();
        let conflicting_params = self.get_conflicting_parameters();

        UserDataParameterSummary {
            context_only: context_params.iter()
                .filter(|p| !user_data_params.contains(p))
                .cloned()
                .collect(),
            user_data_only: user_data_params.iter()
                .filter(|p| !context_params.contains(p))
                .cloned()
                .collect(),
            conflicting: conflicting_params,
            total_unique: self.parameter_count(),
            context_priority: true, // Default resolver setting
        }
    }
}

/// Summary of parameter sources in UserDataWithContext
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserDataParameterSummary {
    /// Parameters that only exist in the context
    pub context_only: Vec<String>,
    /// Parameters that only exist in user data
    pub user_data_only: Vec<String>,
    /// Parameters that exist in both (resolver determines precedence)
    pub conflicting: Vec<String>,
    /// Total number of unique parameters
    pub total_unique: usize,
    /// Whether context has priority over user data
    pub context_priority: bool,
}

/// Value extraction rule for chaining HTTP requests
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValueExtractionRule {
    /// Name of the extracted value for use in subsequent requests
    pub name: String,
    /// Type of extractor to use
    #[serde(alias = "type")]
    pub extractor_type: ExtractorType,
    /// Source location/path for extraction (JSONPath, regex pattern, header name, etc.)
    pub source: String,
    /// Default value to use if extraction fails
    pub default_value: Option<String>,
    /// Whether this extraction is required (fails request chain if missing)
    #[serde(default)]
    pub required: bool,
}

/// Type of value extractor for response processing
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorType {
    /// Extract value using JSONPath expression from response body
    JsonPath,
    /// Extract value using regular expression from response body
    Regex,
    /// Extract value from response header
    Header,
    /// Extract HTTP status code
    StatusCode,
}

/// Check if a parameter name is a valid identifier (letters, numbers, underscore)
fn is_valid_param_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Default implementation for GlobalConfig
impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(30),
            follow_redirects: Some(true),
            max_concurrent_requests: Some(10),
            headers: None,
            params: None,
        }
    }
}

impl HttpDiffConfig {
    /// Get the base URL for a route in a specific environment
    pub fn get_base_url(&self, route: &Route, environment: &str) -> Result<String> {
        // First check if route has environment-specific override
        if let Some(base_urls) = &route.base_urls {
            if let Some(url) = base_urls.get(environment) {
                return Ok(url.clone());
            }
        }

        // Fall back to environment default
        self.environments
            .get(environment)
            .map(|env| env.base_url.clone())
            .ok_or_else(|| HttpDiffError::InvalidEnvironment {
                environment: environment.to_string(),
            })
    }

    /// Validate chain configuration consistency
    pub fn validate_chain_config(&self) -> Result<()> {
        // Build a map of route names for quick lookup
        let route_names: std::collections::HashSet<String> = 
            self.routes.iter().map(|r| r.name.clone()).collect();

        for route in &self.routes {
            // Validate dependencies exist
            if let Some(deps) = &route.depends_on {
                for dep in deps {
                    if !route_names.contains(dep) {
                        return Err(HttpDiffError::invalid_config(
                            format!(
                                "Route '{}' depends on non-existent route '{}'",
                                route.name, dep
                            )
                        ));
                    }
                }
            }

            // Validate extraction rules
            if let Some(extractions) = &route.extract {
                for extraction in extractions {
                    // Validate extraction rule name is a valid identifier
                    if !is_valid_param_name(&extraction.name) {
                        return Err(HttpDiffError::invalid_config(
                            format!(
                                "Route '{}' has invalid extraction rule name '{}'. Must be alphanumeric with underscores only.",
                                route.name, extraction.name
                            )
                        ));
                    }

                    // Validate source is not empty
                    if extraction.source.trim().is_empty() {
                        return Err(HttpDiffError::invalid_config(
                            format!(
                                "Route '{}' extraction rule '{}' has empty source",
                                route.name, extraction.name
                            )
                        ));
                    }
                }
            }
        }

        // Check for circular dependencies
        self.validate_no_circular_dependencies()?;

        Ok(())
    }

    /// Validate there are no circular dependencies in the route chain
    fn validate_no_circular_dependencies(&self) -> Result<()> {
        use std::collections::{HashMap, VecDeque};

        // Build dependency graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for route in &self.routes {
            let deps = route.depends_on.as_ref().map(|d| d.clone()).unwrap_or_default();
            graph.insert(route.name.clone(), deps);
        }

        // Perform topological sort to detect cycles
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for route in &self.routes {
            in_degree.insert(route.name.clone(), 0);
        }

        // Calculate in-degrees
        for deps in graph.values() {
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Queue nodes with no incoming edges
        let mut queue: VecDeque<String> = VecDeque::new();
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        let mut processed = 0;
        while let Some(node) = queue.pop_front() {
            processed += 1;
            
            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // If we didn't process all nodes, there's a cycle
        if processed != self.routes.len() {
            return Err(HttpDiffError::invalid_config(
                "Circular dependency detected in route chain configuration".to_string()
            ));
        }

        Ok(())
    }

    /// Get routes sorted by dependency order (dependencies first)
    pub fn get_routes_by_dependency_order(&self) -> Result<Vec<&Route>> {
        use std::collections::{HashMap, VecDeque};

        // First validate the configuration
        self.validate_chain_config()?;

        // Build dependency graph and route lookup
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut route_map: HashMap<String, &Route> = HashMap::new();
        
        for route in &self.routes {
            let deps = route.depends_on.as_ref().map(|d| d.clone()).unwrap_or_default();
            graph.insert(route.name.clone(), deps);
            route_map.insert(route.name.clone(), route);
        }

        // Calculate dependency counts (how many routes each route depends on)
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for route in &self.routes {
            let dep_count = route.depends_on.as_ref().map(|d| d.len()).unwrap_or(0);
            in_degree.insert(route.name.clone(), dep_count);
        }

        // Topological sort
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut result: Vec<&Route> = Vec::new();

        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            if let Some(route) = route_map.get(&node) {
                result.push(route);
            }
            
            // Find all routes that depend on this node and reduce their dependency count
            for (route_name, deps) in &graph {
                if deps.contains(&node) {
                    if let Some(degree) = in_degree.get_mut(route_name) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(route_name.clone());
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::context::{DynamicContext, VariableResolver};
    use crate::types::{ExtractedValue, ExtractionType};

    fn create_test_extracted_value(key: &str, value: &str, route_name: &str, env: &str) -> ExtractedValue {
        ExtractedValue::new(
            key.to_string(),
            value.to_string(),
            format!("$.{}", key),
            ExtractionType::JsonPath,
            env.to_string(),
            route_name.to_string(),
        )
    }

    #[test]
    fn test_user_data_basic_operations() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "123".to_string());
        data.insert("org_id".to_string(), "456".to_string());

        let user_data = UserData::new(data);
        
        assert_eq!(user_data.parameter_count(), 2);
        assert!(user_data.has_parameter("user_id"));
        assert!(user_data.has_parameter("org_id"));
        assert!(!user_data.has_parameter("missing"));
        assert_eq!(user_data.get_parameter("user_id"), Some("123"));
        assert_eq!(user_data.get_parameter("missing"), None);
        
        let params = user_data.get_available_parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains(&"user_id".to_string()));
        assert!(params.contains(&"org_id".to_string()));
    }

    #[test]
    fn test_user_data_substitute_placeholders() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "123".to_string());
        data.insert("org_id".to_string(), "456".to_string());

        let user_data = UserData::new(data);
        
        let text = "/api/users/{user_id}/orgs/{org_id}";
        let result = user_data.substitute_placeholders(text, false, true).unwrap();
        assert_eq!(result, "/api/users/123/orgs/456");

        // Test URL encoding
        let mut data_with_special = HashMap::new();
        data_with_special.insert("query".to_string(), "hello world".to_string());
        let user_data_special = UserData::new(data_with_special);
        
        let encoded_result = user_data_special.substitute_placeholders("/search?q={query}", true, true).unwrap();
        assert_eq!(encoded_result, "/search?q=hello%20world");
    }

    #[test]
    fn test_user_data_with_context_creation() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "csv_123".to_string());
        let user_data = UserData::new(data);

        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("token", "abc123", "auth", "dev"));

        let user_data_with_context = user_data.with_context(&context);
        
        assert_eq!(user_data_with_context.parameter_count(), 2);
        assert!(user_data_with_context.has_parameter("user_id"));
        assert!(user_data_with_context.has_parameter("token"));
    }

    #[test]
    fn test_user_data_with_context_precedence() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "csv_123".to_string());
        let user_data = UserData::new(data);

        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "context_456", "auth", "dev"));

        let user_data_with_context = user_data.with_context(&context);
        
        // Context should take precedence by default
        let resolved = user_data_with_context.get_parameter("user_id").unwrap();
        assert_eq!(resolved, Some("context_456".to_string()));
    }

    #[test]
    fn test_user_data_with_context_substitute() {
        let mut data = HashMap::new();
        data.insert("org_id".to_string(), "456".to_string());
        let user_data = UserData::new(data);

        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "123", "auth", "dev"));

        let user_data_with_context = user_data.with_context(&context);
        
        let text = "/api/users/{user_id}/orgs/{org_id}";
        let result = user_data_with_context.substitute_placeholders(text, false).unwrap();
        assert_eq!(result, "/api/users/123/orgs/456");
    }

    #[test]
    fn test_user_data_with_context_parameter_summary() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "csv_123".to_string());
        data.insert("org_id".to_string(), "456".to_string());
        let user_data = UserData::new(data);

        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "context_789", "auth", "dev"));
        context.add_value(create_test_extracted_value("token", "abc123", "auth", "dev"));

        let user_data_with_context = user_data.with_context(&context);
        let summary = user_data_with_context.create_parameter_summary();
        
        assert_eq!(summary.total_unique, 3); // user_id, org_id, token
        assert_eq!(summary.context_only, vec!["token"]);
        assert_eq!(summary.user_data_only, vec!["org_id"]);
        assert_eq!(summary.conflicting, vec!["user_id"]);
        assert!(summary.context_priority);
    }

    #[test] 
    fn test_user_data_with_context_custom_resolver() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "csv_123".to_string());
        let user_data = UserData::new(data);

        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "context_456", "auth", "dev"));

        // Create resolver that prioritizes user data over context
        let resolver = VariableResolver::new().with_context_priority(false);
        let user_data_with_context = UserDataWithContext::with_resolver(user_data, context, resolver);
        
        // User data should take precedence now
        let resolved = user_data_with_context.get_parameter("user_id").unwrap();
        assert_eq!(resolved, Some("csv_123".to_string()));
    }

    #[test]
    fn test_user_data_with_context_get_parameter_sources() {
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "csv_123".to_string());
        data.insert("org_id".to_string(), "456".to_string());
        let user_data = UserData::new(data);

        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "context_789", "auth", "dev"));
        context.add_value(create_test_extracted_value("token", "abc123", "auth", "dev"));

        let user_data_with_context = user_data.with_context(&context);
        
        let context_params = user_data_with_context.get_context_parameters();
        assert_eq!(context_params.len(), 2);
        assert!(context_params.contains(&"user_id".to_string()));
        assert!(context_params.contains(&"token".to_string()));

        let user_data_params = user_data_with_context.get_user_data_parameters();
        assert_eq!(user_data_params.len(), 2);
        assert!(user_data_params.contains(&"user_id".to_string()));
        assert!(user_data_params.contains(&"org_id".to_string()));

        let conflicting = user_data_with_context.get_conflicting_parameters();
        assert_eq!(conflicting, vec!["user_id"]);
    }
}
