use super::app::TuiApp;
use crate::{
    error::{HttpDiffError, Result},
    renderers::tui::app::ViewMode,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

mod events_dashboard;

/// Result of handling an application event
pub enum AppResult {
    /// Continue running the application
    Continue,
    /// Quit the application
    Quit,
}

/// Handle application events (keyboard input, etc.)
pub fn handle_events(app: &mut TuiApp) -> Result<Option<AppResult>> {
    // Check for events with a timeout
    if event::poll(Duration::from_millis(100))
        .map_err(|e| HttpDiffError::general(format!("Failed to poll events: {}", e)))?
    {
        match event::read()
            .map_err(|e| HttpDiffError::general(format!("Failed to read event: {}", e)))?
        {
            Event::Key(key_event) => {
                return Ok(Some(handle_key_event(app, key_event)?));
            }
            Event::Resize(_, _) => {
                // Terminal resize - just continue, ratatui handles this automatically
                return Ok(Some(AppResult::Continue));
            }
            _ => {
                // Other events (mouse, etc.) - ignore for now
                return Ok(Some(AppResult::Continue));
            }
        }
    }

    Ok(None)
}

/// Handle keyboard input events
fn handle_key_event(app: &mut TuiApp, key: KeyEvent) -> Result<AppResult> {
    // Global key handlers (work in all views)
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.quit();
            return Ok(AppResult::Quit);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit();
            return Ok(AppResult::Quit);
        }
        KeyCode::Tab => {
            // In dashboard and configuration views, Tab switches panels instead of views
            if matches!(app.view_mode, ViewMode::Configuration | ViewMode::Dashboard) {
                // Handle in view-specific section (will pass to dashboard handler)
            } else {
                app.next_view();
                return Ok(AppResult::Continue);
            }
        }
        KeyCode::BackTab => {
            // In dashboard and configuration views, BackTab switches panels instead of views
            if matches!(app.view_mode, ViewMode::Configuration | ViewMode::Dashboard) {
                // Handle in view-specific section (will pass to dashboard handler)
            } else {
                app.previous_view();
                return Ok(AppResult::Continue);
            }
        }
        KeyCode::Char('d') => {
            app.toggle_diff_style();
            return Ok(AppResult::Continue);
        }
        KeyCode::Char('h') => {
            app.toggle_headers();
            return Ok(AppResult::Continue);
        }
        KeyCode::Char('e') => {
            app.toggle_errors();
            return Ok(AppResult::Continue);
        }
        _ => {}
    }

    // View-specific key handlers
    match app.view_mode {
        ViewMode::Configuration => handle_configuration_input(app, key),
        ViewMode::Execution => handle_execution_input(app, key),
        ViewMode::ResultsList => handle_results_list_keys(app, key),
        ViewMode::ResultDetail => handle_result_detail_keys(app, key),
        ViewMode::DiffView => handle_diff_view_keys(app, key),
        ViewMode::Dashboard => events_dashboard::handle_dashboard_keys(app, key),
    }
}

/// Handle keys for configuration view
fn handle_configuration_input(app: &mut TuiApp, key: KeyEvent) -> Result<AppResult> {
    match key.code {
        KeyCode::Tab => {
            app.next_panel();
        }
        KeyCode::BackTab => {
            app.previous_panel();
        }
        KeyCode::Up => {
            app.navigate_up();
        }
        KeyCode::Down => {
            app.navigate_down();
        }
        KeyCode::Enter => {
            // Try to load configuration if not loaded, then start execution
            if app.available_environments.is_empty() {
                match app.load_configuration() {
                    Ok(()) => app.clear_error(),
                    Err(e) => {
                        app.set_error(e);
                        return Ok(AppResult::Continue);
                    }
                }
            } else {
                app.toggle_focused_item();
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_focused_item();
        }
        KeyCode::Char('a') => {
            app.select_all_focused();
        }
        KeyCode::Char('n') => {
            app.clear_all_focused();
        }
        KeyCode::F(1) => {
            app.toggle_help();
        }
        KeyCode::Char('i') => {
            // Initialize configuration files
            app.set_error("Config initialization not yet implemented".to_string());
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            // Toggle environment by number (legacy support)
            let index = (c as usize).saturating_sub('0' as usize);
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.toggle_route(index);
            } else {
                app.toggle_environment(index);
            }
        }
        _ => {}
    }
    Ok(AppResult::Continue)
}

/// Handle keys for execution view
fn handle_execution_input(app: &mut TuiApp, key: KeyEvent) -> Result<AppResult> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Cancel execution and return to configuration
            app.cancel_execution();
        }
        _ => {
            // Most keys are disabled during execution
        }
    }
    Ok(AppResult::Continue)
}

