use serde::{Deserialize, Serialize};

/// Condition for conditional route execution
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionCondition {
    /// Variable name to check (from user data or environment)
    pub variable: String,
    /// Comparison operator
    pub operator: ConditionOperator,
    /// Value to compare against
    pub value: String,
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