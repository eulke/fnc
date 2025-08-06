use super::AppResult;
use crate::{
    error::Result,
    renderers::tui::app::{PanelFocus, TuiApp},
};
use crossterm::event::{KeyCode, KeyEvent};

/// Handle keys for dashboard view with panel-focused navigation
pub fn handle_dashboard_keys(app: &mut TuiApp, key: KeyEvent) -> Result<AppResult> {
    match key.code {
        // Tab navigation between panels
        KeyCode::Tab => {
            app.next_dashboard_panel();
        }
        KeyCode::BackTab => {
            app.previous_dashboard_panel();
        }
        // Panel-specific navigation and actions
        KeyCode::Up | KeyCode::Char('k') => {
            handle_dashboard_panel_up(app);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            handle_dashboard_panel_down(app);
        }
        KeyCode::Left | KeyCode::Char('h') => {
            handle_dashboard_panel_left(app);
        }
        KeyCode::Right | KeyCode::Char('l') => {
            handle_dashboard_panel_right(app);
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            handle_dashboard_panel_activate(app)?;
        }
        KeyCode::PageUp => {
            handle_dashboard_page_up(app);
        }
        KeyCode::PageDown => {
            handle_dashboard_page_down(app);
        }
        // Panel-specific shortcuts
        KeyCode::Char('a') => {
            if app.panel_focus == PanelFocus::Configuration {
                app.select_all_focused();
            }
        }
        KeyCode::Char('n') => {
            if app.panel_focus == PanelFocus::Configuration {
                app.clear_all_focused();
            }
        }
        KeyCode::Char('f') => {
            if app.panel_focus == PanelFocus::Results {
                app.toggle_filter_panel();
            }
        }
        KeyCode::Char('c') => {
            if app.panel_focus == PanelFocus::Results {
                app.clear_filters();
            }
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Toggle panel expansion
            app.toggle_panel_expansion(app.panel_focus.clone());
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Execute tests from any panel (main execution trigger)
            if !app.selected_environments.is_empty() && !app.selected_routes.is_empty() {
                app.request_execution();
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Save HTML report from any panel
            match app.generate_html_report() {
                Ok(_) => {
                    // Success feedback is handled by the method
                }
                Err(err) => {
                    app.show_feedback(&err, crate::renderers::tui::app::FeedbackType::Error);
                }
            }
        }
        // Details panel specific keys
        KeyCode::Char('1') => {
            if app.panel_focus == PanelFocus::Details {
                app.switch_details_tab(1);
            } else if app.panel_focus == PanelFocus::Results {
                // Filter shortcuts for results panel
                handle_results_filter_shortcut(app, key);
            }
        }
        KeyCode::Char('2') => {
            if app.panel_focus == PanelFocus::Details {
                app.switch_details_tab(2);
            } else if app.panel_focus == PanelFocus::Results {
                handle_results_filter_shortcut(app, key);
            }
        }
        KeyCode::Char('3') => {
            if app.panel_focus == PanelFocus::Details {
                app.switch_details_tab(3);
            } else if app.panel_focus == PanelFocus::Results {
                handle_results_filter_shortcut(app, key);
            }
        }
        KeyCode::Char('4') => {
            if app.panel_focus == PanelFocus::Details {
                app.switch_details_tab(4);
            } else if app.panel_focus == PanelFocus::Results {
                handle_results_filter_shortcut(app, key);
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if app.panel_focus == PanelFocus::Details {
                // Toggle diff style in details panel
                app.toggle_details_diff_style();
            } else {
                // Global diff style toggle (existing behavior)
                app.toggle_diff_style();
            }
        }
        _ => {}
    }

    Ok(AppResult::Continue)
}

/// Handle up navigation within the focused dashboard panel
fn handle_dashboard_panel_up(app: &mut TuiApp) {
    match app.panel_focus {
        PanelFocus::Configuration => {
            if matches!(
                app.focused_panel,
                crate::renderers::tui::app::FocusedPanel::Environments
            ) {
                app.navigate_up();
            } else if matches!(
                app.focused_panel,
                crate::renderers::tui::app::FocusedPanel::Routes
            ) {
                app.navigate_up();
            }
        }
        PanelFocus::Progress => {
            // No up/down navigation in progress panel
        }
        PanelFocus::Results => {
            app.previous_result();
        }
        PanelFocus::Details => {
            app.scroll_up();
        }
    }
}

/// Handle down navigation within the focused dashboard panel
fn handle_dashboard_panel_down(app: &mut TuiApp) {
    match app.panel_focus {
        PanelFocus::Configuration => {
            if matches!(
                app.focused_panel,
                crate::renderers::tui::app::FocusedPanel::Environments
            ) {
                app.navigate_down();
            } else if matches!(
                app.focused_panel,
                crate::renderers::tui::app::FocusedPanel::Routes
            ) {
                app.navigate_down();
            }
        }
        PanelFocus::Progress => {
            // No up/down navigation in progress panel
        }
        PanelFocus::Results => {
            app.next_result();
        }
        PanelFocus::Details => {
            app.scroll_down();
        }
    }
}

/// Handle left navigation within dashboard panels (intra-panel only)
fn handle_dashboard_panel_left(app: &mut TuiApp) {
    match app.panel_focus {
        PanelFocus::Configuration => {
            // Switch between environments/routes within configuration panel
            app.previous_panel();
        }
        PanelFocus::Progress => {
            // No left navigation within progress panel
        }
        PanelFocus::Results => {
            // No left navigation within results panel
        }
        PanelFocus::Details => {
            // No left navigation within details panel
        }
    }
}

/// Handle right navigation within dashboard panels (intra-panel only)
fn handle_dashboard_panel_right(app: &mut TuiApp) {
    match app.panel_focus {
        PanelFocus::Configuration => {
            // Switch between environments/routes within configuration panel
            app.next_panel();
        }
        PanelFocus::Progress => {
            // No right navigation within progress panel
        }
        PanelFocus::Results => {
            // No right navigation within results panel
        }
        PanelFocus::Details => {
            // No right navigation within details panel
        }
    }
}

/// Handle activation (Enter/Space) within the focused dashboard panel
fn handle_dashboard_panel_activate(app: &mut TuiApp) -> Result<()> {
    match app.panel_focus {
        PanelFocus::Configuration => {
            // Handle configuration actions
            if app.available_environments.is_empty() {
                // Load configuration
                match app.load_configuration() {
                    Ok(()) => app.clear_error(),
                    Err(e) => app.set_error(e),
                }
            } else {
                // Toggle focused item (removed execution trigger - now use R key)
                app.toggle_focused_item();
            }
        }
        PanelFocus::Progress => {
            // No activation actions in progress panel
        }
        PanelFocus::Results => {
            // Select current result for details view
            if app.current_filtered_result().is_some() {
                app.panel_focus = PanelFocus::Details;
            }
        }
        PanelFocus::Details => {
            // No activation actions in details panel currently
        }
    }
    Ok(())
}

/// Handle page up within the focused dashboard panel
fn handle_dashboard_page_up(app: &mut TuiApp) {
    match app.panel_focus {
        PanelFocus::Configuration => {
            // No page navigation in configuration
        }
        PanelFocus::Progress => {
            // No page navigation in progress
        }
        PanelFocus::Results => {
            app.selected_index = app.selected_index.saturating_sub(10);
        }
        PanelFocus::Details => {
            app.page_up();
        }
    }
}

/// Handle page down within the focused dashboard panel
fn handle_dashboard_page_down(app: &mut TuiApp) {
    match app.panel_focus {
        PanelFocus::Configuration => {
            // No page navigation in configuration
        }
        PanelFocus::Progress => {
            // No page navigation in progress
        }
        PanelFocus::Results => {
            let filtered_results = app.filtered_results();
            let max_index = filtered_results.len().saturating_sub(1);
            app.selected_index = (app.selected_index + 10).min(max_index);
        }
        PanelFocus::Details => {
            app.page_down();
        }
    }
}

/// Handle filter shortcuts for results panel
fn handle_results_filter_shortcut(app: &mut TuiApp, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        match c {
            '1' => {
                app.filter_state.current_tab = 0;
                app.filter_state.status_filter = crate::renderers::tui::app::StatusFilter::All;
                app.selected_index = 0;
            }
            '2' => {
                app.filter_state.current_tab = 1;
                app.filter_state.status_filter =
                    crate::renderers::tui::app::StatusFilter::Identical;
                app.selected_index = 0;
            }
            '3' => {
                app.filter_state.current_tab = 2;
                app.filter_state.status_filter =
                    crate::renderers::tui::app::StatusFilter::Different;
                app.selected_index = 0;
            }
            '4' => {
                app.filter_state.current_tab = 3;
                app.filter_state.status_filter =
                    crate::renderers::tui::app::StatusFilter::ErrorsOnly;
                app.selected_index = 0;
            }
            _ => {}
        }
    }
}
