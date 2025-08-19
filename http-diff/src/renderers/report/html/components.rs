//! Reusable HTML components for building professional reports

use super::super::ReportMetadata;
use super::json_diff_renderer::JsonDiffRenderer;
use crate::renderers::diff_processor::DiffProcessor;
use crate::types::ComparisonResult;
use crate::utils::environment_utils::EnvironmentOrderResolver;

/// Reusable HTML components for report generation
pub struct HtmlComponents;

impl HtmlComponents {
    /// Generate executive summary dashboard
    pub fn executive_dashboard(results: &[ComparisonResult], metadata: &ReportMetadata) -> String {
        let total_tests = results.len();
        let failed_count = results.iter().filter(|r| r.has_errors).count();
        let identical_count = results
            .iter()
            .filter(|r| r.is_identical && !r.has_errors)
            .count();
        let different_count = results
            .iter()
            .filter(|r| !r.is_identical && !r.has_errors)
            .count();

        let health_score = Self::calculate_health_score(
            identical_count,
            different_count,
            failed_count,
            total_tests,
        );
        let health_class = Self::health_score_class(health_score);

        format!(
            r#"
        <div class="dashboard">
            <div class="dashboard-header">
                <h2>System Health Overview</h2>
                <div class="timestamp">Generated: {}</div>
            </div>
            
            <div class="metrics-grid">
                <div class="metric-card primary">
                    <div class="metric-icon"></div>
                    <div class="metric-content">
                        <div class="metric-value {}">{}%</div>
                        <div class="metric-label">Health Score</div>
                    </div>
                </div>
                
                <div class="metric-card">
                    <div class="metric-icon"></div>
                    <div class="metric-content">
                        <div class="metric-value success">{}</div>
                        <div class="metric-label">Identical</div>
                        <div class="metric-progress">
                            <div class="progress-bar">
                                <div class="progress-fill success" style="width: {:.1}%"></div>
                            </div>
                        </div>
                    </div>
                </div>
                
                {}
                
                {}
            </div>
            
            <div class="summary-stats">
                <div class="stat-item">
                    <span class="stat-label">Total Routes:</span>
                    <span class="stat-value">{}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">Environments:</span>
                    <span class="stat-value">{}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">Duration:</span>
                    <span class="stat-value">{:.1}s</span>
                </div>
            </div>
        </div>
        "#,
            metadata.timestamp.format("%Y-%m-%d %H:%M:%S %Z"),
            health_class,
            health_score,
            identical_count,
            if total_tests > 0 {
                (identical_count as f32 / total_tests as f32) * 100.0
            } else {
                0.0
            },
            if different_count > 0 {
                format!(
                    r#"
                <div class="metric-card">
                    <div class="metric-icon"></div>
                    <div class="metric-content">
                        <div class="metric-value warning">{}</div>
                        <div class="metric-label">Different</div>
                        <div class="metric-progress">
                            <div class="progress-bar">
                                <div class="progress-fill warning" style="width: {:.1}%"></div>
                            </div>
                        </div>
                    </div>
                </div>
                "#,
                    different_count,
                    if total_tests > 0 {
                        (different_count as f32 / total_tests as f32) * 100.0
                    } else {
                        0.0
                    }
                )
            } else {
                String::new()
            },
            if failed_count > 0 {
                format!(
                    r#"
                <div class="metric-card">
                    <div class="metric-icon"></div>
                    <div class="metric-content">
                        <div class="metric-value error">{}</div>
                        <div class="metric-label">Failed</div>
                        <div class="metric-progress">
                            <div class="progress-bar">
                                <div class="progress-fill error" style="width: {:.1}%"></div>
                            </div>
                        </div>
                    </div>
                </div>
                "#,
                    failed_count,
                    if total_tests > 0 {
                        (failed_count as f32 / total_tests as f32) * 100.0
                    } else {
                        0.0
                    }
                )
            } else {
                String::new()
            },
            total_tests,
            metadata.environments.join(", "),
            metadata.execution_duration.as_secs_f64()
        )
    }

