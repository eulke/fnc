use crate::{
    renderers::tui::{
        app::{FeedbackType, FocusedPanel, PanelFocus, TuiApp},
        theme::{KeyHints, TuiTheme, UiSymbols},
    },
    types::ComparisonResult,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Modifier, Style},
    widgets::{
        BarChart, Block, Borders, Gauge, List, ListItem, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, Tabs,
    },
};

/// Main UI drawing function
pub fn draw(f: &mut Frame, app: &mut TuiApp) {
    let size = f.area();

    // Main layout: title bar + content + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // Status/help bar
        ])
        .split(size);

    // Draw title bar
    draw_title_bar(f, app, chunks[0]);

    // Draw main content - only Dashboard mode is supported
    draw_dashboard_view(f, app, chunks[1]);

    // Draw status bar
    draw_status_bar(f, app, chunks[2]);

    // Draw help overlay if requested
    if app.show_help {
        draw_help_overlay(f, app);
    }

    // Draw action feedback if present
    if let Some(ref feedback) = app.action_feedback {
        draw_feedback_popup(f, feedback);
    }
}

/// Draw the title bar with app info and navigation
fn draw_title_bar(f: &mut Frame, app: &TuiApp, area: Rect) {
    let (total, identical, different, errors) = app.get_filter_counts();

    let title_text = format!(
        "{} HTTP Diff TUI - {} | {} Total: {} | {} Identical: {} | {} Different: {} | {} Errors: {}",
        UiSymbols::QUICK_ACTION,
        app.get_view_title(),
        UiSymbols::LIST,
        total,
        UiSymbols::SUCCESS,
        identical,
        UiSymbols::ERROR,
        different,
        UiSymbols::WARNING,
        errors
    );

    let title = Paragraph::new(title_text)
        .style(TuiTheme::primary_text_style().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(TuiTheme::focused_block("HTTP Diff TUI"));

    f.render_widget(title, area);
}







/// Draw the status/help bar
fn draw_status_bar(f: &mut Frame, app: &TuiApp, area: Rect) {
    // Dynamic help based on currently focused panel
    let panel_help = match app.panel_focus {
        PanelFocus::Configuration => KeyHints::configuration_panel_help(),
        PanelFocus::Progress => KeyHints::progress_panel_help(),
        PanelFocus::Results => KeyHints::results_panel_help(),
        PanelFocus::Details => KeyHints::details_panel_help(),
    };
    let key_hints = KeyHints::format_key_hints(&panel_help);

    let status_content = format!("{} Quick Help | {}", UiSymbols::HELP, key_hints);

    let help_title = format!("{} Quick Help", UiSymbols::HELP);
    let status = Paragraph::new(status_content)
        .style(TuiTheme::secondary_text_style())
        .alignment(Alignment::Center)
        .block(TuiTheme::normal_block(&help_title))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(status, area);
}





/// Draw help overlay
fn draw_help_overlay(f: &mut Frame, app: &TuiApp) {
    let area = centered_rect(80, 60, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    // Dynamic help based on currently focused panel
    let (panel_name, panel_help) = match app.panel_focus {
        PanelFocus::Configuration => {
            ("Configuration Panel", KeyHints::configuration_panel_help())
        }
        PanelFocus::Progress => ("Progress Panel", KeyHints::progress_panel_help()),
        PanelFocus::Results => ("Results Panel", KeyHints::results_panel_help()),
        PanelFocus::Details => ("Details Panel", KeyHints::details_panel_help()),
    };
    let help_text = format!(
        "{} Dashboard Help - {}\n\n{}\n\n{} Navigation Tips:\n• Use Tab to switch between panels\n• Each panel has specific shortcuts\n• Press F1 to close this help",
        UiSymbols::HELP,
        panel_name,
        KeyHints::format_key_hints(&panel_help),
        UiSymbols::TIP
    );

    let help_popup = Paragraph::new(help_text)
        .style(TuiTheme::primary_text_style())
        .block(TuiTheme::focused_block("Help").style(TuiTheme::info_style()))
        .wrap(ratatui::widgets::Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(help_popup, area);
}

/// Draw feedback popup
fn draw_feedback_popup(f: &mut Frame, feedback: &crate::renderers::tui::app::ActionFeedback) {
    let area = centered_rect(50, 10, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let (style, symbol) = match feedback.feedback_type {
        FeedbackType::Success => (TuiTheme::success_style(), UiSymbols::SUCCESS),
        FeedbackType::Warning => (TuiTheme::warning_style(), UiSymbols::WARNING),
        FeedbackType::Error => (TuiTheme::error_style(), UiSymbols::ERROR),
        FeedbackType::Info => (TuiTheme::info_style(), UiSymbols::INFO),
    };

    let feedback_text = format!("{} {}", symbol, feedback.message);
    let feedback_popup = Paragraph::new(feedback_text)
        .style(style)
        .block(Block::default().borders(Borders::ALL).border_style(style))
        .alignment(Alignment::Center);

    f.render_widget(feedback_popup, area);
}


/// Draw the dashboard view with 4 simultaneous panels
fn draw_dashboard_view(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    // Check if any panel is expanded
    let expanded_panel = app
        .panel_sizes
        .iter()
        .find(|(_, size)| **size == crate::renderers::tui::app::PanelSize::Expanded)
        .map(|(panel, _)| panel);

    if let Some(expanded_panel) = expanded_panel {
        // Show only the expanded panel
        match expanded_panel {
            PanelFocus::Configuration => draw_dashboard_configuration_panel(f, app, area),
            PanelFocus::Progress => draw_dashboard_progress_panel(f, app, area),
            PanelFocus::Results => draw_dashboard_results_panel(f, app, area),
            PanelFocus::Details => draw_dashboard_details_panel(f, app, area),
        }
    } else {
        // Normal 2x2 grid layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Top row (Configuration + Progress)
                Constraint::Percentage(50), // Bottom row (Results + Details)
            ])
            .split(area);

        // Top row: Configuration (left) + Progress (right)
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Configuration panel
                Constraint::Percentage(50), // Progress panel
            ])
            .split(main_chunks[0]);

        // Bottom row: Results (left) + Details (right)
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Results panel
                Constraint::Percentage(50), // Details panel
            ])
            .split(main_chunks[1]);

        // Draw each panel with focus indicators
        draw_dashboard_configuration_panel(f, app, top_chunks[0]);
        draw_dashboard_progress_panel(f, app, top_chunks[1]);
        draw_dashboard_results_panel(f, app, bottom_chunks[0]);
        draw_dashboard_details_panel(f, app, bottom_chunks[1]);
    }
}

