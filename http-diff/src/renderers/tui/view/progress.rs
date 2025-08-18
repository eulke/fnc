use crate::renderers::tui::app::TuiApp;
use crate::renderers::tui::{app::PanelFocus, theme::TuiTheme};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Modifier, Style},
    widgets::{BarChart, Block, Borders, Gauge, Paragraph},
};

pub fn draw_dashboard_progress_panel(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let is_focused = app.is_panel_focused(&PanelFocus::Progress);
    let title = app.get_panel_title(&PanelFocus::Progress);
    let has_content = !app.results.is_empty() || app.progress_tracker.as_ref().map_or(false, |t| t.total_requests > 0);
    let has_activity = app.execution_running;

    let block = TuiTheme::panel_block(&title, is_focused, has_content, has_activity);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if app.execution_running {
        draw_progress_execution_view(f, app, inner_area);
    } else if !app.results.is_empty() {
        draw_progress_results_summary(f, app, inner_area);
    } else {
        draw_progress_ready_state(f, inner_area);
    }
}

fn draw_progress_execution_view(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress gauge
            Constraint::Length(3), // Statistics line
            Constraint::Min(1),    // Current operation
        ])
        .split(area);

    let progress_value = if let Some(ref tracker) = app.progress_tracker {
        tracker.progress_percentage().clamp(0.0, 100.0) as u16
    } else {
        0
    };

    let progress_gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(TuiTheme::FOCUS))
        .percent(progress_value)
        .label(if let Some(ref tracker) = app.progress_tracker {
            format!(
                "{}/{} tests ({}%)",
                tracker.completed_requests, tracker.total_requests, progress_value
            )
        } else {
            "Starting...".to_string()
        });

    f.render_widget(progress_gauge, chunks[0]);

    let (elapsed, rate) = if let Some(ref tracker) = app.progress_tracker {
        let elapsed_time = tracker.elapsed_time();
        let elapsed_str = format!("{:.1}s", elapsed_time.as_secs_f64());
        
        let rate_str = if tracker.completed_requests > 0 {
            let elapsed_secs = elapsed_time.as_secs_f64();
            if elapsed_secs > 0.0 {
                format!("{:.1} tests/s", tracker.completed_requests as f64 / elapsed_secs)
            } else {
                "calculating...".to_string()
            }
        } else {
            "starting...".to_string()
        };
        
        (elapsed_str, rate_str)
    } else {
        ("0.0s".to_string(), "starting...".to_string())
    };

    let stats_text = format!("⏱ {elapsed} | 🚀 {rate}");
    let stats_para = Paragraph::new(stats_text)
        .style(TuiTheme::secondary_text_style())
        .alignment(Alignment::Center);
    f.render_widget(stats_para, chunks[1]);

    let operation_para = Paragraph::new(app.current_operation.as_str())
        .style(TuiTheme::primary_text_style())
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(operation_para, chunks[2]);
}

fn draw_progress_results_summary(f: &mut Frame, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status line
            Constraint::Min(5),    // Bar chart
            Constraint::Length(3), // Error summary
        ])
        .split(area);

    let status_text = "✅ Execution Complete";
    let status_para = Paragraph::new(status_text)
        .style(TuiTheme::success_style())
        .alignment(Alignment::Center);
    f.render_widget(status_para, chunks[0]);

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

fn draw_progress_ready_state(f: &mut Frame, area: Rect) {
    let ready_text =
        "🚀 Ready to Execute\n\n1. Select environments\n2. Select routes\n3. Press 'R' to start";
    let ready_para = Paragraph::new(ready_text)
        .style(TuiTheme::secondary_text_style())
        .alignment(Alignment::Center);
    f.render_widget(ready_para, area);
}
