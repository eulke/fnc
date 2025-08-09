//! Terminal User Interface renderer for HTTP diff results
//!
//! This module provides an interactive TUI interface for viewing and navigating
//! HTTP diff results, built on top of ratatui.

#[cfg(feature = "tui")]
pub mod app;
#[cfg(feature = "tui")]
pub mod msg;
#[cfg(feature = "tui")]
pub mod update;
#[cfg(feature = "tui")]
pub mod exec;
#[cfg(feature = "tui")]
pub mod diff_widgets;
#[cfg(feature = "tui")]
pub mod events;
#[cfg(feature = "tui")]
pub mod theme;
#[cfg(feature = "tui")]
pub mod view;

#[cfg(feature = "tui")]
pub use app::{TuiApp, ViewMode};

#[cfg(feature = "tui")]
use crate::{
    error::{HttpDiffError, Result},
    renderers::OutputRenderer,
    types::{ComparisonResult, DiffViewStyle, ExecutionResult},
};

#[cfg(feature = "tui")]
use ratatui::{backend::CrosstermBackend, Terminal};

#[cfg(feature = "tui")]
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[cfg(feature = "tui")]
use std::io::{self, Stdout};

#[cfg(feature = "tui")]
use msg::ExecMsg;

// Use shared ExecMsg from msg.rs

/// Interactive renderer trait for renderers that support user interaction
pub trait InteractiveRenderer: OutputRenderer {
    /// Run the interactive interface with pre-computed results
    fn run_interactive(&self, results: &[ComparisonResult]) -> Result<()>;

    /// Run the complete workflow from configuration to results (TUI-specific)
    fn run_workflow(&self, _args: impl std::fmt::Debug) -> Result<()> {
        // Default implementation falls back to run_interactive with empty results
        self.run_interactive(&[])
    }
}

/// TUI renderer for HTTP diff results
#[cfg(feature = "tui")]
pub struct TuiRenderer {
    diff_style: DiffViewStyle,
    show_headers: bool,
    show_errors: bool,
}

#[cfg(feature = "tui")]
impl TuiRenderer {
    /// Create a new TUI renderer
    pub fn new() -> Self {
        Self {
            diff_style: DiffViewStyle::Unified,
            show_headers: false,
            show_errors: false,
        }
    }

    /// Set diff view style
    pub fn with_diff_style(mut self, style: DiffViewStyle) -> Self {
        self.diff_style = style;
        self
    }

    /// Enable/disable headers display
    pub fn with_headers(mut self, show_headers: bool) -> Self {
        self.show_headers = show_headers;
        self
    }

    /// Enable/disable errors display
    pub fn with_errors(mut self, show_errors: bool) -> Self {
        self.show_errors = show_errors;
        self
    }

    /// Setup terminal for TUI
    fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode()
            .map_err(|e| HttpDiffError::general(format!("Failed to enable raw mode: {}", e)))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| {
            HttpDiffError::general(format!("Failed to enter alternate screen: {}", e))
        })?;
        let backend = CrosstermBackend::new(stdout);
        Terminal::new(backend)
            .map_err(|e| HttpDiffError::general(format!("Failed to create terminal: {}", e)))
    }

    /// Restore terminal after TUI
    fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        disable_raw_mode()
            .map_err(|e| HttpDiffError::general(format!("Failed to disable raw mode: {}", e)))?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| {
            HttpDiffError::general(format!("Failed to leave alternate screen: {}", e))
        })?;
        terminal
            .show_cursor()
            .map_err(|e| HttpDiffError::general(format!("Failed to show cursor: {}", e)))?;
        Ok(())
    }

    /// Run the main TUI event loop
    fn run_app(
        &self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        mut app: TuiApp,
    ) -> Result<()> {
        loop {
            // Clear old feedback messages
            app.clear_old_feedback();

            // Draw the UI
            terminal
                .draw(|f| view::draw(f, &mut app))
                .map_err(|e| HttpDiffError::general(format!("Failed to draw: {}", e)))?;

            // Map input to Msg and update via reducer
            if let Some(msg) = events::next_msg(&app)? {
                let effect = update::update(&mut app, msg);
                match effect {
                    update::Effect::Quit => break,
                    update::Effect::SaveReport => {
                        let _ = app.generate_html_report();
                    }
                    update::Effect::StartExec { .. } | update::Effect::None => {}
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "tui")]
impl Default for TuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "tui")]
impl OutputRenderer for TuiRenderer {
    fn render(&self, execution_result: &ExecutionResult) -> String {
        // Fallback to CLI renderer for non-interactive use
        let cli_renderer =
            crate::renderers::CliRenderer::new().with_diff_style(self.diff_style.clone());

        let cli_renderer = if self.show_errors {
            cli_renderer
        } else {
            crate::renderers::CliRenderer::without_errors().with_diff_style(self.diff_style.clone())
        };

        cli_renderer.render(execution_result)
    }
}

