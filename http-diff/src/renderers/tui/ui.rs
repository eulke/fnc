use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, Scrollbar, ScrollbarOrientation, ScrollbarState},
    style::{Color, Modifier, Style},
    layout::{Alignment, Constraint, Direction, Layout},
};
use crate::{
    renderers::{
        tui::{
            app::{TuiApp, ViewMode, FocusedPanel, FeedbackType},
            theme::{TuiTheme, UiSymbols, KeyHints},
        },
    },
    types::{ComparisonResult, DiffViewStyle},
};

/// Main UI drawing function
pub fn draw(f: &mut Frame, app: &TuiApp) {
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
    
    // Draw main content based on view mode
    match app.view_mode {
        ViewMode::Configuration => draw_configuration_view(f, app, chunks[1]),
        ViewMode::Execution => draw_execution_view(f, app, chunks[1]),
        ViewMode::ResultsList => draw_results_list(f, app, chunks[1]),
        ViewMode::ResultDetail => draw_result_detail(f, app, chunks[1]),
        ViewMode::DiffView => draw_diff_view(f, app, chunks[1]),
    }
    
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
    let (total, identical, different) = app.get_summary();
    
    let title_text = format!(
        "{} HTTP Diff TUI - {} | {} Total: {} | {} Identical: {} | {} Different: {}",
        UiSymbols::QUICK_ACTION,
        app.get_view_title(),
        UiSymbols::LIST,
        total,
        UiSymbols::SUCCESS,
        identical,
        UiSymbols::ERROR,
        different
    );
    
    let title = Paragraph::new(title_text)
        .style(TuiTheme::primary_text_style().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(TuiTheme::focused_block("HTTP Diff TUI"));
    
    f.render_widget(title, area);
}

/// Draw the results list view
fn draw_results_list(f: &mut Frame, app: &TuiApp, area: Rect) {
    // Create table headers
    let headers = ["#", "Route", "Status", "Environments", "Has Diff"];
    let header_cells = headers.iter().map(|h| {
        Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    });
    let header = Row::new(header_cells);

    // Create table rows
    let rows: Vec<Row> = app.results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let status = if result.is_identical { "✓ Identical" } else { "✗ Different" };
            let status_color = if result.is_identical { Color::Green } else { Color::Red };
            
            let environments = result.responses.keys().cloned().collect::<Vec<_>>().join(", ");
            let has_diff = if result.is_identical { "No" } else { "Yes" };
            let has_diff_color = if result.is_identical { Color::Gray } else { Color::Yellow };

            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from((i + 1).to_string()),
                Cell::from(result.route_name.clone()),
                Cell::from(status).style(Style::default().fg(status_color)),
                Cell::from(environments),
                Cell::from(has_diff).style(Style::default().fg(has_diff_color)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(4),     // #
        Constraint::Percentage(30), // Route
        Constraint::Length(12),    // Status
        Constraint::Percentage(40), // Environments
        Constraint::Length(8),     // Has Diff
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Results")
                .border_style(Style::default().fg(Color::Blue))
        )
        .column_spacing(1);

    f.render_widget(table, area);
}

/// Draw the result detail view
fn draw_result_detail(f: &mut Frame, app: &TuiApp, area: Rect) {
    if let Some(result) = app.current_result() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Result header
                Constraint::Min(1),    // Result details
            ])
            .split(area);

        // Draw result header
        draw_result_header(f, result, app.selected_index, app.results.len(), chunks[0]);
        
        // Draw result details
        draw_result_details(f, result, app, chunks[1]);
    } else {
        let no_result = Paragraph::new("No result selected")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Result Detail"));
        f.render_widget(no_result, area);
    }
}

/// Draw result header information
fn draw_result_header(f: &mut Frame, result: &ComparisonResult, index: usize, total: usize, area: Rect) {
    let status = if result.is_identical { 
        ("✓ IDENTICAL", Color::Green) 
    } else { 
        ("✗ DIFFERENT", Color::Red) 
    };
    
    let environments = result.responses.keys().cloned().collect::<Vec<_>>().join(", ");
    
    let header_text = format!(
        "Route: {}\nEnvironments: {}\nStatus: {}\nResult: {}/{}",
        result.route_name,
        environments,
        status.0,
        index + 1,
        total
    );
    
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(status.1))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Result Information")
                .border_style(Style::default().fg(Color::Blue))
        );
    
    f.render_widget(header, area);
}

