use crate::config::types::HttpDiffConfig;
use crate::conditions::types::{ConditionOperator, ExecutionCondition};
use crate::error::{HttpDiffError, Result};
use crate::traits::ConfigValidator;
use std::path::Path;

/// Configuration validator implementation
pub struct ConfigValidatorImpl;

impl ConfigValidator for ConfigValidatorImpl {
    type Config = HttpDiffConfig;

    /// Validate configuration (uses enhanced validation with default context)
    fn validate(&self, config: &HttpDiffConfig) -> Result<()> {
        self.validate_with_context(config, "configuration")
    }
}

impl ConfigValidatorImpl {
    /// Create a new validator
    pub fn new() -> Self {
        Self
    }

    /// Validation with enhanced error context
    pub fn validate_with_context<P: AsRef<Path>>(
        &self,
        config: &HttpDiffConfig,
        config_path: P,
    ) -> Result<()> {
        let config_path_str = config_path.as_ref().to_string_lossy();

        if config.environments.is_empty() {
            return Err(HttpDiffError::invalid_config(format!(
                "No environments configured in {}. Add at least one environment to [environments] section.",
                config_path_str
            )));
        }

        if config.routes.is_empty() {
            return Err(HttpDiffError::invalid_config(format!(
                "No routes configured in {}. Add at least one [[routes]] entry.",
                config_path_str
            )));
        }

        // Ensure no more than one base environment is selected
        let base_count = config.environments.values().filter(|e| e.is_base).count();
        if base_count > 1 {
            return Err(HttpDiffError::invalid_config(format!(
                "Multiple environments are marked as base in {}; only one is allowed",
                config_path_str
            )));
        }

        // Validate HTTP methods and environment references
        for route in &config.routes {
            let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
            if !valid_methods.contains(&route.method.as_str()) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Invalid HTTP method '{}' in route '{}'. Valid methods: {}",
                    route.method,
                    route.name,
                    valid_methods.join(", ")
                )));
            }

            // Validate base URL overrides reference existing environments
            if let Some(base_urls) = &route.base_urls {
                for env_name in base_urls.keys() {
                    if !config.environments.contains_key(env_name) {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Route '{}' references unknown environment '{}' in base_urls. Available environments: {}",
                            route.name,
                            env_name,
                            config.environments.keys().map(String::as_str).collect::<Vec<_>>().join(", ")
                        )));
                    }
                }
            }

            // Validate execution conditions if present
            if let Some(conditions) = &route.conditions {
                self.validate_conditions(conditions, &route.name, &config_path_str)?;
            }
        }

        // Validate URLs in environments
        for (env_name, env) in &config.environments {
            if url::Url::parse(&env.base_url).is_err() {
                return Err(HttpDiffError::invalid_config(format!(
                    "Invalid base_url '{}' in environment '{}'. Must be a valid URL.",
                    env.base_url, env_name
                )));
            }
        }

        // Validate timeout if specified
        if let Some(global) = &config.global {
            if let Some(timeout) = global.timeout_seconds {
                if timeout == 0 || timeout > 300 {
                    return Err(HttpDiffError::invalid_config(
                        "timeout_seconds must be between 1 and 300 seconds".to_string(),
                    ));
                }
            }

            // Validate max_concurrent_requests if specified
            if let Some(max_concurrent) = global.max_concurrent_requests {
                if max_concurrent == 0 || max_concurrent > 100 {
                    return Err(HttpDiffError::invalid_config(
                        "max_concurrent_requests must be between 1 and 100".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate execution conditions for a route
    fn validate_conditions(
        &self,
        conditions: &[ExecutionCondition],
        route_name: &str,
        config_path: &str,
    ) -> Result<()> {
        for (index, condition) in conditions.iter().enumerate() {
            // Validate variable name is not empty or whitespace-only
            if condition.variable.trim().is_empty() {
                return Err(HttpDiffError::invalid_config(format!(
                    "Empty or whitespace-only variable name in condition {} for route '{}' in {}. Variable names must contain non-whitespace characters.",
                    index + 1,
                    route_name,
                    config_path
                )));
            }

            // Validate numeric operations have numeric values
            match condition.operator {
                ConditionOperator::GreaterThan | ConditionOperator::LessThan => {
                    if condition.value.parse::<f64>().is_err() {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Invalid numeric value '{}' for {} operator in condition {} for route '{}' in {}. Numeric operators require numeric values (e.g., '42', '3.14', '-10').",
                            condition.value,
                            format!("{:?}", condition.operator).to_lowercase(),
                            index + 1,
                            route_name,
                            config_path
                        )));
                    }
                }
                ConditionOperator::Exists | ConditionOperator::NotExists => {
                    // For exists/not_exists operators, the value field is not used but we validate it's not confusing
                    if !condition.value.trim().is_empty() {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Unexpected value '{}' for {} operator in condition {} for route '{}' in {}. Exists/NotExists operators do not use the value field - please remove it or use an empty string.",
                            condition.value,
                            format!("{:?}", condition.operator).to_lowercase(),
                            index + 1,
                            route_name,
                            config_path
                        )));
                    }
                }
                ConditionOperator::Equals
                | ConditionOperator::NotEquals
                | ConditionOperator::Contains
                | ConditionOperator::NotContains => {
                    // String operations allow any value, including empty strings
                    // No additional validation needed for these operators
                }
            }

            // Validate variable name contains only valid characters
            if !is_valid_variable_name(&condition.variable) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Invalid variable name '{}' in condition {} for route '{}' in {}. Variable names must contain only letters, numbers, and underscores, and cannot start with a number.",
                    condition.variable,
                    index + 1,
                    route_name,
                    config_path
                )));
            }
        }

        Ok(())
    }
}

