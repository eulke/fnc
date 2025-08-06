use super::app::TuiApp;
use crate::error::{HttpDiffError, Result};
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
            // Tab switches panels in Dashboard mode
            // Handle in view-specific section (will pass to dashboard handler)
        }
        KeyCode::BackTab => {
            // BackTab switches panels in Dashboard mode
            // Handle in view-specific section (will pass to dashboard handler)
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

    // View-specific key handlers - only Dashboard mode is supported
    events_dashboard::handle_dashboard_keys(app, key)
}





