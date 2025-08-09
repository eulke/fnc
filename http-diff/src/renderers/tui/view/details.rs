use crate::renderers::tui::{app::{DetailsTab, PanelFocus, TuiApp}, theme::{TuiTheme, UiSymbols}};
use crate::types::ComparisonResult;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
};

pub fn draw_dashboard_details_panel(f: &mut Frame, app: &TuiApp, area: Rect) {
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

fn draw_detailed_result_with_tabs(
    f: &mut Frame,
    app: &TuiApp,
    result: &ComparisonResult,
    area: Rect,
    is_focused: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let tab_titles = vec!["📋 Overview", "🔍 Diffs", "⚠ Errors", "💡 Tips"];
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL))
        .select(app.details_current_tab.as_index())
        .style(TuiTheme::secondary_text_style())
        .highlight_style(if is_focused { TuiTheme::focused_style() } else { Style::default().fg(TuiTheme::FOCUS).add_modifier(Modifier::BOLD) });

    f.render_widget(tabs, chunks[0]);

    match app.details_current_tab {
        DetailsTab::Overview => draw_details_overview_tab(f, app, result, chunks[1]),
        DetailsTab::Diffs => draw_details_diffs_tab(f, app, result, chunks[1]),
        DetailsTab::Errors => draw_details_errors_tab(f, app, result, chunks[1]),
        DetailsTab::Suggestions => draw_details_suggestions_tab(f, app, result, chunks[1]),
    }
}

fn draw_details_overview_tab(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect) {
    let mut lines = vec![
        format!("🛣 Route: {}", result.route_name),
        "".to_string(),
        format!(
            "📊 Status: {}",
            if result.has_errors { "❌ Errors detected" } else if result.is_identical { "✅ All responses identical" } else { "⚠ Responses differ" }
        ),
        "".to_string(),
    ];

    lines.push("🌍 Environments:".to_string());
    for (env, response) in &result.responses {
        let status_icon = if response.is_success() { "✅" } else { "❌" };
        lines.push(format!("  {} {} - HTTP {}", status_icon, env, response.status));
    }

    if !result.user_context.is_empty() {
        lines.push("".to_string());
        lines.push("Variables".to_string());
        lines.push("══════════".to_string());
        let mut vars: Vec<_> = result.user_context.iter().collect();
        vars.sort_by_key(|(k, _)| *k);
        for (k, v) in vars {
            let val = if v.len() > 80 { format!("{}...", &v[..77]) } else { v.clone() };
            lines.push(format!("  {} = {}", k, val));
        }
    }

    if !result.responses.is_empty() {
        lines.push("".to_string());
        lines.push("📏 Response Sizes:".to_string());
        for (env, response) in &result.responses {
            lines.push(format!("  {} - {} bytes, {} lines", env, response.body.len(), response.line_count()));
        }
    }

    if !result.is_identical && !result.has_errors {
        lines.push("".to_string());
        lines.push(format!("🔍 {} differences found", result.differences.len()));
        let mut categories = std::collections::HashSet::new();
        for diff in &result.differences { categories.insert(&diff.category); }
        for category in categories { lines.push(format!("  • {}", category.name())); }
    }

    let overview_text = lines.join("\n");
    let overview_para = Paragraph::new(overview_text)
        .style(TuiTheme::primary_text_style())
        .scroll((app.scroll_offset as u16, 0))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(overview_para, area);
}

fn draw_details_diffs_tab(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect) {
    if result.differences.is_empty() {
        let no_diffs = "✅ No differences found\n\nAll responses are identical\nacross environments.";
        let para = Paragraph::new(no_diffs).style(TuiTheme::success_style()).alignment(Alignment::Center);
        f.render_widget(para, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    let style_text = format!(
        "📝 {} View (Press D to toggle)",
        match app.details_diff_style { crate::types::DiffViewStyle::Unified => "Unified", crate::types::DiffViewStyle::SideBySide => "Side-by-Side" }
    );
    let style_para = Paragraph::new(style_text).style(TuiTheme::info_style());
    f.render_widget(style_para, chunks[0]);

    use crate::renderers::{diff_processor::DiffProcessor, tui::diff_widgets::DiffWidgetRenderer};
    let processor = DiffProcessor::new();
    match processor.process_comparison_result(result, app.show_headers) {
        Ok(diff_data) => {
            DiffWidgetRenderer::render_diff_view_with_style(f, &diff_data, app, &app.details_diff_style, chunks[1]);
        }
        Err(e) => {
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

fn draw_details_errors_tab(f: &mut Frame, app: &TuiApp, result: &ComparisonResult, area: Rect) {
    if !result.has_errors {
        let no_errors = "✅ No errors detected\n\nAll requests completed\nsuccessfully.";
        let para = Paragraph::new(no_errors).style(TuiTheme::success_style()).alignment(Alignment::Center);
        f.render_widget(para, area);
        return;
    }

    let mut error_lines = vec!["❌ Error Details:".to_string(), "".to_string()];
    for (env, &status) in &result.status_codes {
        if !((200..300).contains(&status)) {
            error_lines.push(format!("🌍 Environment: {}", env));
            error_lines.push(format!("📊 HTTP Status: {}", status));
            if let Some(error_bodies) = &result.error_bodies {
                if let Some(body) = error_bodies.get(env) {
                    error_lines.push("📄 Response Body:".to_string());
                    for line in body.lines().take(5) { error_lines.push(format!("  {}", line)); }
                    if body.lines().count() > 5 { error_lines.push("  ... (truncated)".to_string()); }
                }
            }
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

fn draw_details_suggestions_tab(
    f: &mut Frame,
    app: &TuiApp,
    result: &ComparisonResult,
    area: Rect,
) {
    let mut suggestions = vec!["💡 Suggestions & Recommendations:".to_string(), "".to_string()];

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
                    502..=504 => suggestions.push(format!("  • {} - Service may be unavailable, try again later", env)),
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


