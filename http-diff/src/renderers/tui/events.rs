use super::app::TuiApp;
use super::msg::Msg;
use crate::error::{HttpDiffError, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

mod events_dashboard;

/// Poll for input and translate to a top-level Msg for reducer
pub fn next_msg(app: &TuiApp) -> Result<Option<Msg>> {
    // Check for events with a timeout (16ms = ~60fps for responsive UI)
    if event::poll(Duration::from_millis(16))
        .map_err(|e| HttpDiffError::general(format!("Failed to poll events: {}", e)))?
    {
        match event::read()
            .map_err(|e| HttpDiffError::general(format!("Failed to read event: {}", e)))?
        {
            Event::Key(key_event) => {
                return Ok(handle_key_event(app, key_event));
            }
            Event::Resize(_, _) => {
                // Terminal resize - just continue, ratatui handles this automatically
                return Ok(None);
            }
            _ => {
                // Other events (mouse, etc.) - ignore for now
                return Ok(None);
            }
        }
    }

    Ok(None)
}

/// Handle keyboard input events
fn handle_key_event(app: &TuiApp, key: KeyEvent) -> Option<Msg> {
    // Global key handlers (work in all views)
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Some(Msg::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Msg::Quit)
        }
        KeyCode::Char('d') => return Some(Msg::ToggleDiffStyle),
        KeyCode::Char('h') => return Some(Msg::ToggleHeaders),
        KeyCode::Char('e') => return Some(Msg::ToggleErrors),
        KeyCode::Char('x') | KeyCode::Char('X') => {
            return Some(Msg::ToggleExpanded(app.panel_focus.clone()))
        }
        KeyCode::Tab => return Some(Msg::FocusNextPane),
        KeyCode::BackTab => return Some(Msg::FocusPrevPane),
        KeyCode::F(1) => return Some(Msg::ToggleHelp),
        _ => {}
    }

    // View-specific key handlers - only Dashboard mode is supported
    events_dashboard::map_dashboard_keys_to_msg(app, key)
}