/// Draw the configuration panel in dashboard mode with List widgets
fn draw_dashboard_configuration_panel(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let is_focused = app.is_panel_focused(&PanelFocus::Configuration);
    let title = app.get_panel_title(&PanelFocus::Configuration);
    let has_content = !app.available_environments.is_empty() && !app.available_routes.is_empty();
    let has_activity = !app.selected_environments.is_empty() && !app.selected_routes.is_empty();

    let block = TuiTheme::panel_block(&title, is_focused, has_content, has_activity);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.available_environments.is_empty() {
        let loading = Paragraph::new("📁 Loading configuration...\nPress Enter to load")
            .style(TuiTheme::secondary_text_style())
            .alignment(Alignment::Center);
        f.render_widget(loading, inner_area);
    } else {
        // Horizontal side-by-side layout with status line at bottom
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(95), // Main content area
                Constraint::Percentage(5),  // Status line at bottom
            ])
            .split(inner_area);

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Environments - left side
                Constraint::Percentage(50), // Routes - right side
            ])
            .split(vertical_chunks[0]);

        draw_environments_list_widget(f, app, horizontal_chunks[0], is_focused);
        draw_routes_list_widget(f, app, horizontal_chunks[1], is_focused);
        draw_config_status_line(f, app, vertical_chunks[1]);
    }
}

