use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Paragraph, Table, Row, Cell, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Gauge, BarChart, List, ListItem, Tabs
    },
    style::{Color, Modifier, Style},
    layout::{Alignment, Constraint, Direction, Layout},
};
use crate::{
    renderers::{
        tui::{
            app::{TuiApp, ViewMode, FocusedPanel, PanelFocus, FeedbackType},
            theme::{TuiTheme, UiSymbols, KeyHints},
        },
    },
    types::{ComparisonResult, DiffViewStyle},
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
    
    // Draw main content based on view mode
    match app.view_mode {
        ViewMode::Configuration => draw_configuration_view(f, app, chunks[1]),
        ViewMode::Execution => draw_execution_view(f, app, chunks[1]),
        ViewMode::ResultsList => draw_results_list(f, app, chunks[1]),
        ViewMode::ResultDetail => draw_result_detail(f, app, chunks[1]),
        ViewMode::DiffView => draw_diff_view(f, app, chunks[1]),
        ViewMode::Dashboard => draw_dashboard_view(f, app, chunks[1]),
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

/// Draw the results list view with dynamic filter tabs
fn draw_results_list(f: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter tabs
            Constraint::Min(5),    // Results table
            Constraint::Length(2), // Position indicator
        ])
        .split(area);

    // Draw filter tabs
    draw_filter_tabs(f, app, chunks[0]);
    
    // Draw smart filter panel if enabled
    if app.filter_state.show_filter_panel {
        draw_smart_filter_panel(f, app);
    }
    
    // Get filtered results
    let filtered_results = app.filtered_results();
    
    // Create table headers
    let headers = ["#", "Route", "Status", "Environments", "Summary"];
    let header_cells = headers.iter().map(|h| {
        Cell::from(*h).style(TuiTheme::warning_style().add_modifier(Modifier::BOLD))
    });
    let header = Row::new(header_cells);

    // Create table rows from filtered results
    let rows: Vec<Row> = filtered_results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            // Use simple three-state status display
            let (status_text, status_color) = get_simple_status_display(result);
            
            let environments = result.responses.keys().cloned().collect::<Vec<_>>().join(", ");
            
            // Use meaningful summary  
            let summary = get_meaningful_summary(result);
            
            let diff_color = if result.has_errors {
                TuiTheme::ERROR
            } else if result.is_identical { 
                TuiTheme::TEXT_SECONDARY 
            } else { 
                TuiTheme::WARNING 
            };

            let style = if i == app.selected_index {
                TuiTheme::focused_style().add_modifier(Modifier::BOLD)
            } else {
                TuiTheme::primary_text_style()
            };

            Row::new(vec![
                Cell::from((i + 1).to_string()),
                Cell::from(result.route_name.clone()),
                Cell::from(status_text).style(Style::default().fg(status_color)),
                Cell::from(environments),
                Cell::from(summary).style(Style::default().fg(diff_color)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(4),     // #
        Constraint::Percentage(25), // Route (reduced from 30%)
        Constraint::Length(16),    // Status (increased from 14)
        Constraint::Percentage(30), // Environments (reduced from 40%) 
        Constraint::Percentage(25), // Summary (increased from Length(8) to percentage)
    ];

    let title = format!("{} Results (Filtered: {}/{})", 
        UiSymbols::RESULTS, 
        filtered_results.len(), 
        app.results.len());

    let table = Table::new(rows, widths)
        .header(header)
        .block(TuiTheme::normal_block(&title))
        .column_spacing(1);

    f.render_widget(table, chunks[1]);
    
    // Draw position indicator
    draw_results_position_indicator(f, app, chunks[2]);
}

/// Draw dynamic filter tabs with live counts
fn draw_filter_tabs(f: &mut Frame, app: &TuiApp, area: Rect) {
    let (total, identical, different, errors) = app.get_filter_counts();
    
    let tabs = [
        (format!("All: {}", total), 0),
        (format!("✓ Identical: {}", identical), 1), 
        (format!("✗ Different: {}", different), 2),
        (format!("⚠ Errors: {}", errors), 3),
    ];
    
    let tab_width = area.width / 4;
    let tab_constraints = [
        Constraint::Length(tab_width),
        Constraint::Length(tab_width), 
        Constraint::Length(tab_width),
        Constraint::Length(tab_width),
    ];
    
    let tab_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(tab_constraints)
        .split(area);
    
    for ((tab_text, tab_index), chunk) in tabs.iter().zip(tab_chunks.iter()) {
        let is_active = app.filter_state.current_tab == *tab_index;
        
        let (style, border_style) = if is_active {
            (
                TuiTheme::focused_style().add_modifier(Modifier::BOLD),
                Style::default().fg(TuiTheme::FOCUS).add_modifier(Modifier::BOLD)
            )
        } else {
            (
                TuiTheme::primary_text_style(),
                Style::default().fg(TuiTheme::BORDER_NORMAL)
            )
        };
        
        let tab_symbol = if is_active { "●" } else { "○" };
        let display_text = format!("{} {}", tab_symbol, tab_text);
        
        let tab = Paragraph::new(display_text)
            .style(style)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
            );
        
        f.render_widget(tab, *chunk);
    }
}

/// Draw position indicator showing current selection within filtered results
fn draw_results_position_indicator(f: &mut Frame, app: &TuiApp, area: Rect) {
    let (current_pos, total_filtered) = app.get_filter_position_info();
    
    let position_text = if total_filtered == 0 {
        format!("{} No results match current filter", UiSymbols::INFO)
    } else {
        // Add error hint if current selection is an error
        let error_hint = get_selected_error_hint(app)
            .map(|hint| format!(" ({})", hint))
            .unwrap_or_default();
            
        format!("{} Result {}/{}{} | {} Navigate: ↑↓ | {} Filter: 1-4 | {} Details: →", 
            UiSymbols::FORWARD, 
            current_pos, 
            total_filtered,
            error_hint,
            UiSymbols::UP_DOWN,
            UiSymbols::QUICK_ACTION,
            UiSymbols::DETAILS
        )
    };
    
    let position_style = if total_filtered == 0 {
        TuiTheme::warning_style()
    } else {
        TuiTheme::info_style()
    };
    
    let position_indicator = Paragraph::new(position_text)
        .style(position_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    
    f.render_widget(position_indicator, area);
}

/// Draw the result detail view
fn draw_result_detail(f: &mut Frame, app: &TuiApp, area: Rect) {
    if let Some(result) = app.current_filtered_result() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Result header
                Constraint::Min(1),    // Result details
            ])
            .split(area);

        // Draw result header - use filtered results for position info
        let filtered_results = app.filtered_results();
        draw_result_header(f, result, app.selected_index, filtered_results.len(), chunks[0]);
        
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
    
    // Add error information first if there are errors
    if result.has_errors {
        content.push_str("⚠ ERROR DETAILS\n");
        content.push_str("═══════════════\n\n");
        
        // Show error bodies if available
        if let Some(ref error_bodies) = result.error_bodies {
            for (env_name, error_body) in error_bodies {
                content.push_str(&format!("Error in {}:\n", env_name));
                
                // Try to format JSON error bodies nicely
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(error_body) {
                    match serde_json::to_string_pretty(&json_value) {
                        Ok(pretty_json) => {
                            content.push_str(&format!("  {}\n\n", pretty_json.replace('\n', "\n  ")));
                        }
                        Err(_) => {
                            content.push_str(&format!("  {}\n\n", error_body));
                        }
                    }
                } else {
                    content.push_str(&format!("  {}\n\n", error_body));
                }
            }
        }
        
        // Show status codes for error environments
        content.push_str("HTTP Status Codes:\n");
        for (env_name, &status) in &result.status_codes {
            if status < 200 || status >= 300 {
                let status_description = match status {
                    400 => "Bad Request",
                    401 => "Unauthorized", 
                    403 => "Forbidden",
                    404 => "Not Found",
                    405 => "Method Not Allowed",
                    408 => "Request Timeout",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    502 => "Bad Gateway", 
                    503 => "Service Unavailable",
                    504 => "Gateway Timeout",
                    _ => "Error",
                };
                content.push_str(&format!("  {}: {} ({})\n", env_name, status, status_description));
            }
        }
        content.push_str("\n");
    }
    
    // Add response information
    content.push_str("RESPONSE DETAILS\n");
    content.push_str("════════════════\n\n");
    
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
        
        // Show response body preview for errors
        if result.has_errors && (app.show_errors || response.is_error()) {
            let preview_lines = response.body.lines().take(5).collect::<Vec<_>>();
            if !preview_lines.is_empty() {
                content.push_str("  Body preview:\n");
                for line in preview_lines {
                    content.push_str(&format!("    {}\n", line));
                }
                if response.body.lines().count() > 5 {
                    content.push_str("    ... (use Diff View for full body)\n");
                }
            }
        }
        content.push('\n');
    }
    
    // Add differences information if any
    if !result.is_identical {
        content.push_str("DIFFERENCES FOUND\n");
        content.push_str("═════════════════\n\n");
        for difference in &result.differences {
            content.push_str(&format!("  {:?}: {}\n", difference.category, difference.description));
            
            // Show diff output if available
            if let Some(ref diff_output) = difference.diff_output {
                let preview_lines = diff_output.lines().take(3).collect::<Vec<_>>();
                if !preview_lines.is_empty() {
                    content.push_str("  Preview:\n");
                    for line in preview_lines {
                        content.push_str(&format!("    {}\n", line));
                    }
                    if diff_output.lines().count() > 3 {
                        content.push_str("    ... (use Diff View for complete comparison)\n");
                    }
                }
            }
            content.push('\n');
        }
        content.push_str("(Press Tab or use → to go to Diff View for detailed comparison)\n");
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
    if let Some(result) = app.current_filtered_result() {
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
        ViewMode::Dashboard => KeyHints::format_key_hints(&KeyHints::dashboard_help()),
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
            Constraint::Length(1),     // Selection status bar
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
    
    // Selection status bar
    draw_selection_status_bar(f, app, main_chunks[2]);

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
            
            // Enhanced styling with better visual feedback
            let mut style = if is_selected {
                TuiTheme::selected_style().add_modifier(Modifier::BOLD)
            } else {
                TuiTheme::primary_text_style()
            };
            
            if is_cursor {
                style = if is_selected {
                    // Selected + focused: bright green with bold
                    TuiTheme::focused_style().add_modifier(Modifier::BOLD)
                } else {
                    // Just focused: highlighted
                    TuiTheme::focused_style()
                };
            }
            
            // Add visual emphasis for selected items
            let display_text = if is_selected {
                format!("{} {} {} ✓", cursor, checkbox, env)
            } else {
                format!("{} {} {}", cursor, checkbox, env)
            };
            
            ratatui::widgets::ListItem::new(display_text)
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
            
            // Enhanced styling with better visual feedback
            let mut style = if is_selected {
                TuiTheme::selected_style().add_modifier(Modifier::BOLD)
            } else {
                TuiTheme::primary_text_style()
            };
            
            if is_cursor {
                style = if is_selected {
                    // Selected + focused: bright green with bold
                    TuiTheme::focused_style().add_modifier(Modifier::BOLD)
                } else {
                    // Just focused: highlighted
                    TuiTheme::focused_style()
                };
            }
            
            // Add visual emphasis for selected items
            let display_text = if is_selected {
                format!("{} {} {} ✓", cursor, checkbox, route)
            } else {
                format!("{} {} {}", cursor, checkbox, route)
            };
            
            ratatui::widgets::ListItem::new(display_text)
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
        ViewMode::Dashboard => {
            let hints = KeyHints::dashboard_help();
            format!(
                "{} Dashboard Help\n\n{}",
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

/// Draw the dashboard view with 4 simultaneous panels
fn draw_dashboard_view(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    // Check if any panel is expanded
    let expanded_panel = app.panel_sizes.iter()
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
fn draw_environments_list_widget(f: &mut Frame, app: &mut TuiApp, area: Rect, is_panel_focused: bool) {
    let env_items: Vec<ListItem> = app.available_environments
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
    let is_env_focused = is_panel_focused && matches!(app.focused_panel, FocusedPanel::Environments);
    
    // Create title string that will live long enough
    let title_text = format!("Environments ({}/{})", selected_count, total_count);
    
    // Create block with focus-dependent styling
    let block = if is_env_focused {
        TuiTheme::focused_block(&title_text)
    } else {
        TuiTheme::normal_block(&title_text)
    };
    
    let env_list = List::new(env_items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(TuiTheme::BACKGROUND_SELECTED)
                .fg(TuiTheme::FOCUS)
                .add_modifier(Modifier::BOLD)
        );
    
    // Use stateful rendering for proper cursor positioning
    f.render_stateful_widget(env_list, area, &mut app.env_list_state);
}

/// Draw routes as a proper List widget with enhanced visual feedback
fn draw_routes_list_widget(f: &mut Frame, app: &mut TuiApp, area: Rect, is_panel_focused: bool) {
    let route_items: Vec<ListItem> = app.available_routes
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
    
    let route_list = List::new(route_items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(TuiTheme::BACKGROUND_SELECTED)
                .fg(TuiTheme::FOCUS)
                .add_modifier(Modifier::BOLD)
        );
    
    // Use stateful rendering for proper cursor positioning
    f.render_stateful_widget(route_list, area, &mut app.route_list_state);
}

/// Draw configuration status line with context-sensitive navigation hints
fn draw_config_status_line(f: &mut Frame, app: &TuiApp, area: Rect) {
    // Determine current focused section
    let current_section = match app.focused_panel {
        FocusedPanel::Environments => "📝 Environments",
        FocusedPanel::Routes => "🛣 Routes", 
        FocusedPanel::Actions => "⚡ Actions",
    };
    
    // Create context-sensitive instructions with clear navigation separation
    let navigation_hint = "←→ Switch sections • ↑↓ Navigate • Space Toggle • Tab Switch panels • R Run";
    
    let text = if app.selected_environments.is_empty() || app.selected_routes.is_empty() {
        format!("{} | {} | ⚠ Select items to continue", current_section, navigation_hint)
    } else {
        let total_tests = app.selected_environments.len() * app.selected_routes.len();
        format!("{} | {} | ✅ {} tests ready", current_section, navigation_hint, total_tests)
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
        ((app.completed_tests as f64 / app.total_tests as f64) * 100.0).max(0.0).min(100.0) as u16
    } else {
        0
    };
    
    let progress_gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(TuiTheme::FOCUS))
        .percent(progress_value)
        .label(format!("{}/{} tests ({progress_value}%)", app.completed_tests, app.total_tests));
    
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
        .value_style(Style::default().fg(TuiTheme::TEXT_PRIMARY).add_modifier(Modifier::BOLD));
    
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
    let ready_text = "🚀 Ready to Execute\n\n1. Select environments\n2. Select routes\n3. Press 'R' to start";
    let ready_para = Paragraph::new(ready_text)
        .style(TuiTheme::secondary_text_style())
        .alignment(Alignment::Center);
    f.render_widget(ready_para, area);
}

/// Draw the results panel in dashboard mode
fn draw_dashboard_results_panel(f: &mut Frame, app: &TuiApp, area: Rect) {
    let is_focused = app.is_panel_focused(&PanelFocus::Results);
    let title = app.get_panel_title(&PanelFocus::Results);
    let has_content = !app.results.is_empty();
    let has_activity = app.filter_state.show_filter_panel || 
                       app.filter_state.status_filter != crate::renderers::tui::app::StatusFilter::All;
    
    let block = TuiTheme::panel_block(&title, is_focused, has_content, has_activity);
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    if app.results.is_empty() {
        let empty_text = "No results yet\n\nRun tests from the\nConfiguration panel\nto see results here";
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
        let empty_text = "📋 No result selected\n\nNavigate in Results panel\nto see detailed information";
        let empty_para = Paragraph::new(empty_text)
            .style(TuiTheme::secondary_text_style())
            .alignment(Alignment::Center);
        f.render_widget(empty_para, inner_area);
    }
}

/// Draw detailed result information with tabbed interface
fn draw_detailed_result_with_tabs(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect, is_focused: bool) {
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
            Style::default().fg(TuiTheme::FOCUS).add_modifier(Modifier::BOLD) 
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
        format!("📊 Status: {}", if result.has_errors {
            "❌ Errors detected"
        } else if result.is_identical {
            "✅ All responses identical"
        } else {
            "⚠ Responses differ"
        }),
        "".to_string(),
    ];
    
    // Environment status
    lines.push("🌍 Environments:".to_string());
    for (env, response) in &result.responses {
        let status_icon = if response.is_success() { "✅" } else { "❌" };
        lines.push(format!("  {} {} - HTTP {}", status_icon, env, response.status));
    }
    
    // Response size info
    if !result.responses.is_empty() {
        lines.push("".to_string());
        lines.push("📏 Response Sizes:".to_string());
        for (env, response) in &result.responses {
            lines.push(format!("  {} - {} bytes, {} lines", env, response.body.len(), response.line_count()));
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
        let no_diffs = "✅ No differences found\n\nAll responses are identical\nacross environments.";
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
    let style_text = format!("📝 {} View (Press D to toggle)", 
        match app.details_diff_style {
            crate::types::DiffViewStyle::Unified => "Unified",
            crate::types::DiffViewStyle::SideBySide => "Side-by-Side",
        });
    let style_para = Paragraph::new(style_text)
        .style(TuiTheme::info_style());
    f.render_widget(style_para, chunks[0]);
    
    // Use proper diff processing pipeline for rich visual elements
    use crate::renderers::{diff_processor::DiffProcessor, tui::diff_widgets::DiffWidgetRenderer};
    
    // Process the comparison result into generic diff data
    let processor = DiffProcessor::new();
    match processor.process_comparison_result(result, app.show_headers) {
        Ok(diff_data) => {
            // Use rich TUI widget renderer with details-specific diff style
            DiffWidgetRenderer::render_diff_view_with_style(f, &diff_data, app, &app.details_diff_style, chunks[1]);
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
    
    let mut error_lines = vec![
        "❌ Error Details:".to_string(),
        "".to_string(),
    ];
    
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
fn draw_details_suggestions_tab(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect) {
    let mut suggestions = vec![
        "💡 Suggestions & Recommendations:".to_string(),
        "".to_string(),
    ];
    
    if result.has_errors {
        suggestions.push("🔧 Error Resolution:".to_string());
        
        for (env, &status) in &result.status_codes {
            if !((200..300).contains(&status)) {
                match status {
                    401 => suggestions.push(format!("  • {} - Check authentication credentials", env)),
                    403 => suggestions.push(format!("  • {} - Verify permissions and access rights", env)),
                    404 => suggestions.push(format!("  • {} - Confirm endpoint URL and path", env)),
                    422 => suggestions.push(format!("  • {} - Validate request payload format", env)),
                    429 => suggestions.push(format!("  • {} - Implement rate limiting or retry logic", env)),
                    500 => suggestions.push(format!("  • {} - Check server logs for internal errors", env)),
                    502 | 503 | 504 => suggestions.push(format!("  • {} - Service may be unavailable, try again later", env)),
                    _ => suggestions.push(format!("  • {} - Review HTTP status {} documentation", env, status)),
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

/// Draw selection status bar showing real-time selection counts
fn draw_selection_status_bar(f: &mut Frame, app: &TuiApp, area: Rect) {
    let env_selected = app.selected_environments.len();
    let env_total = app.available_environments.len();
    let route_selected = app.selected_routes.len();
    let route_total = app.available_routes.len();
    
    // Create selection status
    let selection_info = format!(
        "{} Environments: {}/{} selected | {} Routes: {}/{} selected",
        UiSymbols::LIST, env_selected, env_total,
        UiSymbols::ROUTE, route_selected, route_total
    );
    
    // Selection status styling
    let selection_style = if env_selected > 0 && route_selected > 0 {
        TuiTheme::success_style()
    } else {
        TuiTheme::warning_style()
    };
    
    let selection_bar = Paragraph::new(selection_info)
        .style(selection_style.add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    
    f.render_widget(selection_bar, area);
}

/// Draw smart filter panel with environment flow and suggestions
fn draw_smart_filter_panel(f: &mut Frame, app: &TuiApp) {
    let area = centered_rect(80, 70, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Title and close hint
            Constraint::Length(6),  // Environment flow
            Constraint::Length(8),  // Smart suggestions
            Constraint::Min(3),     // Filter options
        ])
        .split(area);

    // Title with close hint
    let title_text = format!("{} Smart Filter Panel | Press 'f' to close", UiSymbols::QUICK_ACTION);
    let title = Paragraph::new(title_text)
        .style(TuiTheme::info_style().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(TuiTheme::focused_block("Advanced Filtering"));
    
    f.render_widget(title, main_chunks[0]);

    // Environment flow visualization
    draw_environment_flow(f, app, main_chunks[1]);
    
    // Smart suggestions
    draw_smart_suggestions(f, app, main_chunks[2]);
    
    // Filter options
    draw_filter_options(f, app, main_chunks[3]);
}

/// Draw environment flow with arrows showing data movement
fn draw_environment_flow(f: &mut Frame, app: &TuiApp, area: Rect) {
    let (total, identical, different, errors) = app.get_filter_counts();
    
    // Create a visual flow of how data moves through environments
    let flow_text = if app.results.is_empty() {
        format!("{} No test results available", UiSymbols::INFO)
    } else {
        let envs: Vec<String> = app.results.iter()
            .flat_map(|r| r.responses.keys())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .cloned()
            .collect();
        
        if envs.len() <= 1 {
            format!("{} Single environment: {}", UiSymbols::ROUTE, envs.get(0).unwrap_or(&"None".to_string()))
        } else {
            format!("{} Data Flow: {} {} {} {} {} → {} Results",
                UiSymbols::COMPARE,
                envs.get(0).unwrap_or(&"Env1".to_string()),
                UiSymbols::FORWARD,
                envs.get(1).unwrap_or(&"Env2".to_string()),
                if envs.len() > 2 { format!("{} {} more", UiSymbols::FORWARD, envs.len() - 2) } else { "".to_string() },
                UiSymbols::DIFF,
                total)
        }
    };
    
    let flow_content = format!("{}\n\n{} {} identical | {} {} different | {} {} errors",
        flow_text,
        UiSymbols::SUCCESS, identical,
        UiSymbols::ERROR, different, 
        UiSymbols::WARNING, errors);
    
    let flow_title = format!("{} Environment Data Flow", UiSymbols::COMPARE);
    let flow = Paragraph::new(flow_content)
        .style(TuiTheme::primary_text_style())
        .alignment(Alignment::Center)
        .block(TuiTheme::normal_block(&flow_title))
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(flow, area);
}

/// Draw smart suggestions based on current results
fn draw_smart_suggestions(f: &mut Frame, app: &TuiApp, area: Rect) {
    let (total, identical, different, errors) = app.get_filter_counts();
    
    let suggestions = if errors > 0 {
        // Analyze error patterns for better suggestions
        let error_analysis = analyze_error_patterns(app);
        
        let mut error_suggestions = vec![
            format!("{} {} errors detected - Press '4' to focus on error analysis", UiSymbols::WARNING, errors),
        ];
        
        if !error_analysis.is_empty() {
            error_suggestions.push(format!("{} Error patterns found: {}", UiSymbols::DETAILS, error_analysis));
        }
        
        error_suggestions.push(format!("{} Press → on error entries to see full details", UiSymbols::TIP));
        error_suggestions
    } else if different > identical {
        vec![
            format!("{} {} differences found - Press '3' to focus on changes", UiSymbols::DIFF, different),
            format!("{} Review API version differences between environments", UiSymbols::TIP),
            format!("{} Check configuration consistency", UiSymbols::SETTINGS),
        ]
    } else if identical == total {
        vec![
            format!("{} All {} responses are identical - Great job!", UiSymbols::SUCCESS, total),
            format!("{} Your environments are consistent", UiSymbols::TIP),
            format!("{} Consider testing edge cases or error scenarios", UiSymbols::QUICK_ACTION),
        ]
    } else {
        vec![
            format!("{} Mixed results: {} identical, {} different", UiSymbols::INFO, identical, different),
            format!("{} Focus on differences with Space or '3' key", UiSymbols::TIP),
            format!("{} Use filters to drill down into specific issues", UiSymbols::DETAILS),
        ]
    };
    
    let suggestions_text = suggestions.join("\n");
    
    let suggestions_title = format!("{} Smart Suggestions", UiSymbols::TIP);
    let suggestions_widget = Paragraph::new(suggestions_text)
        .style(TuiTheme::info_style())
        .block(TuiTheme::normal_block(&suggestions_title))
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(suggestions_widget, area);
}

/// Draw filter options and controls
fn draw_filter_options(f: &mut Frame, app: &TuiApp, area: Rect) {
    let current_filter = match app.filter_state.status_filter {
        crate::renderers::tui::app::StatusFilter::All => "All Results",
        crate::renderers::tui::app::StatusFilter::Identical => "Identical Only",
        crate::renderers::tui::app::StatusFilter::Different => "Different Only", 
        crate::renderers::tui::app::StatusFilter::ErrorsOnly => "Errors Only",
    };
    
    let filter_info = format!(
        "Current Filter: {}\n\n{} Quick Actions:\n• Press 1-4 to switch filter tabs\n• Press 'c' to clear all filters\n• Press ↑↓ to navigate results\n• Press → to view details",
        current_filter,
        UiSymbols::QUICK_ACTION
    );
    
    let filter_title = format!("{} Filter Controls", UiSymbols::SETTINGS);
    let filter_widget = Paragraph::new(filter_info)
        .style(TuiTheme::secondary_text_style())
        .block(TuiTheme::normal_block(&filter_title))
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(filter_widget, area);
}

/// Get simple three-state status display for clean scanning
fn get_simple_status_display(result: &ComparisonResult) -> (String, ratatui::style::Color) {
    if result.has_errors {
        ("⚠ Error".to_string(), TuiTheme::WARNING)
    } else if result.is_identical {
        ("✓ Identical".to_string(), TuiTheme::SUCCESS)
    } else {
        ("✗ Different".to_string(), TuiTheme::ERROR)
    }
}

/// Get meaningful summary for the summary column
fn get_meaningful_summary(result: &ComparisonResult) -> String {
    if result.has_errors {
        extract_error_summary(result)
    } else if result.is_identical {
        "—".to_string()
    } else {
        extract_difference_summary(result)
    }
}

/// Extract meaningful error summary from error information
fn extract_error_summary(result: &ComparisonResult) -> String {
    // Try to get meaningful error message from error_bodies first
    if let Some(ref error_bodies) = result.error_bodies {
        if let Some(first_error) = error_bodies.values().next() {
            // Try to parse JSON and extract meaningful message
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(first_error) {
                // Try multiple common error fields
                let error_fields = ["message", "error", "detail", "description", "reason", "title", "summary"];
                for field in error_fields {
                    if let Some(msg) = json_value.get(field).and_then(|f| f.as_str()) {
                        if !msg.trim().is_empty() {
                            return smart_truncate(msg, 50, None); // 50 chars for table
                        }
                    }
                }
                
                // Try nested error objects
                if let Some(error_obj) = json_value.get("error").and_then(|e| e.as_object()) {
                    for field in error_fields {
                        if let Some(msg) = error_obj.get(field).and_then(|f| f.as_str()) {
                            if !msg.trim().is_empty() {
                                return smart_truncate(msg, 50, None);
                            }
                        }
                    }
                }
            }
            
            // Fallback to first line of error body  
            let preview = first_error
                .lines()
                .next()
                .unwrap_or(first_error)
                .trim();
                
            // Skip empty lines and find meaningful content
            let meaningful_preview = first_error
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or(preview)
                .trim();
                
            return smart_truncate(meaningful_preview, 50, None);
        }
    }
    
    // Fallback to status code analysis
    let error_statuses: Vec<u16> = result.status_codes
        .values()
        .filter(|&&status| status < 200 || status >= 300)
        .copied()
        .collect();
        
    if let Some(&status) = error_statuses.first() {
        match status {
            400 => "Bad request".to_string(),
            401 => "Unauthorized".to_string(),
            403 => "Forbidden".to_string(),
            404 => "Not found".to_string(),
            405 => "Method not allowed".to_string(),
            408 => "Request timeout".to_string(),
            429 => "Rate limited".to_string(),
            500 => "Internal server error".to_string(),
            502 => "Bad gateway".to_string(),
            503 => "Service unavailable".to_string(),
            504 => "Gateway timeout".to_string(),
            _ => format!("HTTP {} error", status),
        }
    } else {
        "Error".to_string()
    }
}

/// Extract meaningful difference summary from differences
fn extract_difference_summary(result: &ComparisonResult) -> String {
    if result.differences.is_empty() {
        return "Differences found".to_string();
    }
    
    let mut categories = std::collections::HashSet::new();
    for diff in &result.differences {
        categories.insert(&diff.category);
    }
    
    // Create more detailed summaries based on available differences
    match categories.len() {
        1 => {
            let category = categories.iter().next().unwrap();
            match category {
                crate::types::DifferenceCategory::Status => {
                    // Try to extract specific status codes from difference descriptions
                    if let Some(diff) = result.differences.iter().find(|d| d.category == crate::types::DifferenceCategory::Status) {
                        if diff.description.len() < 40 {
                            diff.description.clone()
                        } else {
                            "Status codes differ".to_string()
                        }
                    } else {
                        "Status codes differ".to_string()
                    }
                },
                crate::types::DifferenceCategory::Headers => "Headers differ".to_string(), 
                crate::types::DifferenceCategory::Body => "Response body differs".to_string(),
            }
        }
        2 => {
            if categories.contains(&crate::types::DifferenceCategory::Status) && 
               categories.contains(&crate::types::DifferenceCategory::Body) {
                "Status + body differ".to_string()
            } else if categories.contains(&crate::types::DifferenceCategory::Status) {
                "Status + headers differ".to_string()
            } else {
                "Headers + body differ".to_string()
            }
        }
        _ => "Multiple differences".to_string(),
    }
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


/// Get error hint for position indicator when error result is selected
fn get_selected_error_hint(app: &TuiApp) -> Option<String> {
    if let Some(result) = app.current_filtered_result() {
        if result.has_errors {
            // Get meaningful error summary for position indicator (no length limit)
            let error_summary = extract_error_summary_for_context(result);
            
            // Count how many environments have errors
            let error_envs: Vec<&String> = result.status_codes
                .iter()
                .filter(|(_, &status)| status < 200 || status >= 300)
                .map(|(env, _)| env)
                .collect();
            
            if error_envs.len() == 1 {
                Some(format!("{} in {}", error_summary, error_envs[0]))
            } else if error_envs.len() > 1 {
                let env_list = error_envs.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                if error_envs.len() > 3 {
                    Some(format!("{} in {} (+{} more)", env_list, error_summary, error_envs.len() - 3))
                } else {
                    Some(format!("{} in {}", error_summary, env_list))
                }
            } else {
                Some(error_summary)
            }
        } else if !result.is_identical {
            // Show difference summary for non-identical results
            let diff_summary = extract_difference_summary(result);
            Some(diff_summary)
        } else {
            None
        }
    } else {
        None
    }
}

/// Extract error summary for context display (no length limit)
fn extract_error_summary_for_context(result: &ComparisonResult) -> String {
    // Try to get meaningful error message from error_bodies first
    if let Some(ref error_bodies) = result.error_bodies {
        if let Some(first_error) = error_bodies.values().next() {
            // Try to parse JSON and extract meaningful message
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(first_error) {
                // Try multiple common error fields
                let error_fields = ["message", "error", "detail", "description", "reason", "title", "summary"];
                for field in error_fields {
                    if let Some(msg) = json_value.get(field).and_then(|f| f.as_str()) {
                        if !msg.trim().is_empty() {
                            // For context, allow longer messages but still reasonable
                            return smart_truncate(msg, 100, Some(200));
                        }
                    }
                }
                
                // Try nested error objects
                if let Some(error_obj) = json_value.get("error").and_then(|e| e.as_object()) {
                    for field in error_fields {
                        if let Some(msg) = error_obj.get(field).and_then(|f| f.as_str()) {
                            if !msg.trim().is_empty() {
                                return smart_truncate(msg, 100, Some(200));
                            }
                        }
                    }
                }
            }
            
            // Fallback to first meaningful line of error body
            let meaningful_line = first_error
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or(first_error.lines().next().unwrap_or(""))
                .trim();
                
            if !meaningful_line.is_empty() {
                return smart_truncate(meaningful_line, 100, Some(200));
            }
        }
    }
    
    // Fallback to status code analysis
    let error_statuses: Vec<u16> = result.status_codes
        .values()
        .filter(|&&status| status < 200 || status >= 300)
        .copied()
        .collect();
        
    if let Some(&status) = error_statuses.first() {
        match status {
            400 => "Bad request".to_string(),
            401 => "Unauthorized".to_string(),
            403 => "Forbidden".to_string(),
            404 => "Not found".to_string(),
            405 => "Method not allowed".to_string(),
            408 => "Request timeout".to_string(),
            429 => "Rate limited".to_string(),
            500 => "Internal server error".to_string(),
            502 => "Bad gateway".to_string(),
            503 => "Service unavailable".to_string(),
            504 => "Gateway timeout".to_string(),
            _ => format!("HTTP {} error", status),
        }
    } else {
        "Error".to_string()
    }
}

/// Analyze error patterns in results to provide specific insights
fn analyze_error_patterns(app: &TuiApp) -> String {
    let error_results: Vec<&crate::types::ComparisonResult> = app.results
        .iter()
        .filter(|r| r.has_errors)
        .collect();
    
    if error_results.is_empty() {
        return String::new();
    }
    
    // Count different types of errors and environments affected
    let mut status_counts = std::collections::HashMap::new();
    let mut env_errors = std::collections::HashMap::new();
    
    for result in &error_results {
        for (env, &status) in &result.status_codes {
            if status < 200 || status >= 300 {
                *status_counts.entry(status).or_insert(0) += 1;
                *env_errors.entry(env.clone()).or_insert(0) += 1;
            }
        }
    }
    
    // Find most problematic environment
    let most_affected_env = env_errors
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(env, &count)| (env.as_str(), count));
    
    // Generate insight based on most common errors and affected environments
    let most_common = status_counts
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(&status, &count)| (status, count));
    
    if let Some((status, count)) = most_common {
        let status_info = match status {
            400 => format!("{} × Bad Request (check request format)", count),
            401 => format!("{} × Unauthorized (check API keys/tokens)", count),
            403 => format!("{} × Forbidden (check permissions)", count),
            404 => format!("{} × Not Found (verify endpoints exist)", count),
            405 => format!("{} × Method Not Allowed (check HTTP methods)", count),
            408 => format!("{} × Timeout (network or server slow)", count), 
            429 => format!("{} × Rate Limited (reduce request frequency)", count),
            500 => format!("{} × Internal Server Error (check server logs)", count),
            502 => format!("{} × Bad Gateway (proxy/load balancer issues)", count),
            503 => format!("{} × Service Unavailable (service down/overloaded)", count),
            504 => format!("{} × Gateway Timeout (upstream service slow)", count),
            _ => format!("{} × HTTP {} errors", count, status),
        };
        
        if let Some((env, _env_count)) = most_affected_env {
            if env_errors.len() > 1 {
                format!("{} (mostly in {})", status_info, env)
            } else {
                format!("{} (all in {})", status_info, env)
            }
        } else {
            status_info
        }
    } else {
        "Mixed error types across environments".to_string()
    }
}


/// Draw compact results table for dashboard results panel
fn draw_compact_results_table(f: &mut Frame, app: &TuiApp, area: Rect, _is_panel_focused: bool) {
    if area.height < 3 { return; } // Too small to render
    
    let filtered_results = app.filtered_results();
    if filtered_results.is_empty() {
        return;
    }
    
    // Create a simple table with just route name and status
    let header = Row::new(vec!["Route", "Status"])
        .style(TuiTheme::primary_text_style())
        .height(1);
    
    let rows: Vec<Row> = filtered_results
        .iter()
        .enumerate()
        .take(area.height.saturating_sub(2) as usize) // Reserve space for header and border
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
            
            Row::new(vec![
                smart_truncate(&result.route_name, 15, None),
                status,
            ]).style(style)
        })
        .collect();
    
    let table = Table::new(rows, [Constraint::Percentage(70), Constraint::Percentage(30)])
        .header(header);
    
    f.render_widget(table, area);
}

