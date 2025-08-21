//! Value extraction module for extracting values from HTTP responses
//! 
//! This module provides functionality to extract values from HTTP responses using
//! various extraction methods including JsonPath, Regex, Header, and StatusCode.

pub mod extractors;

use crate::error::{HttpDiffError, Result};
use crate::traits::ValueExtractor;
use crate::types::{
    ExtractionRule, ExtractionResult, ExtractionType, ExtractedValue, ExtractionError,
    ValueExtractionContext, HttpResponse,
};
use std::collections::HashMap;
use std::sync::Arc;

pub use extractors::{
    JsonPathExtractor, RegexExtractor, HeaderExtractor, StatusCodeExtractor,
};

/// Main value extraction engine that orchestrates different extractors
#[derive(Clone)]
pub struct ValueExtractionEngine {
    /// Map of extraction type to extractor implementation
    extractors: HashMap<ExtractionType, Arc<dyn ValueExtractor>>,
}

impl Default for ValueExtractionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueExtractionEngine {
    /// Create a new value extraction engine with default extractors
    pub fn new() -> Self {
        let mut extractors: HashMap<ExtractionType, Arc<dyn ValueExtractor>> = HashMap::new();
        
        // Register default extractors
        extractors.insert(ExtractionType::JsonPath, Arc::new(JsonPathExtractor::new()));
        extractors.insert(ExtractionType::Regex, Arc::new(RegexExtractor::new()));
        extractors.insert(ExtractionType::Header, Arc::new(HeaderExtractor::new()));
        extractors.insert(ExtractionType::StatusCode, Arc::new(StatusCodeExtractor::new()));
        
        Self { extractors }
    }

    /// Register a custom extractor for a specific extraction type
    pub fn register_extractor(&mut self, extraction_type: ExtractionType, extractor: Arc<dyn ValueExtractor>) {
        self.extractors.insert(extraction_type, extractor);
    }

    /// Extract values from an HTTP response using multiple extraction rules
    pub fn extract_values(
        &self,
        route_name: String,
        environment: String,
        response: HttpResponse,
        user_context: HashMap<String, String>,
        rules: &[ExtractionRule],
    ) -> Result<ExtractionResult> {
        let context = ValueExtractionContext::new(
            route_name.clone(),
            environment.clone(),
            response,
            user_context,
        );

        let mut result = ExtractionResult::new(context.clone());

        for rule in rules {
            match self.extract_single_value(&context, rule) {
                Ok(Some(value)) => {
                    let extracted_value = ExtractedValue::new(
                        rule.key.clone(),
                        value,
                        rule.pattern.clone(),
                        rule.extraction_type.clone(),
                        environment.clone(),
                        route_name.clone(),
                    );
                    result.add_value(extracted_value);
                }
                Ok(None) => {
                    // Value not found - check if required or has default
                    if rule.required {
                        let error = ExtractionError::new(
                            rule.clone(),
                            format!("Required value not found using pattern: {}", rule.pattern),
                            environment.clone(),
                            route_name.clone(),
                        );
                        result.add_error(error);
                    } else if let Some(default_value) = &rule.default_value {
                        let extracted_value = ExtractedValue::new(
                            rule.key.clone(),
                            default_value.clone(),
                            format!("default: {}", default_value),
                            rule.extraction_type.clone(),
                            environment.clone(),
                            route_name.clone(),
                        );
                        result.add_value(extracted_value);
                    }
                    // If not required and no default, we simply skip this extraction
                }
                Err(err) => {
                    let error = ExtractionError::new(
                        rule.clone(),
                        format!("Extraction failed: {}", err),
                        environment.clone(),
                        route_name.clone(),
                    );
                    result.add_error(error);
                }
            }
        }

        Ok(result)
    }

    /// Extract a single value using a specific extraction rule
    pub fn extract_single_value(
        &self,
        context: &ValueExtractionContext,
        rule: &ExtractionRule,
    ) -> Result<Option<String>> {
        let extractor = self.extractors.get(&rule.extraction_type)
            .ok_or_else(|| HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                format!("No extractor registered for type: {:?}", rule.extraction_type),
            ))?;

