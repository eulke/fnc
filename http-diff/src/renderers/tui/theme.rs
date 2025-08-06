//! TUI Design System and Theme
//! 
//! This module provides consistent colors, styles, and UI components for the TUI

use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};

/// TUI Color Scheme following modern terminal design principles
pub struct TuiTheme;

impl TuiTheme {
    // Primary Colors
    pub const FOCUS: Color = Color::Rgb(97, 175, 239);      // Bright Blue
    pub const SUCCESS: Color = Color::Rgb(152, 195, 121);   // Green
    pub const WARNING: Color = Color::Rgb(229, 192, 123);   // Yellow
    pub const ERROR: Color = Color::Rgb(224, 108, 117);     // Red
    pub const INFO: Color = Color::Rgb(198, 120, 221);      // Purple
    
    // UI Colors
    pub const TEXT_PRIMARY: Color = Color::Rgb(171, 178, 191);    // Light Gray
    pub const TEXT_SECONDARY: Color = Color::Rgb(92, 99, 112);    // Dark Gray
    pub const TEXT_DISABLED: Color = Color::Rgb(92, 99, 112);     // Dark Gray
    pub const BACKGROUND: Color = Color::Rgb(40, 44, 52);         // Dark Background
    pub const BACKGROUND_SELECTED: Color = Color::Rgb(61, 67, 81); // Selected Background
    pub const BORDER_NORMAL: Color = Color::Rgb(92, 99, 112);     // Normal Border
    pub const BORDER_FOCUSED: Color = Color::Rgb(97, 175, 239);   // Focused Border