/// Check if a variable name is valid (letters, numbers, underscore, cannot start with number)
fn is_valid_variable_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    let mut chars = name.chars();
    // First character cannot be a digit
    if let Some(first_char) = chars.next() {
        if first_char.is_numeric() {
            return false;
        }
        if !first_char.is_alphabetic() && first_char != '_' {
            return false;
        }
    }
    
    // Remaining characters must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

impl Default for ConfigValidatorImpl {
    fn default() -> Self {
        Self::new()
    }
}

// Add convenience methods to HttpDiffConfig
impl HttpDiffConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        let validator = ConfigValidatorImpl::new();
        validator.validate(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::types::{ConditionOperator, ExecutionCondition};
    use crate::config::types::{Environment, Route};
    use crate::traits::ConfigValidator;
    use std::collections::HashMap;

    fn create_test_config_with_conditions(conditions: Vec<ExecutionCondition>) -> HttpDiffConfig {
        let mut environments = HashMap::new();
        environments.insert(
            "dev".to_string(),
            Environment {
                base_url: "https://dev.example.com".to_string(),
                headers: None,
                is_base: true,
            },
        );

        let route = Route {
            name: "test_route".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
            conditions: Some(conditions),
        };

        HttpDiffConfig {
            environments,
            global: None,
            routes: vec![route],
        }
    }

    #[test]
    fn test_valid_conditions() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![
            ExecutionCondition {
                variable: "user_type".to_string(),
                operator: ConditionOperator::Equals,
                value: "admin".to_string(),
            },
            ExecutionCondition {
                variable: "age".to_string(),
                operator: ConditionOperator::GreaterThan,
                value: "18".to_string(),
            },
            ExecutionCondition {
                variable: "account_exists".to_string(),
                operator: ConditionOperator::Exists,
                value: "".to_string(),
            },
        ];

        let config = create_test_config_with_conditions(conditions);
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_empty_variable_name_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "".to_string(),
            operator: ConditionOperator::Equals,
            value: "test".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Empty or whitespace-only variable name"));
    }

    #[test]
    fn test_whitespace_only_variable_name_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "   ".to_string(),
            operator: ConditionOperator::Equals,
            value: "test".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Empty or whitespace-only variable name"));
    }

    #[test]
    fn test_invalid_numeric_value_for_greater_than_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "age".to_string(),
            operator: ConditionOperator::GreaterThan,
            value: "not_a_number".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Invalid numeric value"));
        assert!(error_message.contains("greaterthan"));
    }

    #[test]
    fn test_invalid_numeric_value_for_less_than_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "score".to_string(),
            operator: ConditionOperator::LessThan,
            value: "invalid".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Invalid numeric value"));
        assert!(error_message.contains("lessthan"));
    }

    #[test]
    fn test_exists_operator_with_value_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "user_id".to_string(),
            operator: ConditionOperator::Exists,
            value: "should_be_empty".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Unexpected value"));
        assert!(error_message.contains("exists"));
    }

    #[test]
    fn test_not_exists_operator_with_value_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "temp_flag".to_string(),
            operator: ConditionOperator::NotExists,
            value: "should_be_empty".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Unexpected value"));
        assert!(error_message.contains("notexists"));
    }

    #[test]
    fn test_invalid_variable_name_with_special_characters_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "user-name".to_string(), // Hyphens are not allowed
            operator: ConditionOperator::Equals,
            value: "test".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Invalid variable name"));
    }

    #[test]
    fn test_invalid_variable_name_starting_with_number_fails() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![ExecutionCondition {
            variable: "123user".to_string(), // Cannot start with number
            operator: ConditionOperator::Equals,
            value: "test".to_string(),
        }];

        let config = create_test_config_with_conditions(conditions);
        let result = validator.validate(&config);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Invalid variable name"));
    }

    #[test]
    fn test_valid_variable_names_pass() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![
            ExecutionCondition {
                variable: "user_name".to_string(), // Underscores are allowed
                operator: ConditionOperator::Equals,
                value: "test".to_string(),
            },
            ExecutionCondition {
                variable: "_private_var".to_string(), // Can start with underscore
                operator: ConditionOperator::Equals,
                value: "test".to_string(),
            },
            ExecutionCondition {
                variable: "var123".to_string(), // Can contain numbers
                operator: ConditionOperator::Equals,
                value: "test".to_string(),
            },
        ];

        let config = create_test_config_with_conditions(conditions);
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_numeric_operations_with_valid_values_pass() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![
            ExecutionCondition {
                variable: "age".to_string(),
                operator: ConditionOperator::GreaterThan,
                value: "42".to_string(),
            },
            ExecutionCondition {
                variable: "score".to_string(),
                operator: ConditionOperator::LessThan,
                value: "100.5".to_string(), // Decimal numbers are valid
            },
            ExecutionCondition {
                variable: "balance".to_string(),
                operator: ConditionOperator::GreaterThan,
                value: "-10".to_string(), // Negative numbers are valid
            },
        ];

        let config = create_test_config_with_conditions(conditions);
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_string_operations_allow_any_value() {
        let validator = ConfigValidatorImpl::new();
        let conditions = vec![
            ExecutionCondition {
                variable: "name".to_string(),
                operator: ConditionOperator::Equals,
                value: "".to_string(), // Empty string is allowed for string operations
            },
            ExecutionCondition {
                variable: "description".to_string(),
                operator: ConditionOperator::Contains,
                value: "special chars: !@#$%".to_string(),
            },
            ExecutionCondition {
                variable: "status".to_string(),
                operator: ConditionOperator::NotEquals,
                value: "123".to_string(), // Numbers as strings are allowed
            },
        ];

        let config = create_test_config_with_conditions(conditions);
        assert!(validator.validate(&config).is_ok());
    }
}
