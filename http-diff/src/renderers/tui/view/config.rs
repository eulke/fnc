use crate::renderers::tui::{
    app::{FocusedPanel, PanelFocus, TuiApp},
    theme::TuiTheme,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Modifier, Style},
    widgets::{List, ListItem, Paragraph},
};

use super::draw_scrollbar;

pub fn draw_dashboard_configuration_panel(f: &mut Frame, app: &mut TuiApp, area: Rect) {
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
        return;
    }

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
            let selected = app.is_environment_selected(i);
            let checkbox = if selected { "☑" } else { "☐" };
            let text = format!("{} {}", checkbox, env);
            let style = if selected {
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

    let is_env_focused =
        is_panel_focused && matches!(app.focused_panel, FocusedPanel::Environments);
    let title_text = format!("Environments ({}/{})", selected_count, total_count);
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

    f.render_stateful_widget(env_list, area, &mut app.env_list_state);

    let viewport_height = area.height.saturating_sub(2) as usize;
    draw_scrollbar(
        f,
        &mut app.env_scrollbar_state,
        area,
        app.available_environments.len(),
        viewport_height,
    );
}

fn draw_routes_list_widget(f: &mut Frame, app: &mut TuiApp, area: Rect, is_panel_focused: bool) {
    let route_items: Vec<ListItem> = app
        .available_routes
        .iter()
        .enumerate()
        .map(|(i, route)| {
            let selected = app.is_route_selected(i);
            let checkbox = if selected { "☑" } else { "☐" };
            let text = format!("{} {}", checkbox, route);
            let style = if selected {
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

    let is_route_focused = is_panel_focused && matches!(app.focused_panel, FocusedPanel::Routes);
    let title_text = format!("Routes ({}/{})", selected_count, total_count);
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

    f.render_stateful_widget(route_list, area, &mut app.route_list_state);

    let viewport_height = area.height.saturating_sub(2) as usize;
    draw_scrollbar(
        f,
        &mut app.route_scrollbar_state,
        area,
        app.available_routes.len(),
        viewport_height,
    );
}

fn draw_config_status_line(f: &mut Frame, app: &TuiApp, area: Rect) {
    let current_section = match app.focused_panel {
        FocusedPanel::Environments => "📝 Environments",
        FocusedPanel::Routes => "🛣 Routes",
        FocusedPanel::Actions => "⚡ Actions",
    };

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