    /// Generate comprehensive response details section showing all routes with filtering support
    pub fn response_details_section(
        results: &[ComparisonResult],
        show_unchanged: bool,
        max_routes: Option<usize>,
    ) -> String {
        if results.is_empty() {
            return String::new();
        }

        let diff_processor = DiffProcessor::new();
        let json_renderer = JsonDiffRenderer::new();
        let mut route_cards = String::new();
        let mut processed_count = 0;

        // Categorize routes for better organization
        let (identical_routes, different_routes, failed_routes) = Self::categorize_routes(results);

        // Process all route types in order: failed first (highest priority), then different, then identical
        let all_routes_ordered = [failed_routes, different_routes, identical_routes].concat();

        for result in all_routes_ordered {
            if let Some(max) = max_routes {
                if processed_count >= max {
                    break;
                }
            }

            let route_card =
                Self::render_route_card(result, &diff_processor, &json_renderer, show_unchanged);

            route_cards.push_str(&route_card);
            processed_count += 1;
        }

        let header_note = if let Some(max) = max_routes {
            if results.len() > max {
                format!(
                    "Showing top {} routes. {} additional routes available.",
                    max,
                    results.len() - max
                )
            } else {
                format!("Showing all {} routes.", results.len())
            }
        } else {
            format!("Showing all {} routes.", results.len())
        };

        format!(
            r#"
        <div class="response-details-section">
            <div class="response-details-header">
                <h2>Response Details</h2>
                <div class="response-details-note">{}</div>
            </div>
            <div class="response-details-content">
                {}
            </div>
        </div>
        "#,
            header_note, route_cards
        )
    }

    /// Categorize routes into identical, different, and failed groups
    fn categorize_routes(
        results: &[ComparisonResult],
    ) -> (
        Vec<&ComparisonResult>,
        Vec<&ComparisonResult>,
        Vec<&ComparisonResult>,
    ) {
        let mut identical_routes = Vec::new();
        let mut different_routes = Vec::new();
        let mut failed_routes = Vec::new();

        for result in results {
            if result.has_errors {
                failed_routes.push(result);
            } else if result.is_identical {
                identical_routes.push(result);
            } else {
                different_routes.push(result);
            }
        }

        (identical_routes, different_routes, failed_routes)
    }

    /// Render a single route card with expandable content
    fn render_route_card(
        result: &ComparisonResult,
        diff_processor: &DiffProcessor,
        json_renderer: &JsonDiffRenderer,
        show_unchanged: bool,
    ) -> String {
        let route_status = Self::get_route_status(result);
        // Create shared resolver for consistent environment ordering
        let resolver = result.create_environment_resolver();
        
        // Validate environment consistency before rendering
        if let Err(e) = result.validate_environment_consistency() {
            return format!(
                r#"<div class="error-message">Environment validation failed: {}</div>"#,
                Self::escape_html(&e.to_string())
            );
        }
        
        let status_badge = Self::get_status_badge(result);
        let user_context = Self::format_user_context(result);
        let status_codes = Self::format_status_codes_with_resolver(result, &resolver);
        let curl_commands = Self::render_curl_commands_with_resolver(result, &resolver);

        // Generate content based on route type
        let expandable_content = if result.has_errors {
            Self::render_failed_route_content(result, &curl_commands)
        } else if result.is_identical {
            Self::render_identical_route_content(result, &curl_commands)
        } else {
            // Route has differences - use existing diff processing
            match diff_processor.process_comparison_result(result, false) {
                Ok(diff_data) => {
                    if let Some(body_diff) = &diff_data.body {
                        if body_diff.has_differences {
                            let diff_summary = json_renderer.render_diff_summary(body_diff);
                            let diff_content =
                                json_renderer.render_body_diff(body_diff, show_unchanged);
                            Self::render_different_route_content(
                                result,
                                &diff_summary,
                                &diff_content,
                                &curl_commands,
                            )
                        } else {
                            Self::render_identical_route_content(result, &curl_commands)
                        }
                    } else {
                        Self::render_identical_route_content(result, &curl_commands)
                    }
                }
                Err(_) => Self::render_failed_route_content(result, &curl_commands),
            }
        };

        format!(
            r#"
        <div class="route-diff-section" data-status="{}" data-route-name="{}">
            <div class="route-diff-header">
                <div class="route-info">
                    <h3 class="route-name">{}</h3>
                    <div class="route-expand-icon">▼</div>
                    <div class="route-meta">
                        <div class="route-status-info">
                            {}
                            <span class="route-context">{}</span>
                        </div>
                        <div class="route-status-codes">{}</div>
                    </div>
                </div>
            </div>
            <div class="route-diff-body">
                {}
            </div>
        </div>
        "#,
            route_status,
            result.route_name.replace(" ", "-").to_lowercase(),
            result.route_name,
            status_badge,
            user_context,
            status_codes,
            expandable_content
        )
    }

