//! Individual extractor implementations for different value extraction types

use crate::error::{HttpDiffError, Result};
use crate::traits::ValueExtractor;
use crate::types::{
    ExtractionRule, ExtractionResult, ExtractionType, ExtractedValue, ExtractionError,
    ValueExtractionContext,
};
use jsonpath_rust::JsonPathFinder;
use regex::Regex;
use serde_json::Value as JsonValue;

/// JsonPath extractor for extracting values from JSON response bodies
#[derive(Debug, Clone)]
pub struct JsonPathExtractor;

impl JsonPathExtractor {
    /// Create a new JsonPath extractor
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonPathExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueExtractor for JsonPathExtractor {
    fn extract_values(
        &self,
        context: &ValueExtractionContext,
        rules: &[ExtractionRule],
    ) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(context.clone());

        for rule in rules {
            if !self.supports_rule(rule) {
                continue;
            }

            match self.extract_single_value(context, rule) {
                Ok(Some(value)) => {
                    let extracted_value = ExtractedValue::new(
                        rule.key.clone(),
                        value,
                        rule.pattern.clone(),
                        rule.extraction_type.clone(),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_value(extracted_value);
                }
                Ok(None) => {
                    // Value not found - handled by the calling code
                }
                Err(err) => {
                    let error = ExtractionError::new(
                        rule.clone(),
                        format!("JsonPath extraction failed: {}", err),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_error(error);
                }
            }
        }

        Ok(result)
    }

    fn extract_single_value(
        &self,
        context: &ValueExtractionContext,
        rule: &ExtractionRule,
    ) -> Result<Option<String>> {
        if !self.supports_rule(rule) {
            return Err(HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                "JsonPath extractor does not support this rule type".to_string(),
            ));
        }

        // Parse the response body as JSON
        let _json_value: JsonValue = serde_json::from_str(&context.response.body)
            .map_err(|err| HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                format!("Failed to parse response body as JSON: {}", err),
            ))?;

        // Create JsonPath finder
        let finder = JsonPathFinder::from_str(&context.response.body, &rule.pattern)
            .map_err(|err| HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                format!("Invalid JsonPath expression '{}': {}", rule.pattern, err),
            ))?;

        // Find the value using JsonPath
        let found_value = finder.find();
        
        match found_value {
            JsonValue::Null => Ok(None),
            JsonValue::Array(arr) if arr.is_empty() => Ok(None),
            JsonValue::Array(arr) => {
                // Return the first element if array has values
                if let Some(first) = arr.first() {
                    Ok(Some(json_value_to_string(first)))
                } else {
                    Ok(None)
                }
            }
            value => Ok(Some(json_value_to_string(&value))),
        }
    }

    fn supports_rule(&self, rule: &ExtractionRule) -> bool {
        rule.extraction_type == ExtractionType::JsonPath
    }
}

/// Regex extractor for extracting values using regular expressions
#[derive(Debug, Clone)]
pub struct RegexExtractor;

impl RegexExtractor {
    /// Create a new Regex extractor
    pub fn new() -> Self {
        Self
    }
}

impl Default for RegexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueExtractor for RegexExtractor {
    fn extract_values(
        &self,
        context: &ValueExtractionContext,
        rules: &[ExtractionRule],
    ) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(context.clone());

        for rule in rules {
            if !self.supports_rule(rule) {
                continue;
            }

            match self.extract_single_value(context, rule) {
                Ok(Some(value)) => {
                    let extracted_value = ExtractedValue::new(
                        rule.key.clone(),
                        value,
                        rule.pattern.clone(),
                        rule.extraction_type.clone(),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_value(extracted_value);
                }
                Ok(None) => {
                    // Value not found - handled by the calling code
                }
                Err(err) => {
                    let error = ExtractionError::new(
                        rule.clone(),
                        format!("Regex extraction failed: {}", err),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_error(error);
                }
            }
        }

        Ok(result)
    }

    fn extract_single_value(
        &self,
        context: &ValueExtractionContext,
        rule: &ExtractionRule,
    ) -> Result<Option<String>> {
        if !self.supports_rule(rule) {
            return Err(HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                "Regex extractor does not support this rule type".to_string(),
            ));
        }

        // Compile the regex pattern
        let regex = Regex::new(&rule.pattern)
            .map_err(|err| HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                format!("Invalid regex pattern '{}': {}", rule.pattern, err),
            ))?;

