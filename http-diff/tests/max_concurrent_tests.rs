//! Tests for max_concurrent_requests configuration field

use http_diff::config::{
    builder::HttpDiffConfigBuilder,
    global_builder::GlobalConfigBuilder,
    types::{Environment, GlobalConfig, HttpDiffConfig, Route},
    validator::ConfigValidatorImpl,
};
use http_diff::traits::ConfigValidator;
use std::collections::HashMap;

#[cfg(test)]
mod max_concurrent_config_tests {
    use super::*;

    #[test]
    fn test_global_config_default_max_concurrent() {
        let config = GlobalConfig::default();
        assert_eq!(config.max_concurrent_requests, Some(10));
    }

    #[test]
    fn test_global_config_builder_max_concurrent() {
        let config = GlobalConfigBuilder::new()
            .max_concurrent_requests(5)
            .timeout(60)
            .build();

        assert_eq!(config.max_concurrent_requests, Some(5));
        assert_eq!(config.timeout_seconds, Some(60));
    }

    #[test]
    fn test_global_config_builder_with_existing_config() {
        let existing = GlobalConfig {
            timeout_seconds: Some(30),
            follow_redirects: Some(true),
            max_concurrent_requests: Some(20),
            headers: None,
            params: None,
        };

        let config = GlobalConfigBuilder::from_config(existing)
            .max_concurrent_requests(15)
            .build();

        assert_eq!(config.max_concurrent_requests, Some(15));
        assert_eq!(config.timeout_seconds, Some(30));
        assert_eq!(config.follow_redirects, Some(true));
    }

    #[test]
    fn test_main_builder_max_concurrent_convenience() {
        let config = HttpDiffConfigBuilder::new()
            .environment("test", "https://test.example.com", None)
            .get_route("health", "/health")
            .max_concurrent_requests(8)
            .build()
            .unwrap();

        let global = config.global.unwrap();
        assert_eq!(global.max_concurrent_requests, Some(8));
    }

    #[test]
    fn test_fluent_builder_chaining() {
        let config = HttpDiffConfigBuilder::new()
            .environment("test", "https://test.example.com", None)
            .get_route("health", "/health")
            .timeout(45)
            .max_concurrent_requests(12)
            .follow_redirects(false)
            .build()
            .unwrap();

        let global = config.global.unwrap();
        assert_eq!(global.timeout_seconds, Some(45));
        assert_eq!(global.max_concurrent_requests, Some(12));
        assert_eq!(global.follow_redirects, Some(false));
    }
}

#[cfg(test)]
mod max_concurrent_validation_tests {
    use super::*;
    use http_diff::error::HttpDiffError;

    fn create_test_config_with_max_concurrent(max_concurrent: Option<usize>) -> HttpDiffConfig {
        let mut environments = HashMap::new();
        environments.insert(
            "test".to_string(),
            Environment {
                base_url: "https://test.example.com".to_string(),
                headers: None,
                is_base: false,
            },
        );

        let global = GlobalConfig {
            timeout_seconds: Some(30),
            follow_redirects: Some(true),
            max_concurrent_requests: max_concurrent,
            headers: None,
            params: None,
        };

        let route = Route {
            name: "health".to_string(),
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
        };

        HttpDiffConfig {
            environments,
            global: Some(global),
            routes: vec![route],
        }
    }

    #[test]
    fn test_valid_max_concurrent_requests() {
        let validator = ConfigValidatorImpl::new();

        // Test valid values
        for valid_value in [1, 5, 10, 50, 100] {
            let config = create_test_config_with_max_concurrent(Some(valid_value));
            assert!(
                validator.validate(&config).is_ok(),
                "Expected {} to be valid",
                valid_value
            );
        }
    }

    #[test]
    fn test_invalid_max_concurrent_requests_zero() {
        let validator = ConfigValidatorImpl::new();
        let config = create_test_config_with_max_concurrent(Some(0));

        let result = validator.validate(&config);
        assert!(result.is_err());

        if let Err(HttpDiffError::InvalidConfig { message }) = result {
            assert!(message.contains("max_concurrent_requests must be between 1 and 100"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[test]
    fn test_invalid_max_concurrent_requests_too_high() {
        let validator = ConfigValidatorImpl::new();
        let config = create_test_config_with_max_concurrent(Some(101));

        let result = validator.validate(&config);
        assert!(result.is_err());

        if let Err(HttpDiffError::InvalidConfig { message }) = result {
            assert!(message.contains("max_concurrent_requests must be between 1 and 100"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[test]
    fn test_none_max_concurrent_requests_is_valid() {
        let validator = ConfigValidatorImpl::new();
        let config = create_test_config_with_max_concurrent(None);

        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_boundary_values() {
        let validator = ConfigValidatorImpl::new();

        // Test boundary values
        let config_min = create_test_config_with_max_concurrent(Some(1));
        assert!(validator.validate(&config_min).is_ok());

        let config_max = create_test_config_with_max_concurrent(Some(100));
        assert!(validator.validate(&config_max).is_ok());
    }
}

#[cfg(test)]
mod max_concurrent_runner_tests {
    use super::*;

    #[test]
    fn test_runner_config_validation() {
        // Test that we can build valid configs with max_concurrent_requests
        let config = HttpDiffConfigBuilder::new()
            .environment("test", "https://test.example.com", None)
            .get_route("health", "/health")
            .max_concurrent_requests(5)
            .build()
            .unwrap();

        // Verify the configuration was built correctly
        let global = config.global.unwrap();
        assert_eq!(global.max_concurrent_requests, Some(5));
    }

    #[test]
    fn test_config_defaults_when_no_global_config() {
        let config = HttpDiffConfigBuilder::new()
            .environment("test", "https://test.example.com", None)
            .get_route("health", "/health")
            .build()
            .unwrap();

        // Should work without error - will use defaults when needed
        assert!(
            config.global.is_none() || config.global.unwrap().max_concurrent_requests.is_some()
        );
    }

    #[test]
    fn test_config_with_explicit_none() {
        let global = GlobalConfig {
            timeout_seconds: Some(30),
            follow_redirects: Some(true),
            max_concurrent_requests: None, // Explicitly None
            headers: None,
            params: None,
        };

        let config = HttpDiffConfigBuilder::new()
            .environment("test", "https://test.example.com", None)
            .get_route("health", "/health")
            .global_config(global)
            .build()
            .unwrap();

        // Should build successfully
        let global = config.global.unwrap();
        assert_eq!(global.max_concurrent_requests, None);
        assert_eq!(global.timeout_seconds, Some(30));
    }
}

#[cfg(test)]
mod max_concurrent_edge_cases {
    use super::*;

    #[test]
    fn test_very_large_value() {
        let config = HttpDiffConfigBuilder::new()
            .environment("test", "https://test.example.com", None)
            .get_route("health", "/health")
            .max_concurrent_requests(1000) // Will fail validation
            .build();

        // Should fail validation
        assert!(config.is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let config = GlobalConfig {
            timeout_seconds: Some(30),
            follow_redirects: Some(true),
            max_concurrent_requests: Some(15),
            headers: None,
            params: None,
        };

        // Test serialization and deserialization
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: GlobalConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            config.max_concurrent_requests,
            deserialized.max_concurrent_requests
        );
        assert_eq!(config.timeout_seconds, deserialized.timeout_seconds);
        assert_eq!(config.follow_redirects, deserialized.follow_redirects);
    }
}