/// Draw detailed result information
fn draw_result_details(f: &mut Frame, result: &ComparisonResult, app: &TuiApp, area: Rect) {
    let mut content = String::new();
    
    // Add response information
    for (env_name, response) in &result.responses {
        content.push_str(&format!("Environment: {}\n", env_name));
        content.push_str(&format!("  Status: {} {}\n", response.status, 
                                 if response.is_success() { "✓" } else { "✗" }));
        content.push_str(&format!("  URL: {}\n", response.url));
        
        if app.show_headers && !response.headers.is_empty() {
            content.push_str("  Headers:\n");
            for (key, value) in &response.headers {
                content.push_str(&format!("    {}: {}\n", key, value));
            }
        }
        
        content.push_str(&format!("  Body length: {} bytes\n", response.body.len()));
        content.push('\n');
    }
    
    // Add differences information if any
    if !result.is_identical {
        content.push_str("Differences found:\n");
        for difference in &result.differences {
            content.push_str(&format!("  {:?}: {}\n", difference.category, difference.description));
        }
        content.push_str("\n(Use Diff View for detailed comparison)\n");
    }

    // Create scrollable paragraph
    let lines: Vec<&str> = content.lines().collect();
    let visible_lines = if app.scroll_offset < lines.len() {
        &lines[app.scroll_offset..]
    } else {
        &[]
    };
    let visible_content = visible_lines.join("\n");
    
    let paragraph = Paragraph::new(visible_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Details")
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(paragraph, area);
    
    // Add scrollbar if content is longer than area
    if lines.len() > area.height as usize {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(lines.len())
            .position(app.scroll_offset);
        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin { vertical: 1, horizontal: 1 }),
            &mut scrollbar_state,
        );
    }
}

/// Draw the diff view
fn draw_diff_view(f: &mut Frame, app: &TuiApp, area: Rect) {
    if let Some(result) = app.current_result() {
        if result.is_identical {
            let no_diff = Paragraph::new("No differences found between environments")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Green))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Diff View")
                        .border_style(Style::default().fg(Color::Blue))
                );
            f.render_widget(no_diff, area);
        } else {
            draw_response_diff(f, result, app, area);
        }
    } else {
        let no_result = Paragraph::new("No result selected")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Diff View"));
        f.render_widget(no_result, area);
    }
}

/// Draw response differences using rich TUI widgets
fn draw_response_diff(f: &mut Frame, result: &ComparisonResult, app: &TuiApp, area: Rect) {
    use crate::renderers::{diff_processor::DiffProcessor, tui::diff_widgets::DiffWidgetRenderer};
    
    // Process the comparison result into generic diff data
    let processor = DiffProcessor::new();
    match processor.process_comparison_result(result, app.show_headers) {
        Ok(diff_data) => {
            // Use rich TUI widget renderer to display diff data
            DiffWidgetRenderer::render_diff_view(f, &diff_data, app, area);
        }
        Err(e) => {
            // Render error message
            let error_text = format!("{} Error processing diff data: {}", UiSymbols::ERROR, e);
            let error_paragraph = Paragraph::new(error_text)
                .style(TuiTheme::error_style())
                .block(TuiTheme::normal_block("Diff Error"))
                .alignment(Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true });
            
            f.render_widget(error_paragraph, area);
        }
    }
}

/// Draw the status/help bar
fn draw_status_bar(f: &mut Frame, app: &TuiApp, area: Rect) {
    let key_hints = match app.view_mode {
        ViewMode::Configuration => KeyHints::format_key_hints(&KeyHints::configuration_help()),
        ViewMode::Execution => KeyHints::format_key_hints(&KeyHints::execution_help()),
        _ => KeyHints::format_key_hints(&KeyHints::results_help()),
    };
    
    let status_content = format!(
        "{} Headers: {} | {} Errors: {} | {} Diff: {} | {}",
        UiSymbols::SETTINGS,
        if app.show_headers { "ON" } else { "OFF" },
        UiSymbols::ERROR,
        if app.show_errors { "ON" } else { "OFF" },
        UiSymbols::RESULTS,
        match app.diff_style {
            DiffViewStyle::Unified => "Unified",
            DiffViewStyle::SideBySide => "Side-by-Side",
        },
        key_hints
    );
    
    let help_title = format!("{} Quick Help", UiSymbols::HELP);
    let status = Paragraph::new(status_content)
        .style(TuiTheme::secondary_text_style())
        .alignment(Alignment::Center)
        .block(TuiTheme::normal_block(&help_title))
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(status, area);
}