        // Search in the response body
        if let Some(captures) = regex.captures(&context.response.body) {
            // If there are named groups, try to extract them first
            if captures.len() > 1 {
                // Return the first capture group
                if let Some(group) = captures.get(1) {
                    return Ok(Some(group.as_str().to_string()));
                }
            }
            
            // Return the full match if no capture groups
            if let Some(full_match) = captures.get(0) {
                return Ok(Some(full_match.as_str().to_string()));
            }
        }

        Ok(None)
    }

    fn supports_rule(&self, rule: &ExtractionRule) -> bool {
        rule.extraction_type == ExtractionType::Regex
    }
}

/// Header extractor for extracting values from HTTP response headers
#[derive(Debug, Clone)]
pub struct HeaderExtractor;

impl HeaderExtractor {
    /// Create a new Header extractor
    pub fn new() -> Self {
        Self
    }
}

impl Default for HeaderExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueExtractor for HeaderExtractor {
    fn extract_values(
        &self,
        context: &ValueExtractionContext,
        rules: &[ExtractionRule],
    ) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(context.clone());

        for rule in rules {
            if !self.supports_rule(rule) {
                continue;
            }

            match self.extract_single_value(context, rule) {
                Ok(Some(value)) => {
                    let extracted_value = ExtractedValue::new(
                        rule.key.clone(),
                        value,
                        rule.pattern.clone(),
                        rule.extraction_type.clone(),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_value(extracted_value);
                }
                Ok(None) => {
                    // Value not found - handled by the calling code
                }
                Err(err) => {
                    let error = ExtractionError::new(
                        rule.clone(),
                        format!("Header extraction failed: {}", err),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_error(error);
                }
            }
        }

        Ok(result)
    }

    fn extract_single_value(
        &self,
        context: &ValueExtractionContext,
        rule: &ExtractionRule,
    ) -> Result<Option<String>> {
        if !self.supports_rule(rule) {
            return Err(HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                "Header extractor does not support this rule type".to_string(),
            ));
        }

        // Look for the header in a case-insensitive manner
        let header_name = rule.pattern.to_lowercase();
        
        for (key, value) in &context.response.headers {
            if key.to_lowercase() == header_name {
                return Ok(Some(value.clone()));
            }
        }

        Ok(None)
    }

    fn supports_rule(&self, rule: &ExtractionRule) -> bool {
        rule.extraction_type == ExtractionType::Header
    }
}

/// StatusCode extractor for extracting HTTP status code as a string
#[derive(Debug, Clone)]
pub struct StatusCodeExtractor;

impl StatusCodeExtractor {
    /// Create a new StatusCode extractor
    pub fn new() -> Self {
        Self
    }
}

impl Default for StatusCodeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueExtractor for StatusCodeExtractor {
    fn extract_values(
        &self,
        context: &ValueExtractionContext,
        rules: &[ExtractionRule],
    ) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(context.clone());

        for rule in rules {
            if !self.supports_rule(rule) {
                continue;
            }

            match self.extract_single_value(context, rule) {
                Ok(Some(value)) => {
                    let extracted_value = ExtractedValue::new(
                        rule.key.clone(),
                        value,
                        "status_code".to_string(), // Use a standard pattern name
                        rule.extraction_type.clone(),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_value(extracted_value);
                }
                Ok(None) => {
                    // Status code should always be available
                    let error = ExtractionError::new(
                        rule.clone(),
                        "Status code extraction returned None (unexpected)".to_string(),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_error(error);
                }
                Err(err) => {
                    let error = ExtractionError::new(
                        rule.clone(),
                        format!("Status code extraction failed: {}", err),
                        context.environment.clone(),
                        context.route_name.clone(),
                    );
                    result.add_error(error);
                }
            }
        }

        Ok(result)
    }

    fn extract_single_value(
        &self,
        context: &ValueExtractionContext,
        rule: &ExtractionRule,
    ) -> Result<Option<String>> {
        if !self.supports_rule(rule) {
            return Err(HttpDiffError::value_extraction_failed(
                context.route_name.clone(),
                rule.key.clone(),
                "StatusCode extractor does not support this rule type".to_string(),
            ));
        }

        // Extract the status code as a string
        Ok(Some(context.response.status.to_string()))
    }

    fn supports_rule(&self, rule: &ExtractionRule) -> bool {
        rule.extraction_type == ExtractionType::StatusCode
    }
}

/// Helper function to convert JSON values to strings
fn json_value_to_string(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
        JsonValue::Array(_) | JsonValue::Object(_) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HttpResponse, ExtractionRule, ExtractionType, ValueExtractionContext};
    use std::collections::HashMap;

