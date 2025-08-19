use std::collections::HashMap;
use std::sync::Arc;
use crate::types::{ComparisonResult, HttpResponse};
use crate::error::{HttpDiffError, Result};

/// Centralized environment management utilities
/// 
/// This module provides consistent environment ordering and handling
/// across all components to prevent environment misalignment issues.

#[derive(Debug, Clone)]
pub struct EnvironmentOrderResolver {
    /// Ordered list of environment names based on configuration priority
    environment_order: Vec<String>,
    /// Base environment (if configured)
    base_environment: Option<String>,
}

impl EnvironmentOrderResolver {
    /// Create a new resolver with configuration-based ordering
    pub fn new(configured_environments: &[String], base_environment: Option<String>) -> Self {
        let mut environment_order = configured_environments.to_vec();
        
        // If base environment is specified, ensure it's first
        if let Some(ref base_env) = base_environment {
            if let Some(pos) = environment_order.iter().position(|e| e == base_env) {
                environment_order.remove(pos);
                environment_order.insert(0, base_env.clone());
            }
        }
        
        Self {
            environment_order,
            base_environment,
        }
    }
    
    /// Create resolver from available responses (fallback when config not available)
    pub fn from_responses(responses: &HashMap<String, HttpResponse>, base_environment: Option<String>) -> Self {
        let mut environment_names: Vec<String> = responses.keys().cloned().collect();
        environment_names.sort(); // Fallback to alphabetical for consistency
        
        Self::new(&environment_names, base_environment)
    }
    
    /// Get deterministically ordered environment names
    pub fn get_ordered_environments(&self, available_environments: &[String]) -> Vec<String> {
        let mut ordered = Vec::new();
        
        // First, add environments in configuration order
        for env in &self.environment_order {
            if available_environments.contains(env) {
                ordered.push(env.clone());
            }
        }
        
        // Add any remaining environments (shouldn't happen with proper config)
        for env in available_environments {
            if !ordered.contains(env) {
                ordered.push(env.clone());
            }
        }
        
        ordered
    }
    
    /// Extract ordered environment names from responses HashMap
    pub fn extract_ordered_environments(&self, responses: &HashMap<String, HttpResponse>) -> Vec<String> {
        let available: Vec<String> = responses.keys().cloned().collect();
        self.get_ordered_environments(&available)
    }
    
    /// Get the base environment (if any)
    pub fn base_environment(&self) -> Option<&String> {
        self.base_environment.as_ref()
    }
    
    /// Check if an environment is the base environment
    pub fn is_base_environment(&self, env: &str) -> bool {
        self.base_environment.as_ref().map_or(false, |base| base == env)
    }

    /// Get all configured environment names in order
    pub fn get_all_configured_environments(&self) -> Vec<String> {
        self.environment_order.clone()
    }
}

/// Ordered environment data structures to replace HashMap usage
#[derive(Debug, Clone)]
pub struct OrderedEnvironmentResponses {
    environments: Vec<String>,
    responses: HashMap<String, HttpResponse>,
}

impl OrderedEnvironmentResponses {
    /// Create from resolver and responses
    pub fn new(resolver: &EnvironmentOrderResolver, responses: HashMap<String, HttpResponse>) -> Self {
        let environments = resolver.extract_ordered_environments(&responses);
        Self { environments, responses }
    }
    
    /// Get ordered environment names
    pub fn environments(&self) -> &[String] {
        &self.environments
    }
    
    /// Get response for environment
    pub fn get_response(&self, env: &str) -> Option<&HttpResponse> {
        self.responses.get(env)
    }
    
    /// Iterate over environments and responses in order
    pub fn iter(&self) -> impl Iterator<Item = (&String, &HttpResponse)> {
        self.environments.iter().filter_map(move |env| {
            self.responses.get(env).map(|response| (env, response))
        })
    }
    
    /// Get the underlying HashMap for compatibility
    pub fn as_hashmap(&self) -> &HashMap<String, HttpResponse> {
        &self.responses
    }
    
    /// Convert to HashMap (for backward compatibility)
    pub fn into_hashmap(self) -> HashMap<String, HttpResponse> {
        self.responses
    }
}

#[derive(Debug, Clone)]
pub struct OrderedStatusCodes {
    environments: Vec<String>,
    status_codes: HashMap<String, u16>,
}

impl OrderedStatusCodes {
    /// Create from resolver and status codes
    pub fn new(resolver: &EnvironmentOrderResolver, status_codes: HashMap<String, u16>) -> Self {
        let available: Vec<String> = status_codes.keys().cloned().collect();
        let environments = resolver.get_ordered_environments(&available);
        Self { environments, status_codes }
    }
    