/// Draw the configuration view for environment and route selection
fn draw_configuration_view(f: &mut Frame, app: &TuiApp, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),        // Main selection panels
            Constraint::Length(4),     // Action buttons
        ])
        .split(area);

    let selection_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    // Environment selection panel
    draw_environment_selection_enhanced(f, app, selection_chunks[0]);
    
    // Route selection panel
    draw_route_selection_enhanced(f, app, selection_chunks[1]);
    
    // Action buttons panel
    draw_action_buttons(f, app, main_chunks[1]);

    // Show error message if present
    if let Some(ref error) = app.error_message {
        draw_error_popup(f, error);
    }
}

/// Draw enhanced environment selection panel
fn draw_environment_selection_enhanced(f: &mut Frame, app: &TuiApp, area: Rect) {
    let is_focused = matches!(app.focused_panel, FocusedPanel::Environments);
    
    let items: Vec<ratatui::widgets::ListItem> = app.available_environments
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let is_selected = app.is_environment_selected(i);
            let is_cursor = is_focused && i == app.selected_env_index;
            
            let checkbox = if is_selected { UiSymbols::SELECTED } else { UiSymbols::UNSELECTED };
            let cursor = if is_cursor { UiSymbols::FOCUSED_INDICATOR } else { UiSymbols::UNFOCUSED_INDICATOR };
            
            let mut style = if is_selected {
                TuiTheme::selected_style()
            } else {
                TuiTheme::primary_text_style()
            };
            
            if is_cursor {
                style = TuiTheme::focused_style();
            }
            
            ratatui::widgets::ListItem::new(format!("{} {} {}", cursor, checkbox, env))
                .style(style)
        })
        .collect();

    let title = format!("{} Environments ({}/{})", 
        UiSymbols::LIST,
        app.selected_environments.len(),
        app.available_environments.len());
    let block = if is_focused {
        TuiTheme::focused_block(&title)
    } else {
        TuiTheme::normal_block(&title)
    };
    
    let list = ratatui::widgets::List::new(items).block(block);
    f.render_widget(list, area);
}

/// Draw enhanced route selection panel
fn draw_route_selection_enhanced(f: &mut Frame, app: &TuiApp, area: Rect) {
    let is_focused = matches!(app.focused_panel, FocusedPanel::Routes);
    
    let items: Vec<ratatui::widgets::ListItem> = app.available_routes
        .iter()
        .enumerate()
        .map(|(i, route)| {
            let is_selected = app.is_route_selected(i);
            let is_cursor = is_focused && i == app.selected_route_index;
            
            let checkbox = if is_selected { UiSymbols::SELECTED } else { UiSymbols::UNSELECTED };
            let cursor = if is_cursor { UiSymbols::FOCUSED_INDICATOR } else { UiSymbols::UNFOCUSED_INDICATOR };
            
            let mut style = if is_selected {
                TuiTheme::selected_style()
            } else {
                TuiTheme::primary_text_style()
            };
            
            if is_cursor {
                style = TuiTheme::focused_style();
            }
            
            ratatui::widgets::ListItem::new(format!("{} {} {}", cursor, checkbox, route))
                .style(style)
        })
        .collect();

    let title = format!("{} Routes ({}/{})", 
        UiSymbols::LIST,
        app.selected_routes.len(),
        app.available_routes.len());
    let block = if is_focused {
        TuiTheme::focused_block(&title)
    } else {
        TuiTheme::normal_block(&title)
    };
    
    let list = ratatui::widgets::List::new(items).block(block);
    f.render_widget(list, area);
}