    fn create_test_response() -> HttpResponse {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Request-ID".to_string(), "req-123456".to_string());
        headers.insert("Authorization".to_string(), "Bearer token-abc123".to_string());

        HttpResponse::new(
            201,
            headers,
            r#"{
                "user_id": 42,
                "name": "John Doe",
                "email": "john@example.com",
                "metadata": {
                    "created_at": "2023-01-01T00:00:00Z",
                    "tags": ["user", "premium"]
                },
                "score": 95.5
            }"#.to_string(),
            "https://api.example.com/users".to_string(),
            "curl -X POST https://api.example.com/users".to_string(),
        )
    }

    fn create_test_context() -> ValueExtractionContext {
        ValueExtractionContext::new(
            "test_route".to_string(),
            "test_env".to_string(),
            create_test_response(),
            HashMap::new(),
        )
    }

    #[test]
    fn test_jsonpath_extractor() {
        let extractor = JsonPathExtractor::new();
        let context = create_test_context();

        // Test simple field extraction
        let rule = ExtractionRule::new(
            "user_id".to_string(),
            ExtractionType::JsonPath,
            "$.user_id".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("42".to_string()));

        // Test nested field extraction
        let rule = ExtractionRule::new(
            "created_at".to_string(),
            ExtractionType::JsonPath,
            "$.metadata.created_at".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("2023-01-01T00:00:00Z".to_string()));

        // Test array element extraction
        let rule = ExtractionRule::new(
            "first_tag".to_string(),
            ExtractionType::JsonPath,
            "$.metadata.tags[0]".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("user".to_string()));

        // Test non-existent field
        let rule = ExtractionRule::new(
            "nonexistent".to_string(),
            ExtractionType::JsonPath,
            "$.nonexistent".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, None);

        // Test number field
        let rule = ExtractionRule::new(
            "score".to_string(),
            ExtractionType::JsonPath,
            "$.score".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("95.5".to_string()));
    }

    #[test]
    fn test_regex_extractor() {
        let extractor = RegexExtractor::new();
        let context = create_test_context();

        // Test simple regex extraction
        let rule = ExtractionRule::new(
            "email_domain".to_string(),
            ExtractionType::Regex,
            r#""email":\s*"[^@]+@([^"]+)""#.to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("example.com".to_string()));

        // Test full match without groups
        let rule = ExtractionRule::new(
            "user_id_field".to_string(),
            ExtractionType::Regex,
            r#""user_id":\s*\d+"#.to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some(r#""user_id": 42"#.to_string()));

        // Test non-matching regex
        let rule = ExtractionRule::new(
            "nonexistent".to_string(),
            ExtractionType::Regex,
            r#""nonexistent":\s*"[^"]+""#.to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, None);

        // Test invalid regex
        let rule = ExtractionRule::new(
            "invalid".to_string(),
            ExtractionType::Regex,
            "[".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule);
        assert!(result.is_err());
    }

    #[test]
    fn test_header_extractor() {
        let extractor = HeaderExtractor::new();
        let context = create_test_context();

        // Test existing header extraction
        let rule = ExtractionRule::new(
            "content_type".to_string(),
            ExtractionType::Header,
            "Content-Type".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("application/json".to_string()));

        // Test case-insensitive header extraction
        let rule = ExtractionRule::new(
            "request_id".to_string(),
            ExtractionType::Header,
            "x-request-id".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("req-123456".to_string()));

        // Test non-existent header
        let rule = ExtractionRule::new(
            "nonexistent".to_string(),
            ExtractionType::Header,
            "Non-Existent-Header".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_status_code_extractor() {
        let extractor = StatusCodeExtractor::new();
        let context = create_test_context();

        // Test status code extraction
        let rule = ExtractionRule::new(
            "status".to_string(),
            ExtractionType::StatusCode,
            "".to_string(),
        );
        
        let result = extractor.extract_single_value(&context, &rule).unwrap();
        assert_eq!(result, Some("201".to_string()));
    }

    #[test]
    fn test_extractor_supports_rule() {
        let jsonpath_extractor = JsonPathExtractor::new();
        let regex_extractor = RegexExtractor::new();
        let header_extractor = HeaderExtractor::new();
        let status_extractor = StatusCodeExtractor::new();

        let jsonpath_rule = ExtractionRule::new("test".to_string(), ExtractionType::JsonPath, "$.test".to_string());
        let regex_rule = ExtractionRule::new("test".to_string(), ExtractionType::Regex, "test".to_string());
        let header_rule = ExtractionRule::new("test".to_string(), ExtractionType::Header, "Test".to_string());
        let status_rule = ExtractionRule::new("test".to_string(), ExtractionType::StatusCode, "".to_string());

        assert!(jsonpath_extractor.supports_rule(&jsonpath_rule));
        assert!(!jsonpath_extractor.supports_rule(&regex_rule));
        assert!(!jsonpath_extractor.supports_rule(&header_rule));
        assert!(!jsonpath_extractor.supports_rule(&status_rule));

        assert!(!regex_extractor.supports_rule(&jsonpath_rule));
        assert!(regex_extractor.supports_rule(&regex_rule));
        assert!(!regex_extractor.supports_rule(&header_rule));
        assert!(!regex_extractor.supports_rule(&status_rule));

        assert!(!header_extractor.supports_rule(&jsonpath_rule));
        assert!(!header_extractor.supports_rule(&regex_rule));
        assert!(header_extractor.supports_rule(&header_rule));
        assert!(!header_extractor.supports_rule(&status_rule));

        assert!(!status_extractor.supports_rule(&jsonpath_rule));
        assert!(!status_extractor.supports_rule(&regex_rule));
        assert!(!status_extractor.supports_rule(&header_rule));
        assert!(status_extractor.supports_rule(&status_rule));
    }

    #[test]
    fn test_extract_values_method() {
        let jsonpath_extractor = JsonPathExtractor::new();
        let context = create_test_context();

        let rules = vec![
            ExtractionRule::new("user_id".to_string(), ExtractionType::JsonPath, "$.user_id".to_string()),
            ExtractionRule::new("name".to_string(), ExtractionType::JsonPath, "$.name".to_string()),
            ExtractionRule::new("nonexistent".to_string(), ExtractionType::JsonPath, "$.nonexistent".to_string()),
        ];

        let result = jsonpath_extractor.extract_values(&context, &rules).unwrap();
        
        assert_eq!(result.extracted_values.len(), 2);
        assert!(result.errors.is_empty());
        
        let value_map = result.to_key_value_map();
        assert_eq!(value_map.get("user_id"), Some(&"42".to_string()));
        assert_eq!(value_map.get("name"), Some(&"John Doe".to_string()));
        assert_eq!(value_map.get("nonexistent"), None);
    }

    #[test]
    fn test_json_value_to_string() {
        assert_eq!(json_value_to_string(&JsonValue::String("test".to_string())), "test");
        assert_eq!(json_value_to_string(&JsonValue::Number(serde_json::Number::from(42))), "42");
        assert_eq!(json_value_to_string(&JsonValue::Bool(true)), "true");
        assert_eq!(json_value_to_string(&JsonValue::Null), "null");
        
        let array = JsonValue::Array(vec![JsonValue::String("a".to_string()), JsonValue::String("b".to_string())]);
        assert_eq!(json_value_to_string(&array), r#"["a","b"]"#);
        
        let object = serde_json::json!({"key": "value"});
        assert_eq!(json_value_to_string(&object), r#"{"key":"value"}"#);
    }
}