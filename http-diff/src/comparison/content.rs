/// Content normalization and type detection for HTTP response comparison
use std::collections::HashMap;

/// Content normalizer with type-aware processing
pub struct ContentNormalizer {
    ignore_whitespace: bool,
}

impl ContentNormalizer {
    /// Create a new content normalizer
    pub fn new(ignore_whitespace: bool) -> Self {
        Self { ignore_whitespace }
    }

    /// Normalize content based on detected or specified content type
    pub fn normalize(&self, text: &str, content_type: Option<&str>) -> String {
        if !self.ignore_whitespace {
            return text.to_string();
        }

        let detected_content_type = content_type.unwrap_or_else(|| self.detect_content_type(text));

        match detected_content_type {
            "application/json" | "json" => self.normalize_json(text),
            "application/xml" | "text/xml" | "xml" => self.normalize_xml(text),
            "text/html" | "html" => self.normalize_html(text),
            "text/plain" | "text" => self.normalize_plain_text(text),
            _ => self.normalize_plain_text(text),
        }
    }

    /// Detect content type from text content
    pub fn detect_content_type(&self, text: &str) -> &str {
        let trimmed = text.trim();

        if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return "application/json";
        }

        if trimmed.starts_with('<') && trimmed.contains('>') {
            return "application/xml";
        }

        "text/plain"
    }

    /// Normalize JSON content for semantic comparison
    fn normalize_json(&self, text: &str) -> String {
        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(json_value) => {
                // Pretty print with consistent formatting
                serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| text.to_string())
            }
            Err(_) => {
                // Fallback to plain text normalization if JSON parsing fails
                self.normalize_plain_text(text)
            }
        }
    }

    /// Normalize XML content (basic implementation)
    fn normalize_xml(&self, text: &str) -> String {
        // Basic XML normalization - remove extra whitespace between tags
        // Note: For production use, consider using a proper XML parser
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Normalize HTML content (basic implementation)
    fn normalize_html(&self, text: &str) -> String {
        // Basic HTML normalization - similar to XML but preserve some structure
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Normalize plain text content
    fn normalize_plain_text(&self, text: &str) -> String {
        text.lines()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Header normalizer for case-insensitive comparison
pub struct HeaderNormalizer {
    ignore_headers: Vec<String>,
}

impl HeaderNormalizer {
    /// Create a new header normalizer with headers to ignore
    pub fn new(ignore_headers: Vec<String>) -> Self {
        Self { ignore_headers }
    }

    /// Normalize headers to lowercase for comparison while preserving original case for display
    pub fn normalize(
        &self,
        headers: &HashMap<String, String>,
    ) -> HashMap<String, (String, String)> {
        headers
            .iter()
            .filter(|(key, _)| !self.ignore_headers.contains(&key.to_lowercase()))
            .map(|(k, v)| (k.to_lowercase(), (k.clone(), v.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_detection() {
        let normalizer = ContentNormalizer::new(true);

        // Test JSON detection
        assert_eq!(
            normalizer.detect_content_type(r#"{"key": "value"}"#),
            "application/json"
        );
        assert_eq!(
            normalizer.detect_content_type(r#"[1, 2, 3]"#),
            "application/json"
        );

        // Test XML detection
        assert_eq!(
            normalizer.detect_content_type("<root><item>value</item></root>"),
            "application/xml"
        );

        // Test plain text fallback
        assert_eq!(
            normalizer.detect_content_type("This is plain text"),
            "text/plain"
        );
        assert_eq!(
            normalizer.detect_content_type("{invalid json"),
            "text/plain"
        );
    }

    #[test]
    fn test_json_normalization() {
        let normalizer = ContentNormalizer::new(true);

        // Same JSON data but with different formatting
        let json1 = r#"{"name":"John","age":30,"city":"NYC"}"#;
        let json2 = r#"{
  "age": 30,
  "city": "NYC",
  "name": "John"
}"#;

        let normalized1 = normalizer.normalize(json1, Some("application/json"));
        let normalized2 = normalizer.normalize(json2, Some("application/json"));

        // Should be identical after normalization
        assert_eq!(normalized1, normalized2);
    }

    #[test]
    fn test_plain_text_normalization() {
        let normalizer = ContentNormalizer::new(true);

        let text1 = "  line1  \n  line2  \n  line3  ";
        let text2 = "line1\nline2\nline3";

        let normalized1 = normalizer.normalize(text1, Some("text/plain"));
        let normalized2 = normalizer.normalize(text2, Some("text/plain"));

        assert_eq!(normalized1, normalized2);
    }

    #[test]
    fn test_whitespace_preservation() {
        let normalizer = ContentNormalizer::new(false);

        let text1 = "  line1  \n  line2  ";
        let text2 = "line1\nline2";

        let normalized1 = normalizer.normalize(text1, Some("text/plain"));
        let normalized2 = normalizer.normalize(text2, Some("text/plain"));

        // Should preserve whitespace when ignore_whitespace is false
        assert_ne!(normalized1, normalized2);
        assert_eq!(normalized1, text1);
        assert_eq!(normalized2, text2);
    }

    #[test]
    fn test_header_normalization() {
        let ignore_headers = vec!["date".to_string(), "x-request-id".to_string()];
        let normalizer = HeaderNormalizer::new(ignore_headers);

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert(
            "Date".to_string(),
            "Mon, 01 Jan 2024 00:00:00 GMT".to_string(),
        );
        headers.insert("X-Version".to_string(), "1.0".to_string());
        headers.insert("x-request-id".to_string(), "req-123".to_string());

        let normalized = normalizer.normalize(&headers);

        // Should contain normalized keys but preserve original keys and values
        assert!(normalized.contains_key("content-type"));
        assert!(normalized.contains_key("x-version"));

        // Should filter out ignored headers
        assert!(!normalized.contains_key("date"));
        assert!(!normalized.contains_key("x-request-id"));

        // Should preserve original case in values
        let (original_key, original_value) = &normalized["content-type"];
        assert_eq!(original_key, "Content-Type");
        assert_eq!(original_value, "application/json");
    }
}