    /// Get style for focused/selected elements
    pub fn focused_style() -> Style {
        Style::default()
            .fg(Self::TEXT_PRIMARY)
            .bg(Self::BACKGROUND_SELECTED)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for selected items
    pub fn selected_style() -> Style {
        Style::default()
            .fg(Self::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for disabled/inactive elements
    pub fn disabled_style() -> Style {
        Style::default()
            .fg(Self::TEXT_DISABLED)
    }

    /// Get style for error messages
    pub fn error_style() -> Style {
        Style::default()
            .fg(Self::ERROR)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for warning messages
    pub fn warning_style() -> Style {
        Style::default()
            .fg(Self::WARNING)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for success messages
    pub fn success_style() -> Style {
        Style::default()
            .fg(Self::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for info messages
    pub fn info_style() -> Style {
        Style::default()
            .fg(Self::INFO)
    }

    /// Get style for primary text
    pub fn primary_text_style() -> Style {
        Style::default()
            .fg(Self::TEXT_PRIMARY)
    }

    /// Get style for secondary text
    pub fn secondary_text_style() -> Style {
        Style::default()
            .fg(Self::TEXT_SECONDARY)
    }

    /// Create a focused block with enhanced styling
    pub fn focused_block(title: &str) -> Block {
        Block::default()
            .title(format!(" {} {} ", UiSymbols::FOCUSED_INDICATOR, title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::BORDER_FOCUSED).add_modifier(Modifier::BOLD))
            .title_style(Style::default().fg(Self::FOCUS).add_modifier(Modifier::BOLD))
    }

    /// Create a normal block with standard styling
    pub fn normal_block(title: &str) -> Block {
        Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::BORDER_NORMAL))
            .title_style(Style::default().fg(Self::TEXT_PRIMARY))
    }

    /// Create a block for panels with different states
    pub fn panel_block(title: &str, is_focused: bool, has_content: bool, has_activity: bool) -> Block {
        let icon = if has_activity {
            UiSymbols::QUICK_ACTION
        } else if has_content {
            UiSymbols::SUCCESS
        } else {
            UiSymbols::UNFOCUSED_INDICATOR
        };
        
        let full_title = if is_focused {
            format!(" {} {} ", UiSymbols::FOCUSED_INDICATOR, title)
        } else {
            format!(" {} {} ", icon, title)
        };
        
        let border_style = if is_focused {
            Style::default().fg(Self::BORDER_FOCUSED).add_modifier(Modifier::BOLD)
        } else if has_activity {
            Style::default().fg(Self::WARNING)
        } else if has_content {
            Style::default().fg(Self::SUCCESS)
        } else {
            Style::default().fg(Self::BORDER_NORMAL)
        };
        
        let title_style = if is_focused {
            Style::default().fg(Self::FOCUS).add_modifier(Modifier::BOLD)
        } else if has_activity {
            Style::default().fg(Self::WARNING)
        } else {
            Style::default().fg(Self::TEXT_PRIMARY)
        };
        
        Block::default()
            .title(full_title)
            .borders(Borders::ALL)
            .border_style(border_style)
            .title_style(title_style)
    }

    /// Create an action button style
    pub fn button_style(focused: bool, enabled: bool) -> Style {
        match (focused, enabled) {
            (true, true) => Style::default()
                .bg(Self::FOCUS)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            (false, true) => Style::default()
                .bg(Self::TEXT_SECONDARY)
                .fg(Self::TEXT_PRIMARY),
            (_, false) => Style::default()
                .bg(Color::Black)
                .fg(Self::TEXT_DISABLED),
        }
    }
}

/// UI Symbols for consistent iconography
pub struct UiSymbols;

impl UiSymbols {
    // Selection symbols
    pub const SELECTED: &'static str = "☑";
    pub const UNSELECTED: &'static str = "☐";
    pub const FOCUSED_INDICATOR: &'static str = "►";
    pub const UNFOCUSED_INDICATOR: &'static str = " ";
    
    // Status symbols
    pub const SUCCESS: &'static str = "✓";
    pub const ERROR: &'static str = "✗";
    pub const WARNING: &'static str = "⚠";
    pub const INFO: &'static str = "ℹ";
    
    // Action symbols
    pub const PLAY: &'static str = "►";
    pub const PAUSE: &'static str = "⏸";
    pub const STOP: &'static str = "■";
    pub const SETTINGS: &'static str = "⚙";
    pub const HELP: &'static str = "❓";
    
    // Navigation symbols
    pub const UP_DOWN: &'static str = "↑↓";
    pub const LEFT_RIGHT: &'static str = "←→";
    pub const BACK: &'static str = "⟨";
    pub const FORWARD: &'static str = "⟩";
    
    // Content symbols
    pub const RESULTS: &'static str = "📊";
    pub const DETAILS: &'static str = "🔍";
    pub const LIST: &'static str = "📋";
    pub const QUICK_ACTION: &'static str = "⚡";
    pub const TIP: &'static str = "💡";
    
    // Diff symbols
    pub const ROUTE: &'static str = "🛣";
    pub const HEADERS: &'static str = "📤";
    pub const BODY: &'static str = "📄";
    pub const COMPARE: &'static str = "🔍";
    pub const DIFF: &'static str = "📝";
}

/// Key hint formatting for consistent display
pub struct KeyHints;

impl KeyHints {
    /// Format a key hint with consistent styling
    pub fn format_key_hint(key: &str, description: &str) -> String {
        format!("[{}] {}", key, description)
    }

    /// Format multiple key hints separated by pipes
    pub fn format_key_hints(hints: &[(&str, &str)]) -> String {
        hints
            .iter()
            .map(|(key, desc)| Self::format_key_hint(key, desc))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// Get help text for configuration view
    pub fn configuration_help() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Tab", "Switch panels"),
            ("↑↓", "Navigate"),
            ("Space", "Toggle"),
            ("A", "Select all"),
            ("N", "Clear all"),
            ("Enter", "Start tests"),
            ("F1", "Help"),
            ("q", "Quit"),
        ]
    }

    /// Get help text for execution view
    pub fn execution_help() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Ctrl+C", "Cancel"),
            ("q", "Quit"),
        ]
    }

    /// Get help text for results view
    pub fn results_help() -> Vec<(&'static str, &'static str)> {
        vec![
            ("↑↓", "Navigate"),
            ("→", "Details"),
            ("Space", "Quick diff"),
            ("1-4", "Filter tabs"),
            ("f", "Filter panel"),
            ("c", "Clear filters"),
            ("d", "Toggle diff style"),
            ("Tab", "Cycle views"),
            ("q", "Quit"),
        ]
    }
    
    /// Get help text for dashboard view
    pub fn dashboard_help() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Tab", "Switch panels"),
            ("↑↓←→", "Navigate"),
            ("R", "Run tests"),
            ("S", "Save HTML report"),
            ("1-4", "Tabs (Details)"),
            ("D", "Toggle diff"),
            ("x", "Expand"),
            ("q", "Quit"),
        ]
    }
}