/// Draw environments as a proper List widget with enhanced visual feedback
fn draw_environments_list_widget(
    f: &mut Frame,
    app: &mut TuiApp,
    area: Rect,
    is_panel_focused: bool,
) {
    let env_items: Vec<ListItem> = app
        .available_environments
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let is_selected = app.is_environment_selected(i);

            let checkbox = if is_selected { "☑" } else { "☐" };
            let text = format!("{} {}", checkbox, env);

            // Different styling for selected items (green checkboxes) vs normal items
            let style = if is_selected {
                Style::default()
                    .fg(TuiTheme::SUCCESS)
                    .add_modifier(Modifier::BOLD)
            } else {
                TuiTheme::primary_text_style()
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let selected_count = app.selected_environments.len();
    let total_count = app.available_environments.len();

    // Determine if this section is currently focused
    let is_env_focused =
        is_panel_focused && matches!(app.focused_panel, FocusedPanel::Environments);

    // Create title string that will live long enough
    let title_text = format!("Environments ({}/{})", selected_count, total_count);

    // Create block with focus-dependent styling
    let block = if is_env_focused {
        TuiTheme::focused_block(&title_text)
    } else {
        TuiTheme::normal_block(&title_text)
    };

    let env_list = List::new(env_items).block(block).highlight_style(
        Style::default()
            .bg(TuiTheme::BACKGROUND_SELECTED)
            .fg(TuiTheme::FOCUS)
            .add_modifier(Modifier::BOLD),
    );

    // Use stateful rendering for proper cursor positioning
    f.render_stateful_widget(env_list, area, &mut app.env_list_state);

    // Draw scrollbar if content exceeds viewport
    let viewport_height = area.height.saturating_sub(2) as usize; // Account for borders
    draw_scrollbar(
        f,
        &mut app.env_scrollbar_state,
        area,
        app.available_environments.len(),
        viewport_height,
    );
}

/// Draw routes as a proper List widget with enhanced visual feedback
fn draw_routes_list_widget(f: &mut Frame, app: &mut TuiApp, area: Rect, is_panel_focused: bool) {
    let route_items: Vec<ListItem> = app
        .available_routes
        .iter()
        .enumerate()
        .map(|(i, route)| {
            let is_selected = app.is_route_selected(i);

            let checkbox = if is_selected { "☑" } else { "☐" };
            let text = format!("{} {}", checkbox, route);

            // Different styling for selected items (green checkboxes) vs normal items
            let style = if is_selected {
                Style::default()
                    .fg(TuiTheme::SUCCESS)
                    .add_modifier(Modifier::BOLD)
            } else {
                TuiTheme::primary_text_style()
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let selected_count = app.selected_routes.len();
    let total_count = app.available_routes.len();

    // Determine if this section is currently focused
    let is_route_focused = is_panel_focused && matches!(app.focused_panel, FocusedPanel::Routes);

    // Create title string that will live long enough
    let title_text = format!("Routes ({}/{})", selected_count, total_count);

    // Create block with focus-dependent styling
    let block = if is_route_focused {
        TuiTheme::focused_block(&title_text)
    } else {
        TuiTheme::normal_block(&title_text)
    };

    let route_list = List::new(route_items).block(block).highlight_style(
        Style::default()
            .bg(TuiTheme::BACKGROUND_SELECTED)
            .fg(TuiTheme::FOCUS)
            .add_modifier(Modifier::BOLD),
    );

    // Use stateful rendering for proper cursor positioning
    f.render_stateful_widget(route_list, area, &mut app.route_list_state);

    // Draw scrollbar if content exceeds viewport
    let viewport_height = area.height.saturating_sub(2) as usize; // Account for borders
    draw_scrollbar(
        f,
        &mut app.route_scrollbar_state,
        area,
        app.available_routes.len(),
        viewport_height,
    );
}

/// Draw configuration status line with current state information
fn draw_config_status_line(f: &mut Frame, app: &TuiApp, area: Rect) {
    // Determine current focused section
    let current_section = match app.focused_panel {
        FocusedPanel::Environments => "📝 Environments",
        FocusedPanel::Routes => "🛣 Routes",
        FocusedPanel::Actions => "⚡ Actions",
    };

    // Show clean state information without duplicate navigation help
    let text = if app.selected_environments.is_empty() || app.selected_routes.is_empty() {
        format!("{} | ⚠ Select items to continue", current_section)
    } else {
        let total_tests = app.selected_environments.len() * app.selected_routes.len();
        format!("{} | ✅ {} tests ready", current_section, total_tests)
    };

    let style = if app.selected_environments.is_empty() || app.selected_routes.is_empty() {
        TuiTheme::warning_style()
    } else {
        TuiTheme::success_style()
    };

    let instruction = Paragraph::new(text)
        .style(style)
        .alignment(Alignment::Center);
    f.render_widget(instruction, area);
}

/// Draw the progress panel in dashboard mode with rich widgets
fn draw_dashboard_progress_panel(f: &mut Frame, app: &TuiApp, area: Rect) {
    let is_focused = app.is_panel_focused(&PanelFocus::Progress);
    let title = app.get_panel_title(&PanelFocus::Progress);
    let has_content = !app.results.is_empty() || app.total_tests > 0;
    let has_activity = app.execution_running;

    let block = TuiTheme::panel_block(&title, is_focused, has_content, has_activity);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.execution_running {
        // Active execution - show progress with widgets
        draw_progress_execution_view(f, app, inner_area);
    } else if !app.results.is_empty() {
        // Completed execution - show results summary with charts
        draw_progress_results_summary(f, app, inner_area);
    } else {
        // Ready state - show instructions
        draw_progress_ready_state(f, inner_area);
    }
}

/// Draw active execution progress with Gauge widget
fn draw_progress_execution_view(f: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress gauge
            Constraint::Length(3), // Statistics line
            Constraint::Min(1),    // Current operation
        ])
        .split(area);

    // Main progress gauge
    let progress_value = if app.total_tests > 0 {
        ((app.completed_tests as f64 / app.total_tests as f64) * 100.0).clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let progress_gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(TuiTheme::FOCUS))
        .percent(progress_value)
        .label(format!(
            "{}/{} tests ({progress_value}%)",
            app.completed_tests, app.total_tests
        ));

    f.render_widget(progress_gauge, chunks[0]);

    // Execution statistics
    let elapsed = if let Some(start_time) = app.execution_start_time {
        format!("{:.1}s", start_time.elapsed().as_secs_f64())
    } else {
        "0.0s".to_string()
    };

    let rate = if app.completed_tests > 0 {
        if let Some(start_time) = app.execution_start_time {
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            if elapsed_secs > 0.0 {
                format!("{:.1} tests/s", app.completed_tests as f64 / elapsed_secs)
            } else {
                "calculating...".to_string()
            }
        } else {
            "calculating...".to_string()
        }
    } else {
        "starting...".to_string()
    };

    let stats_text = format!("⏱ {elapsed} | 🚀 {rate}");
    let stats_para = Paragraph::new(stats_text)
        .style(TuiTheme::secondary_text_style())
        .alignment(Alignment::Center);
    f.render_widget(stats_para, chunks[1]);

    // Current operation
    let operation_para = Paragraph::new(app.current_operation.as_str())
        .style(TuiTheme::primary_text_style())
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(operation_para, chunks[2]);
}

/// Draw results summary with BarChart widget
fn draw_progress_results_summary(f: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status line
            Constraint::Min(5),    // Bar chart
            Constraint::Length(3), // Error summary
        ])
        .split(area);

    // Status line
    let status_text = "✅ Execution Complete";
    let status_para = Paragraph::new(status_text)
        .style(TuiTheme::success_style())
        .alignment(Alignment::Center);
    f.render_widget(status_para, chunks[0]);

    // Results bar chart
    let (total, identical, different, errors) = app.get_filter_counts();

    let chart_data = [
        ("✅ OK", identical as u64),
        ("⚠ Diff", different as u64),
        ("❌ Err", errors as u64),
    ];

    let results_chart = BarChart::default()
        .block(Block::default().title("Test Results").borders(Borders::ALL))
        .data(&chart_data)
        .bar_width(5)
        .bar_gap(2)
        .bar_style(Style::default().fg(TuiTheme::SUCCESS))
        .value_style(
            Style::default()
                .fg(TuiTheme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(results_chart, chunks[1]);

    // Error summary if any
    if errors > 0 {
        let error_text = format!("⚠ {} errors detected - check Details panel", errors);
        let error_para = Paragraph::new(error_text)
            .style(TuiTheme::error_style())
            .alignment(Alignment::Center);
        f.render_widget(error_para, chunks[2]);
    } else {
        let success_text = format!("🎉 {} tests completed successfully", total);
        let success_para = Paragraph::new(success_text)
            .style(TuiTheme::success_style())
            .alignment(Alignment::Center);
        f.render_widget(success_para, chunks[2]);
    }
}

/// Draw ready state instructions
fn draw_progress_ready_state(f: &mut Frame, area: Rect) {
    let ready_text =
        "🚀 Ready to Execute\n\n1. Select environments\n2. Select routes\n3. Press 'R' to start";
    let ready_para = Paragraph::new(ready_text)
        .style(TuiTheme::secondary_text_style())
        .alignment(Alignment::Center);
    f.render_widget(ready_para, area);
}

/// Draw the results panel in dashboard mode
fn draw_dashboard_results_panel(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let is_focused = app.is_panel_focused(&PanelFocus::Results);
    let title = app.get_panel_title(&PanelFocus::Results);
    let has_content = !app.results.is_empty();
    let has_activity = app.filter_state.show_filter_panel
        || app.filter_state.status_filter != crate::renderers::tui::app::StatusFilter::All;

    let block = TuiTheme::panel_block(&title, is_focused, has_content, has_activity);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.results.is_empty() {
        let empty_text =
            "No results yet\n\nRun tests from the\nConfiguration panel\nto see results here";
        let empty_para = Paragraph::new(empty_text)
            .style(TuiTheme::secondary_text_style())
            .alignment(Alignment::Center);
        f.render_widget(empty_para, inner_area);
    } else {
        // Show compact results table
        draw_compact_results_table(f, app, inner_area, is_focused);
    }
}

/// Draw the details panel in dashboard mode with tabs
fn draw_dashboard_details_panel(f: &mut Frame, app: &TuiApp, area: Rect) {
    let is_focused = app.is_panel_focused(&PanelFocus::Details);
    let title = app.get_panel_title(&PanelFocus::Details);
    let has_content = app.current_filtered_result().is_some();
    let has_activity = has_content && app.current_result_has_differences();

    let block = TuiTheme::panel_block(&title, is_focused, has_content, has_activity);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if let Some(result) = app.current_filtered_result() {
        draw_detailed_result_with_tabs(f, app, result, inner_area, is_focused);
    } else {
        let empty_text =
            "📋 No result selected\n\nNavigate in Results panel\nto see detailed information";
        let empty_para = Paragraph::new(empty_text)
            .style(TuiTheme::secondary_text_style())
            .alignment(Alignment::Center);
        f.render_widget(empty_para, inner_area);
    }
}

/// Draw detailed result information with tabbed interface
fn draw_detailed_result_with_tabs(
    f: &mut Frame,
    app: &TuiApp,
    result: &ComparisonResult,
    area: Rect,
    is_focused: bool,
) {
    // Split area for tabs and content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(1),    // Tab content
        ])
        .split(area);

    // Simple tab titles
    let tab_titles = vec!["📋 Overview", "🔍 Diffs", "⚠ Errors", "💡 Tips"];

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL))
        .select(app.details_current_tab.as_index())
        .style(TuiTheme::secondary_text_style())
        .highlight_style(if is_focused {
            TuiTheme::focused_style()
        } else {
            Style::default()
                .fg(TuiTheme::FOCUS)
                .add_modifier(Modifier::BOLD)
        });

    f.render_widget(tabs, chunks[0]);

    // Render tab content
    match app.details_current_tab {
        crate::renderers::tui::app::DetailsTab::Overview => {
            draw_details_overview_tab(f, app, result, chunks[1]);
        }
        crate::renderers::tui::app::DetailsTab::Diffs => {
            draw_details_diffs_tab(f, app, result, chunks[1]);
        }
        crate::renderers::tui::app::DetailsTab::Errors => {
            draw_details_errors_tab(f, app, result, chunks[1]);
        }
        crate::renderers::tui::app::DetailsTab::Suggestions => {
            draw_details_suggestions_tab(f, app, result, chunks[1]);
        }
    }
}