    /// Iterate over environments and status codes in order
    pub fn iter(&self) -> impl Iterator<Item = (&String, u16)> {
        self.environments.iter().filter_map(move |env| {
            self.status_codes.get(env).map(|&status| (env, status))
        })
    }
    
    /// Get status code for environment
    pub fn get(&self, env: &str) -> Option<u16> {
        self.status_codes.get(env).copied()
    }
    
    /// Get the underlying HashMap for compatibility
    pub fn as_hashmap(&self) -> &HashMap<String, u16> {
        &self.status_codes
    }
}

/// Environment consistency validation utilities
pub struct EnvironmentValidator;

impl EnvironmentValidator {
    /// Validate that comparison result has consistent environment ordering
    pub fn validate_comparison_result(result: &ComparisonResult, resolver: &EnvironmentOrderResolver) -> Result<()> {
        let response_envs = resolver.extract_ordered_environments(&result.responses);
        let available_status_envs: Vec<String> = result.status_codes.keys().cloned().collect();
        let status_envs = resolver.get_ordered_environments(&available_status_envs);
        
        // Check that all environment sets are consistent
        if response_envs.len() != status_envs.len() {
            return Err(HttpDiffError::environment_mismatch(format!(
                "Environment count mismatch: {} responses vs {} status codes",
                response_envs.len(),
                status_envs.len()
            )));
        }
        
        // Validate base environment consistency
        if let Some(base_env) = &result.base_environment {
            if !response_envs.contains(base_env) {
                return Err(HttpDiffError::invalid_base_environment(
                    base_env.clone(),
                    "Base environment not found in responses".to_string()
                ));
            }
            
            if let Some(resolver_base) = resolver.base_environment() {
                if base_env != resolver_base {
                    return Err(HttpDiffError::environment_mismatch(format!(
                        "Base environment mismatch: result has '{}' but resolver has '{}'",
                        base_env, resolver_base
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate minimum environment count for comparison
    pub fn validate_minimum_environments(environments: &[String]) -> Result<()> {
        if environments.len() < 2 {
            return Err(HttpDiffError::environment_validation(format!(
                "At least 2 environments required for comparison, got {}",
                environments.len()
            )));
        }
        Ok(())
    }
}

/// Type-safe environment containers that prevent misalignment at compile time
#[derive(Debug, Clone)]
pub struct TypedEnvironmentData<T> {
    /// Immutable, shared environment ordering
    environment_order: Arc<[String]>,
    /// Data mapped to environments in the same order
    data: Vec<T>,
}

impl<T> TypedEnvironmentData<T> {
    /// Create new typed environment data with validation
    pub fn new(resolver: &EnvironmentOrderResolver, mut data_map: HashMap<String, T>) -> Result<Self> {
        // Use the resolver's configured environment order, not just what's available
        let environment_order = resolver.get_all_configured_environments();
        
        // Validate that all environments have data and extract in order
        let mut data = Vec::with_capacity(environment_order.len());
        for env in &environment_order {
            match data_map.remove(env) {
                Some(value) => data.push(value),
                None => return Err(HttpDiffError::environment_validation(format!(
                    "Missing data for environment: {}", env
                ))),
            }
        }
        
        Ok(Self {
            environment_order: Arc::from(environment_order.into_boxed_slice()),
            data,
        })
    }
    
    /// Get the first item deterministically (safe replacement for HashMap.iter().next())
    pub fn first(&self) -> Option<(&str, &T)> {
        if let (Some(env), Some(data)) = (self.environment_order.first(), self.data.first()) {
            Some((env.as_str(), data))
        } else {
            None
        }
    }
    
    /// Iterate over environments and data in deterministic order
    pub fn iter(&self) -> impl Iterator<Item = (&str, &T)> {
        self.environment_order.iter().zip(self.data.iter()).map(|(env, data)| (env.as_str(), data))
    }
    
    /// Get environment names in order
    pub fn environment_names(&self) -> &[String] {
        &self.environment_order
    }
    
    /// Get data for specific environment
    pub fn get(&self, env: &str) -> Option<&T> {
        self.environment_order.iter().position(|e| e == env)
            .and_then(|index| self.data.get(index))
    }
}

/// Trait for ensuring environment alignment across data structures
pub trait EnvironmentAligned {
    /// Check if environment ordering is consistent
    fn validate_alignment(&self) -> Result<()>;
    
    /// Get deterministic first item
    fn first_item(&self) -> Option<(&str, &dyn std::fmt::Debug)>;
}

impl<T: std::fmt::Debug> EnvironmentAligned for TypedEnvironmentData<T> {
    fn validate_alignment(&self) -> Result<()> {
        if self.environment_order.len() != self.data.len() {
            return Err(HttpDiffError::environment_validation(format!(
                "Environment count mismatch: {} names vs {} data items",
                self.environment_order.len(),
                self.data.len()
            )));
        }
        Ok(())
    }
    
    fn first_item(&self) -> Option<(&str, &dyn std::fmt::Debug)> {
        self.first().map(|(env, data)| (env, data as &dyn std::fmt::Debug))
    }
}

/// Type aliases for common use cases
pub type TypedEnvironmentResponses = TypedEnvironmentData<HttpResponse>;
pub type TypedEnvironmentStatusCodes = TypedEnvironmentData<u16>;

/// Legacy utility functions for backward compatibility
/// These extract environments from HashMap and sort deterministically
pub mod legacy {
    use super::*;
    
    /// Extract environment names from responses HashMap with deterministic ordering
    pub fn extract_environment_names(responses: &HashMap<String, HttpResponse>) -> Vec<String> {
        let mut environments: Vec<String> = responses.keys().cloned().collect();
        environments.sort(); // Deterministic fallback
        environments
    }
    
    /// Sort environments with optional base environment first
    pub fn sort_environments_with_base(mut environments: Vec<String>, base_environment: Option<&String>) -> Vec<String> {
        if let Some(base_env) = base_environment {
            if let Some(pos) = environments.iter().position(|e| e == base_env) {
                environments.remove(pos);
                environments.sort(); // Sort the rest
                environments.insert(0, base_env.clone());
            } else {
                environments.sort();
            }
        } else {
            environments.sort();
        }
        environments
    }
    
    /// Find base environment from HashMap-based responses
    pub fn resolve_base_environment(responses: &HashMap<String, HttpResponse>, configured_base: Option<&String>) -> Option<String> {
        if let Some(base_env) = configured_base {
            if responses.contains_key(base_env) {
                return Some(base_env.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::types::HttpResponse;
    
    fn create_test_response(status: u16) -> HttpResponse {
        HttpResponse::new(
            status,
            HashMap::new(),
            "test body".to_string(),
            "https://test.com".to_string(),
            "curl test".to_string(),
        )
    }
    
    #[test]
    fn test_environment_order_resolver_with_base() {
        let environments = vec!["prod".to_string(), "dev".to_string(), "staging".to_string()];
        let resolver = EnvironmentOrderResolver::new(&environments, Some("dev".to_string()));
        
        let ordered = resolver.get_ordered_environments(&environments);
        
        // Base environment should be first
        assert_eq!(ordered[0], "dev");
        assert_eq!(ordered.len(), 3);
        assert!(ordered.contains(&"prod".to_string()));
        assert!(ordered.contains(&"staging".to_string()));
    }
    
    #[test]
    fn test_environment_order_resolver_no_base() {
        let environments = vec!["prod".to_string(), "dev".to_string(), "staging".to_string()];
        let resolver = EnvironmentOrderResolver::new(&environments, None);
        
        let ordered = resolver.get_ordered_environments(&environments);
        
        // Should maintain configuration order
        assert_eq!(ordered, environments);
    }
    
    #[test]
    fn test_ordered_environment_responses() {
        let mut responses = HashMap::new();
        responses.insert("dev".to_string(), create_test_response(200));
        responses.insert("prod".to_string(), create_test_response(200));
        
        let environments = vec!["prod".to_string(), "dev".to_string()];
        let resolver = EnvironmentOrderResolver::new(&environments, Some("prod".to_string()));
        
        let ordered_responses = OrderedEnvironmentResponses::new(&resolver, responses);
        
        let env_order: Vec<_> = ordered_responses.environments().to_vec();
        assert_eq!(env_order[0], "prod"); // Base environment first
        assert_eq!(env_order[1], "dev");
    }
    
    #[test]
    fn test_environment_validator() {
        let mut result = ComparisonResult::new("test".to_string(), HashMap::new());
        result.add_response("dev".to_string(), create_test_response(200));
        result.add_response("prod".to_string(), create_test_response(200));
        result.base_environment = Some("dev".to_string());
        
        let environments = vec!["dev".to_string(), "prod".to_string()];
        let resolver = EnvironmentOrderResolver::new(&environments, Some("dev".to_string()));
        
        // Should validate successfully
        assert!(EnvironmentValidator::validate_comparison_result(&result, &resolver).is_ok());
    }
    
    #[test]
    fn test_legacy_functions() {
        let mut responses = HashMap::new();
        responses.insert("prod".to_string(), create_test_response(200));
        responses.insert("dev".to_string(), create_test_response(200));
        
        let environments = legacy::extract_environment_names(&responses);
        
        // Should be sorted alphabetically
        assert_eq!(environments[0], "dev");
        assert_eq!(environments[1], "prod");
        
        let sorted = legacy::sort_environments_with_base(environments, Some(&"prod".to_string()));
        // Base environment should be first
        assert_eq!(sorted[0], "prod");
        assert_eq!(sorted[1], "dev");
    }
}