#[cfg(feature = "tui")]
impl InteractiveRenderer for TuiRenderer {
    fn run_interactive(&self, results: &[ComparisonResult]) -> Result<()> {
        // Allow TUI to launch even with empty results to show error/status information
        // if results.is_empty() {
        //     return Err(HttpDiffError::general("No results to display"));
        // }

        let mut terminal = Self::setup_terminal()?;

        let app = TuiApp::new(
            results.to_vec(),
            self.diff_style.clone(),
            self.show_headers,
            self.show_errors,
        );

        let result = self.run_app(&mut terminal, app);

        // Always try to restore terminal, even if the app failed
        if let Err(restore_err) = Self::restore_terminal(&mut terminal) {
            eprintln!("Failed to restore terminal: {}", restore_err);
        }

        result
    }

    fn run_workflow(&self, args: impl std::fmt::Debug) -> Result<()> {
        self.run_workflow_impl(args)
    }
}

#[cfg(feature = "tui")]
impl TuiRenderer {
    /// Run the complete TUI workflow from configuration to results
    fn run_workflow_impl(&self, _args: impl std::fmt::Debug) -> Result<()> {
        let mut terminal = Self::setup_terminal()?;

        // Create TUI app in configuration state to handle the complete workflow
        let mut app =
            TuiApp::new_for_workflow(self.diff_style.clone(), self.show_headers, self.show_errors);

        // Try to load configuration automatically on startup
        if let Err(e) = app.load_configuration() {
            app.set_error(e);
        }

        let result = self.run_workflow_app(&mut terminal, app);

        // Always try to restore terminal, even if the app failed
        if let Err(restore_err) = Self::restore_terminal(&mut terminal) {
            eprintln!("Failed to restore terminal: {}", restore_err);
        }

        result
    }

    /// Run the main TUI workflow loop
    fn run_workflow_app(
        &self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        mut app: TuiApp,
    ) -> Result<()> {
        // Create a channel for receiving execution messages from async tasks
        let (tx, rx) = std::sync::mpsc::channel::<ExecMsg>();
        let mut execution_handle: Option<std::thread::JoinHandle<()>> = None;

        loop {
            // Clear old feedback messages
            app.clear_old_feedback();

            // Check for execution messages
            while let Ok(message) = rx.try_recv() {
                let _ = update::update(&mut app, msg::Msg::Exec(message));
                if !app.execution_running { execution_handle = None; }
            }

            // Handle execution cancellation
            if app.execution_cancelled && execution_handle.is_some() {
                if let Some(handle) = execution_handle.take() {
                    // Note: We can't gracefully cancel std::thread, but we can reset the UI state
                    // The thread will complete but we'll ignore its results
                    app.panel_focus = crate::renderers::tui::app::PanelFocus::Configuration;
                    app.execution_running = false;
                    app.execution_requested = false;
                    app.execution_cancelled = false;
                    app.current_operation = "Execution cancelled".to_string();
                    app.show_feedback(
                        "Execution cancelled by user",
                        crate::renderers::tui::app::FeedbackType::Warning,
                    );

                    // Let the thread finish in the background
                    std::thread::spawn(move || {
                        let _ = handle.join();
                    });
                }
            }

            // Draw the UI
            terminal
                .draw(|f| view::draw(f, &mut app))
                .map_err(|e| HttpDiffError::general(format!("Failed to draw: {}", e)))?;

            // Map input to Msg and update
            if let Some(msg) = events::next_msg(&app)? {
                let effect = update::update(&mut app, msg);
                match effect {
                    update::Effect::Quit => {
                        if let Some(handle) = execution_handle.take() {
                            std::thread::spawn(move || { let _ = handle.join(); });
                        }
                        break;
                    }
                    update::Effect::SaveReport => { let _ = app.generate_html_report(); }
                    update::Effect::StartExec { config_path, users, envs, routes, include_headers, include_errors } => {
                        if execution_handle.is_none() {
                            let tx_clone = tx.clone();
                            execution_handle = Some(exec::spawn(tx_clone, config_path, users, envs, routes, include_headers, include_errors));
                        }
                    }
                    update::Effect::None => {}
                }
            }
        }
        Ok(())
    }
}

// Provide stub implementations when TUI feature is disabled
#[cfg(not(feature = "tui"))]
pub struct TuiRenderer;

#[cfg(not(feature = "tui"))]
impl TuiRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn with_diff_style(self, _style: DiffViewStyle) -> Self {
        self
    }

    pub fn with_headers(self, _show_headers: bool) -> Self {
        self
    }

    pub fn with_errors(self, _show_errors: bool) -> Self {
        self
    }
}

#[cfg(not(feature = "tui"))]
impl Default for TuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "tui"))]
impl OutputRenderer for TuiRenderer {
    fn render(&self, execution_result: &ExecutionResult) -> String {
        let cli_renderer = crate::renderers::CliRenderer::new();
        cli_renderer.render(execution_result)
    }
}

#[cfg(not(feature = "tui"))]
impl InteractiveRenderer for TuiRenderer {
    fn run_interactive(&self, _results: &[ComparisonResult]) -> Result<()> {
        Err(HttpDiffError::general(
            "TUI feature not compiled. Use --no-tui to use CLI renderer.",
        ))
    }

    fn run_workflow(&self, _args: impl std::fmt::Debug) -> Result<()> {
        Err(HttpDiffError::general(
            "TUI feature not compiled. Use --no-tui to use CLI renderer.",
        ))
    }
}