/// Draw overview tab content
fn draw_details_overview_tab(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect) {
    let mut lines = vec![
        format!("🛣 Route: {}", result.route_name),
        "".to_string(),
        format!(
            "📊 Status: {}",
            if result.has_errors {
                "❌ Errors detected"
            } else if result.is_identical {
                "✅ All responses identical"
            } else {
                "⚠ Responses differ"
            }
        ),
        "".to_string(),
    ];

    // Environment status
    lines.push("🌍 Environments:".to_string());
    for (env, response) in &result.responses {
        let status_icon = if response.is_success() { "✅" } else { "❌" };
        lines.push(format!(
            "  {} {} - HTTP {}",
            status_icon, env, response.status
        ));
    }

    // Variables section
    if !result.user_context.is_empty() {
        lines.push("".to_string());
        lines.push("Variables".to_string());
        lines.push("══════════".to_string());
        let mut vars: Vec<_> = result.user_context.iter().collect();
        vars.sort_by_key(|(k, _)| *k);
        for (k, v) in vars {
            let val = if v.len() > 80 {
                format!("{}...", &v[..77])
            } else {
                v.clone()
            };
            lines.push(format!("  {} = {}", k, val));
        }
    }

    // Response size info
    if !result.responses.is_empty() {
        lines.push("".to_string());
        lines.push("📏 Response Sizes:".to_string());
        for (env, response) in &result.responses {
            lines.push(format!(
                "  {} - {} bytes, {} lines",
                env,
                response.body.len(),
                response.line_count()
            ));
        }
    }

    // Difference summary
    if !result.is_identical && !result.has_errors {
        lines.push("".to_string());
        lines.push(format!("🔍 {} differences found", result.differences.len()));

        let mut categories = std::collections::HashSet::new();
        for diff in &result.differences {
            categories.insert(&diff.category);
        }

        for category in categories {
            lines.push(format!("  • {}", category.name()));
        }
    }

    let overview_text = lines.join("\n");
    let overview_para = Paragraph::new(overview_text)
        .style(TuiTheme::primary_text_style())
        .scroll((app.scroll_offset as u16, 0))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(overview_para, area);
}