        if !extractor.supports_rule(rule) {
            return Err(HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                format!("Extractor does not support rule: {:?}", rule),
            ));
        }

        extractor.extract_single_value(context, rule)
    }

    /// Get available extraction types
    pub fn available_extraction_types(&self) -> Vec<ExtractionType> {
        self.extractors.keys().cloned().collect()
    }

    /// Check if an extraction type is supported
    pub fn supports_extraction_type(&self, extraction_type: &ExtractionType) -> bool {
        self.extractors.contains_key(extraction_type)
    }

    /// Validate extraction rules before execution
    pub fn validate_rules(&self, rules: &[ExtractionRule]) -> Result<()> {
        for rule in rules {
            if !self.supports_extraction_type(&rule.extraction_type) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Unsupported extraction type '{}' for rule '{}'",
                    rule.extraction_type.name(),
                    rule.key
                )));
            }

            // Validate rule-specific constraints
            match rule.extraction_type {
                ExtractionType::JsonPath => {
                    if rule.pattern.is_empty() {
                        return Err(HttpDiffError::invalid_config(format!(
                            "JsonPath pattern cannot be empty for rule '{}'",
                            rule.key
                        )));
                    }
                }
                ExtractionType::Regex => {
                    if rule.pattern.is_empty() {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Regex pattern cannot be empty for rule '{}'",
                            rule.key
                        )));
                    }
                    // Validate regex pattern compilation
                    if let Err(err) = regex::Regex::new(&rule.pattern) {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Invalid regex pattern '{}' for rule '{}': {}",
                            rule.pattern, rule.key, err
                        )));
                    }
                }
                ExtractionType::Header => {
                    if rule.pattern.is_empty() {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Header name cannot be empty for rule '{}'",
                            rule.key
                        )));
                    }
                }
                ExtractionType::StatusCode => {
                    // StatusCode doesn't use pattern, but we can validate it's not misleading
                    if !rule.pattern.is_empty() {
                        return Err(HttpDiffError::invalid_config(format!(
                            "StatusCode extraction does not use pattern, but pattern '{}' was provided for rule '{}'",
                            rule.pattern, rule.key
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Builder for creating value extraction engines with custom configurations
#[derive(Default)]
pub struct ValueExtractionEngineBuilder {
    custom_extractors: HashMap<ExtractionType, Arc<dyn ValueExtractor>>,
}

impl ValueExtractionEngineBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom extractor for a specific extraction type
    pub fn with_extractor(mut self, extraction_type: ExtractionType, extractor: Arc<dyn ValueExtractor>) -> Self {
        self.custom_extractors.insert(extraction_type, extractor);
        self
    }

    /// Build the value extraction engine
    pub fn build(self) -> ValueExtractionEngine {
        let mut engine = ValueExtractionEngine::new();
        
        // Override with custom extractors
        for (extraction_type, extractor) in self.custom_extractors {
            engine.register_extractor(extraction_type, extractor);
        }
        
        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExtractionType, ExtractionRule};
    use std::collections::HashMap;

    fn create_test_response() -> HttpResponse {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Request-ID".to_string(), "abc123".to_string());

        HttpResponse::new(
            200,
            headers,
            r#"{"user_id": 42, "name": "John Doe", "email": "john@example.com"}"#.to_string(),
            "https://api.example.com/users/42".to_string(),
            "curl -X GET https://api.example.com/users/42".to_string(),
        )
    }

    #[test]
    fn test_value_extraction_engine_creation() {
        let engine = ValueExtractionEngine::new();
        
        assert!(engine.supports_extraction_type(&ExtractionType::JsonPath));
        assert!(engine.supports_extraction_type(&ExtractionType::Regex));
        assert!(engine.supports_extraction_type(&ExtractionType::Header));
        assert!(engine.supports_extraction_type(&ExtractionType::StatusCode));
        
        let available_types = engine.available_extraction_types();
        assert_eq!(available_types.len(), 4);
    }

    #[test]
    fn test_extraction_rule_validation() {
        let engine = ValueExtractionEngine::new();
        
        // Valid rules
        let valid_rules = vec![
            ExtractionRule::new("user_id".to_string(), ExtractionType::JsonPath, "$.user_id".to_string()),
            ExtractionRule::new("content_type".to_string(), ExtractionType::Header, "Content-Type".to_string()),
            ExtractionRule::new("status".to_string(), ExtractionType::StatusCode, "".to_string()),
        ];
        
        assert!(engine.validate_rules(&valid_rules).is_ok());
        
        // Invalid rule - empty JsonPath pattern
        let invalid_rules = vec![
            ExtractionRule::new("user_id".to_string(), ExtractionType::JsonPath, "".to_string()),
        ];
        
        assert!(engine.validate_rules(&invalid_rules).is_err());
        
        // Invalid rule - invalid regex pattern
        let invalid_regex_rules = vec![
            ExtractionRule::new("email".to_string(), ExtractionType::Regex, "[".to_string()),
        ];
        
        assert!(engine.validate_rules(&invalid_regex_rules).is_err());
    }

    #[test]
    fn test_extraction_with_multiple_rules() {
        let engine = ValueExtractionEngine::new();
        let response = create_test_response();
        
        let rules = vec![
            ExtractionRule::new("user_id".to_string(), ExtractionType::JsonPath, "$.user_id".to_string()),
            ExtractionRule::new("content_type".to_string(), ExtractionType::Header, "Content-Type".to_string()),
            ExtractionRule::new("status_code".to_string(), ExtractionType::StatusCode, "".to_string()),
        ];
        
        let result = engine.extract_values(
            "test_route".to_string(),
            "test_env".to_string(),
            response,
            HashMap::new(),
            &rules,
        );
        
        assert!(result.is_ok());
        let extraction_result = result.unwrap();
        assert_eq!(extraction_result.extracted_values.len(), 3);
        assert!(extraction_result.errors.is_empty());
        
        // Check extracted values
        let value_map = extraction_result.to_key_value_map();
        assert_eq!(value_map.get("user_id"), Some(&"42".to_string()));
        assert_eq!(value_map.get("content_type"), Some(&"application/json".to_string()));
        assert_eq!(value_map.get("status_code"), Some(&"200".to_string()));
    }

    #[test]
    fn test_extraction_with_required_rule_failure() {
        let engine = ValueExtractionEngine::new();
        let response = create_test_response();
        
        let rules = vec![
            ExtractionRule::new("nonexistent".to_string(), ExtractionType::JsonPath, "$.nonexistent".to_string()).required(),
        ];
        
        let result = engine.extract_values(
            "test_route".to_string(),
            "test_env".to_string(),
            response,
            HashMap::new(),
            &rules,
        );
        
        assert!(result.is_ok());
        let extraction_result = result.unwrap();
        assert!(extraction_result.extracted_values.is_empty());
        assert_eq!(extraction_result.errors.len(), 1);
        assert!(extraction_result.has_errors());
    }

    #[test]
    fn test_extraction_with_default_value() {
        let engine = ValueExtractionEngine::new();
        let response = create_test_response();
        
        let rules = vec![
            ExtractionRule::new("nonexistent".to_string(), ExtractionType::JsonPath, "$.nonexistent".to_string())
                .with_default_value("default_value".to_string()),
        ];
        
        let result = engine.extract_values(
            "test_route".to_string(),
            "test_env".to_string(),
            response,
            HashMap::new(),
            &rules,
        );
        
        assert!(result.is_ok());
        let extraction_result = result.unwrap();
        assert_eq!(extraction_result.extracted_values.len(), 1);
        assert!(extraction_result.errors.is_empty());
        
        let value_map = extraction_result.to_key_value_map();
        assert_eq!(value_map.get("nonexistent"), Some(&"default_value".to_string()));
    }

    #[test]
    fn test_builder_pattern() {
        let engine = ValueExtractionEngineBuilder::new().build();
        
        assert!(engine.supports_extraction_type(&ExtractionType::JsonPath));
        assert!(engine.supports_extraction_type(&ExtractionType::Regex));
        assert!(engine.supports_extraction_type(&ExtractionType::Header));
        assert!(engine.supports_extraction_type(&ExtractionType::StatusCode));
    }
}