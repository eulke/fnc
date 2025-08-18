use crate::renderers::tui::{
    app::{ActionFeedback, FeedbackType, PanelFocus, TuiApp},
    theme::{KeyHints, TuiTheme, UiSymbols},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::Modifier,
    widgets::{Block, Borders, Paragraph},
};

pub mod config;
pub mod details;
pub mod progress;
pub mod results;

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
        PanelFocus::Configuration => ("Configuration Panel", KeyHints::configuration_panel_help()),
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
fn draw_feedback_popup(f: &mut Frame, feedback: &ActionFeedback) {
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
    if let Some(expanded_panel) = app.expanded_panel.as_ref() {
        // Show only the expanded panel
        match expanded_panel {
            PanelFocus::Configuration => config::draw_dashboard_configuration_panel(f, app, area),
            PanelFocus::Progress => progress::draw_dashboard_progress_panel(f, app, area),
            PanelFocus::Results => results::draw_dashboard_results_panel(f, app, area),
            PanelFocus::Details => details::draw_dashboard_details_panel(f, app, area),
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
        config::draw_dashboard_configuration_panel(f, app, top_chunks[0]);
        progress::draw_dashboard_progress_panel(f, app, top_chunks[1]);
        results::draw_dashboard_results_panel(f, app, bottom_chunks[0]);
        details::draw_dashboard_details_panel(f, app, bottom_chunks[1]);
    }
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

/// Draw a scrollbar for a scrollable component
pub(super) fn draw_scrollbar(
    f: &mut Frame,
    scrollbar_state: &mut ratatui::widgets::ScrollbarState,
    area: Rect,
    content_length: usize,
    viewport_height: usize,
) {
    use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
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