/// Draw diffs tab content
fn draw_details_diffs_tab(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect) {
    if result.differences.is_empty() {
        let no_diffs =
            "✅ No differences found\n\nAll responses are identical\nacross environments.";
        let para = Paragraph::new(no_diffs)
            .style(TuiTheme::success_style())
            .alignment(Alignment::Center);
        f.render_widget(para, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Style indicator
            Constraint::Min(1),    // Diff content
        ])
        .split(area);

    // Diff style indicator
    let style_text = format!(
        "📝 {} View (Press D to toggle)",
        match app.details_diff_style {
            crate::types::DiffViewStyle::Unified => "Unified",
            crate::types::DiffViewStyle::SideBySide => "Side-by-Side",
        }
    );
    let style_para = Paragraph::new(style_text).style(TuiTheme::info_style());
    f.render_widget(style_para, chunks[0]);

    // Use proper diff processing pipeline for rich visual elements
    use crate::renderers::{diff_processor::DiffProcessor, tui::diff_widgets::DiffWidgetRenderer};

    // Process the comparison result into generic diff data
    let processor = DiffProcessor::new();
    match processor.process_comparison_result(result, app.show_headers) {
        Ok(diff_data) => {
            // Use rich TUI widget renderer with details-specific diff style
            DiffWidgetRenderer::render_diff_view_with_style(
                f,
                &diff_data,
                app,
                &app.details_diff_style,
                chunks[1],
            );
        }
        Err(e) => {
            // Render error message with graceful fallback
            let error_text = format!("{} Error processing diff data: {}", UiSymbols::ERROR, e);
            let error_paragraph = Paragraph::new(error_text)
                .style(TuiTheme::error_style())
                .alignment(Alignment::Center)
                .scroll((app.scroll_offset as u16, 0))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(error_paragraph, chunks[1]);
        }
    }
}

