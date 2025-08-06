//! Rich TUI diff widgets using proper ratatui components
//!
//! This module provides widget-based rendering for diff data using native
//! ratatui widgets with proper styling, colors, and interactivity.

use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    style::Modifier,
    widgets::{Cell, List, ListItem, Paragraph, Row, Table, Wrap},
};

use super::{
    app::TuiApp,
    theme::{TuiTheme, UiSymbols},
};
use crate::{
    renderers::diff_data::{BodyDiffData, DiffData, DiffOperation, HeaderDiffData},
    types::DiffViewStyle,
};

/// Rich TUI diff widget renderer
pub struct DiffWidgetRenderer;

impl DiffWidgetRenderer {
    /// Render complete diff data using proper TUI widgets
    pub fn render_diff_view(f: &mut Frame, diff_data: &DiffData, app: &TuiApp, area: Rect) {
        Self::render_diff_view_with_style(f, diff_data, app, &app.diff_style, area);
    }

    /// Render complete diff data using proper TUI widgets with custom diff style
    pub fn render_diff_view_with_style(
        f: &mut Frame,
        diff_data: &DiffData,
        app: &TuiApp,
        style: &DiffViewStyle,
        area: Rect,
    ) {
        if diff_data.is_empty() {
            Self::render_no_differences(f, diff_data, area);
            return;
        }

        // Calculate layout based on what content we have
        let mut constraints = vec![Constraint::Length(3)]; // Route info

        if diff_data.headers.is_some() {
            constraints.push(Constraint::Min(8)); // Headers section
        }

        if diff_data.body.is_some() {
            constraints.push(Constraint::Min(10)); // Body section
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut chunk_idx = 0;

        // Render route information
        Self::render_route_info(f, diff_data, style, chunks[chunk_idx]);
        chunk_idx += 1;

        // Render header differences if present
        if let Some(ref headers) = diff_data.headers {
            Self::render_header_diff_widget(f, headers, style, chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Render body differences if present
        if let Some(ref body) = diff_data.body {
            Self::render_body_diff_widget(f, body, style, app, chunks[chunk_idx]);
        }
    }

    /// Render route information header
    fn render_route_info(f: &mut Frame, diff_data: &DiffData, style: &DiffViewStyle, area: Rect) {
        let style_text = match style {
            DiffViewStyle::Unified => "Unified",
            DiffViewStyle::SideBySide => "Side-by-Side",
        };

        let info_text = format!(
            "{} Route: {}  {} Style: {}",
            UiSymbols::ROUTE,
            diff_data.route_name,
            UiSymbols::SETTINGS,
            style_text
        );

        let info_paragraph = Paragraph::new(info_text)
            .style(TuiTheme::primary_text_style().add_modifier(Modifier::BOLD))
            .block(TuiTheme::normal_block("Diff Information"))
            .alignment(Alignment::Left);

        f.render_widget(info_paragraph, area);
    }

    /// Render "no differences" message
    fn render_no_differences(f: &mut Frame, diff_data: &DiffData, area: Rect) {
        let message = format!(
            "{} No differences found between environments\n\nRoute: {}",
            UiSymbols::SUCCESS,
            diff_data.route_name
        );

        let paragraph = Paragraph::new(message)
            .style(TuiTheme::success_style())
            .block(TuiTheme::normal_block("Diff Results"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Render header differences using ratatui Table widget
    fn render_header_diff_widget(
        f: &mut Frame,
        headers: &HeaderDiffData,
        style: &DiffViewStyle,
        area: Rect,
    ) {
        match style {
            DiffViewStyle::Unified => Self::render_headers_unified_widget(f, headers, area),
            DiffViewStyle::SideBySide => Self::render_headers_side_by_side_widget(f, headers, area),
        }
    }

    /// Render unified header diff as a table
    fn render_headers_unified_widget(f: &mut Frame, headers: &HeaderDiffData, area: Rect) {
        let mut rows = Vec::new();

        for row in &headers.rows {
            let header_name = row.context.as_deref().unwrap_or("Unknown");

            match row.operation {
                DiffOperation::Removed => {
                    if let Some(ref content) = row.left_content {
                        let env_label = format!("- {}", headers.env1.to_uppercase());
                        rows.push(Row::new(vec![
                            Cell::from(header_name).style(TuiTheme::primary_text_style()),
                            Cell::from(env_label).style(TuiTheme::error_style()),
                            Cell::from(content.as_str()).style(TuiTheme::error_style()),
                        ]));
                    }
                }
                DiffOperation::Added => {
                    if let Some(ref content) = row.right_content {
                        let env_label = format!("+ {}", headers.env2.to_uppercase());
                        rows.push(Row::new(vec![
                            Cell::from(header_name).style(TuiTheme::primary_text_style()),
                            Cell::from(env_label).style(TuiTheme::success_style()),
                            Cell::from(content.as_str()).style(TuiTheme::success_style()),
                        ]));
                    }
                }
                DiffOperation::Changed => {
                    // Show both values for changed headers
                    if let Some(ref content1) = row.left_content {
                        let env_label = format!("- {}", headers.env1.to_uppercase());
                        rows.push(Row::new(vec![
                            Cell::from(header_name)
                                .style(TuiTheme::primary_text_style().add_modifier(Modifier::BOLD)),
                            Cell::from(env_label).style(TuiTheme::error_style()),
                            Cell::from(content1.as_str()).style(TuiTheme::error_style()),
                        ]));
                    }
                    if let Some(ref content2) = row.right_content {
                        let env_label = format!("+ {}", headers.env2.to_uppercase());
                        rows.push(Row::new(vec![
                            Cell::from("").style(TuiTheme::primary_text_style()), // Empty for continuation
                            Cell::from(env_label).style(TuiTheme::success_style()),
                            Cell::from(content2.as_str()).style(TuiTheme::success_style()),
                        ]));
                    }
                }
                DiffOperation::Unchanged => {
                    // This shouldn't appear in diff data, but handle gracefully
                    if let Some(ref content) = row.left_content {
                        rows.push(Row::new(vec![
                            Cell::from(header_name).style(TuiTheme::primary_text_style()),
                            Cell::from("  BOTH").style(TuiTheme::secondary_text_style()),
                            Cell::from(content.as_str()).style(TuiTheme::primary_text_style()),
                        ]));
                    }
                }
            }
        }

        // Create table headers
        let header_row = Row::new(vec!["Header", "Environment", "Value"])
            .style(TuiTheme::primary_text_style().add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let block_title = format!("{} Header Differences", UiSymbols::HEADERS);
        let table = Table::new(
            rows,
            [
                Constraint::Length(20),
                Constraint::Length(15),
                Constraint::Min(30),
            ],
        )
        .header(header_row)
        .block(TuiTheme::normal_block(&block_title).border_style(TuiTheme::warning_style()))
        .column_spacing(1);

        f.render_widget(table, area);
    }

    /// Render side-by-side header diff as a table
    fn render_headers_side_by_side_widget(f: &mut Frame, headers: &HeaderDiffData, area: Rect) {
        let mut rows = Vec::new();

        for row in &headers.rows {
            let header_name = row.context.as_deref().unwrap_or("Unknown");

            let (left_content, left_style) = match &row.left_content {
                Some(content) => match row.operation {
                    DiffOperation::Removed | DiffOperation::Changed => {
                        (content.as_str(), TuiTheme::error_style())
                    }
                    _ => (content.as_str(), TuiTheme::primary_text_style()),
                },
                None => ("(missing)", TuiTheme::secondary_text_style()),
            };

            let (right_content, right_style) = match &row.right_content {
                Some(content) => match row.operation {
                    DiffOperation::Added | DiffOperation::Changed => {
                        (content.as_str(), TuiTheme::success_style())
                    }
                    _ => (content.as_str(), TuiTheme::primary_text_style()),
                },
                None => ("(missing)", TuiTheme::secondary_text_style()),
            };

            let header_style = match row.operation {
                DiffOperation::Changed => {
                    TuiTheme::primary_text_style().add_modifier(Modifier::BOLD)
                }
                _ => TuiTheme::primary_text_style(),
            };

            rows.push(Row::new(vec![
                Cell::from(header_name).style(header_style),
                Cell::from(left_content).style(left_style),
                Cell::from(right_content).style(right_style),
            ]));
        }

        // Create table headers
        let env1_upper = headers.env1.to_uppercase();
        let env2_upper = headers.env2.to_uppercase();
        let header_row = Row::new(vec!["Header", &env1_upper, &env2_upper])
            .style(TuiTheme::primary_text_style().add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let block_title = format!("{} Header Differences", UiSymbols::HEADERS);
        let table = Table::new(
            rows,
            [
                Constraint::Length(20),
                Constraint::Percentage(40),
                Constraint::Percentage(40),
            ],
        )
        .header(header_row)
        .block(TuiTheme::normal_block(&block_title).border_style(TuiTheme::warning_style()))
        .column_spacing(1);

        f.render_widget(table, area);
    }

    /// Render body differences using ratatui List widget
    fn render_body_diff_widget(
        f: &mut Frame,
        body: &BodyDiffData,
        style: &DiffViewStyle,
        app: &TuiApp,
        area: Rect,
    ) {
        if body.is_large_response {
            Self::render_large_response_widget(f, body, area);
            return;
        }

        match style {
            DiffViewStyle::Unified => Self::render_body_unified_widget(f, body, app, area),
            DiffViewStyle::SideBySide => Self::render_body_side_by_side_widget(f, body, app, area),
        }
    }

    /// Render unified body diff as a list
    fn render_body_unified_widget(f: &mut Frame, body: &BodyDiffData, app: &TuiApp, area: Rect) {
        let mut items = Vec::new();

        for (_line_num, row) in body.rows.iter().enumerate() {
            let (content, style, prefix) = match row.operation {
                DiffOperation::Unchanged => {
                    if let Some(ref content) = row.left_content {
                        (content.as_str(), TuiTheme::primary_text_style(), "  ")
                    } else {
                        continue;
                    }
                }
                DiffOperation::Removed => {
                    if let Some(ref content) = row.left_content {
                        (content.as_str(), TuiTheme::error_style(), "- ")
                    } else {
                        continue;
                    }
                }
                DiffOperation::Added => {
                    if let Some(ref content) = row.right_content {
                        (content.as_str(), TuiTheme::success_style(), "+ ")
                    } else {
                        continue;
                    }
                }
                DiffOperation::Changed => {
                    // For changed lines, show both with different prefixes
                    if let Some(ref content1) = row.left_content {
                        items.push(
                            ListItem::new(format!("- {}", content1)).style(TuiTheme::error_style()),
                        );
                    }
                    if let Some(ref content2) = row.right_content {
                        (content2.as_str(), TuiTheme::success_style(), "+ ")
                    } else {
                        continue;
                    }
                }
            };

            let line_text = format!("{}{}", prefix, content);
            items.push(ListItem::new(line_text).style(style));
        }

        // Apply scrolling offset
        let visible_items: Vec<ListItem> = if app.scroll_offset < items.len() {
            items.into_iter().skip(app.scroll_offset).collect()
        } else {
            vec![]
        };

        let block_title = format!("{} Body Differences (Unified)", UiSymbols::BODY);
        let list = List::new(visible_items)
            .block(TuiTheme::normal_block(&block_title).border_style(TuiTheme::warning_style()));

        f.render_widget(list, area);
    }

    /// Render side-by-side body diff using layout
    fn render_body_side_by_side_widget(
        f: &mut Frame,
        body: &BodyDiffData,
        app: &TuiApp,
        area: Rect,
    ) {
        // Split area into two columns
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Prepare left and right content
        let mut left_items = Vec::new();
        let mut right_items = Vec::new();

        for row in &body.rows {
            match row.operation {
                DiffOperation::Unchanged => {
                    if let Some(ref content) = row.left_content {
                        left_items.push(
                            ListItem::new(format!("  {}", content))
                                .style(TuiTheme::primary_text_style()),
                        );
                        right_items.push(
                            ListItem::new(format!("  {}", content))
                                .style(TuiTheme::primary_text_style()),
                        );
                    }
                }
                DiffOperation::Removed => {
                    if let Some(ref content) = row.left_content {
                        left_items.push(
                            ListItem::new(format!("- {}", content)).style(TuiTheme::error_style()),
                        );
                        right_items.push(ListItem::new("").style(TuiTheme::primary_text_style()));
                    }
                }
                DiffOperation::Added => {
                    if let Some(ref content) = row.right_content {
                        left_items.push(ListItem::new("").style(TuiTheme::primary_text_style()));
                        right_items.push(
                            ListItem::new(format!("+ {}", content))
                                .style(TuiTheme::success_style()),
                        );
                    }
                }
                DiffOperation::Changed => {
                    let left_content = row.left_content.as_deref().unwrap_or("");
                    let right_content = row.right_content.as_deref().unwrap_or("");

                    left_items.push(
                        ListItem::new(format!("- {}", left_content)).style(TuiTheme::error_style()),
                    );
                    right_items.push(
                        ListItem::new(format!("+ {}", right_content))
                            .style(TuiTheme::success_style()),
                    );
                }
            }
        }

        // Apply scrolling offset to both sides
        let visible_left_items: Vec<ListItem> = if app.scroll_offset < left_items.len() {
            left_items.into_iter().skip(app.scroll_offset).collect()
        } else {
            vec![]
        };

        let visible_right_items: Vec<ListItem> = if app.scroll_offset < right_items.len() {
            right_items.into_iter().skip(app.scroll_offset).collect()
        } else {
            vec![]
        };

        // Render left side
        let left_title = body.env1.to_uppercase();
        let left_list = List::new(visible_left_items)
            .block(TuiTheme::normal_block(&left_title).border_style(TuiTheme::error_style()));

        // Render right side
        let right_title = body.env2.to_uppercase();
        let right_list = List::new(visible_right_items)
            .block(TuiTheme::normal_block(&right_title).border_style(TuiTheme::success_style()));

        f.render_widget(left_list, chunks[0]);
        f.render_widget(right_list, chunks[1]);
    }

    /// Render large response summary widget
    fn render_large_response_widget(f: &mut Frame, body: &BodyDiffData, area: Rect) {
        if let Some(ref summary) = body.summary {
            let summary_text = format!(
                "{} Large Response Comparison Summary\n\n\
                {} Responses are too large for detailed diff\n\n\
                Environment Comparison:\n\
                • {}: {} bytes, {} lines\n\
                • {}: {} bytes, {} lines\n\n\
                {} Differences:\n\
                • Size difference: {} bytes\n\
                • Line difference: {} lines\n\n\
                {} Sample differences:\n{}",
                UiSymbols::INFO,
                UiSymbols::WARNING,
                body.env1.to_uppercase(),
                summary.size1,
                summary.lines1,
                body.env2.to_uppercase(),
                summary.size2,
                summary.lines2,
                UiSymbols::COMPARE,
                (summary.size1 as i64 - summary.size2 as i64).abs(),
                (summary.lines1 as i64 - summary.lines2 as i64).abs(),
                UiSymbols::DIFF,
                summary.sample_differences.join("\n")
            );

            let block_title = format!("{} Body Summary", UiSymbols::BODY);
            let paragraph = Paragraph::new(summary_text)
                .style(TuiTheme::primary_text_style())
                .block(TuiTheme::normal_block(&block_title).border_style(TuiTheme::info_style()))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left);

            f.render_widget(paragraph, area);
        }
    }
}
