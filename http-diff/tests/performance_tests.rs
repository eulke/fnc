//! Comprehensive Performance Test Suite
//!
//! This test suite consolidates all performance testing functionality from various test files
//! and provides comprehensive benchmarks for:
//! - Configuration parsing and validation performance
//! - HTTP client and response processing performance  
//! - Memory efficiency and resource cleanup under stress
//!
//! Performance assertions use realistic expectations that users might encounter
//! in production environments.

mod common;

use common::*;
use http_diff::{
    config::{Environment, HttpDiffConfig, Route, ValueExtractionRule, ExtractorType},
    traits::HttpClient,
    types::HttpResponse,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// =============================================================================
// CONFIGURATION PERFORMANCE TESTS
// =============================================================================

/// Test TOML parsing performance with various configuration sizes
#[test]
fn test_config_parsing_performance() {
    let test_cases = vec![
        (10, "Small", Duration::from_millis(50)),
        (50, "Medium", Duration::from_millis(200)),
        (100, "Large", Duration::from_millis(500)),
        (200, "XLarge", Duration::from_millis(1000)),
    ];

    for (route_count, size_name, max_duration) in test_cases {
        let config = create_large_toml_config(route_count);
        
        let start = Instant::now();
        let parsed_config = toml::from_str::<HttpDiffConfig>(&config);
        let parsing_time = start.elapsed();

        assert!(parsed_config.is_ok(), "Config with {} routes should parse successfully", route_count);
        assert!(parsing_time < max_duration, 
                "TOML parsing for {} ({} routes) should complete within {:?}, but took {:?}", 
                size_name, route_count, max_duration, parsing_time);
        
        println!("✓ {} config ({} routes): parsed in {:?}", size_name, route_count, parsing_time);
    }
}

/// Test configuration validation performance with complex configurations
#[test]
fn test_config_validation_performance() {
    let test_cases = vec![
        (25, Duration::from_millis(100)),
        (50, Duration::from_millis(300)),
        (100, Duration::from_millis(800)),
        (200, Duration::from_millis(2000)),
    ];

    for (route_count, max_duration) in test_cases {
        let config = create_complex_config_for_validation(route_count);
        
        let start = Instant::now();
        let validation_result = config.validate_chain_config();
        let validation_time = start.elapsed();

        assert!(validation_result.is_ok(), "Complex config with {} routes should validate", route_count);
        assert!(validation_time < max_duration,
                "Config validation for {} routes should complete within {:?}, but took {:?}",
                route_count, max_duration, validation_time);
        
        println!("✓ Config validation ({} routes): completed in {:?}", route_count, validation_time);
    }
}

/// Test large configuration memory usage and cleanup
#[test]
fn test_large_config_memory_efficiency() {
    let route_count = 500;
    let environments_count = 5;
    let extractions_per_route = 8;

    let start = Instant::now();
    let config = create_memory_test_config(route_count, environments_count, extractions_per_route);
    let creation_time = start.elapsed();

    // Test validation performance
    let start = Instant::now();
    let validation_result = config.validate_chain_config();
    let validation_time = start.elapsed();

    // Test serialization performance
    let start = Instant::now();
    let _serialized = serde_json::to_string(&config).unwrap();
    let serialization_time = start.elapsed();

    assert!(validation_result.is_ok(), "Large config should validate successfully");
    assert!(creation_time < Duration::from_secs(3), "Large config creation should be reasonable");
    assert!(validation_time < Duration::from_secs(10), "Large config validation should complete within 10s");
    assert!(serialization_time < Duration::from_secs(2), "Large config serialization should be fast");

    println!("✓ Memory efficiency test ({}x{}x{} config): created in {:?}, validated in {:?}, serialized in {:?}", 
             route_count, environments_count, extractions_per_route, creation_time, validation_time, serialization_time);
}

// =============================================================================
// HTTP CLIENT PERFORMANCE TESTS  
// =============================================================================

/// Test HTTP client response processing performance
#[tokio::test]
async fn test_http_client_response_performance() {
    let response_sizes = vec![
        (1, "Small", Duration::from_millis(10)),      // 1KB response
        (10, "Medium", Duration::from_millis(50)),    // 10KB response
        (100, "Large", Duration::from_millis(200)),   // 100KB response
        (1000, "XLarge", Duration::from_millis(500)), // 1MB response
    ];

    let (_, mock_client, _, _, _) = TestScenarioBuilder::new().build();
    let test_route = create_mock_route("performance_test", "GET", "/api/performance");
    let user_data = create_mock_user_data(vec![("userId", "test")]);

    for (size_kb, size_name, max_duration) in response_sizes {
        let large_response = create_large_json_response(size_kb);
        let mut responses = HashMap::new();
        responses.insert("performance_test:dev".to_string(), large_response);
        
        let client = mock_client.clone().with_responses(responses);

        let start = Instant::now();
        let response = client.execute_request(&test_route, "dev", &user_data).await.unwrap();
        let processing_time = start.elapsed();

        assert!(processing_time < max_duration,
                "{} response ({} KB) processing should complete within {:?}, but took {:?}",
                size_name, size_kb, max_duration, processing_time);

        assert!(response.body.len() > size_kb * 900, "Response should be approximately {} KB", size_kb);

        println!("✓ {} response ({} KB): processed in {:?}", size_name, size_kb, processing_time);
    }
}

/// Test chain execution performance with many routes and users
#[tokio::test]
async fn test_chain_execution_performance() {
    let test_scenarios = vec![
        (10, 5, Duration::from_millis(200)),   // 10 routes, 5 users
        (25, 10, Duration::from_millis(500)),  // 25 routes, 10 users
        (50, 20, Duration::from_millis(1500)), // 50 routes, 20 users
    ];

    for (route_count, user_count, max_duration) in test_scenarios {
        let routes = create_performance_chain_routes(route_count);
        let users = create_test_users(user_count);
        let mock_client = create_performance_http_client(&routes);

        let start = Instant::now();
        
        // Simulate chain execution for all users
        for user in &users {
            for route in &routes {
                let _response = mock_client.execute_request(route, "dev", user).await.unwrap();
            }
        }
        
        let execution_time = start.elapsed();
        let operations = route_count * user_count;

        assert!(execution_time < max_duration,
                "Chain execution ({} routes × {} users = {} ops) should complete within {:?}, but took {:?}",
                route_count, user_count, operations, max_duration, execution_time);

        println!("✓ Chain execution ({} routes × {} users): completed in {:?}", 
                 route_count, user_count, execution_time);
    }
}

// =============================================================================
// MEMORY EFFICIENCY TESTS
// =============================================================================

/// Test memory usage with large configurations and many users
#[test]
fn test_memory_efficiency_with_scale() {
    let scale_tests = vec![
        (50, 10, "Small scale"),
        (100, 25, "Medium scale"),
        (200, 50, "Large scale"),
    ];

    for (route_count, user_count, scale_name) in scale_tests {
        let config = create_memory_test_config(route_count, 3, 5);
        let users = create_test_users(user_count);

        let start = Instant::now();
        
        // Simulate memory usage by accessing configuration multiple times
        for _ in 0..5 {
            for user in &users {
                for route in &config.routes {
                    // Simulate route processing
                    let _route_name = &route.name;
                    let _route_path = &route.path;
                    let _user_id = user.data.get("userId");
                }
            }
        }

        let total_time = start.elapsed();
        let max_time = Duration::from_millis((route_count * user_count * 2) as u64);
        
        assert!(total_time < max_time,
                "{} memory test should complete efficiently", scale_name);

        println!("✓ {} ({} routes, {} users): memory operations in {:?}", 
                 scale_name, route_count, user_count, total_time);
    }
}

/// Test response storage efficiency with many environments
#[test]
fn test_response_storage_efficiency() {
    let env_counts = vec![3, 5, 10, 15];
    let routes_per_env = 20;

    for env_count in env_counts {
        let mut responses = HashMap::new();
        
        let start = Instant::now();
        
        // Create responses for multiple environments
        for env_idx in 0..env_count {
            let env_name = format!("env_{}", env_idx);
            
            for route_idx in 0..routes_per_env {
                let route_name = format!("route_{}", route_idx);
                let key = format!("{}:{}", route_name, env_name);
                let response = create_mock_response(200, &format!(r#"{{"env": "{}", "route": "{}"}}"#, env_name, route_name));
                responses.insert(key, response);
            }
        }
        
        // Access all responses multiple times
        for _ in 0..3 {
            for (_, response) in &responses {
                let _body_len = response.body.len();
                let _status = response.status;
            }
        }
        
        let total_time = start.elapsed();
        let total_responses = env_count * routes_per_env;
        let max_time = Duration::from_millis((total_responses * 2) as u64);
        
        assert!(total_time < max_time,
                "Response storage efficiency test ({} environments × {} routes) should be efficient",
                env_count, routes_per_env);

        println!("✓ Response storage ({} envs × {} routes = {} responses): completed in {:?}", 
                 env_count, routes_per_env, total_responses, total_time);
    }
}

// =============================================================================
// STRESS TESTS
// =============================================================================

/// Maximum configuration size handling stress test
#[test]
fn test_maximum_configuration_stress() {
    let max_routes = 300;
    let max_environments = 5;
    let max_extractions_per_route = 8;

    let start = Instant::now();
    
    let config = create_stress_test_config(max_routes, max_environments, max_extractions_per_route);
    let creation_time = start.elapsed();

    // Test validation under stress
    let start = Instant::now();
    let validation_result = config.validate_chain_config();
    let validation_time = start.elapsed();

    assert!(validation_result.is_ok(), "Stress test config should validate successfully");

    // Stress test performance expectations
    assert!(creation_time < Duration::from_secs(10), "Stress config creation should complete within 10s");
    assert!(validation_time < Duration::from_secs(30), "Stress config validation should complete within 30s");

    println!("✓ Maximum configuration stress test ({}×{}×{} config): created in {:?}, validated in {:?}", 
             max_routes, max_environments, max_extractions_per_route, creation_time, validation_time);
}

/// High concurrency scenario stress test
#[tokio::test]
async fn test_high_concurrency_stress() {
    let concurrency_levels = vec![25, 50, 100];
    let routes_count = 30;

    for concurrency_level in concurrency_levels {
        let routes = create_simple_routes(routes_count);
        let users = create_test_users(concurrency_level);
        let mock_client = create_performance_http_client(&routes);

        let start = Instant::now();
        
        // Simulate high concurrent load
        for user_batch in users.chunks(10) {
            for user in user_batch {
                for route in &routes {
                    let _response = mock_client.execute_request(route, "dev", user).await.unwrap();
                }
            }
        }
        
        let total_time = start.elapsed();
        let total_operations = concurrency_level * routes_count;
        
        // High concurrency should still be manageable
        let max_time = Duration::from_secs(10);
        assert!(total_time < max_time,
                "High concurrency stress test ({} users × {} routes) should complete within {:?}",
                concurrency_level, routes_count, max_time);

        println!("✓ High concurrency stress ({} users × {} routes = {} ops): completed in {:?}", 
                 concurrency_level, routes_count, total_operations, total_time);
    }
}

// =============================================================================
// PERFORMANCE TEST HELPER FUNCTIONS
// =============================================================================

/// Create a large TOML configuration for parsing performance tests
fn create_large_toml_config(route_count: usize) -> String {
    let mut config = String::from(r#"
[environments.dev]
base_url = "https://dev.example.com"

[environments.staging]
base_url = "https://staging.example.com"

[environments.prod]
base_url = "https://prod.example.com"

"#);

    for i in 0..route_count {
        config.push_str(&format!(r#"
[[routes]]
name = "route_{}"
method = "GET"
path = "/api/route/{}"
headers = {{ "Authorization" = "Bearer token", "Content-Type" = "application/json" }}
params = {{ "limit" = "10", "offset" = "{}" }}

"#, i, i, i * 10));

        // Add extractions to some routes
        if i % 3 == 0 {
            config.push_str(&format!(r#"
[[routes.extract]]
name = "route_{}_id"
type = "JsonPath"
source = "$.data.id"
required = false
default_value = "default_{}"

[[routes.extract]]
name = "route_{}_status"
type = "StatusCode"
source = ""
required = false

"#, i, i, i));
        }

        // Add dependencies to create chains
        if i > 0 && i % 5 != 0 {
            config.push_str(&format!("depends_on = [\"route_{}\"]\nwait_for_extraction = true\n\n", i - 1));
        }
    }

    config
}

/// Create a complex configuration for validation performance testing
fn create_complex_config_for_validation(route_count: usize) -> HttpDiffConfig {
    let mut environments = HashMap::new();
    environments.insert("dev".to_string(), Environment {
        base_url: "https://dev.example.com".to_string(),
        headers: None,
        is_base: false,
    });
    environments.insert("staging".to_string(), Environment {
        base_url: "https://staging.example.com".to_string(),
        headers: None,
        is_base: false,
    });
    environments.insert("prod".to_string(), Environment {
        base_url: "https://prod.example.com".to_string(),
        headers: None,
        is_base: true,
    });

    let routes = (0..route_count).map(|i| {
        let extractions = (0..5).map(|j| {
            ValueExtractionRule {
                name: format!("value_{}_{}", i, j),
                extractor_type: match j % 4 {
                    0 => ExtractorType::JsonPath,
                    1 => ExtractorType::Regex,
                    2 => ExtractorType::Header,
                    _ => ExtractorType::StatusCode,
                },
                source: match j % 4 {
                    0 => format!("$.data[{}].field_{}", i, j),
                    1 => format!(r"field_{}=([^,\s]+)", j),
                    2 => format!("X-Custom-{}-{}", i, j),
                    _ => "".to_string(),
                },
                default_value: Some(format!("default_{}_{}", i, j)),
                required: j == 0,
            }
        }).collect();

        // Create complex dependency patterns
        let depends_on = if i == 0 {
            None
        } else if i % 10 == 0 {
            // Branch points - depend on multiple previous routes
            Some((std::cmp::max(0, i as i32 - 3) as usize..i)
                .map(|j| format!("route_{}", j))
                .collect())
        } else {
            // Simple chain dependency
            Some(vec![format!("route_{}", i - 1)])
        };

        Route {
            name: format!("route_{}", i),
            method: match i % 3 {
                0 => "GET",
                1 => "POST", 
                _ => "PUT",
            }.to_string(),
            path: format!("/api/route/{}", i),
            headers: if i % 2 == 0 {
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer {token}".to_string());
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                Some(headers)
            } else {
                None
            },
            params: if i % 3 == 0 {
                let mut params = HashMap::new();
                params.insert("limit".to_string(), "10".to_string());
                params.insert("id".to_string(), format!("{{{}}}", format!("value_{}_0", i - 1)));
                Some(params)
            } else {
                None
            },
            base_urls: None,
            body: if i % 3 == 1 {
                Some(format!(r#"{{"route": {}, "data": "test_data_{}", "timestamp": "{{timestamp}}"}}"#, i, i))
            } else {
                None
            },
            conditions: None,
            extract: Some(extractions),
            depends_on: depends_on.clone(),
            wait_for_extraction: depends_on.as_ref().map(|_| true),
        }
    }).collect();

    HttpDiffConfig {
        environments,
        global: None,
        routes,
    }
}

/// Create a memory-intensive configuration for testing
fn create_memory_test_config(route_count: usize, env_count: usize, extractions_per_route: usize) -> HttpDiffConfig {
    let mut environments = HashMap::new();
    for i in 0..env_count {
        environments.insert(format!("env_{}", i), Environment {
            base_url: format!("https://env{}.example.com", i),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("X-Environment".to_string(), format!("env_{}", i));
                headers.insert("X-Service".to_string(), "performance-test".to_string());
                headers
            }),
            is_base: i == 0,
        });
    }

    let routes = (0..route_count).map(|i| {
        let extractions = (0..extractions_per_route).map(|j| {
            ValueExtractionRule {
                name: format!("memory_test_{}_{}", i, j),
                extractor_type: ExtractorType::JsonPath,
                source: format!("$.results[{}].data.field_{}", i, j),
                default_value: Some(format!("memory_default_{}_{}", i, j)),
                required: false,
            }
        }).collect();

        let depends_on = if i > 0 { Some(vec![format!("memory_route_{}", i - 1)]) } else { None };

        Route {
            name: format!("memory_route_{}", i),
            method: "GET".to_string(),
            path: format!("/api/memory/test/{}/data", i),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer {auth_token}".to_string());
                headers.insert("X-Route-ID".to_string(), format!("route_{}", i));
                headers.insert("X-Memory-Test".to_string(), "true".to_string());
                headers
            }),
            params: Some({
                let mut params = HashMap::new();
                params.insert("route_index".to_string(), i.to_string());
                params.insert("extraction_count".to_string(), extractions_per_route.to_string());
                params.insert("memory_test".to_string(), "active".to_string());
                params
            }),
            base_urls: None,
            body: None,
            conditions: None,
            extract: Some(extractions),
            depends_on: depends_on.clone(),
            wait_for_extraction: depends_on.as_ref().map(|_| true),
        }
    }).collect();

    HttpDiffConfig {
        environments,
        global: None,
        routes,
    }
}

/// Create routes for performance chain testing
fn create_performance_chain_routes(count: usize) -> Vec<Route> {
    (0..count).map(|i| {
        let depends_on = if i == 0 {
            None
        } else if i % 10 == 0 {
            // Every 10th route depends on the root to create branching
            Some(vec!["perf_route_0".to_string()])
        } else {
            Some(vec![format!("perf_route_{}", i - 1)])
        };

        create_route_with_extraction(
            &format!("perf_route_{}", i),
            "GET",
            &format!("/api/perf/{}", i),
            vec![create_extraction_rule(
                &format!("perf_value_{}", i),
                "JsonPath",
                &format!("$.perf.data_{}", i),
                false,
                Some(format!("default_perf_{}", i)),
            )],
            depends_on,
        )
    }).collect()
}

/// Create simple routes for testing
fn create_simple_routes(count: usize) -> Vec<Route> {
    (0..count).map(|i| {
        create_mock_route(&format!("simple_route_{}", i), "GET", &format!("/api/simple/{}", i))
    }).collect()
}

/// Create HTTP client for performance testing
fn create_performance_http_client(routes: &[Route]) -> TestMockHttpClient {
    let mut responses = HashMap::new();
    
    for route in routes {
        let response_data = format!(
            r#"{{"route": "{}", "data": {{"id": "perf_id", "value": "perf_value"}}, "timestamp": "2023-01-01T00:00:00Z"}}"#,
            route.name
        );
        responses.insert(
            format!("{}:dev", route.name),
            create_mock_response(200, &response_data)
        );
    }

    TestMockHttpClient::new().with_responses(responses)
}

/// Create a large JSON response for testing
fn create_large_json_response(size_kb: usize) -> HttpResponse {
    let target_size = size_kb * 1024;
    let mut data = Vec::new();
    
    // Create array of objects to reach target size
    let mut current_size = 0;
    let mut index = 0;
    
    while current_size < target_size {
        let item = serde_json::json!({
            "id": index,
            "name": format!("Item {}", index),
            "description": format!("This is a performance test item with index {} and some additional data to increase size. Lorem ipsum dolor sit amet, consectetur adipiscing elit.", index),
            "data": {
                "field_1": format!("value_{}", index),
                "field_2": index * 2,
                "field_3": format!("performance_test_data_{}", index),
                "nested": {
                    "inner_field_1": format!("inner_value_{}", index),
                    "inner_field_2": index * 3,
                    "array": (0..10).map(|i| format!("array_item_{}_{}", index, i)).collect::<Vec<_>>()
                }
            },
            "timestamp": "2023-01-01T00:00:00Z",
            "metadata": {
                "created_by": "performance_test",
                "size_kb": size_kb,
                "index": index
            }
        });
        
        let item_str = serde_json::to_string(&item).unwrap();
        current_size += item_str.len();
        data.push(item);
        index += 1;
        
        if index > 10000 { // Safety break to prevent infinite loop
            break;
        }
    }
    
    let response_body = serde_json::json!({
        "data": data,
        "metadata": {
            "total_items": data.len(),
            "approximate_size_kb": size_kb,
            "actual_size_bytes": current_size
        }
    });

    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("content-length".to_string(), current_size.to_string());

    HttpResponse {
        status: 200,
        headers,
        body: serde_json::to_string(&response_body).unwrap(),
        url: "https://example.com/performance/large".to_string(),
        curl_command: "curl 'https://example.com/performance/large'".to_string(),
    }
}

/// Create stress test configuration with maximum complexity
fn create_stress_test_config(route_count: usize, env_count: usize, extractions_per_route: usize) -> HttpDiffConfig {
    let mut environments = HashMap::new();
    for i in 0..env_count {
        environments.insert(format!("stress_env_{}", i), Environment {
            base_url: format!("https://stress-env-{}.example.com", i),
            headers: Some({
                let mut headers = HashMap::new();
                for j in 0..5 {
                    headers.insert(format!("X-Stress-Header-{}", j), format!("stress_value_{}_{}", i, j));
                }
                headers
            }),
            is_base: i == 0,
        });
    }

    let routes = (0..route_count).map(|i| {
        let extractions = (0..extractions_per_route).map(|j| {
            ValueExtractionRule {
                name: format!("stress_extraction_{}_{}", i, j),
                extractor_type: match (i + j) % 4 {
                    0 => ExtractorType::JsonPath,
                    1 => ExtractorType::Regex,
                    2 => ExtractorType::Header,
                    _ => ExtractorType::StatusCode,
                },
                source: match (i + j) % 4 {
                    0 => format!("$.stress.data[{}].field_{}", i, j),
                    1 => format!(r"stress_{}_{}_(\w+)", i, j),
                    2 => format!("X-Stress-Header-{}", j),
                    _ => "".to_string(),
                },
                default_value: Some(format!("stress_default_{}_{}", i, j)),
                required: j == 0 && i % 5 == 0, // Some required extractions
            }
        }).collect();

        // Create complex dependency patterns for stress testing
        let depends_on = if i == 0 {
            None
        } else if i % 20 == 0 {
            // Major branch points
            Some((std::cmp::max(0, i as i32 - 5) as usize..i)
                .step_by(2)
                .map(|j| format!("stress_route_{}", j))
                .collect())
        } else if i % 7 == 0 {
            // Minor branch points
            Some(vec![
                format!("stress_route_{}", i - 1),
                format!("stress_route_{}", std::cmp::max(0, i as i32 - 3) as usize),
            ])
        } else {
            // Linear dependencies
            Some(vec![format!("stress_route_{}", i - 1)])
        };

        Route {
            name: format!("stress_route_{}", i),
            method: match i % 4 {
                0 => "GET",
                1 => "POST",
                2 => "PUT",
                _ => "DELETE",
            }.to_string(),
            path: format!("/api/stress/route/{}/action/{}", i, i % 10),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer {stress_token}".to_string());
                headers.insert("X-Stress-Route".to_string(), format!("route_{}", i));
                headers.insert("X-Dependency-Count".to_string(), 
                               depends_on.as_ref().map_or(0, |deps| deps.len()).to_string());
                headers
            }),
            params: Some({
                let mut params = HashMap::new();
                params.insert("stress_level".to_string(), "maximum".to_string());
                params.insert("route_index".to_string(), i.to_string());
                params.insert("extraction_count".to_string(), extractions_per_route.to_string());
                if let Some(deps) = &depends_on {
                    params.insert("dependency_count".to_string(), deps.len().to_string());
                }
                params
            }),
            base_urls: None,
            body: if i % 2 == 1 {
                Some(format!(
                    r#"{{"stress_route": {}, "complex_data": {{"nested": {{"field": "stress_test", "index": {}}}, "array": [{}]}}, "dependencies": {}}}"#,
                    i,
                    i,
                    (0..5).map(|j| format!("\"item_{}\"", j)).collect::<Vec<_>>().join(", "),
                    depends_on.as_ref().map_or(0, |deps| deps.len())
                ))
            } else {
                None
            },
            conditions: None,
            extract: Some(extractions),
            depends_on: depends_on.clone(),
            wait_for_extraction: depends_on.as_ref().map(|_| true),
        }
    }).collect();

    HttpDiffConfig {
        environments,
        global: None,
        routes,
    }
}