/// Draw errors tab content
fn draw_details_errors_tab(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect) {
    if !result.has_errors {
        let no_errors = "✅ No errors detected\n\nAll requests completed\nsuccessfully.";
        let para = Paragraph::new(no_errors)
            .style(TuiTheme::success_style())
            .alignment(Alignment::Center);
        f.render_widget(para, area);
        return;
    }

    let mut error_lines = vec!["❌ Error Details:".to_string(), "".to_string()];

    // Show error details for each environment
    for (env, &status) in &result.status_codes {
        if !((200..300).contains(&status)) {
            error_lines.push(format!("🌍 Environment: {}", env));
            error_lines.push(format!("📊 HTTP Status: {}", status));

            // Add error body if available
            if let Some(error_bodies) = &result.error_bodies {
                if let Some(body) = error_bodies.get(env) {
                    error_lines.push("📄 Response Body:".to_string());

                    // Show first few lines of error body
                    for line in body.lines().take(5) {
                        error_lines.push(format!("  {}", line));
                    }
                    if body.lines().count() > 5 {
                        error_lines.push("  ... (truncated)".to_string());
                    }
                }
            }

            // Add curl command for reproduction
            if let Some(response) = result.responses.get(env) {
                error_lines.push("".to_string());
                error_lines.push("🔧 Curl Command:".to_string());
                error_lines.push(format!("  {}", response.curl_command));
            }

            error_lines.push("".to_string());
        }
    }

    let error_text = error_lines.join("\n");
    let error_para = Paragraph::new(error_text)
        .style(TuiTheme::primary_text_style())
        .scroll((app.scroll_offset as u16, 0))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(error_para, area);
}

