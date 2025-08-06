//! Documentation generation for HTTP diff test results
//!
//! This module provides functionality to generate comprehensive markdown
//! documentation from HTTP comparison results, including statistics,
//! route analysis, and difference summaries.

use crate::error::Result;
use crate::types::ComparisonResult;
use std::collections::HashMap;

/// Generate comprehensive request documentation in markdown format
pub fn generate_request_documentation(results: &[ComparisonResult]) -> Result<String> {
    let mut doc = String::new();

    doc.push_str("# HTTP Diff Test Documentation\n");
    doc.push_str(&format!(
        "Generated: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Summary statistics
    let total_tests = results.len();
    let identical_count = results.iter().filter(|r| r.is_identical).count();
    let different_count = total_tests - identical_count;

    doc.push_str("## Test Summary\n");
    doc.push_str(&format!("- Total test scenarios: {}\n", total_tests));
    doc.push_str(&format!("- Identical responses: {}\n", identical_count));
    doc.push_str(&format!("- Different responses: {}\n", different_count));
    doc.push_str(&format!(
        "- Success rate: {:.1}%\n\n",
        (identical_count as f32 / total_tests as f32) * 100.0
    ));

    // Environment information
    if let Some(first_result) = results.first() {
        let environments: Vec<String> = first_result.responses.keys().cloned().collect();
        doc.push_str("## Environments Tested\n");
        for env in &environments {
            doc.push_str(&format!("- {}\n", env));
        }
        doc.push('\n');
    }

    // Route analysis
    let mut routes_analysis: HashMap<String, (usize, usize)> = HashMap::new();
    for result in results {
        let entry = routes_analysis
            .entry(result.route_name.clone())
            .or_insert((0, 0));
        if result.is_identical {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }

    doc.push_str("## Route Analysis\n");
    for (route_name, (identical, different)) in routes_analysis {
        let total = identical + different;
        let success_rate = (identical as f32 / total as f32) * 100.0;
        doc.push_str(&format!("### {}\n", route_name));
        doc.push_str(&format!("- Total tests: {}\n", total));
        doc.push_str(&format!(
            "- Identical: {} ({:.1}%)\n",
            identical, success_rate
        ));
        doc.push_str(&format!(
            "- Different: {} ({:.1}%)\n\n",
            different,
            100.0 - success_rate
        ));
    }

    // Differences summary
    if different_count > 0 {
        doc.push_str("## Differences Found\n");
        for result in results.iter().filter(|r| !r.is_identical) {
            doc.push_str(&format!(
                "### {} (User: {:?})\n",
                result.route_name, result.user_context
            ));
            for diff in &result.differences {
                doc.push_str(&format!("- {:?}: {}\n", diff.category, diff.description));
            }
            doc.push('\n');
        }
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Difference, DifferenceCategory, HttpResponse};
    use std::collections::HashMap;

    #[test]
    fn test_generate_request_documentation() {
        // Create mock comparison results
        let mut responses = HashMap::new();
        responses.insert(
            "test".to_string(),
            HttpResponse {
                status: 200,
                headers: HashMap::new(),
                body: "{}".to_string(),
                url: "https://test.example.com".to_string(),
                curl_command: "curl 'https://test.example.com'".to_string(),
            },
        );

        let mut status_codes1 = HashMap::new();
        status_codes1.insert("prod".to_string(), 200u16);
        status_codes1.insert("staging".to_string(), 200u16);

        let result1 = ComparisonResult {
            route_name: "user-profile".to_string(),
            user_context: {
                let mut ctx = HashMap::new();
                ctx.insert("userId".to_string(), "123".to_string());
                ctx
            },
            responses: responses.clone(),
            differences: vec![], // Identical
            is_identical: true,
            status_codes: status_codes1,
            has_errors: false,
            error_bodies: None,
        };

        let mut different_responses = responses.clone();
        different_responses.insert(
            "prod".to_string(),
            HttpResponse {
                status: 404,
                headers: HashMap::new(),
                body: "Not found".to_string(),
                url: "https://prod.example.com".to_string(),
                curl_command: "curl 'https://prod.example.com'".to_string(),
            },
        );

        let mut status_codes2 = HashMap::new();
        status_codes2.insert("prod".to_string(), 404u16);
        status_codes2.insert("staging".to_string(), 200u16);

        let mut error_bodies2 = HashMap::new();
        error_bodies2.insert("prod".to_string(), "Not found".to_string());

        let result2 = ComparisonResult {
            route_name: "health-check".to_string(),
            user_context: {
                let mut ctx = HashMap::new();
                ctx.insert("userId".to_string(), "123".to_string());
                ctx
            },
            responses: different_responses,
            differences: vec![Difference {
                category: DifferenceCategory::Status,
                description: "Status differs".to_string(),
                diff_output: None,
            }],
            is_identical: false,
            status_codes: status_codes2,
            has_errors: true,
            error_bodies: Some(error_bodies2),
        };

        let results = vec![result1, result2];
        let documentation = generate_request_documentation(&results).unwrap();

        // Verify documentation content
        assert!(documentation.contains("# HTTP Diff Test Documentation"));
        assert!(documentation.contains("Generated:"));
        assert!(documentation.contains("## Test Summary"));
        assert!(documentation.contains("- Total test scenarios: 2"));
        assert!(documentation.contains("- Identical responses: 1"));
        assert!(documentation.contains("- Different responses: 1"));
        assert!(documentation.contains("## Environments Tested"));
        assert!(documentation.contains("## Route Analysis"));
        assert!(documentation.contains("### user-profile"));
        assert!(documentation.contains("### health-check"));
        assert!(documentation.contains("## Differences Found"));
    }
}
