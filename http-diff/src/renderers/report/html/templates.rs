//! HTML templates with embedded CSS for self-contained reports

use super::super::ReportMetadata;
use super::{components::HtmlComponents, DiffDetailLevel};
use crate::types::ComparisonResult;

/// HTML template generator for executive reports
pub struct HtmlTemplate;

impl HtmlTemplate {
    /// Create a new HTML template
    pub fn new() -> Self {
        Self
    }

    /// Render complete HTML report
    pub fn render(
        &self,
        results: &[ComparisonResult],
        metadata: &ReportMetadata,
        _include_technical: bool,
        diff_detail_level: &DiffDetailLevel,
        max_diff_routes: Option<usize>,
        show_unchanged_lines: bool,
    ) -> String {
        let response_details_section = match diff_detail_level {
            DiffDetailLevel::Executive => String::new(),
            DiffDetailLevel::Basic | DiffDetailLevel::Detailed => {
                let show_unchanged =
                    *diff_detail_level == DiffDetailLevel::Detailed && show_unchanged_lines;
                HtmlComponents::response_details_section(results, show_unchanged, max_diff_routes)
            }
        };

        let js_content = Self::embedded_javascript();

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>HTTP API Diff Report - {}</title>
    <meta name="description" content="Comprehensive API comparison analysis between environments">
    <style>{}</style>
</head>
<body>
    <div class="container">
        <header class="report-header" role="banner">
            <div class="header-content">
                <h1>HTTP API Diff Report</h1>
                <div class="report-subtitle">Comprehensive API comparison analysis</div>
                <nav class="report-nav" role="navigation" aria-label="Report sections">
                    <button class="nav-btn nav-btn--active" data-section="dashboard" aria-pressed="true">
                        Overview
                    </button>
                    <button class="nav-btn" data-section="technical" aria-pressed="false">
                        Technical Details
                    </button>
                    <button class="nav-btn" data-section="actions" aria-pressed="false">
                        Recommended Actions
                    </button>
                </nav>
            </div>
        </header>
        
        <main role="main">
            <!-- Executive Dashboard Section -->
            <section id="dashboard" class="report-section report-section--active" role="region" aria-labelledby="dashboard-heading">
                <div class="section-header">
                    <h2 id="dashboard-heading" class="visually-hidden">Executive Dashboard</h2>
                    <div class="breadcrumb" aria-label="Navigation breadcrumb">
                        <span class="breadcrumb-item breadcrumb-item--active">Overview</span>
                    </div>
                </div>
                {}
            </section>
            
            <!-- Technical Details Section -->
            <section id="technical" class="report-section" role="region" aria-labelledby="technical-heading" hidden>
                <div class="section-header">
                    <h2 id="technical-heading">Technical Analysis</h2>
                    <div class="breadcrumb" aria-label="Navigation breadcrumb">
                        <button class="breadcrumb-link" data-section="dashboard">Overview</button>
                        <span class="breadcrumb-separator">›</span>
                        <span class="breadcrumb-item breadcrumb-item--active">Technical Details</span>
                    </div>
                </div>
                
                <!-- Modern Filter Controls -->
                <aside class="filter-controls" role="complementary" aria-labelledby="filter-heading">
                    <h3 id="filter-heading">
                        Filter & Search Results
                        <button class="help-icon" aria-label="Show keyboard shortcuts" title="Keyboard shortcuts available">
                            <span aria-hidden="true">?</span>
                        </button>
                    </h3>
                    <div class="filter-row">
                        <div class="filter-group">
                            <label for="status-filter">Status:</label>
                            <div class="filter-toggle" role="group" aria-labelledby="status-filter">
                                <button class="filter-btn status-filter-btn filter-btn--active" data-status="all" aria-pressed="true">
                                    All <span class="badge" id="all-count" aria-label="Total routes">0</span>
                                </button>
                                <button class="filter-btn status-filter-btn" data-status="different" aria-pressed="false">
                                    Different <span class="badge" id="different-count" aria-label="Different routes">0</span>
                                </button>
                                <button class="filter-btn status-filter-btn" data-status="failed" aria-pressed="false">
                                    Failed <span class="badge" id="failed-count" aria-label="Failed routes">0</span>
                                </button>
                                <button class="filter-btn status-filter-btn" data-status="identical" aria-pressed="false">
                                    Identical <span class="badge" id="identical-count" aria-label="Identical routes">0</span>
                                </button>
                            </div>
                        </div>
                        <div class="filter-group">
                            <label for="route-search">Search routes:</label>
                            <input type="search" id="route-search" class="filter-search" placeholder="Search by route name..." 
                                   aria-describedby="search-help" autocomplete="off" />
                            <div id="search-help" class="visually-hidden">Search routes by name, method, or path</div>
                        </div>
                        <div class="filter-group filter-group--actions">
                            <button class="expand-all-btn" id="expand-toggle" aria-pressed="false">
                                Expand All
                            </button>
                            <button class="clear-filters" aria-label="Clear all filters and reset view">
                                Clear Filters
                            </button>
                        </div>
                    </div>
                    <div class="filter-status" role="status" aria-live="polite">
                        <span class="filter-stats">Showing all routes</span>
                        <div class="keyboard-hints" aria-label="Available keyboard shortcuts">
                            <kbd>/</kbd> Search • <kbd>1-4</kbd> Filter • <kbd>E</kbd> Expand • <kbd>R</kbd> Reset
                        </div>
                    </div>
                </aside>
                
                <!-- Technical Details Content -->
                <div class="technical-content">
                    {}
                </div>
            </section>
            
            <!-- Recommendations Section -->
            <section id="actions" class="report-section" role="region" aria-labelledby="actions-heading" hidden>
                <div class="section-header">
                    <h2 id="actions-heading">Recommended Actions</h2>
                    <div class="breadcrumb" aria-label="Navigation breadcrumb">
                        <button class="breadcrumb-link" data-section="dashboard">Overview</button>
                        <span class="breadcrumb-separator">›</span>
                        <span class="breadcrumb-item breadcrumb-item--active">Recommended Actions</span>
                    </div>
                </div>
                {}
            </section>
        </main>
        
        <footer class="report-footer">
            <div class="footer-content">
                <div class="generated-info">
                    Generated by HTTP-Diff CLI • {} • {} environments tested
                </div>
                <div class="footer-note">
                    This report provides an executive summary of API endpoint comparisons across environments.
                </div>
            </div>
        </footer>
    </div>
    {}
</body>
</html>"#,
            metadata.timestamp.format("%Y-%m-%d"),
            Self::embedded_css(),
            HtmlComponents::executive_dashboard(results, metadata),
            response_details_section,
            HtmlComponents::recommendations(results),
            // Technical details are now integrated into each route card
            metadata.timestamp.format("%Y-%m-%d %H:%M %Z"),
            metadata.environments.len(),
            js_content
        )
    }

    /// Embedded CSS for self-contained reports
    fn embedded_css() -> &'static str {
        r#"
        /* Modern Design System for HTTP Diff Reports */
        :root {
            /* Color System - High contrast, modern palette */
            --color-primary: #0969da;
            --color-primary-hover: #0550ae;
            --color-primary-light: #dbeafe;
            
            --color-success: #1f883d;
            --color-success-light: #dcfce7;
            --color-success-border: #bbf7d0;
            
            --color-warning: #d97706; 
            --color-warning-light: #fef3c7;
            --color-warning-border: #fed7aa;
            
            --color-error: #da3633;
            --color-error-light: #fee2e2;
            --color-error-border: #fecaca;
            
            /* Neutral grays */
            --color-gray-50: #f9fafb;
            --color-gray-100: #f3f4f6;
            --color-gray-200: #e5e7eb;
            --color-gray-300: #d1d5db;
            --color-gray-400: #9ca3af;
            --color-gray-500: #6b7280;
            --color-gray-600: #4b5563;
            --color-gray-700: #374151;
            --color-gray-800: #1f2937;
            --color-gray-900: #111827;
            
            /* Background colors */
            --bg-primary: #ffffff;
            --bg-secondary: #f9fafb;
            --bg-elevated: #ffffff;
            --bg-overlay: rgba(0, 0, 0, 0.8);
            
            /* Typography Scale */
            --font-family-base: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', sans-serif;
            --font-family-mono: 'SFMono-Regular', 'Monaco', 'Inconsolata', 'Roboto Mono', 'Courier New', monospace;
            
            --text-xs: 0.75rem;      /* 12px */
            --text-sm: 0.875rem;     /* 14px */
            --text-base: 1rem;       /* 16px */
            --text-lg: 1.125rem;     /* 18px */
            --text-xl: 1.25rem;      /* 20px */
            --text-2xl: 1.5rem;      /* 24px */
            --text-3xl: 1.875rem;    /* 30px */
            --text-4xl: 2.25rem;     /* 36px */
            
            --font-weight-normal: 400;
            --font-weight-medium: 500;
            --font-weight-semibold: 600;
            --font-weight-bold: 700;
            
            /* Spacing Scale - Based on 4px grid */
            --space-1: 0.25rem;      /* 4px */
            --space-2: 0.5rem;       /* 8px */
            --space-3: 0.75rem;      /* 12px */
            --space-4: 1rem;         /* 16px */
            --space-5: 1.25rem;      /* 20px */
            --space-6: 1.5rem;       /* 24px */
            --space-8: 2rem;         /* 32px */
            --space-10: 2.5rem;      /* 40px */
            --space-12: 3rem;        /* 48px */
            --space-16: 4rem;        /* 64px */
            --space-20: 5rem;        /* 80px */
            
            /* Border radius */
            --radius-sm: 0.25rem;    /* 4px */
            --radius-md: 0.5rem;     /* 8px */
            --radius-lg: 0.75rem;    /* 12px */
            --radius-xl: 1rem;       /* 16px */
            
            /* Shadows */
            --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
            --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
            --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
            --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
            
            /* Animation */
            --transition-fast: 150ms ease;
            --transition-base: 250ms ease;
            --transition-slow: 350ms ease;
            
            /* Z-index scale */
            --z-dropdown: 1000;
            --z-sticky: 1020;
            --z-fixed: 1030;
            --z-modal: 1040;
            --z-popover: 1050;
            --z-tooltip: 1060;
        }
        
        /* Reset and base styles */
        *, *::before, *::after {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }
        
        html {
            font-size: 16px;
            line-height: 1.5;
            -webkit-text-size-adjust: 100%;
            -webkit-font-smoothing: antialiased;
            -moz-osx-font-smoothing: grayscale;
        }
        
        body {
            font-family: var(--font-family-base);
            font-size: var(--text-base);
            line-height: 1.6;
            color: var(--color-gray-900);
            background-color: var(--bg-secondary);
            min-height: 100vh;
        }
        
        /* Container and layout */
        .container {
            max-width: 1280px;
            margin: 0 auto;
            padding: var(--space-4);
            background: var(--bg-primary);
            border-radius: var(--radius-xl);
            box-shadow: var(--shadow-xl);
            margin-top: var(--space-6);
            margin-bottom: var(--space-6);
            min-height: calc(100vh - 3rem);
        }
        
        @media (max-width: 768px) {
            .container {
                margin: var(--space-2);
                padding: var(--space-3);
                border-radius: var(--radius-lg);
                margin-top: var(--space-2);
                margin-bottom: var(--space-2);
                min-height: calc(100vh - 1rem);
            }
        }
        
        /* Typography */
        h1, h2, h3, h4, h5, h6 {
            font-weight: var(--font-weight-semibold);
            line-height: 1.25;
            margin-bottom: var(--space-4);
            color: var(--color-gray-900);
        }
        
        h1 { font-size: var(--text-4xl); }
        h2 { font-size: var(--text-3xl); }
        h3 { font-size: var(--text-2xl); }
        h4 { font-size: var(--text-xl); }
        h5 { font-size: var(--text-lg); }
        h6 { font-size: var(--text-base); }
        
        p {
            margin-bottom: var(--space-4);
            color: var(--color-gray-700);
        }
        
        /* Header */
        .report-header {
            text-align: center;
            padding: var(--space-12) 0;
            border-bottom: 1px solid var(--color-gray-200);
            margin-bottom: var(--space-10);
            background: linear-gradient(135deg, var(--bg-primary) 0%, var(--bg-secondary) 100%);
            border-radius: var(--radius-xl) var(--radius-xl) 0 0;
            margin: calc(-1 * var(--space-4)) calc(-1 * var(--space-4)) var(--space-10) calc(-1 * var(--space-4));
            padding-left: var(--space-4);
            padding-right: var(--space-4);
        }
        
        .report-header h1 {
            font-size: var(--text-4xl);
            font-weight: var(--font-weight-bold);
            color: var(--color-gray-900);
            margin-bottom: var(--space-3);
            letter-spacing: -0.025em;
        }
        
        .report-subtitle {
            font-size: var(--text-lg);
            color: var(--color-gray-600);
            font-weight: var(--font-weight-medium);
        }
        
        .header-content {
            display: flex;
            flex-direction: column;
            gap: var(--space-6);
            align-items: center;
        }
        
        /* Modern navigation tabs */
        .report-nav {
            display: flex;
            gap: var(--space-2);
            background: var(--bg-primary);
            padding: var(--space-2);
            border-radius: var(--radius-lg);
            border: 1px solid var(--color-gray-200);
            box-shadow: var(--shadow-sm);
        }
        
        .nav-btn {
            padding: var(--space-3) var(--space-6);
            border: none;
            background: transparent;
            color: var(--color-gray-600);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            cursor: pointer;
            border-radius: var(--radius-md);
            transition: all var(--transition-fast);
            display: flex;
            align-items: center;
            gap: var(--space-2);
            white-space: nowrap;
        }
        
        .nav-btn:hover {
            background: var(--color-gray-100);
            color: var(--color-gray-900);
        }
        
        .nav-btn--active {
            background: var(--color-primary);
            color: white;
            box-shadow: var(--shadow-sm);
        }
        
        /* Section management */
        .report-section {
            padding: var(--space-8) 0;
        }
        
        .report-section[hidden] {
            display: none;
        }
        
        .report-section--active {
            display: block;
        }
        
        .section-header {
            margin-bottom: var(--space-8);
        }
        
        /* Breadcrumb navigation */
        .breadcrumb {
            display: flex;
            align-items: center;
            gap: var(--space-2);
            font-size: var(--text-sm);
            color: var(--color-gray-600);
            margin-bottom: var(--space-4);
        }
        
        .breadcrumb-link {
            background: none;
            border: none;
            color: var(--color-primary);
            text-decoration: underline;
            cursor: pointer;
            font-size: var(--text-sm);
            padding: var(--space-1);
            border-radius: var(--radius-sm);
            transition: background-color var(--transition-fast);
        }
        
        .breadcrumb-link:hover {
            background: var(--color-primary-light);
        }
        
        .breadcrumb-separator {
            color: var(--color-gray-400);
            font-weight: var(--font-weight-medium);
        }
        
        .breadcrumb-item--active {
            color: var(--color-gray-900);
            font-weight: var(--font-weight-medium);
        }
        
        /* Enhanced accessibility and utility classes */
        .visually-hidden {
            position: absolute !important;
            width: 1px !important;
            height: 1px !important;
            padding: 0 !important;
            margin: -1px !important;
            overflow: hidden !important;
            clip: rect(0, 0, 0, 0) !important;
            white-space: nowrap !important;
            border: 0 !important;
        }
        
        .badge {
            background: var(--color-gray-200);
            color: var(--color-gray-700);
            padding: var(--space-1) var(--space-2);
            border-radius: var(--radius-sm);
            font-size: var(--text-xs);
            font-weight: var(--font-weight-semibold);
            margin-left: var(--space-2);
            min-width: 1.5rem;
            text-align: center;
            display: inline-block;
        }
        
        .filter-btn--active .badge {
            background: rgba(255, 255, 255, 0.3);
            color: white;
        }
        
        .filter-group--actions {
            margin-left: auto;
        }
        
        .filter-status {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-top: var(--space-4);
            padding-top: var(--space-4);
            border-top: 1px solid var(--color-gray-200);
        }
        
        .keyboard-hints {
            display: flex;
            align-items: center;
            gap: var(--space-2);
            font-size: var(--text-xs);
            color: var(--color-gray-500);
        }
        
        .keyboard-hints kbd {
            background: var(--color-gray-100);
            color: var(--color-gray-700);
            padding: var(--space-1) var(--space-2);
            border-radius: var(--radius-sm);
            font-family: var(--font-family-mono);
            font-size: var(--text-xs);
            border: 1px solid var(--color-gray-300);
            box-shadow: 0 1px 0 var(--color-gray-300);
        }
        
        .technical-content {
            margin-top: var(--space-8);
        }
        
        @media (max-width: 768px) {
            .report-header {
                padding: var(--space-8) 0;
                margin: calc(-1 * var(--space-3)) calc(-1 * var(--space-3)) var(--space-8) calc(-1 * var(--space-3));
                padding-left: var(--space-3);
                padding-right: var(--space-3);
            }
            
            .report-header h1 {
                font-size: var(--text-3xl);
            }
            
            .report-subtitle {
                font-size: var(--text-base);
            }
            
            .header-content {
                gap: var(--space-4);
            }
            
            .report-nav {
                flex-direction: column;
                width: 100%;
                max-width: 300px;
            }
            
            .nav-btn {
                justify-content: center;
                width: 100%;
            }
            
            .filter-status {
                flex-direction: column;
                align-items: flex-start;
                gap: var(--space-3);
            }
            
            .keyboard-hints {
                align-self: stretch;
                justify-content: center;
            }
            
            .breadcrumb {
                justify-content: center;
            }
            
            .filter-group--actions {
                margin-left: 0;
                width: 100%;
                justify-content: space-between;
            }
        }
        
        /* Dashboard Components */
        .dashboard {
            margin-bottom: var(--space-12);
        }
        
        .dashboard-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: var(--space-8);
            flex-wrap: wrap;
            gap: var(--space-4);
        }
        
        .dashboard-header h2 {
            color: var(--color-gray-900);
            font-size: var(--text-3xl);
            font-weight: var(--font-weight-bold);
            margin-bottom: 0;
        }
        
        .timestamp {
            color: var(--color-gray-600);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            background: var(--color-gray-100);
            padding: var(--space-2) var(--space-3);
            border-radius: var(--radius-md);
        }
        
        /* Modern metrics grid */
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: var(--space-6);
            margin-bottom: var(--space-10);
        }
        
        @media (max-width: 768px) {
            .metrics-grid {
                grid-template-columns: 1fr;
                gap: var(--space-4);
                margin-bottom: var(--space-8);
            }
            
            .dashboard-header {
                flex-direction: column;
                align-items: flex-start;
                text-align: left;
                gap: var(--space-3);
            }
        }
        
        /* Modern metric cards */
        .metric-card {
            background: var(--bg-primary);
            border: 1px solid var(--color-gray-200);
            border-radius: var(--radius-lg);
            padding: var(--space-6);
            box-shadow: var(--shadow-sm);
            transition: all var(--transition-fast);
            position: relative;
            overflow: hidden;
        }
        
        .metric-card:hover {
            border-color: var(--color-primary);
            box-shadow: var(--shadow-lg);
            transform: translateY(-1px);
        }
        
        .metric-card.primary {
            border-color: var(--color-primary);
            background: linear-gradient(135deg, var(--color-primary-light) 0%, var(--bg-primary) 100%);
        }
        
        .metric-card .metric-icon {
            font-size: var(--text-3xl);
            margin-bottom: var(--space-4);
            opacity: 0.8;
        }
        
        .metric-value {
            font-size: var(--text-4xl);
            font-weight: var(--font-weight-bold);
            margin-bottom: var(--space-2);
            line-height: 1;
        }
        
        .metric-value.success { color: var(--color-success); }
        .metric-value.warning { color: var(--color-warning); }
        .metric-value.error { color: var(--color-error); }
        
        .metric-label {
            color: var(--color-gray-600);
            font-size: var(--text-base);
            font-weight: var(--font-weight-medium);
            margin-bottom: var(--space-3);
        }
        
        /* Modern progress bars */
        .progress-bar {
            background: var(--color-gray-200);
            height: 6px;
            border-radius: var(--radius-sm);
            overflow: hidden;
            margin-top: var(--space-3);
        }
        
        .progress-fill {
            height: 100%;
            border-radius: var(--radius-sm);
            transition: width var(--transition-slow);
            position: relative;
        }
        
        .progress-fill.success { 
            background: linear-gradient(90deg, var(--color-success) 0%, #22c55e 100%);
        }
        .progress-fill.warning { 
            background: linear-gradient(90deg, var(--color-warning) 0%, #f59e0b 100%);
        }
        .progress-fill.error { 
            background: linear-gradient(90deg, var(--color-error) 0%, #ef4444 100%);
        }
        
        /* Summary statistics */
        .summary-stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: var(--space-6);
            background: var(--bg-secondary);
            border-radius: var(--radius-lg);
            padding: var(--space-6);
            border: 1px solid var(--color-gray-200);
        }
        
        @media (max-width: 768px) {
            .summary-stats {
                grid-template-columns: 1fr;
                gap: var(--space-4);
                padding: var(--space-4);
            }
        }
        
        .stat-item {
            text-align: center;
            padding: var(--space-4);
            background: var(--bg-primary);
            border-radius: var(--radius-md);
            border: 1px solid var(--color-gray-100);
        }
        
        .stat-label {
            display: block;
            color: var(--color-gray-600);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            margin-bottom: var(--space-2);
            text-transform: uppercase;
            letter-spacing: 0.025em;
        }
        
        .stat-value {
            font-size: var(--text-xl);
            font-weight: var(--font-weight-bold);
            color: var(--color-gray-900);
        }
        
        /* Modern status badges and indicators */
        .status-badge {
            display: inline-flex;
            align-items: center;
            gap: var(--space-1);
            padding: var(--space-1) var(--space-3);
            border-radius: var(--radius-md);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            border: 1px solid transparent;
            transition: all var(--transition-fast);
        }
        
        .status-badge.success {
            background: var(--color-success-light);
            color: var(--color-success);
            border-color: var(--color-success-border);
        }
        
        .status-badge.warning {
            background: var(--color-warning-light);
            color: var(--color-warning);
            border-color: var(--color-warning-border);
        }
        
        .status-badge.error {
            background: var(--color-error-light);
            color: var(--color-error);
            border-color: var(--color-error-border);
        }
        
        .status-codes {
            display: flex;
            gap: var(--space-2);
            flex-wrap: wrap;
        }
        
        .status-code {
            display: inline-flex;
            align-items: center;
            padding: var(--space-1) var(--space-2);
            border-radius: var(--radius-sm);
            font-family: var(--font-family-mono);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            border: 1px solid transparent;
            min-width: 4rem;
            justify-content: center;
        }
        
        .status-code.success {
            background: var(--color-success-light);
            color: var(--color-success);
            border-color: var(--color-success-border);
        }
        
        .status-code.warning {
            background: var(--color-warning-light);
            color: var(--color-warning);
            border-color: var(--color-warning-border);
        }
        
        .status-code.error {
            background: var(--color-error-light);
            color: var(--color-error);
            border-color: var(--color-error-border);
        }
        
        /* Results Section (legacy support) */
        .results-section {
            margin-bottom: var(--space-12);
        }
        
        .results-section h2 {
            color: var(--color-gray-900);
            margin-bottom: var(--space-6);
            font-size: var(--text-3xl);
            font-weight: var(--font-weight-bold);
        }
        
        /* Response Details Section (new unified section) */
        .response-details-section {
            margin-bottom: var(--space-12);
        }
        
        .response-details-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: var(--space-8);
            flex-wrap: wrap;
            gap: var(--space-4);
        }
        
        .response-details-header h2 {
            color: var(--color-gray-900);
            font-size: var(--text-3xl);
            font-weight: var(--font-weight-bold);
            margin-bottom: 0;
        }
        
        .response-details-note {
            color: var(--color-gray-600);
            font-size: var(--text-sm);
            background: var(--color-gray-100);
            padding: var(--space-2) var(--space-3);
            border-radius: var(--radius-md);
        }
        
        .response-details-content {
            display: flex;
            flex-direction: column;
            gap: var(--space-6);
        }
        
        @media (max-width: 768px) {
            .response-details-header {
                flex-direction: column;
                align-items: flex-start;
            }
            
            .response-details-content {
                gap: var(--space-4);
            }
        }
        
        /* Modern recommendations section */
        .recommendations-section {
            margin-bottom: var(--space-12);
        }
        
        .recommendations-section h2 {
            color: var(--color-gray-900);
            margin-bottom: var(--space-6);
            font-size: var(--text-3xl);
            font-weight: var(--font-weight-bold);
        }
        
        .recommendation {
            display: flex;
            align-items: flex-start;
            padding: var(--space-6);
            border-radius: var(--radius-lg);
            margin-bottom: var(--space-4);
            border: 1px solid;
            position: relative;
            background: var(--bg-primary);
            transition: all var(--transition-fast);
        }
        
        .recommendation:hover {
            transform: translateY(-1px);
            box-shadow: var(--shadow-lg);
        }
        
        .recommendation.success {
            border-color: var(--color-success-border);
            background: linear-gradient(135deg, var(--color-success-light) 0%, var(--bg-primary) 100%);
        }
        
        .recommendation.warning {
            border-color: var(--color-warning-border);
            background: linear-gradient(135deg, var(--color-warning-light) 0%, var(--bg-primary) 100%);
        }
        
        .recommendation.error {
            border-color: var(--color-error-border);
            background: linear-gradient(135deg, var(--color-error-light) 0%, var(--bg-primary) 100%);
        }
        
        .recommendation.info {
            border-color: var(--color-primary);
            background: linear-gradient(135deg, var(--color-primary-light) 0%, var(--bg-primary) 100%);
        }
        
        .recommendation-icon {
            font-size: var(--text-2xl);
            margin-right: var(--space-4);
            margin-top: var(--space-1);
            opacity: 0.8;
        }
        
        .recommendation-content {
            flex: 1;
        }
        
        .recommendation-content strong {
            display: block;
            margin-bottom: var(--space-2);
            font-size: var(--text-lg);
            font-weight: var(--font-weight-semibold);
            color: var(--color-gray-900);
        }
        
        .recommendation-content p {
            margin-bottom: 0;
            color: var(--color-gray-700);
            line-height: 1.5;
        }
        
        /* Modern route card system */
        .route-diff-section {
            background: var(--bg-primary);
            border: 1px solid var(--color-gray-200);
            border-radius: var(--radius-lg);
            margin-bottom: var(--space-6);
            overflow: hidden;
            box-shadow: var(--shadow-sm);
            transition: all var(--transition-fast);
        }
        
        .route-diff-section:hover {
            border-color: var(--color-gray-300);
            box-shadow: var(--shadow-md);
        }
        
        .route-diff-section.collapsed .route-diff-body {
            display: none;
        }
        
        /* Route header with click interaction */
        .route-diff-header {
            padding: var(--space-6);
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--color-gray-200);
            cursor: pointer;
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            transition: background-color var(--transition-fast);
            user-select: none;
        }
        
        .route-diff-header:hover {
            background: var(--color-gray-100);
        }
        
        .route-info {
            flex: 1;
            min-width: 0;
        }
        
        .route-name {
            font-size: var(--text-xl);
            font-weight: var(--font-weight-semibold);
            color: var(--color-gray-900);
            margin-bottom: var(--space-3);
            display: flex;
            align-items: center;
            gap: var(--space-2);
        }
        
        .route-expand-icon {
            color: var(--color-gray-500);
            font-size: var(--text-sm);
            transition: transform var(--transition-fast);
            margin-left: auto;
            padding: var(--space-1);
            display: inline-block;
            width: 1.25em;
            text-align: center;
        }
        
        .route-diff-section.collapsed .route-expand-icon {
            transform: none;
        }
        
        .route-meta {
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
            gap: var(--space-4);
        }
        
        .route-status-info {
            display: flex;
            align-items: center;
            gap: var(--space-3);
        }
        
        .route-context {
            color: var(--color-gray-600);
            font-size: var(--text-sm);
            font-family: var(--font-family-mono);
            background: var(--color-gray-100);
            padding: var(--space-1) var(--space-2);
            border-radius: var(--radius-sm);
        }
        
        /* Route body content */
        .route-diff-body {
            background: var(--bg-primary);
        }
        
        /* Content type specific styling */
        .identical-route-content {
            padding: var(--space-6);
        }
        
        .identical-summary {
            display: flex;
            align-items: center;
            gap: var(--space-4);
            padding: var(--space-6);
            background: var(--color-success-light);
            border-radius: var(--radius-lg);
            margin-bottom: var(--space-6);
            border: 1px solid var(--color-success-border);
        }
        
        .identical-icon {
            font-size: var(--text-3xl);
            opacity: 0.8;
        }
        
        .identical-message h4 {
            margin-bottom: var(--space-2);
            color: var(--color-success);
            font-weight: var(--font-weight-semibold);
        }
        
        .identical-message p {
            margin: 0;
            color: var(--color-gray-700);
        }
        
        .different-route-content {
            padding: 0;
        }
        
        /* JSON body side-by-side diff */
        .json-diff-container {
            background: var(--bg-primary);
            border-top: 1px solid var(--color-gray-200);
        }

        .json-diff-header {
            display: grid;
            grid-template-columns: 1fr 1fr auto;
            gap: var(--space-3);
            align-items: center;
            padding: var(--space-4) var(--space-6);
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--color-gray-200);
        }

        .json-env {
            font-size: var(--text-sm);
            font-weight: var(--font-weight-semibold);
            color: var(--color-gray-700);
            text-transform: uppercase;
            letter-spacing: 0.03em;
        }

        .json-env-right {
            text-align: right;
        }

        .json-actions {
            display: flex;
            gap: var(--space-2);
        }

        .json-action-btn {
            background: var(--color-primary);
            color: #fff;
            border: none;
            padding: var(--space-2) var(--space-3);
            border-radius: var(--radius-sm);
            cursor: pointer;
            font-size: var(--text-sm);
            transition: background var(--transition-fast);
        }

        .json-action-btn:hover { background: var(--color-primary-hover); }

        .json-diff-content {
            overflow-x: auto;
        }

        .json-diff-table {
            width: 100%;
            border-collapse: collapse;
            table-layout: fixed;
            line-height: 0.8;
        }

        .json-line-number {
            width: 3.5rem;
            padding: var(--space-2) var(--space-3);
            text-align: right;
            color: var(--color-gray-500);
            font-family: var(--font-family-mono);
            font-size: var(--text-sm);
            background: var(--bg-secondary);
            vertical-align: top;
            user-select: none;
        }

        .json-code-block {
            width: 50%;
            padding: var(--space-2) var(--space-3);
            font-family: var(--font-family-mono);
            font-size: var(--text-sm);
            vertical-align: top;
            white-space: pre-wrap;
            word-break: break-word;
        }

        .json-code-block pre { margin: 0; }
        .json-code-block code { color: var(--color-gray-900); }

        /* Diff state colors */
        .json-row-unchanged .json-code-block { background: var(--bg-primary); }
        .json-row-added .json-code-block.json-right { background: var(--color-success-light); }
        .json-row-removed .json-code-block.json-left { background: var(--color-error-light); }
        .json-row-changed .json-code-block.json-left { background: var(--color-warning-light); }
        .json-row-changed .json-code-block.json-right { background: var(--color-warning-light); }

        @media (max-width: 768px) {
            .json-diff-header {
                grid-template-columns: 1fr;
                gap: var(--space-2);
            }
            .json-env-right { text-align: left; }
        }

        .difference-summary {
            display: flex;
            align-items: center;
            gap: var(--space-4);
            padding: var(--space-6);
            background: var(--color-warning-light);
            border-bottom: 1px solid var(--color-warning-border);
        }
        
        .difference-icon {
            font-size: var(--text-3xl);
            opacity: 0.8;
        }
        
        .difference-message h4 {
            margin-bottom: var(--space-2);
            color: var(--color-warning);
            font-weight: var(--font-weight-semibold);
        }
        
        .difference-message p {
            margin: 0;
            color: var(--color-gray-700);
        }
        
        /* Compact diff stats (added/removed/changed) */
        .diff-summary-badge {
            display: flex;
            align-items: center;
            gap: var(--space-4);
            padding: var(--space-4) var(--space-6);
            background: var(--bg-primary);
            border-bottom: 1px solid var(--color-gray-200);
        }

        .diff-summary-badge .summary-item {
            display: inline-flex;
            align-items: center;
            gap: var(--space-2);
            border: 0;
            padding: 0;
        }

        .diff-summary-badge .summary-count {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            min-width: 2.25rem;
            height: 1.75rem;
            padding: 0 var(--space-2);
            border-radius: 9999px;
            font-size: var(--text-sm);
            font-weight: var(--font-weight-semibold);
            border: 1px solid transparent;
        }

        .diff-summary-badge .summary-label {
            font-size: var(--text-sm);
            color: var(--color-gray-600);
        }

        .diff-summary-badge .summary-count.added {
            background: var(--color-success-light);
            color: var(--color-success);
            border-color: var(--color-success-border);
        }
        .diff-summary-badge .summary-count.removed {
            background: var(--color-error-light);
            color: var(--color-error);
            border-color: var(--color-error-border);
        }
        .diff-summary-badge .summary-count.changed {
            background: var(--color-warning-light);
            color: var(--color-warning);
            border-color: var(--color-warning-border);
        }

        .failed-route-content {
            padding: var(--space-6);
        }
        
        .failure-summary {
            display: flex;
            align-items: center;
            gap: var(--space-4);
            padding: var(--space-6);
            background: var(--color-error-light);
            border-radius: var(--radius-lg);
            margin-bottom: var(--space-6);
            border: 1px solid var(--color-error-border);
        }
        
        .failure-icon {
            font-size: var(--text-3xl);
            opacity: 0.8;
        }
        
        .failure-message h4 {
            margin-bottom: var(--space-2);
            color: var(--color-error);
            font-weight: var(--font-weight-semibold);
        }
        
        .failure-message p {
            margin: 0;
            color: var(--color-gray-700);
        }
        
        /* Technical reproduction section */
        .technical-reproduction {
            padding: var(--space-6);
            background: var(--bg-secondary);
            border-top: 1px solid var(--color-gray-200);
        }
        
        .technical-reproduction h4 {
            color: var(--color-gray-900);
            margin-bottom: var(--space-4);
            font-size: var(--text-lg);
            font-weight: var(--font-weight-semibold);
        }
        
        .curl-commands {
            display: flex;
            flex-direction: column;
            gap: var(--space-4);
        }
        
        .curl-command {
            background: var(--bg-primary);
            border: 1px solid var(--color-gray-200);
            border-radius: var(--radius-md);
            overflow: hidden;
        }
        
        .env-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: var(--space-3) var(--space-4);
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--color-gray-200);
        }
        
        .env-name {
            font-weight: var(--font-weight-semibold);
            color: var(--color-gray-900);
            font-size: var(--text-sm);
            text-transform: uppercase;
            letter-spacing: 0.025em;
        }
        
        .command-box {
            position: relative;
            background: var(--color-gray-900);
            padding: var(--space-4);
            overflow-x: auto;
        }
        
        .command-box code {
            color: var(--color-gray-100);
            font-family: var(--font-family-mono);
            font-size: var(--text-sm);
            line-height: 1.5;
            word-break: break-all;
            display: block;
            padding-right: var(--space-10);
        }
        
        .copy-btn {
            position: absolute;
            top: var(--space-3);
            right: var(--space-3);
            background: var(--color-gray-700);
            border: none;
            color: var(--color-gray-100);
            padding: var(--space-2) var(--space-3);
            border-radius: var(--radius-sm);
            cursor: pointer;
            font-size: var(--text-sm);
            transition: background-color var(--transition-fast);
            display: flex;
            align-items: center;
            gap: var(--space-1);
        }
        
        .copy-btn:hover {
            background: var(--color-gray-600);
        }
        
        .copy-btn:active {
            background: var(--color-primary);
        }
        
        /* Mobile responsiveness for route cards */
        @media (max-width: 768px) {
            .route-diff-header {
                padding: var(--space-4);
            }
            
            .route-name {
                font-size: var(--text-lg);
            }
            
            .route-meta {
                flex-direction: column;
                align-items: flex-start;
                gap: var(--space-3);
            }
            
            .identical-route-content,
            .failed-route-content {
                padding: var(--space-4);
            }
            
            .identical-summary,
            .failure-summary {
                padding: var(--space-4);
                margin-bottom: var(--space-4);
            }
            
            .technical-reproduction {
                padding: var(--space-4);
            }
            
            .command-box {
                padding: var(--space-3);
            }
            
            .copy-btn {
                position: static;
                margin-top: var(--space-3);
                align-self: flex-start;
            }
        }
        
        /* Modern filter controls */
        .filter-controls {
            background: var(--bg-primary);
            border: 1px solid var(--color-gray-200);
            border-radius: var(--radius-lg);
            padding: var(--space-6);
            margin-bottom: var(--space-10);
            box-shadow: var(--shadow-sm);
        }
        
        .filter-controls h3 {
            margin: 0 0 var(--space-6) 0;
            color: var(--color-gray-900);
            font-size: var(--text-xl);
            font-weight: var(--font-weight-semibold);
        }
        
        .help-icon {
            color: var(--color-gray-500);
            cursor: help;
            margin-left: var(--space-2);
            padding: var(--space-1);
            border-radius: var(--radius-sm);
            transition: all var(--transition-fast);
        }
        
        .help-icon:hover {
            background: var(--color-gray-100);
            color: var(--color-primary);
        }
        
        .filter-row {
            display: flex;
            flex-wrap: wrap;
            gap: var(--space-6);
            align-items: center;
            margin-bottom: var(--space-4);
        }
        
        .filter-row:last-child {
            margin-bottom: 0;
        }
        
        .filter-group {
            display: flex;
            align-items: center;
            gap: var(--space-3);
        }
        
        .filter-group label {
            font-weight: var(--font-weight-medium);
            color: var(--color-gray-700);
            font-size: var(--text-sm);
        }
        
        .filter-toggle {
            display: inline-flex;
            background: var(--bg-secondary);
            border-radius: var(--radius-md);
            border: 1px solid var(--color-gray-200);
            overflow: hidden;
            box-shadow: var(--shadow-sm);
        }
        
        .filter-btn {
            padding: var(--space-2) var(--space-4);
            border: none;
            background: transparent;
            color: var(--color-gray-600);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            cursor: pointer;
            transition: all var(--transition-fast);
            display: flex;
            align-items: center;
            gap: var(--space-2);
            position: relative;
        }
        
        .filter-btn:hover {
            background: var(--color-gray-100);
            color: var(--color-gray-900);
        }
        
        .filter-btn.active, .filter-btn--active {
            background: var(--color-primary);
            color: white;
            box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.2);
        }
        
        .filter-search {
            min-width: 250px;
            padding: var(--space-3) var(--space-4);
            border: 1px solid var(--color-gray-300);
            border-radius: var(--radius-md);
            font-size: var(--text-sm);
            transition: all var(--transition-fast);
            background: var(--bg-primary);
        }
        
        .filter-search:focus {
            outline: none;
            border-color: var(--color-primary);
            box-shadow: 0 0 0 3px var(--color-primary-light);
        }
        
        .filter-search::placeholder {
            color: var(--color-gray-500);
        }
        
        .expand-all-btn {
            background: var(--color-gray-600);
            color: white;
            border: none;
            padding: var(--space-3) var(--space-4);
            border-radius: var(--radius-md);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            cursor: pointer;
            transition: all var(--transition-fast);
        }
        
        .expand-all-btn:hover {
            background: var(--color-gray-700);
        }
        
        .clear-filters {
            background: var(--color-gray-500);
            color: white;
            border: none;
            padding: var(--space-3) var(--space-4);
            border-radius: var(--radius-md);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
            cursor: pointer;
            transition: all var(--transition-fast);
        }
        
        .clear-filters:hover {
            background: var(--color-gray-600);
        }
        
        .filter-stats {
            color: var(--color-gray-600);
            font-size: var(--text-sm);
            font-weight: var(--font-weight-medium);
        }
        
        /* Mobile filter controls */
        @media (max-width: 768px) {
            .filter-controls {
                padding: var(--space-4);
                margin-bottom: var(--space-8);
            }
            
            .filter-row {
                flex-direction: column;
                align-items: stretch;
                gap: var(--space-4);
            }
            
            .filter-group {
                flex-direction: column;
                align-items: stretch;
                gap: var(--space-2);
            }
            
            .filter-toggle {
                justify-content: center;
            }
            
            .filter-search {
                min-width: auto;
            }
        }
        
        /* Footer */
        .report-footer {
            margin-top: var(--space-16);
            padding-top: var(--space-8);
            border-top: 1px solid var(--color-gray-200);
            text-align: center;
            color: var(--color-gray-600);
        }
        
        .footer-content {
            display: flex;
            flex-direction: column;
            gap: var(--space-2);
        }
        
        .generated-info {
            font-weight: var(--font-weight-medium);
            font-size: var(--text-sm);
        }
        
        .footer-note {
            font-size: var(--text-sm);
            font-style: italic;
        }
        
        @media (max-width: 768px) {
            .report-footer {
                margin-top: var(--space-12);
                padding-top: var(--space-6);
            }
        }
        
        
        
        @keyframes highlight-flash {
            0% { box-shadow: 0 0 10px var(--color-primary-light); }
            50% { box-shadow: 0 0 20px var(--color-primary-light); }
            100% { box-shadow: none; }
        }
        
        .large-response-alert {
            display: flex;
            align-items: center;
            gap: var(--space-6);
            padding: var(--space-6);
            background: var(--color-primary-light);
            border-radius: var(--radius-lg);
            border: 1px solid var(--color-primary);
            margin-bottom: var(--space-8);
        }
        
        .large-response-icon {
            font-size: var(--text-4xl);
            opacity: 0.8;
        }
        
        .large-response-content h4 {
            margin: 0 0 var(--space-2) 0;
            color: var(--color-primary);
            font-size: var(--text-xl);
            font-weight: var(--font-weight-semibold);
        }
        
        .large-response-content p {
            margin: 0;
            color: var(--color-gray-700);
        }
        
        .large-response-stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: var(--space-4);
            margin-bottom: var(--space-8);
        }
        
        .response-stat {
            padding: var(--space-4);
            background: var(--bg-primary);
            border-radius: var(--radius-md);
            border: 1px solid var(--color-gray-200);
            box-shadow: var(--shadow-sm);
        }
        
        .response-stat.full-width {
            grid-column: 1 / -1;
        }
        
        .response-summary {
            background: var(--bg-secondary);
            border-radius: var(--radius-lg);
            padding: var(--space-6);
            border: 1px solid var(--color-gray-200);
        }
        
        .summary-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: var(--space-3) 0;
            border-bottom: 1px solid var(--color-gray-200);
        }
        
        .summary-item:last-child {
            border-bottom: none;
        }
        
        .summary-label {
            font-weight: var(--font-weight-medium);
            color: var(--color-gray-700);
        }
        
        .summary-value {
            color: var(--color-gray-900);
            font-weight: var(--font-weight-semibold);
        }
        
        /* Error details styling */
        .error-details {
            margin-bottom: var(--space-6);
        }
        
        .error-details h5 {
            color: var(--color-gray-900);
            margin-bottom: var(--space-4);
            font-weight: var(--font-weight-semibold);
        }
        
        .error-detail {
            background: var(--bg-secondary);
            border: 1px solid var(--color-error-border);
            border-radius: var(--radius-md);
            padding: var(--space-4);
            margin-bottom: var(--space-3);
        }
        
        .error-env {
            font-weight: var(--font-weight-semibold);
            color: var(--color-error);
            margin-bottom: var(--space-2);
            text-transform: uppercase;
            font-size: var(--text-sm);
        }
        
        .error-message {
            color: var(--color-gray-700);
            font-family: var(--font-family-mono);
            font-size: var(--text-sm);
            line-height: 1.5;
        }
        
        .troubleshooting {
            background: var(--color-primary-light);
            border-radius: var(--radius-md);
            padding: var(--space-6);
            border: 1px solid var(--color-primary);
        }
        
        .troubleshooting h5 {
            color: var(--color-primary);
            margin-bottom: var(--space-4);
            font-weight: var(--font-weight-semibold);
        }
        
        .troubleshooting ul {
            margin: 0;
            padding-left: var(--space-6);
            color: var(--color-gray-700);
        }
        
        .troubleshooting li {
            margin-bottom: var(--space-2);
            line-height: 1.6;
        }
        
        /* Print styles - Modern and clean */
        @media print {
            :root {
                --bg-primary: white;
                --bg-secondary: white;
                --shadow-sm: none;
                --shadow-md: none;
                --shadow-lg: none;
                --shadow-xl: none;
            }
            
            body { 
                background: white;
                color: black;
            }
            
            .container { 
                box-shadow: none; 
                margin: 0;
                padding: 0;
                border-radius: 0;
                max-width: none;
            }
            
            .copy-btn,
            .filter-controls,
            .json-action-btn,
            .expand-all-btn,
            .clear-filters {
                display: none !important;
            }
            
            .route-diff-section {
                break-inside: avoid;
                margin-bottom: var(--space-4);
            }
        }

        "#
    }

    /// Embedded JavaScript for interactive functionality
    fn embedded_javascript() -> &'static str {
        r#"
        <script>
        // Modern HTTP Diff Report Controller with Three-Tier Navigation
        class ModernReportController {
            constructor() {
                this.currentSection = 'dashboard';
                this.routes = [];
                this.routeStats = { all: 0, different: 0, failed: 0, identical: 0 };
                this.filters = {
                    status: 'all',
                    search: ''
                };
                this.init();
            }

            init() {
                this.setupNavigation();
                this.collectRoutes();
                this.updateRouteCounts();
                this.setupFilters();
                this.setupRouteInteractions();
                this.setupKeyboardShortcuts();
                this.setupAccessibility();
                this.loadStateFromUrl();
            }

            // Navigation Management
            setupNavigation() {
                const navButtons = document.querySelectorAll('.nav-btn');
                navButtons.forEach(btn => {
                    btn.addEventListener('click', (e) => {
                        e.preventDefault();
                        const targetSection = btn.dataset.section;
                        this.navigateToSection(targetSection);
                    });
                });

                // Breadcrumb navigation
                const breadcrumbLinks = document.querySelectorAll('.breadcrumb-link');
                breadcrumbLinks.forEach(link => {
                    link.addEventListener('click', (e) => {
                        e.preventDefault();
                        const targetSection = link.dataset.section;
                        this.navigateToSection(targetSection);
                    });
                });
            }

            navigateToSection(sectionId) {
                // Update current section
                this.currentSection = sectionId;
                
                // Hide all sections
                document.querySelectorAll('.report-section').forEach(section => {
                    section.hidden = true;
                    section.classList.remove('report-section--active');
                });
                
                // Show target section
                const targetSection = document.getElementById(sectionId);
                if (targetSection) {
                    targetSection.hidden = false;
                    targetSection.classList.add('report-section--active');
                    targetSection.scrollIntoView({ behavior: 'smooth', block: 'start' });
                }
                
                // Update navigation buttons
                document.querySelectorAll('.nav-btn').forEach(btn => {
                    const isActive = btn.dataset.section === sectionId;
                    btn.classList.toggle('nav-btn--active', isActive);
                    btn.setAttribute('aria-pressed', isActive);
                });
                
                // Update URL fragment
                this.updateUrlFragment();
                
                // Trigger section-specific initialization
                this.initializeSection(sectionId);
            }

            initializeSection(sectionId) {
                switch(sectionId) {
                    case 'technical':
                        // Ensure filters are ready
                        this.applyFilters();
                        break;
                    case 'dashboard':
                        // Could trigger dashboard animations
                        break;
                    case 'actions':
                        // Could highlight critical actions
                        break;
                }
            }

            // Route Management
            collectRoutes() {
                const routeSections = document.querySelectorAll('.route-diff-section');
                
                this.routes = Array.from(routeSections).map(section => {
                    const routeNameElement = section.querySelector('.route-name');
                    const routeName = routeNameElement ? 
                        routeNameElement.textContent.replace(/📍|🔗|⚡/, '').trim() : 'Unknown';
                    
                    const status = section.dataset.status || 'unknown';
                    const hasDiff = section.querySelector('.json-diff-container') !== null;
                    
                    return {
                        element: section,
                        name: routeName,
                        status: status,
                        hasDiff,
                        visible: true
                    };
                });
                
            }

            updateRouteCounts() {
                // Reset counts
                this.routeStats = { all: 0, different: 0, failed: 0, identical: 0 };
                
                // Count routes by status
                this.routes.forEach(route => {
                    this.routeStats.all++;
                    if (this.routeStats[route.status] !== undefined) {
                        this.routeStats[route.status]++;
                    }
                });
                
                // Update badge counts in filter buttons
                Object.keys(this.routeStats).forEach(status => {
                    const badge = document.getElementById(`${status}-count`);
                    if (badge) {
                        badge.textContent = this.routeStats[status];
                        badge.setAttribute('aria-label', `${this.routeStats[status]} ${status} routes`);
                    }
                });
                
            }

            // Enhanced Filtering System
            setupFilters() {
                // Status filter buttons with enhanced UX
                const statusButtons = document.querySelectorAll('.status-filter-btn');
                statusButtons.forEach(btn => {
                    btn.addEventListener('click', (e) => {
                        e.preventDefault();
                        const status = btn.dataset.status;
                        this.setStatusFilter(status);
                    });
                });

                // Enhanced search with debouncing
                const searchInput = document.getElementById('route-search');
                if (searchInput) {
                    let searchTimeout;
                    searchInput.addEventListener('input', (e) => {
                        clearTimeout(searchTimeout);
                        searchTimeout = setTimeout(() => {
                            this.setSearchFilter(e.target.value);
                        }, 300);
                    });
                    
                    // Clear search on escape
                    searchInput.addEventListener('keydown', (e) => {
                        if (e.key === 'Escape') {
                            e.target.value = '';
                            this.setSearchFilter('');
                        }
                    });
                }

                // Clear filters with enhanced UX
                const clearBtn = document.querySelector('.clear-filters');
                if (clearBtn) {
                    clearBtn.addEventListener('click', (e) => {
                        e.preventDefault();
                        this.clearAllFilters();
                        
                        // Visual feedback
                        clearBtn.style.background = '#22c55e';
                        setTimeout(() => clearBtn.style.background = '', 300);
                    });
                }

                // Expand/collapse all with smart toggling
                const expandBtn = document.getElementById('expand-toggle');
                if (expandBtn) {
                    expandBtn.addEventListener('click', (e) => {
                        e.preventDefault();
                        const isExpanded = expandBtn.getAttribute('aria-pressed') === 'true';
                        this.toggleAllRoutes(!isExpanded);
                        expandBtn.setAttribute('aria-pressed', !isExpanded);
                        expandBtn.textContent = !isExpanded ? 'Collapse All' : 'Expand All';
                    });
                }
            }

            setStatusFilter(status) {
                this.filters.status = status;
                this.updateFilterButtons();
                this.applyFilters();
                this.updateUrlParams();
            }

            setSearchFilter(search) {
                this.filters.search = search.toLowerCase().trim();
                this.applyFilters();
                this.updateUrlParams();
            }

            clearAllFilters() {
                this.filters = { status: 'all', search: '' };
                
                // Reset UI elements
                const searchInput = document.getElementById('route-search');
                if (searchInput) searchInput.value = '';
                
                this.updateFilterButtons();
                this.applyFilters();
                this.updateUrlParams();
            }

            updateFilterButtons() {
                const buttons = document.querySelectorAll('.status-filter-btn');
                buttons.forEach(btn => {
                    const isActive = btn.dataset.status === this.filters.status;
                    btn.classList.toggle('filter-btn--active', isActive);
                    btn.setAttribute('aria-pressed', isActive);
                });
            }

            applyFilters() {
                let visibleCount = 0;
                const hasFilters = this.filters.status !== 'all' || this.filters.search;
                
                this.routes.forEach(route => {
                    let visible = true;

                    // Status filter
                    if (this.filters.status !== 'all' && route.status !== this.filters.status) {
                        visible = false;
                    }

                    // Search filter
                    if (this.filters.search && !route.name.toLowerCase().includes(this.filters.search)) {
                        visible = false;
                    }

                    // Apply visibility
                    route.visible = visible;
                    if (visible) {
                        route.element.style.display = 'block';
                        route.element.removeAttribute('aria-hidden');
                        visibleCount++;
                    } else {
                        route.element.style.display = 'none';
                        route.element.setAttribute('aria-hidden', 'true');
                    }
                });

                this.updateFilterStatus(visibleCount, hasFilters);
            }

            updateFilterStatus(visibleCount, hasFilters) {
                const statusElement = document.querySelector('.filter-stats');
                if (statusElement) {
                    const statusText = hasFilters ? 
                        `Showing ${visibleCount} of ${this.routes.length} routes` :
                        `Showing all ${this.routes.length} routes`;
                    statusElement.textContent = statusText;
                }
            }

            // Route Interactions
            setupRouteInteractions() {
                // Enhanced route header clicking
                const routeHeaders = document.querySelectorAll('.route-diff-header');
                routeHeaders.forEach(header => {
                    header.addEventListener('click', (e) => {
                        // Skip if clicking on action buttons
                        if (e.target.closest('.json-action-btn, .copy-btn')) return;
                        
                        const section = header.closest('.route-diff-section');
                        const isCollapsed = section.classList.contains('collapsed');
                        section.classList.toggle('collapsed');
                        const icon = section.querySelector('.route-expand-icon');
                        if (icon) {
                            icon.textContent = section.classList.contains('collapsed') ? '▶' : '▼';
                        }
                        
                        // Scroll into view if expanding
                        if (isCollapsed) {
                            setTimeout(() => {
                                section.scrollIntoView({ 
                                    behavior: 'smooth', 
                                    block: 'nearest' 
                                });
                            }, 100);
                        }
                    });
                    
                    // Add hover effect
                    header.addEventListener('mouseenter', () => {
                        header.style.backgroundColor = 'var(--color-gray-100)';
                    });
                    
                    header.addEventListener('mouseleave', () => {
                        header.style.backgroundColor = '';
                    });
                });
            }

            toggleAllRoutes(expand) {
                const sections = document.querySelectorAll('.route-diff-section');
                sections.forEach((section) => {
                    if (expand) {
                        section.classList.remove('collapsed');
                    } else {
                        section.classList.add('collapsed');
                    }
                    const icon = section.querySelector('.route-expand-icon');
                    if (icon) {
                        icon.textContent = expand ? '▼' : '▶';
                    }
                });
                
            }

            // Keyboard Shortcuts
            setupKeyboardShortcuts() {
                document.addEventListener('keydown', (e) => {
                    // Skip if in input field
                    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

                    switch(e.key) {
                        case '/':
                            e.preventDefault();
                            if (this.currentSection === 'technical') {
                                const searchInput = document.getElementById('route-search');
                                if (searchInput) {
                                    searchInput.focus();
                                    searchInput.select();
                                }
                            }
                            break;
                        case '1':
                            e.preventDefault();
                            if (this.currentSection === 'technical') this.setStatusFilter('all');
                            break;
                        case '2':
                            e.preventDefault();
                            if (this.currentSection === 'technical') this.setStatusFilter('different');
                            break;
                        case '3':
                            e.preventDefault();
                            if (this.currentSection === 'technical') this.setStatusFilter('failed');
                            break;
                        case '4':
                            e.preventDefault();
                            if (this.currentSection === 'technical') this.setStatusFilter('identical');
                            break;
                        case 'e':
                            e.preventDefault();
                            if (this.currentSection === 'technical') this.toggleAllRoutes(true);
                            break;
                        case 'c':
                            e.preventDefault();
                            if (this.currentSection === 'technical') this.toggleAllRoutes(false);
                            break;
                        case 'r':
                            e.preventDefault();
                            if (this.currentSection === 'technical') this.clearAllFilters();
                            break;
                        case 'ArrowLeft':
                            e.preventDefault();
                            this.navigateSections(-1);
                            break;
                        case 'ArrowRight':
                            e.preventDefault();
                            this.navigateSections(1);
                            break;
                    }
                });
            }

            navigateSections(direction) {
                const sections = ['dashboard', 'technical', 'actions'];
                const currentIndex = sections.indexOf(this.currentSection);
                const newIndex = Math.max(0, Math.min(sections.length - 1, currentIndex + direction));
                if (newIndex !== currentIndex) {
                    this.navigateToSection(sections[newIndex]);
                }
            }

            // Accessibility Enhancements
            setupAccessibility() {
                // Help modal for keyboard shortcuts
                const helpIcon = document.querySelector('.help-icon');
                if (helpIcon) {
                    helpIcon.addEventListener('click', () => {
                        this.showKeyboardHelp();
                    });
                }
                
                // Focus management
                document.addEventListener('focusin', (e) => {
                    // Add focus ring for keyboard users
                    if (e.target.matches('.nav-btn, .filter-btn, .route-diff-header')) {
                        e.target.style.outline = '2px solid var(--color-primary)';
                        e.target.style.outlineOffset = '2px';
                    }
                });
                
                document.addEventListener('focusout', (e) => {
                    e.target.style.outline = '';
                    e.target.style.outlineOffset = '';
                });
            }

            showKeyboardHelp() {
                const helpText = `
🚀 HTTP Diff Report - Keyboard Shortcuts

Navigation:
← → Arrow keys - Navigate between sections
1-4 - Quick filter (All, Different, Failed, Identical)

Technical View:
/ - Focus search box
E - Expand all routes  
C - Collapse all routes
R - Reset all filters

General:
Esc - Close dialogs
Tab - Navigate elements
Enter/Space - Activate buttons

💡 Pro tip: Use arrow keys to quickly switch between Overview, Technical Details, and Recommended Actions!
                `.trim();
                
                alert(helpText);
            }

            // URL State Management
            loadStateFromUrl() {
                const hash = window.location.hash.substring(1);
                const params = new URLSearchParams(window.location.search);
                
                // Load section from hash
                if (hash && ['dashboard', 'technical', 'actions'].includes(hash)) {
                    this.navigateToSection(hash);
                }
                
                // Load filters from params
                if (params.has('status')) {
                    this.setStatusFilter(params.get('status'));
                }
                
                if (params.has('search')) {
                    const search = params.get('search');
                    this.filters.search = search;
                    const searchInput = document.getElementById('route-search');
                    if (searchInput) searchInput.value = search;
                }
            }

            updateUrlFragment() {
                const newHash = `#${this.currentSection}`;
                if (window.location.hash !== newHash) {
                    window.history.replaceState(null, null, newHash);
                }
            }

            updateUrlParams() {
                const params = new URLSearchParams();
                
                if (this.filters.status !== 'all') {
                    params.set('status', this.filters.status);
                }
                
                if (this.filters.search) {
                    params.set('search', this.filters.search);
                }

                const search = params.toString();
                const newUrl = window.location.pathname + 
                             (search ? '?' + search : '') + 
                             window.location.hash;
                
                window.history.replaceState(null, null, newUrl);
            }
        }

        // Utility Functions
        function copyToClipboard(button) {
            const code = button.closest('.command-box').querySelector('code');
            if (!code) return;
            
            navigator.clipboard.writeText(code.textContent).then(() => {
                const originalText = button.innerHTML;
                button.innerHTML = 'Copied';
                button.style.background = 'var(--color-success)';
                
                setTimeout(() => {
                    button.innerHTML = originalText;
                    button.style.background = '';
                }, 2000);
            }).catch(err => {
                console.error('Failed to copy:', err);
                button.innerHTML = 'Failed';
                setTimeout(() => button.innerHTML = 'Copy', 2000);
            });
        }

        // Initialize when DOM is ready
        document.addEventListener('DOMContentLoaded', () => {
            // Initialize modern report controller
            window.reportController = new ModernReportController();
            
        });
        </script>
        "#
    }
}

impl Default for HtmlTemplate {
    fn default() -> Self {
        Self::new()
    }
}