/// Draw suggestions tab content
fn draw_details_suggestions_tab(
    f: &mut Frame,
    app: &TuiApp,
    result: &ComparisonResult,
    area: Rect,
) {
    let mut suggestions = vec![
        "💡 Suggestions & Recommendations:".to_string(),
        "".to_string(),
    ];

    if result.has_errors {
        suggestions.push("🔧 Error Resolution:".to_string());

        for (env, &status) in &result.status_codes {
            if !((200..300).contains(&status)) {
                match status {
                    401 => {
                        suggestions.push(format!("  • {} - Check authentication credentials", env))
                    }
                    403 => suggestions.push(format!(
                        "  • {} - Verify permissions and access rights",
                        env
                    )),
                    404 => suggestions.push(format!("  • {} - Confirm endpoint URL and path", env)),
                    422 => {
                        suggestions.push(format!("  • {} - Validate request payload format", env))
                    }
                    429 => suggestions.push(format!(
                        "  • {} - Implement rate limiting or retry logic",
                        env
                    )),
                    500 => suggestions.push(format!(
                        "  • {} - Check server logs for internal errors",
                        env
                    )),
                    502..=504 => suggestions.push(format!(
                        "  • {} - Service may be unavailable, try again later",
                        env
                    )),
                    _ => suggestions.push(format!(
                        "  • {} - Review HTTP status {} documentation",
                        env, status
                    )),
                }
            }
        }
        suggestions.push("".to_string());
    }

    if !result.is_identical && !result.has_errors {
        suggestions.push("🔍 Difference Analysis:".to_string());
        suggestions.push("  • Compare response schemas between environments".to_string());
        suggestions.push("  • Check for data consistency issues".to_string());
        suggestions.push("  • Verify environment-specific configurations".to_string());
        suggestions.push("  • Review API versioning across environments".to_string());
        suggestions.push("".to_string());
    }

    // General suggestions
    suggestions.push("⚡ Performance Tips:".to_string());
    suggestions.push("  • Use filters to focus on specific result types".to_string());
    suggestions.push("  • Press 'x' to expand this panel for better visibility".to_string());
    suggestions.push("  • Use 1-4 keys to quickly switch between tabs".to_string());
    suggestions.push("  • Press 'D' in Diffs tab to toggle view style".to_string());

    let suggestions_text = suggestions.join("\n");
    let suggestions_para = Paragraph::new(suggestions_text)
        .style(TuiTheme::primary_text_style())
        .scroll((app.scroll_offset as u16, 0))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(suggestions_para, area);
}


