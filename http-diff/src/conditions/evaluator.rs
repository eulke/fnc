use crate::config::{Route, UserData};
use crate::error::Result;
use crate::traits::ConditionEvaluator;
use super::types::{ConditionOperator, ConditionResult, ExecutionCondition};

/// Default implementation of condition evaluation
#[derive(Debug, Clone)]
pub struct ConditionEvaluatorImpl;

impl ConditionEvaluatorImpl {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate a single condition against user data
    fn evaluate_single_condition(
        &self,
        condition: &ExecutionCondition,
        user_data: &UserData,
    ) -> Result<ConditionResult> {
        let actual_value = self.get_variable_value(&condition.variable, user_data);

        let (passed, reason) = match &condition.operator {
            ConditionOperator::Exists => (actual_value.is_some(), None),
            ConditionOperator::NotExists => (actual_value.is_none(), None),
            ConditionOperator::Equals => {
                let passed = actual_value
                    .as_ref()
                    .map(|v| v == &condition.value)
                    .unwrap_or(false);
                let reason = if !passed && actual_value.is_none() {
                    Some(format!("Variable '{}' not found in user data", condition.variable))
                } else {
                    None
                };
                (passed, reason)
            }
            ConditionOperator::NotEquals => {
                let passed = actual_value
                    .as_ref()
                    .map(|v| v != &condition.value)
                    .unwrap_or(true);
                (passed, None)
            }
            ConditionOperator::Contains => {
                let passed = actual_value
                    .as_ref()
                    .map(|v| v.contains(&condition.value))
                    .unwrap_or(false);
                let reason = if !passed && actual_value.is_none() {
                    Some(format!("Variable '{}' not found in user data", condition.variable))
                } else {
                    None
                };
                (passed, reason)
            }
            ConditionOperator::NotContains => {
                let passed = actual_value
                    .as_ref()
                    .map(|v| !v.contains(&condition.value))
                    .unwrap_or(true);
                (passed, None)
            }
            ConditionOperator::GreaterThan => {
                self.evaluate_numeric_condition(&actual_value, &condition.value, |a, b| a > b)
            }
            ConditionOperator::LessThan => {
                self.evaluate_numeric_condition(&actual_value, &condition.value, |a, b| a < b)
            }
        };

        Ok(ConditionResult {
            condition: condition.clone(),
            passed,
            actual_value,
            reason,
        })
    }

    /// Get variable value from user data or environment
    fn get_variable_value(&self, variable: &str, user_data: &UserData) -> Option<String> {
        // First check user data
        if let Some(value) = user_data.data.get(variable) {
            return Some(value.clone());
        }

        // Then check environment variables if prefixed with "env."
        if let Some(env_var) = variable.strip_prefix("env.") {
            return std::env::var(env_var).ok();
        }

        None
    }

    /// Evaluate numeric comparison
    fn evaluate_numeric_condition(
        &self,
        actual_value: &Option<String>,
        expected_value: &str,
        comparator: fn(f64, f64) -> bool,
    ) -> (bool, Option<String>) {
        match actual_value {
            Some(actual) => match (actual.parse::<f64>(), expected_value.parse::<f64>()) {
                (Ok(actual_num), Ok(expected_num)) => (comparator(actual_num, expected_num), None),
                _ => (
                    false,
                    Some("Cannot parse values as numbers".to_string()),
                ),
            },
            None => (
                false,
                Some("Variable not found in user data".to_string()),
            ),
        }
    }
}

impl Default for ConditionEvaluatorImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionEvaluator for ConditionEvaluatorImpl {
    fn should_execute_route(&self, route: &Route, user_data: &UserData) -> Result<bool> {
        let conditions = match &route.conditions {
            Some(conditions) => conditions,
            None => return Ok(true), // No conditions = always execute
        };

        if conditions.is_empty() {
            return Ok(true);
        }

        let results = self.evaluate_conditions(conditions, user_data)?;

        // All conditions must pass for route to execute
        Ok(results.iter().all(|r| r.passed))
    }

    fn evaluate_conditions(
        &self,
        conditions: &[ExecutionCondition],
        user_data: &UserData,
    ) -> Result<Vec<ConditionResult>> {
        conditions
            .iter()
            .map(|condition| self.evaluate_single_condition(condition, user_data))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_user_data() -> UserData {
        let mut data = HashMap::new();
        data.insert("user_type".to_string(), "premium".to_string());
        data.insert("user_id".to_string(), "1500".to_string());
        data.insert("status".to_string(), "active".to_string());
        UserData { data }
    }

    #[test]
    fn test_equals_condition_passes() {
        let evaluator = ConditionEvaluatorImpl::new();
        let condition = ExecutionCondition {
            variable: "user_type".to_string(),
            operator: ConditionOperator::Equals,
            value: "premium".to_string(),
        };
        let user_data = create_test_user_data();

        let result = evaluator.evaluate_single_condition(&condition, &user_data).unwrap();
        assert!(result.passed);
        assert_eq!(result.actual_value, Some("premium".to_string()));
    }

    #[test]
    fn test_equals_condition_fails() {
        let evaluator = ConditionEvaluatorImpl::new();
        let condition = ExecutionCondition {
            variable: "user_type".to_string(),
            operator: ConditionOperator::Equals,
            value: "basic".to_string(),
        };
        let user_data = create_test_user_data();

        let result = evaluator.evaluate_single_condition(&condition, &user_data).unwrap();
        assert!(!result.passed);
        assert_eq!(result.actual_value, Some("premium".to_string()));
    }

    #[test]
    fn test_greater_than_condition() {
        let evaluator = ConditionEvaluatorImpl::new();
        let condition = ExecutionCondition {
            variable: "user_id".to_string(),
            operator: ConditionOperator::GreaterThan,
            value: "1000".to_string(),
        };
        let user_data = create_test_user_data();

        let result = evaluator.evaluate_single_condition(&condition, &user_data).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_contains_condition() {
        let evaluator = ConditionEvaluatorImpl::new();
        let condition = ExecutionCondition {
            variable: "status".to_string(),
            operator: ConditionOperator::Contains,
            value: "act".to_string(),
        };
        let user_data = create_test_user_data();

        let result = evaluator.evaluate_single_condition(&condition, &user_data).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_exists_condition() {
        let evaluator = ConditionEvaluatorImpl::new();
        let condition = ExecutionCondition {
            variable: "user_type".to_string(),
            operator: ConditionOperator::Exists,
            value: "".to_string(),
        };
        let user_data = create_test_user_data();

        let result = evaluator.evaluate_single_condition(&condition, &user_data).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_not_exists_condition() {
        let evaluator = ConditionEvaluatorImpl::new();
        let condition = ExecutionCondition {
            variable: "nonexistent_field".to_string(),
            operator: ConditionOperator::NotExists,
            value: "".to_string(),
        };
        let user_data = create_test_user_data();

        let result = evaluator.evaluate_single_condition(&condition, &user_data).unwrap();
        assert!(result.passed);
    }
}