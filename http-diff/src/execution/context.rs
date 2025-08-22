use crate::config::types::UserData;
use crate::error::{HttpDiffError, Result};
use crate::types::{ExtractedValue, ExtractionType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// A dynamic context that stores extracted values across route executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicContext {
    /// Map of variable name to extracted value
    values: HashMap<String, ExtractedValue>,
    /// Set of required variable names that must be present
    required_variables: HashSet<String>,
    /// Metadata about when this context was created
    created_at: chrono::DateTime<chrono::Utc>,
    /// Context ID for debugging and tracking
    context_id: String,
    /// The user data row this context belongs to (for isolation)
    user_data_index: Option<usize>,
}

impl DynamicContext {
    /// Create a new empty dynamic context
    pub fn new() -> Self {
        Self::with_id(uuid::Uuid::new_v4().to_string())
    }

    /// Create a new dynamic context with a specific ID
    pub fn with_id(context_id: String) -> Self {
        Self {
            values: HashMap::new(),
            required_variables: HashSet::new(),
            created_at: chrono::Utc::now(),
            context_id,
            user_data_index: None,
        }
    }

    /// Create a new dynamic context for a specific user data row
    pub fn for_user_data(user_data_index: usize) -> Self {
        let mut context = Self::new();
        context.user_data_index = Some(user_data_index);
        context
    }

    /// Add an extracted value to the context
    pub fn add_value(&mut self, value: ExtractedValue) {
        self.values.insert(value.key.clone(), value);
    }

    /// Add multiple extracted values to the context
    pub fn add_values(&mut self, values: Vec<ExtractedValue>) {
        for value in values {
            self.add_value(value);
        }
    }

    /// Get an extracted value by key
    pub fn get_value(&self, key: &str) -> Option<&ExtractedValue> {
        self.values.get(key)
    }