/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}










/// Smart truncation with better limits and word boundaries
fn smart_truncate(text: &str, table_limit: usize, context_limit: Option<usize>) -> String {
    let limit = context_limit.unwrap_or(table_limit);
    if text.len() <= limit {
        text.to_string()
    } else {
        // Find last word boundary before limit
        if let Some(pos) = text[..limit.saturating_sub(3)].rfind(' ') {
            format!("{}...", &text[..pos])
        } else {
            format!("{}...", &text[..limit.saturating_sub(3)])
        }
    }
}

/// Draw a scrollbar for a scrollable component
fn draw_scrollbar(
    f: &mut Frame,
    scrollbar_state: &mut ScrollbarState,
    area: Rect,
    content_length: usize,
    viewport_height: usize,
) {
    // Only show scrollbar if content exceeds viewport
    if content_length > viewport_height && area.width > 0 {
        let scrollbar_area = Rect {
            x: area.right().saturating_sub(1),
            y: area.y,
            width: 1,
            height: area.height,
        };

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(TuiTheme::secondary_text_style());

        f.render_stateful_widget(scrollbar, scrollbar_area, scrollbar_state);
    }
}




/// Draw compact results table for dashboard results panel
fn draw_compact_results_table(f: &mut Frame, app: &mut TuiApp, area: Rect, _is_panel_focused: bool) {
    if area.height < 3 {
        return;
    } // Too small to render

    let filtered_results = app.filtered_results();
    if filtered_results.is_empty() {
        return;
    }
    let results_count = filtered_results.len();

    // Create a simple table with just route name and status
    let header = Row::new(vec!["Route", "Status"])
        .style(TuiTheme::primary_text_style())
        .height(1);

    // Create all rows without limiting by viewport - scrolling will handle visibility
    let rows: Vec<Row> = filtered_results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let is_selected = i == app.selected_index;
            let status = if result.has_errors {
                format!("{} Error", UiSymbols::ERROR)
            } else if result.is_identical {
                format!("{} OK", UiSymbols::SUCCESS)
            } else {
                format!("{} Diff", UiSymbols::WARNING)
            };

            let style = if is_selected {
                TuiTheme::focused_style()
            } else {
                TuiTheme::primary_text_style()
            };

            Row::new(vec![smart_truncate(&result.route_name, 15, None), status]).style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .header(header)
    .row_highlight_style(
        Style::default()
            .bg(TuiTheme::BACKGROUND_SELECTED)
            .fg(TuiTheme::FOCUS)
            .add_modifier(Modifier::BOLD),
    );

    // Use stateful rendering for proper scrolling and selection
    f.render_stateful_widget(table, area, &mut app.results_table_state);

    // Draw scrollbar if content exceeds viewport
    let viewport_height = area.height.saturating_sub(3) as usize; // Account for header and borders
    draw_scrollbar(
        f,
        &mut app.results_scrollbar_state,
        area,
        results_count,
        viewport_height,
    );
}
