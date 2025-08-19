use crate::renderers::tui::app::TuiApp;
use crate::renderers::tui::{
    app::PanelFocus,
    theme::{TuiTheme, UiSymbols},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs},
};

use super::draw_scrollbar;

pub fn draw_dashboard_results_panel(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let is_focused = app.is_panel_focused(&PanelFocus::Results);
    let title = app.get_panel_title(&PanelFocus::Results);
    let has_content = !app.results.is_empty();
    let has_activity = app.filter_state.status_filter
        != crate::renderers::tui::app::StatusFilter::All;

    let block = TuiTheme::panel_block(&title, is_focused, has_content, has_activity);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.results.is_empty() {
        let empty_text =
            "No results yet\n\nRun tests from the\nConfiguration panel\nto see results here";
        let empty_para = Paragraph::new(empty_text)
            .style(TuiTheme::secondary_text_style())
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty_para, inner_area);
        return;
    }

    if inner_area.height < 6 {
        return;
    }

    // Split the inner area to make room for filter tabs
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter tabs
            Constraint::Min(3),    // Results table (minimum 3 lines)
        ])
        .split(inner_area);

    // Render filter tabs
    render_filter_tabs(f, app, chunks[0]);

    let filtered_results = app.filtered_results();
    if filtered_results.is_empty() {
        let empty_text = "No results match current filter";
        let empty_para = Paragraph::new(empty_text)
            .style(TuiTheme::secondary_text_style())
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty_para, chunks[1]);
        return;
    }
    let results_count = filtered_results.len();

    let header = Row::new(vec!["Route", "Status"]) // compact
        .style(TuiTheme::primary_text_style())
        .height(1);

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

            Row::new(vec![smart_truncate(&result.route_name, 15), status]).style(style)
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
    )
    .block(Block::default().borders(Borders::NONE));

    f.render_stateful_widget(table, chunks[1], &mut app.results_table_state);

    let viewport_height = chunks[1].height.saturating_sub(3) as usize;
    draw_scrollbar(
        f,
        &mut app.results_scrollbar_state,
        chunks[1],
        results_count,
        viewport_height,
    );
}

fn render_filter_tabs(f: &mut Frame, app: &TuiApp, area: Rect) {
    let (total, identical, different, errors) = app.get_filter_counts();

    let tab_titles = vec![
        format!("All ({})", total),
        format!("✓ Identical ({})", identical),
        format!("⚠ Different ({})", different),
        format!("✗ Errors ({})", errors),
    ];

    let tabs = Tabs::new(tab_titles)
        .select(app.filter_state.current_tab)
        .style(TuiTheme::secondary_text_style())
        .highlight_style(TuiTheme::focused_style())
        .divider(" | ")
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM));

    f.render_widget(tabs, area);
}

fn smart_truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    if let Some(pos) = text[..limit.saturating_sub(3)].rfind(' ') {
        format!("{}...", &text[..pos])
    } else {
        format!("{}...", &text[..limit.saturating_sub(3)])
    }
}