    /// Get the extracted value string by key (convenience method)
    pub fn get_value_string(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|v| v.value.as_str())
    }

    /// Check if a value exists in the context
    pub fn has_value(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get all value keys in the context
    pub fn get_all_keys(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    /// Get all extracted values as a key-value map
    pub fn to_key_value_map(&self) -> HashMap<String, String> {
        self.values
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }

    /// Mark a variable as required
    pub fn mark_required(&mut self, variable_name: String) {
        self.required_variables.insert(variable_name);
    }

    /// Mark multiple variables as required
    pub fn mark_required_variables(&mut self, variable_names: Vec<String>) {
        for name in variable_names {
            self.mark_required(name);
        }
    }

    /// Check if all required variables are present
    pub fn validate_required_variables(&self) -> Result<()> {
        let missing_variables: Vec<String> = self
            .required_variables
            .iter()
            .filter(|var| !self.values.contains_key(*var))
            .cloned()
            .collect();

        if !missing_variables.is_empty() {
            return Err(HttpDiffError::chain_dependency_error(
                "context_validation",
                format!(
                    "Missing required variables in context: {}. Available variables: {}",
                    missing_variables.join(", "),
                    self.values.keys().cloned().collect::<Vec<_>>().join(", ")
                ).as_str(),
            ));
        }

        Ok(())
    }

    /// Check for variable name conflicts
    pub fn check_conflicts(&self, new_variable: &str) -> Result<()> {
        if self.values.contains_key(new_variable) {
            let existing = &self.values[new_variable];
            return Err(HttpDiffError::chain_dependency_error(
                "variable_conflict",
                format!(
                    "Variable '{}' already exists in context. Existing value extracted from route '{}' in environment '{}' at {}",
                    new_variable,
                    existing.route_name,
                    existing.environment,
                    existing.extracted_at.format("%Y-%m-%d %H:%M:%S UTC")
                ).as_str(),
            ));
        }
        Ok(())
    }

    /// Check variable type consistency (all values from same route should have consistent types)
    pub fn check_type_consistency(&self, route_name: &str) -> Result<()> {
        let route_values: Vec<&ExtractedValue> = self
            .values
            .values()
            .filter(|v| v.route_name == route_name)
            .collect();

        if route_values.len() <= 1 {
            return Ok(());
        }

        // Group by extraction type for consistency checking
        let mut type_groups: HashMap<ExtractionType, Vec<&ExtractedValue>> = HashMap::new();
        for value in &route_values {
            type_groups
                .entry(value.extraction_type.clone())
                .or_default()
                .push(value);
        }

        // Check for inconsistent patterns within same extraction type
        for (extraction_type, values) in type_groups {
            if values.len() > 1 {
                let patterns: HashSet<&String> = values.iter().map(|v| &v.extraction_rule).collect();
                if patterns.len() > 1 {
                    return Err(HttpDiffError::chain_dependency_error(
                        route_name,
                        format!(
                            "Inconsistent extraction patterns for type {:?} in route '{}': {}",
                            extraction_type,
                            route_name,
                            patterns.into_iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                        ).as_str(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get context metadata
    pub fn get_context_id(&self) -> &str {
        &self.context_id
    }

    /// Get user data index if available
    pub fn get_user_data_index(&self) -> Option<usize> {
        self.user_data_index
    }

    /// Get creation timestamp
    pub fn get_created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    /// Get number of values in context
    pub fn value_count(&self) -> usize {
        self.values.len()
    }

    /// Check if context is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Merge values from another context (with conflict checking)
    pub fn merge(&mut self, other: &DynamicContext) -> Result<()> {
        for (key, value) in &other.values {
            self.check_conflicts(key)?;
            self.values.insert(key.clone(), value.clone());
        }

        // Merge required variables
        self.required_variables.extend(other.required_variables.clone());

        Ok(())
    }

    /// Create a filtered view of the context with only specific keys
    pub fn filter(&self, keys: &[String]) -> Self {
        let filtered_values: HashMap<String, ExtractedValue> = self
            .values
            .iter()
            .filter(|(k, _)| keys.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Self {
            values: filtered_values,
            required_variables: self
                .required_variables
                .intersection(&keys.iter().cloned().collect())
                .cloned()
                .collect(),
            created_at: self.created_at,
            context_id: format!("{}_filtered", self.context_id),
            user_data_index: self.user_data_index,
        }
    }

    /// Serialize context to JSON for debugging and persistence
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            HttpDiffError::general(format!("Failed to serialize context to JSON: {}", e))
        })
    }

    /// Deserialize context from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| {
            HttpDiffError::general(format!("Failed to deserialize context from JSON: {}", e))
        })
    }
}

impl Default for DynamicContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages context scoping and isolation between different user data rows
#[derive(Debug, Clone)]
pub struct ContextScope {
    /// The user data index this scope belongs to
    user_data_index: usize,
    /// The dynamic context for this scope
    context: DynamicContext,
    /// Routes that have completed in this scope
    completed_routes: HashSet<String>,
    /// Routes that have completed extraction in this scope
    extraction_completed_routes: HashSet<String>,
}

impl ContextScope {
    /// Create a new context scope for a user data index
    pub fn new(user_data_index: usize) -> Self {
        Self {
            user_data_index,
            context: DynamicContext::for_user_data(user_data_index),
            completed_routes: HashSet::new(),
            extraction_completed_routes: HashSet::new(),
        }
    }

    /// Get the user data index for this scope
    pub fn get_user_data_index(&self) -> usize {
        self.user_data_index
    }

    /// Get a reference to the context
    pub fn get_context(&self) -> &DynamicContext {
        &self.context
    }

    /// Get a mutable reference to the context
    pub fn get_context_mut(&mut self) -> &mut DynamicContext {
        &mut self.context
    }

    /// Mark a route as completed in this scope
    pub fn mark_route_completed(&mut self, route_name: &str) {
        self.completed_routes.insert(route_name.to_string());
    }

    /// Mark extraction as completed for a route in this scope
    pub fn mark_extraction_completed(&mut self, route_name: &str) {
        self.extraction_completed_routes.insert(route_name.to_string());
    }

    /// Check if a route has completed in this scope
    pub fn is_route_completed(&self, route_name: &str) -> bool {
        self.completed_routes.contains(route_name)
    }

    /// Check if extraction has completed for a route in this scope
    pub fn is_extraction_completed(&self, route_name: &str) -> bool {
        self.extraction_completed_routes.contains(route_name)
    }

    /// Add extracted values from a route to this scope's context
    pub fn add_extracted_values(&mut self, values: Vec<ExtractedValue>) -> Result<()> {
        for value in values {
            self.context.check_conflicts(&value.key)?;
            self.context.add_value(value);
        }
        Ok(())
    }

    /// Get completion statistics for this scope
    pub fn get_completion_stats(&self, total_routes: usize) -> ContextScopeStats {
        ContextScopeStats {
            user_data_index: self.user_data_index,
            total_routes,
            completed_routes: self.completed_routes.len(),
            extraction_completed_routes: self.extraction_completed_routes.len(),
            context_values: self.context.value_count(),
        }
    }

    /// Reset the scope for a new execution
    pub fn reset(&mut self) {
        self.completed_routes.clear();
        self.extraction_completed_routes.clear();
        self.context = DynamicContext::for_user_data(self.user_data_index);
    }
}

/// Statistics about a context scope
#[derive(Debug, Clone, Serialize)]
pub struct ContextScopeStats {
    /// User data index for this scope
    pub user_data_index: usize,
    /// Total number of routes
    pub total_routes: usize,
    /// Number of completed routes
    pub completed_routes: usize,
    /// Number of routes with completed extraction
    pub extraction_completed_routes: usize,
    /// Number of values in the context
    pub context_values: usize,
}

impl ContextScopeStats {
    /// Check if all routes are completed in this scope
    pub fn is_complete(&self) -> bool {
        self.completed_routes == self.total_routes
    }

    /// Get completion percentage for this scope
    pub fn completion_percentage(&self) -> f64 {
        if self.total_routes == 0 {
            100.0
        } else {
            (self.completed_routes as f64 / self.total_routes as f64) * 100.0
        }
    }

    /// Check if all extractions are complete for completed routes
    pub fn all_extractions_complete(&self) -> bool {
        self.extraction_completed_routes == self.completed_routes
    }
}

/// Resolves parameter references using both user data and dynamic context
#[derive(Debug, Clone)]
pub struct VariableResolver {
    /// Whether to prioritize context values over user data
    context_priority: bool,
    /// Whether to be strict about missing variables
    strict_mode: bool,
}

impl VariableResolver {
    /// Create a new variable resolver
    pub fn new() -> Self {
        Self {
            context_priority: true, // Context values override CSV by default
            strict_mode: true,
        }
    }

    /// Set whether context values should take priority over user data
    pub fn with_context_priority(mut self, priority: bool) -> Self {
        self.context_priority = priority;
        self
    }

    /// Set whether to be strict about missing variables
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Resolve a parameter value using context and user data
    pub fn resolve_parameter(
        &self,
        param_name: &str,
        context: &DynamicContext,
        user_data: &UserData,
    ) -> Result<Option<String>> {
        // Check context first if context has priority
        if self.context_priority {
            if let Some(value) = context.get_value_string(param_name) {
                return Ok(Some(value.to_string()));
            }
        }

        // Check user data
        if let Some(value) = user_data.data.get(param_name) {
            return Ok(Some(value.clone()));
        }

        // Check context second if user data has priority
        if !self.context_priority {
            if let Some(value) = context.get_value_string(param_name) {
                return Ok(Some(value.to_string()));
            }
        }

        // Handle missing parameter
        if self.strict_mode {
            let available_context = context.get_all_keys().join(", ");
            let available_user_data = user_data.data.keys().cloned().collect::<Vec<_>>().join(", ");
            return Err(HttpDiffError::MissingPathParameter {
                param: param_name.to_string(),
                available_params: format!(
                    "Context: [{}], User Data: [{}]",
                    available_context, available_user_data
                ),
            });
        }

        Ok(None)
    }

    /// Substitute placeholders in text using context and user data
    pub fn substitute_placeholders(
        &self,
        text: &str,
        context: &DynamicContext,
        user_data: &UserData,
        url_encode: bool,
    ) -> Result<String> {
        let mut result = String::with_capacity(text.len() + 50);
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
                    match self.resolve_parameter(&param_name, context, user_data)? {
                        Some(value) => {
                            if url_encode {
                                result.push_str(&urlencoding::encode(&value));
                            } else {
                                result.push_str(&value);
                            }
                        }
                        None => {
                            // Non-strict mode: preserve the original placeholder
                            result.push('{');
                            result.push_str(&param_name);
                            result.push('}');
                        }
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

    /// Get all available parameters from context and user data
    pub fn get_available_parameters(
        &self,
        context: &DynamicContext,
        user_data: &UserData,
    ) -> HashMap<String, String> {
        let mut available = HashMap::new();

        // Add user data first
        for (key, value) in &user_data.data {
            available.insert(key.clone(), value.clone());
        }

        // Add/override with context values (respecting priority)
        if self.context_priority {
            for (key, value) in context.to_key_value_map() {
                available.insert(key, value);
            }
        } else {
            // Only add context values that don't exist in user data
            for (key, value) in context.to_key_value_map() {
                available.entry(key).or_insert(value);
            }
        }

        available
    }
}

impl Default for VariableResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a parameter name is a valid identifier (letters, numbers, underscore)
fn is_valid_param_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Manages the lifecycle of dynamic contexts and provides thread-safe access
#[derive(Debug)]
pub struct ContextManager {
    /// Map of user data index to context scope
    scopes: Arc<RwLock<HashMap<usize, ContextScope>>>,
    /// Variable resolver for parameter substitution
    resolver: VariableResolver,
    /// Global context for values that should be shared across all scopes
    global_context: Arc<RwLock<DynamicContext>>,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new() -> Self {
        Self {
            scopes: Arc::new(RwLock::new(HashMap::new())),
            resolver: VariableResolver::new(),
            global_context: Arc::new(RwLock::new(DynamicContext::new())),
        }
    }

    /// Create a new context manager with custom resolver
    pub fn with_resolver(resolver: VariableResolver) -> Self {
        Self {
            scopes: Arc::new(RwLock::new(HashMap::new())),
            resolver,
            global_context: Arc::new(RwLock::new(DynamicContext::new())),
        }
    }

    /// Create or get a context scope for a user data index
    pub fn get_or_create_scope(&self, user_data_index: usize) -> Result<ContextScope> {
        let scopes = self.scopes.read().map_err(|_| {
            HttpDiffError::general("Failed to acquire read lock on context scopes")
        })?;

        if let Some(scope) = scopes.get(&user_data_index) {
            Ok(scope.clone())
        } else {
            drop(scopes); // Release read lock before acquiring write lock
            let mut scopes = self.scopes.write().map_err(|_| {
                HttpDiffError::general("Failed to acquire write lock on context scopes")
            })?;

            // Double-check in case another thread created the scope
            if let Some(scope) = scopes.get(&user_data_index) {
                Ok(scope.clone())
            } else {
                let scope = ContextScope::new(user_data_index);
                scopes.insert(user_data_index, scope.clone());
                Ok(scope)
            }
        }
    }

    /// Update a context scope
    pub fn update_scope(&self, user_data_index: usize, scope: ContextScope) -> Result<()> {
        let mut scopes = self.scopes.write().map_err(|_| {
            HttpDiffError::general("Failed to acquire write lock on context scopes")
        })?;
        scopes.insert(user_data_index, scope);
        Ok(())
    }

    /// Add extracted values to a specific scope
    pub fn add_values_to_scope(
        &self,
        user_data_index: usize,
        values: Vec<ExtractedValue>,
    ) -> Result<()> {
        let mut scope = self.get_or_create_scope(user_data_index)?;
        scope.add_extracted_values(values)?;
        self.update_scope(user_data_index, scope)?;
        Ok(())
    }

    /// Mark a route as completed in a specific scope
    pub fn mark_route_completed(&self, user_data_index: usize, route_name: &str) -> Result<()> {
        let mut scope = self.get_or_create_scope(user_data_index)?;
        scope.mark_route_completed(route_name);
        self.update_scope(user_data_index, scope)?;
        Ok(())
    }

    /// Mark extraction as completed for a route in a specific scope
    pub fn mark_extraction_completed(
        &self,
        user_data_index: usize,
        route_name: &str,
    ) -> Result<()> {
        let mut scope = self.get_or_create_scope(user_data_index)?;
        scope.mark_extraction_completed(route_name);
        self.update_scope(user_data_index, scope)?;
        Ok(())
    }

    /// Substitute placeholders using context and user data
    pub fn substitute_placeholders(
        &self,
        user_data_index: usize,
        text: &str,
        user_data: &UserData,
        url_encode: bool,
    ) -> Result<String> {
        let scope = self.get_or_create_scope(user_data_index)?;
        let context = scope.get_context();

        self.resolver
            .substitute_placeholders(text, context, user_data, url_encode)
    }

    /// Get available parameters for a specific scope
    pub fn get_available_parameters(
        &self,
        user_data_index: usize,
        user_data: &UserData,
    ) -> Result<HashMap<String, String>> {
        let scope = self.get_or_create_scope(user_data_index)?;
        let context = scope.get_context();

        Ok(self.resolver.get_available_parameters(context, user_data))
    }

    /// Add a value to the global context
    pub fn add_global_value(&self, value: ExtractedValue) -> Result<()> {
        let mut global_context = self.global_context.write().map_err(|_| {
            HttpDiffError::general("Failed to acquire write lock on global context")
        })?;
        global_context.check_conflicts(&value.key)?;
        global_context.add_value(value);
        Ok(())
    }

    /// Get all context scope statistics
    pub fn get_all_scope_stats(&self, total_routes: usize) -> Result<Vec<ContextScopeStats>> {
        let scopes = self.scopes.read().map_err(|_| {
            HttpDiffError::general("Failed to acquire read lock on context scopes")
        })?;

        Ok(scopes
            .values()
            .map(|scope| scope.get_completion_stats(total_routes))
            .collect())
    }

    /// Reset all scopes for a new execution
    pub fn reset_all_scopes(&self) -> Result<()> {
        let mut scopes = self.scopes.write().map_err(|_| {
            HttpDiffError::general("Failed to acquire write lock on context scopes")
        })?;

        for scope in scopes.values_mut() {
            scope.reset();
        }

        // Reset global context as well
        let mut global_context = self.global_context.write().map_err(|_| {
            HttpDiffError::general("Failed to acquire write lock on global context")
        })?;
        *global_context = DynamicContext::new();

        Ok(())
    }

    /// Get the number of active scopes
    pub fn scope_count(&self) -> Result<usize> {
        let scopes = self.scopes.read().map_err(|_| {
            HttpDiffError::general("Failed to acquire read lock on context scopes")
        })?;
        Ok(scopes.len())
    }

    /// Validate all contexts in all scopes
    pub fn validate_all_contexts(&self) -> Result<()> {
        let scopes = self.scopes.read().map_err(|_| {
            HttpDiffError::general("Failed to acquire read lock on context scopes")
        })?;

        for (index, scope) in scopes.iter() {
            if let Err(e) = scope.get_context().validate_required_variables() {
                return Err(HttpDiffError::chain_dependency_error(
                    format!("scope_{}", index),
                    format!("Context validation failed for user data index {}: {}", index, e),
                ));
            }
        }

        // Validate global context
        let global_context = self.global_context.read().map_err(|_| {
            HttpDiffError::general("Failed to acquire read lock on global context")
        })?;
        global_context.validate_required_variables()?;

        Ok(())
    }

    /// Serialize all contexts to JSON for debugging
    pub fn serialize_all_contexts(&self) -> Result<String> {
        let scopes = self.scopes.read().map_err(|_| {
            HttpDiffError::general("Failed to acquire read lock on context scopes")
        })?;

        let global_context = self.global_context.read().map_err(|_| {
            HttpDiffError::general("Failed to acquire read lock on global context")
        })?;

        let mut scope_contexts = HashMap::new();
        for (index, scope) in scopes.iter() {
            scope_contexts.insert(*index, scope.get_context().clone());
        }

        #[derive(serde::Serialize)]
        struct FullContextData {
            global: DynamicContext,
            scopes: HashMap<usize, DynamicContext>,
        }

        let full_data = FullContextData {
            global: global_context.clone(),
            scopes: scope_contexts,
        };

        serde_json::to_string_pretty(&full_data).map_err(|e| {
            HttpDiffError::general(format!("Failed to serialize all contexts to JSON: {}", e))
        })
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn create_test_user_data(data: Vec<(&str, &str)>) -> UserData {
        UserData {
            data: data.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    #[test]
    fn test_dynamic_context_creation() {
        let context = DynamicContext::new();
        assert!(context.is_empty());
        assert_eq!(context.value_count(), 0);
        assert!(!context.get_context_id().is_empty());
    }

    #[test]
    fn test_dynamic_context_add_value() {
        let mut context = DynamicContext::new();
        let value = create_test_extracted_value("user_id", "123", "auth", "dev");

        context.add_value(value.clone());
        assert!(!context.is_empty());
        assert_eq!(context.value_count(), 1);
        assert!(context.has_value("user_id"));
        assert_eq!(context.get_value_string("user_id"), Some("123"));
    }

    #[test]
    fn test_dynamic_context_required_variables() {
        let mut context = DynamicContext::new();
        context.mark_required("user_id".to_string());

        // Should fail validation without the required variable
        assert!(context.validate_required_variables().is_err());

        // Should pass after adding the required variable
        let value = create_test_extracted_value("user_id", "123", "auth", "dev");
        context.add_value(value);
        assert!(context.validate_required_variables().is_ok());
    }

    #[test]
    fn test_dynamic_context_conflicts() {
        let mut context = DynamicContext::new();
        let value1 = create_test_extracted_value("user_id", "123", "auth", "dev");
        let _value2 = create_test_extracted_value("user_id", "456", "users", "staging");

        context.add_value(value1);
        assert!(context.check_conflicts("user_id").is_err());
        assert!(context.check_conflicts("other_var").is_ok());
    }

    #[test]
    fn test_dynamic_context_merge() {
        let mut context1 = DynamicContext::new();
        let mut context2 = DynamicContext::new();

        context1.add_value(create_test_extracted_value("user_id", "123", "auth", "dev"));
        context2.add_value(create_test_extracted_value("profile_id", "456", "profile", "dev"));

        assert!(context1.merge(&context2).is_ok());
        assert_eq!(context1.value_count(), 2);
        assert!(context1.has_value("user_id"));
        assert!(context1.has_value("profile_id"));
    }

    #[test]
    fn test_dynamic_context_merge_conflict() {
        let mut context1 = DynamicContext::new();
        let mut context2 = DynamicContext::new();

        context1.add_value(create_test_extracted_value("user_id", "123", "auth", "dev"));
        context2.add_value(create_test_extracted_value("user_id", "456", "profile", "dev"));

        assert!(context1.merge(&context2).is_err());
    }

    #[test]
    fn test_dynamic_context_filter() {
        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "123", "auth", "dev"));
        context.add_value(create_test_extracted_value("profile_id", "456", "profile", "dev"));
        context.add_value(create_test_extracted_value("org_id", "789", "org", "dev"));

        let filtered = context.filter(&["user_id".to_string(), "org_id".to_string()]);
        assert_eq!(filtered.value_count(), 2);
        assert!(filtered.has_value("user_id"));
        assert!(filtered.has_value("org_id"));
        assert!(!filtered.has_value("profile_id"));
    }

    #[test]
    fn test_context_scope() {
        let mut scope = ContextScope::new(0);
        assert_eq!(scope.get_user_data_index(), 0);

        scope.mark_route_completed("auth");
        assert!(scope.is_route_completed("auth"));
        assert!(!scope.is_route_completed("users"));

        scope.mark_extraction_completed("auth");
        assert!(scope.is_extraction_completed("auth"));
    }

    #[test]
    fn test_context_scope_add_values() {
        let mut scope = ContextScope::new(0);
        let values = vec![
            create_test_extracted_value("user_id", "123", "auth", "dev"),
            create_test_extracted_value("token", "abc", "auth", "dev"),
        ];

        assert!(scope.add_extracted_values(values).is_ok());
        assert_eq!(scope.get_context().value_count(), 2);
    }

    #[test]
    fn test_context_scope_stats() {
        let mut scope = ContextScope::new(0);
        scope.mark_route_completed("auth");
        scope.mark_extraction_completed("auth");

        let stats = scope.get_completion_stats(3);
        assert_eq!(stats.user_data_index, 0);
        assert_eq!(stats.total_routes, 3);
        assert_eq!(stats.completed_routes, 1);
        assert_eq!(stats.extraction_completed_routes, 1);
        assert!(!stats.is_complete());
        assert!((stats.completion_percentage() - 33.33).abs() < 0.1);
    }

    #[test]
    fn test_variable_resolver_context_priority() {
        let resolver = VariableResolver::new().with_context_priority(true);
        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "context_123", "auth", "dev"));

        let user_data = create_test_user_data(vec![("user_id", "csv_456")]);

        let result = resolver.resolve_parameter("user_id", &context, &user_data).unwrap();
        assert_eq!(result, Some("context_123".to_string()));
    }

    #[test]
    fn test_variable_resolver_user_data_priority() {
        let resolver = VariableResolver::new().with_context_priority(false);
        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "context_123", "auth", "dev"));

        let user_data = create_test_user_data(vec![("user_id", "csv_456")]);

        let result = resolver.resolve_parameter("user_id", &context, &user_data).unwrap();
        assert_eq!(result, Some("csv_456".to_string()));
    }

    #[test]
    fn test_variable_resolver_substitute_placeholders() {
        let resolver = VariableResolver::new().with_strict_mode(false);
        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "123", "auth", "dev"));

        let user_data = create_test_user_data(vec![("org_id", "456")]);

        let text = "/api/users/{user_id}/orgs/{org_id}/missing/{unknown}";
        let result = resolver
            .substitute_placeholders(text, &context, &user_data, false)
            .unwrap();

        assert_eq!(result, "/api/users/123/orgs/456/missing/{unknown}");
    }

    #[test]
    fn test_variable_resolver_strict_mode() {
        let resolver = VariableResolver::new().with_strict_mode(true);
        let context = DynamicContext::new();
        let user_data = create_test_user_data(vec![]);

        let result = resolver.resolve_parameter("missing", &context, &user_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found in user data"));
    }

    #[test]
    fn test_context_manager() {
        let manager = ContextManager::new();
        
        // Create scopes
        let scope1 = manager.get_or_create_scope(0).unwrap();
        let scope2 = manager.get_or_create_scope(1).unwrap();
        assert_ne!(scope1.get_user_data_index(), scope2.get_user_data_index());

        // Add values
        let values = vec![create_test_extracted_value("user_id", "123", "auth", "dev")];
        assert!(manager.add_values_to_scope(0, values).is_ok());

        // Check scope count
        assert_eq!(manager.scope_count().unwrap(), 2);
    }

    #[test]
    fn test_context_manager_substitute() {
        let manager = ContextManager::new();
        let values = vec![create_test_extracted_value("user_id", "123", "auth", "dev")];
        manager.add_values_to_scope(0, values).unwrap();

        let user_data = create_test_user_data(vec![("org_id", "456")]);
        let text = "/api/users/{user_id}/orgs/{org_id}";

        let result = manager
            .substitute_placeholders(0, text, &user_data, false)
            .unwrap();
        assert_eq!(result, "/api/users/123/orgs/456");
    }

    #[test]
    fn test_context_serialization() {
        let mut context = DynamicContext::new();
        context.add_value(create_test_extracted_value("user_id", "123", "auth", "dev"));
        context.mark_required("user_id".to_string());

        let json = context.to_json().unwrap();
        assert!(json.contains("user_id"));
        assert!(json.contains("123"));

        let deserialized = DynamicContext::from_json(&json).unwrap();
        assert_eq!(deserialized.value_count(), 1);
        assert_eq!(deserialized.get_value_string("user_id"), Some("123"));
    }
}