/// Draw action buttons panel
fn draw_action_buttons(f: &mut Frame, app: &TuiApp, area: Rect) {
    let is_focused = matches!(app.focused_panel, FocusedPanel::Actions);
    let has_selections = !app.selected_environments.is_empty() && !app.selected_routes.is_empty();
    
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Start Tests button
    let start_style = TuiTheme::button_style(is_focused, has_selections);
    let start_text = format!("{} Start Tests", UiSymbols::PLAY);
    let start_button = Paragraph::new(start_text)
        .style(start_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(start_button, chunks[0]);

    // Select All button
    let select_all_style = TuiTheme::button_style(false, true);
    let select_all_text = format!("{} Select All", UiSymbols::SELECTED);
    let select_all_button = Paragraph::new(select_all_text)
        .style(select_all_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(select_all_button, chunks[1]);

    // Clear All button
    let clear_all_style = TuiTheme::button_style(false, true);
    let clear_all_text = format!("{} Clear All", UiSymbols::UNSELECTED);
    let clear_all_button = Paragraph::new(clear_all_text)
        .style(clear_all_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(clear_all_button, chunks[2]);

    // Help button
    let help_style = TuiTheme::button_style(false, true);
    let help_text = format!("{} Help (F1)", UiSymbols::HELP);
    let help_button = Paragraph::new(help_text)
        .style(help_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help_button, chunks[3]);
}

/// Draw help overlay
fn draw_help_overlay(f: &mut Frame, app: &TuiApp) {
    let area = centered_rect(80, 60, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let help_text = match app.view_mode {
        ViewMode::Configuration => {
            let hints = KeyHints::configuration_help();
            format!(
                "{} Configuration Help\n\n{}\n\n{} Navigation Tips:\n• Use Tab to switch between panels\n• Use ↑↓ to navigate within panels\n• Use Space or Enter to toggle selections\n• Press F1 to close this help",
                UiSymbols::HELP,
                KeyHints::format_key_hints(&hints),
                UiSymbols::TIP
            )
        }
        ViewMode::Execution => {
            let hints = KeyHints::execution_help();
            format!(
                "{} Execution Help\n\n{}\n\nPlease wait for tests to complete...",
                UiSymbols::HELP,
                KeyHints::format_key_hints(&hints)
            )
        }
        _ => {
            let hints = KeyHints::results_help();
            format!(
                "{} Results Help\n\n{}",
                UiSymbols::HELP,
                KeyHints::format_key_hints(&hints)
            )
        }
    };

    let help_popup = Paragraph::new(help_text)
        .style(TuiTheme::primary_text_style())
        .block(
            TuiTheme::focused_block("Help")
                .style(TuiTheme::info_style())
        )
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

/// Draw the execution view showing real-time progress
fn draw_execution_view(f: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Summary
            Constraint::Length(3),  // Progress bar
            Constraint::Length(3),  // Current operation
            Constraint::Min(1),     // Stats and info
        ])
        .split(area);

    // Execution summary
    let summary_text = format!(
        "Environments: {}\nRoutes: {}\nTotal Tests: {}",
        app.selected_environments.join(", "),
        app.selected_routes.join(", "),
        app.total_tests
    );
    
    let summary = Paragraph::new(summary_text)
        .block(
            Block::default()
                .title("Test Execution")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
        );
    
    f.render_widget(summary, chunks[0]);

    // Progress bar
    let progress = if app.total_tests > 0 {
        let raw_progress = (app.completed_tests as f64 / app.total_tests as f64) * 100.0;
        // Clamp progress to 0-100 range
        raw_progress.max(0.0).min(100.0)
    } else {
        0.0
    };

    let gauge = ratatui::widgets::Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(progress as u16)
        .label(format!(
            "{}/{} tests ({:.1}%)",
            app.completed_tests,
            app.total_tests,
            progress
        ));

    f.render_widget(gauge, chunks[1]);

    // Current operation
    let operation = Paragraph::new(app.current_operation.clone())
        .block(Block::default().title("Current Operation").borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(operation, chunks[2]);

    // Elapsed time and stats
    let elapsed_text = if let Some(start_time) = app.execution_start_time {
        let elapsed = start_time.elapsed();
        format!("Elapsed: {:.1}s", elapsed.as_secs_f64())
    } else {
        "Starting...".to_string()
    };

    let stats = Paragraph::new(elapsed_text)
        .block(Block::default().title("Statistics").borders(Borders::ALL));

    f.render_widget(stats, chunks[3]);
}

/// Draw error popup (helper function)
fn draw_error_popup(f: &mut Frame, error_message: &str) {
    let area = centered_rect(60, 20, f.area());
    
    f.render_widget(ratatui::widgets::Clear, area);
    
    let block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(format!("{}\n\nPress any key to dismiss", error_message))
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));

    f.render_widget(paragraph, area);
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