/// Handle keys specific to the results list view
fn handle_results_list_keys(app: &mut TuiApp, key: KeyEvent) -> Result<AppResult> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.previous_result();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_result();
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            // Use filtered result instead of raw result
            if app.current_filtered_result().is_some() {
                app.view_mode = ViewMode::ResultDetail;
            }
        }
        KeyCode::Char(' ') => {
            // Space bar goes directly to diff view if there are differences
            if let Some(result) = app.current_filtered_result() {
                if !result.is_identical {
                    app.view_mode = ViewMode::DiffView;
                } else {
                    app.view_mode = ViewMode::ResultDetail;
                }
            }
        }
        KeyCode::Home => {
            app.selected_index = 0;
        }
        KeyCode::End => {
            let filtered_results = app.filtered_results();
            app.selected_index = filtered_results.len().saturating_sub(1);
        }
        KeyCode::PageUp => {
            app.selected_index = app.selected_index.saturating_sub(10);
        }
        KeyCode::PageDown => {
            let filtered_results = app.filtered_results();
            let max_index = filtered_results.len().saturating_sub(1);
            app.selected_index = (app.selected_index + 10).min(max_index);
        }
        // Filter tab navigation
        KeyCode::Char('1') => {
            app.filter_state.current_tab = 0;
            app.filter_state.status_filter = crate::renderers::tui::app::StatusFilter::All;
            app.selected_index = 0;
        }
        KeyCode::Char('2') => {
            app.filter_state.current_tab = 1;
            app.filter_state.status_filter = crate::renderers::tui::app::StatusFilter::Identical;
            app.selected_index = 0;
        }
        KeyCode::Char('3') => {
            app.filter_state.current_tab = 2;
            app.filter_state.status_filter = crate::renderers::tui::app::StatusFilter::Different;
            app.selected_index = 0;
        }
        KeyCode::Char('4') => {
            app.filter_state.current_tab = 3;
            app.filter_state.status_filter = crate::renderers::tui::app::StatusFilter::ErrorsOnly;
            app.selected_index = 0;
        }
        KeyCode::Char('f') => {
            // Toggle filter panel for advanced filtering
            app.toggle_filter_panel();
        }
        KeyCode::Char('c') => {
            // Clear all filters
            app.clear_filters();
        }
        _ => {}
    }

    Ok(AppResult::Continue)
}

/// Handle keys specific to the result detail view
fn handle_result_detail_keys(app: &mut TuiApp, key: KeyEvent) -> Result<AppResult> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.scroll_up();
            } else {
                app.previous_result();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.scroll_down();
            } else {
                app.next_result();
            }
        }
        KeyCode::Left | KeyCode::Backspace => {
            app.view_mode = ViewMode::ResultsList;
        }
        KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') => {
            if app.current_filtered_result().is_some() {
                app.view_mode = ViewMode::DiffView;
            }
        }
        KeyCode::PageUp => {
            app.page_up();
        }
        KeyCode::PageDown => {
            app.page_down();
        }
        KeyCode::Home => {
            app.scroll_to_top();
        }
        KeyCode::End => {
            // This would need content height to be accurate
            app.scroll_to_bottom(100); // Placeholder value
        }
        _ => {}
    }

    Ok(AppResult::Continue)
}

/// Handle keys specific to the diff view
fn handle_diff_view_keys(app: &mut TuiApp, key: KeyEvent) -> Result<AppResult> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.scroll_up();
            } else {
                app.previous_result();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.scroll_down();
            } else {
                app.next_result();
            }
        }
        KeyCode::Left | KeyCode::Backspace => {
            app.view_mode = ViewMode::ResultDetail;
        }
        KeyCode::PageUp => {
            app.page_up();
        }
        KeyCode::PageDown => {
            app.page_down();
        }
        KeyCode::Home => {
            app.scroll_to_top();
        }
        KeyCode::End => {
            // This would need content height to be accurate
            app.scroll_to_bottom(100); // Placeholder value
        }
        KeyCode::Char('1') => {
            // Quick switch to first result
            app.selected_index = 0;
        }
        KeyCode::Char('2'..='9') => {
            // Quick switch to numbered result
            if let Some(digit) = key.code.to_string().chars().next() {
                if let Some(index) = digit.to_digit(10) {
                    let target_index = (index as usize).saturating_sub(1);
                    if target_index < app.results.len() {
                        app.selected_index = target_index;
                    }
                }
            }
        }
        _ => {}
    }

    Ok(AppResult::Continue)
}
