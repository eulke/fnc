//! Reusable HTML components for building professional reports

use super::super::ReportMetadata;
use crate::types::ComparisonResult;

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
                <h2>🏥 System Health Overview</h2>
                <div class="timestamp">Generated: {}</div>
            </div>
            
            <div class="metrics-grid">
                <div class="metric-card primary">
                    <div class="metric-icon">🎯</div>
                    <div class="metric-content">
                        <div class="metric-value {}">{}%</div>
                        <div class="metric-label">Health Score</div>
                    </div>
                </div>
                
                <div class="metric-card">
                    <div class="metric-icon">✅</div>
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
                    <div class="metric-icon">⚠️</div>
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
                    <div class="metric-icon">🔥</div>
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

    /// Generate detailed results table
    pub fn results_table(results: &[ComparisonResult]) -> String {
        let mut rows = String::new();

        for result in results {
            let status_badge = if result.has_errors {
                r#"<span class="status-badge error">🔥 Failed</span>"#
            } else if !result.is_identical {
                r#"<span class="status-badge warning">⚠️ Different</span>"#
            } else {
                r#"<span class="status-badge success">✅ Identical</span>"#
            };

            let user_context = if result.user_context.is_empty() {
                "default".to_string()
            } else {
                result
                    .user_context
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let status_codes = result
                .status_codes
                .iter()
                .map(|(env, code)| {
                    let class = if *code >= 200 && *code < 300 {
                        "success"
                    } else if *code >= 400 {
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
                .join(" ");

            rows.push_str(&format!(
                r#"
            <tr>
                <td><strong>{}</strong></td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td class="status-codes">{}</td>
            </tr>
            "#,
                result.route_name,
                user_context,
                status_badge,
                result.differences.len(),
                status_codes
            ));
        }

        format!(
            r#"
        <div class="results-section">
            <h2>📊 Detailed Test Results</h2>
            <div class="table-container">
                <table class="results-table">
                    <thead>
                        <tr>
                            <th>Route</th>
                            <th>User Context</th>
                            <th>Status</th>
                            <th>Differences</th>
                            <th>Status Codes</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
            </div>
        </div>
        "#,
            rows
        )
    }

    /// Generate recommendations section
    pub fn recommendations(results: &[ComparisonResult]) -> String {
        let failed_count = results.iter().filter(|r| r.has_errors).count();
        let different_count = results
            .iter()
            .filter(|r| !r.is_identical && !r.has_errors)
            .count();

        if failed_count == 0 && different_count == 0 {
            return format!(
                r#"
            <div class="recommendations-section">
                <h2>🎯 Recommendations</h2>
                <div class="recommendation success">
                    <div class="recommendation-icon">✅</div>
                    <div class="recommendation-content">
                        <strong>All Systems Operational</strong>
                        <p>All API endpoints are functioning identically across environments. No action required.</p>
                    </div>
                </div>
            </div>
            "#
            );
        }

        let mut recommendations = Vec::new();

        if failed_count > 0 {
            recommendations.push(
                r#"
            <div class="recommendation error">
                <div class="recommendation-icon">🔥</div>
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
                <div class="recommendation-icon">⚠️</div>
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
            <div class="recommendation-icon">🔧</div>
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
            <h2>🎯 Recommended Actions</h2>
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