    /// Get route status for filtering
    fn get_route_status(result: &ComparisonResult) -> &'static str {
        if result.has_errors {
            "failed"
        } else if result.is_identical {
            "identical"
        } else {
            "different"
        }
    }

    /// Get status badge HTML
    fn get_status_badge(result: &ComparisonResult) -> &'static str {
        if result.has_errors {
            r#"<span class="status-badge error">Failed</span>"#
        } else if result.is_identical {
            r#"<span class="status-badge success">Identical</span>"#
        } else {
            r#"<span class="status-badge warning">Different</span>"#
        }
    }

    /// Format user context for display
    fn format_user_context(result: &ComparisonResult) -> String {
        if result.user_context.is_empty() {
            "default".to_string()
        } else {
            result
                .user_context
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    /// Format status codes for display with deterministic environment ordering
    fn format_status_codes(result: &ComparisonResult) -> String {
        let resolver = result.create_environment_resolver();
        Self::format_status_codes_with_resolver(result, &resolver)
    }

    /// Format status codes for display with shared resolver (performance optimized)
    fn format_status_codes_with_resolver(result: &ComparisonResult, resolver: &EnvironmentOrderResolver) -> String {
        let ordered_status_codes = result.get_ordered_status_codes(resolver);
        
        ordered_status_codes
            .iter()
            .map(|(env, code)| {
                let class = if code >= 200 && code < 300 {
                    "success"
                } else if code >= 400 {
                    "error"
                } else {
                    "warning"
                };
                format!(
                    r#"<span class="status-code {}">{}: {}</span>"#,
                    class,
                    env.to_uppercase(),
                    code
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Render curl commands for a route with deterministic environment ordering
    fn render_curl_commands(result: &ComparisonResult) -> String {
        let resolver = result.create_environment_resolver();
        Self::render_curl_commands_with_resolver(result, &resolver)
    }

    /// Render curl commands for a route with shared resolver (performance optimized)
    fn render_curl_commands_with_resolver(result: &ComparisonResult, resolver: &EnvironmentOrderResolver) -> String {
        let mut curl_commands = String::new();

        let ordered_responses = result.get_ordered_responses(resolver);

        for (env, response) in ordered_responses.iter() {
            let status_class = if response.status >= 200 && response.status < 300 {
                "success"
            } else if response.status >= 400 {
                "error"
            } else {
                "warning"
            };

            curl_commands.push_str(&format!(
                r#"
                <div class="curl-command">
                    <div class="env-header">
                        <span class="env-name">{}</span>
                        <span class="status-code {}">{}</span>
                    </div>
                    <div class="command-box">
                        <code>{}</code>
                        <button class="copy-btn" onclick="copyToClipboard(this)">Copy</button>
                    </div>
                </div>
                "#,
                env.to_uppercase(),
                status_class,
                response.status,
                Self::escape_html(&response.curl_command)
            ));
        }

        format!(
            r#"
            <div class="technical-reproduction">
                <h4>Technical Reproduction Guide</h4>
                <div class="curl-commands">
                    {}
                </div>
            </div>
            "#,
            curl_commands
        )
    }

    /// Render content for identical routes
    fn render_identical_route_content(result: &ComparisonResult, curl_commands: &str) -> String {
        // Get response details from first response in deterministic environment order
        let response_summary = if let Some(response) = result.get_first_response_data() {
            format!(
                r#"
                <div class="response-summary">
                    <div class="summary-item">
                        <span class="summary-label">Response Size:</span>
                        <span class="summary-value">{} bytes</span>
                    </div>
                    <div class="summary-item">
                        <span class="summary-label">Content Type:</span>
                        <span class="summary-value">{}</span>
                    </div>
                    <div class="summary-item">
                        <span class="summary-label">Response Body Lines:</span>
                        <span class="summary-value">{}</span>
                    </div>
                </div>
                "#,
                response.body.len(),
                response
                    .headers
                    .get("content-type")
                    .unwrap_or(&"unknown".to_string()),
                crate::utils::response_summary::count_lines_efficient(&response.body)
            )
        } else {
            String::new()
        };

        format!(
            r#"
            <div class="identical-route-content">
                <div class="identical-summary">
                    <div class="identical-icon"></div>
                    <div class="identical-message">
                        <h4>Responses Are Identical</h4>
                        <p>All environments returned identical responses for this endpoint.</p>
                    </div>
                </div>
                {}
                {}
            </div>
            "#,
            response_summary, curl_commands
        )
    }

    /// Render content for failed routes
    fn render_failed_route_content(result: &ComparisonResult, curl_commands: &str) -> String {
        let error_details = if let Some(error_bodies) = &result.error_bodies {
            let mut errors = String::new();
            for (env, error_body) in error_bodies {
                errors.push_str(&format!(
                    r#"
                    <div class="error-detail">
                        <div class="error-env">{}</div>
                        <div class="error-message">{}</div>
                    </div>
                    "#,
                    env.to_uppercase(),
                    Self::escape_html(error_body)
                ));
            }
            errors
        } else {
            String::new()
        };

        format!(
            r#"
            <div class="failed-route-content">
                <div class="failure-summary">
                    <div class="failure-icon"></div>
                    <div class="failure-message">
                        <h4>Route Failed</h4>
                        <p>One or more environments returned error responses.</p>
                    </div>
                </div>
                <div class="error-details">
                    <h5>Error Details:</h5>
                    {}
                </div>
                <div class="troubleshooting">
                    <h5>Troubleshooting Steps:</h5>
                    <ul>
                        <li>Check endpoint availability and configuration</li>
                        <li>Verify network connectivity between environments</li>
                        <li>Review service logs for detailed error information</li>
                        <li>Use the curl commands below to reproduce the issue</li>
                    </ul>
                </div>
                {}
            </div>
            "#,
            error_details, curl_commands
        )
    }

    /// Render content for different routes (with diffs)
    fn render_different_route_content(
        _result: &ComparisonResult,
        diff_summary: &str,
        diff_content: &str,
        curl_commands: &str,
    ) -> String {
        format!(
            r#"
            <div class="different-route-content">
                <div class="difference-summary">
                    <div class="difference-icon"></div>
                    <div class="difference-message">
                        <h4>Response Differences Detected</h4>
                        <p>Environments returned different responses for this endpoint.</p>
                    </div>
                </div>
                {}
                <div class="diff-content">
                    {}
                </div>
                {}
            </div>
            "#,
            diff_summary, diff_content, curl_commands
        )
    }

    /// Escape HTML special characters
    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Generate recommendations section
    pub fn recommendations(results: &[ComparisonResult]) -> String {
        let failed_count = results.iter().filter(|r| r.has_errors).count();
        let different_count = results
            .iter()
            .filter(|r| !r.is_identical && !r.has_errors)
            .count();

        if failed_count == 0 && different_count == 0 {
            return r#"
            <div class="recommendations-section">
                <h2>Recommendations</h2>
                <div class="recommendation success">
                    <div class="recommendation-icon"></div>
                    <div class="recommendation-content">
                        <strong>All Systems Operational</strong>
                        <p>All API endpoints are functioning identically across environments. No action required.</p>
                    </div>
                </div>
            </div>
            "#.to_string();
        }

        let mut recommendations = Vec::new();

        if failed_count > 0 {
            recommendations.push(
                r#"
            <div class="recommendation error">
                <div class="recommendation-icon"></div>
                <div class="recommendation-content">
                    <strong>Address Critical Failures First</strong>
                    <p>Focus on failed endpoints as they indicate service disruptions or errors.</p>
                </div>
            </div>
            "#,
            );
        }

        if different_count > 0 {
            recommendations.push(r#"
            <div class="recommendation warning">
                <div class="recommendation-icon"></div>
                <div class="recommendation-content">
                    <strong>Review Environment Differences</strong>
                    <p>Different responses may indicate configuration inconsistencies between environments.</p>
                </div>
            </div>
            "#);
        }

        recommendations.push(
            r#"
        <div class="recommendation info">
            <div class="recommendation-icon"></div>
            <div class="recommendation-content">
                <strong>Use Curl Commands for Debugging</strong>
                <p>Copy the provided curl commands to reproduce and investigate issues locally.</p>
            </div>
        </div>
        "#,
        );

        format!(
            r#"
        <div class="recommendations-section">
            <h2>Recommended Actions</h2>
            {}
        </div>
        "#,
            recommendations.join("")
        )
    }

    /// Calculate health score (0-100)
    fn calculate_health_score(
        identical: usize,
        different: usize,
        failed: usize,
        total: usize,
    ) -> u8 {
        if total == 0 {
            return 100;
        }

        let identical_weight = 100.0;
        let different_weight = 50.0; // Partial credit for working but different endpoints
        let failed_weight = 0.0;

        let weighted_score = (identical as f32 * identical_weight
            + different as f32 * different_weight
            + failed as f32 * failed_weight)
            / (total as f32 * identical_weight);

        (weighted_score * 100.0).round() as u8
    }

    /// Get CSS class for health score
    fn health_score_class(score: u8) -> &'static str {
        match score {
            90..=100 => "success",
            75..=89 => "warning",
            _ => "error",
        }
    }
}
