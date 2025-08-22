use serde::{Deserialize, Serialize};
use crate::error::{HttpDiffError, Result};

/// Condition for conditional route execution
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionCondition {
    /// Variable name to check (from user data or environment)
    pub variable: String,
    /// Comparison operator
    pub operator: ConditionOperator,
    /// Value to compare against (optional for existence operators)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Comparison operators for execution conditions
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Check if variable equals value
    Equals,
    /// Check if variable does not equal value
    NotEquals,
    /// Check if variable contains value as substring
    Contains,
    /// Check if variable does not contain value as substring
    NotContains,
    /// Check if variable is numerically greater than value
    GreaterThan,
    /// Check if variable is numerically less than value
    LessThan,
    /// Check if variable exists (has any value)
    Exists,
    /// Check if variable does not exist
    NotExists,
}

impl ExecutionCondition {
    /// Create a new condition with validation
    pub fn new(
        variable: impl Into<String>,
        operator: ConditionOperator,
        value: Option<impl Into<String>>,
    ) -> Result<Self> {
        let variable = variable.into();
        let value = value.map(|v| v.into());
        
        let condition = Self {
            variable,
            operator,
            value,
        };
        
        condition.validate()?;
        Ok(condition)
    }
    
    /// Create an existence condition (convenience method)
    pub fn exists(variable: impl Into<String>) -> Self {
        Self {
            variable: variable.into(),
            operator: ConditionOperator::Exists,
            value: None,
        }
    }
    
    /// Create a non-existence condition (convenience method)
    pub fn not_exists(variable: impl Into<String>) -> Self {
        Self {
            variable: variable.into(),
            operator: ConditionOperator::NotExists,
            value: None,
        }
    }
    
    /// Create an equality condition (convenience method)
    pub fn equals(variable: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            variable: variable.into(),
            operator: ConditionOperator::Equals,
            value: Some(value.into()),
        }
    }
    
    /// Validate that the condition is properly configured
    pub fn validate(&self) -> Result<()> {
        match self.operator {
            ConditionOperator::Exists | ConditionOperator::NotExists => {
                // These operators don't need values, any provided value will be ignored
                Ok(())
            }
            ConditionOperator::Equals
            | ConditionOperator::NotEquals
            | ConditionOperator::Contains
            | ConditionOperator::NotContains
            | ConditionOperator::GreaterThan
            | ConditionOperator::LessThan => {
                if self.value.is_none() || self.value.as_ref().is_none_or(|v| v.is_empty()) {
                    return Err(HttpDiffError::invalid_config(format!(
                        "Operator '{:?}' requires a non-empty value but none was provided for variable '{}'", 
                        self.operator, self.variable
                    )));
                }
                Ok(())
            }
        }
    }
    
    /// Get the value, ensuring it exists for operators that require it
    pub fn get_value(&self) -> Result<&str> {
        match self.operator {
            ConditionOperator::Exists | ConditionOperator::NotExists => {
                Err(HttpDiffError::invalid_config(format!(
                    "Operator '{:?}' does not use a value", 
                    self.operator
                )))
            }
            _ => {
                self.value.as_deref().ok_or_else(|| {
                    HttpDiffError::invalid_config(format!(
                        "Operator '{:?}' requires a value but none was provided for variable '{}'", 
                        self.operator, self.variable
                    ))
                })
            }
        }
    }
    
    /// Check if this operator requires a value
    pub fn requires_value(&self) -> bool {
        !matches!(self.operator, ConditionOperator::Exists | ConditionOperator::NotExists)
    }
}

/// Result of evaluating a single condition
#[derive(Debug, Clone)]
pub struct ConditionResult {
    /// The condition that was evaluated
    pub condition: ExecutionCondition,
    /// Whether the condition passed
    pub passed: bool,
    /// The actual value found for the variable (if any)
    pub actual_value: Option<String>,
    /// Optional reason for failure
    pub reason: Option<String>,
}