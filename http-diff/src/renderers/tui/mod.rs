//! Terminal User Interface renderer for HTTP diff results
//! 
//! This module provides an interactive TUI interface for viewing and navigating
//! HTTP diff results, built on top of ratatui.

#[cfg(feature = "tui")]
pub mod app;
#[cfg(feature = "tui")]
pub mod ui;
#[cfg(feature = "tui")]
pub mod events;
#[cfg(feature = "tui")]
pub mod theme;
#[cfg(feature = "tui")]
pub mod diff_renderer;
#[cfg(feature = "tui")]
pub mod diff_widgets;

#[cfg(feature = "tui")]
pub use app::{TuiApp, ViewMode};

#[cfg(feature = "tui")]
use crate::{
    types::{ComparisonResult, DiffViewStyle, ExecutionResult},
    renderers::OutputRenderer,
    error::{HttpDiffError, Result},
};

#[cfg(feature = "tui")]
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

#[cfg(feature = "tui")]
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[cfg(feature = "tui")]
use std::io::{self, Stdout};

/// Messages sent from async execution task to TUI
#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
pub enum ExecutionMessage {
    /// Progress update with complete progress information
    Progress {
        completed: usize,
        total: usize,
        successful: usize,
        failed: usize,
        percentage: f64,
        operation: String,
    },
    /// Execution completed successfully
    Completed(Vec<ComparisonResult>),
    /// Execution failed with error
    Failed(String),
}

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
        enable_raw_mode().map_err(|e| HttpDiffError::general(format!("Failed to enable raw mode: {}", e)))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)
            .map_err(|e| HttpDiffError::general(format!("Failed to enter alternate screen: {}", e)))?;
        let backend = CrosstermBackend::new(stdout);
        Terminal::new(backend)
            .map_err(|e| HttpDiffError::general(format!("Failed to create terminal: {}", e)))
    }

    /// Restore terminal after TUI
    fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        disable_raw_mode().map_err(|e| HttpDiffError::general(format!("Failed to disable raw mode: {}", e)))?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .map_err(|e| HttpDiffError::general(format!("Failed to leave alternate screen: {}", e)))?;
        terminal.show_cursor()
            .map_err(|e| HttpDiffError::general(format!("Failed to show cursor: {}", e)))?;
        Ok(())
    }

    /// Run the main TUI event loop
    fn run_app(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: TuiApp) -> Result<()> {
        loop {
            // Clear old feedback messages
            app.clear_old_feedback();
            
            // Draw the UI
            terminal.draw(|f| ui::draw(f, &app))
                .map_err(|e| HttpDiffError::general(format!("Failed to draw: {}", e)))?;

            // Handle events
            if let Some(result) = events::handle_events(&mut app)? {
                match result {
                    events::AppResult::Quit => break,
                    events::AppResult::Continue => continue,
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
        let cli_renderer = crate::renderers::CliRenderer::new()
            .with_diff_style(self.diff_style.clone());
        
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
        let mut app = TuiApp::new_for_workflow(
            self.diff_style.clone(),
            self.show_headers,
            self.show_errors,
        );

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
    fn run_workflow_app(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: TuiApp) -> Result<()> {
        // Create a channel for receiving execution messages from async tasks
        let (tx, rx) = std::sync::mpsc::channel::<ExecutionMessage>();
        let mut execution_handle: Option<std::thread::JoinHandle<()>> = None;
        
        loop {
            // Clear old feedback messages
            app.clear_old_feedback();
            
            // Check if execution has been requested
            if app.execution_requested && execution_handle.is_none() {
                // Start async HTTP execution
                app.start_execution();
                let tx_clone = tx.clone();
                let config_path = app.config_path.clone();
                let users_file = app.users_file.clone();
                let selected_environments = app.selected_environments.clone();
                let selected_routes = app.selected_routes.clone();
                let include_headers = self.show_headers;
                let include_errors = self.show_errors;
                
                execution_handle = Some(std::thread::spawn(move || {
                    // Create a runtime in this thread for HTTP execution
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let _ = tx_clone.send(ExecutionMessage::Failed(
                                format!("Failed to create runtime: {}", e)
                            ));
                            return;
                        }
                    };
                    
                    rt.block_on(async {
                        // Set a timeout for the entire execution
                        let timeout_duration = std::time::Duration::from_secs(300); // 5 minutes timeout
                        
                        match tokio::time::timeout(timeout_duration, Self::execute_http_tests_async(
                            tx_clone.clone(),
                            config_path,
                            users_file,
                            selected_environments,
                            selected_routes,
                            include_headers,
                            include_errors,
                        )).await {
                            Ok(_) => {
                                // Execution completed normally (success or failure already handled)
                            }
                            Err(_) => {
                                // Timeout occurred
                                let _ = tx_clone.send(ExecutionMessage::Failed(
                                    "Execution timed out after 5 minutes".to_string()
                                ));
                            }
                        }
                    });
                }));
            }
            
            // Check for execution messages
            while let Ok(message) = rx.try_recv() {
                match message {
                    ExecutionMessage::Progress { completed, total, successful: _, failed: _, percentage: _, operation } => {
                        // Update with accurate progress data from ProgressTracker
                        app.total_tests = total;
                        app.update_execution_progress(completed, operation);
                        // TODO: Store successful/failed counts in app state if needed for display
                    }
                    ExecutionMessage::Completed(results) => {
                        app.complete_execution(results);
                        execution_handle = None;
                    }
                    ExecutionMessage::Failed(error) => {
                        app.set_error(format!("Execution failed: {}", error));
                        app.view_mode = ViewMode::Configuration;
                        app.execution_running = false;
                        app.execution_requested = false;
                        app.execution_cancelled = false;
                        app.current_operation = "Execution failed".to_string();
                        execution_handle = None;
                    }
                }
            }
            
            // Handle execution cancellation
            if app.execution_cancelled && execution_handle.is_some() {
                if let Some(handle) = execution_handle.take() {
                    // Note: We can't gracefully cancel std::thread, but we can reset the UI state
                    // The thread will complete but we'll ignore its results
                    app.view_mode = ViewMode::Configuration;
                    app.execution_running = false;
                    app.execution_requested = false;
                    app.execution_cancelled = false;
                    app.current_operation = "Execution cancelled".to_string();
                    app.show_feedback("Execution cancelled by user", crate::renderers::tui::app::FeedbackType::Warning);
                    
                    // Let the thread finish in the background
                    std::thread::spawn(move || {
                        let _ = handle.join();
                    });
                }
            }
            
            // Draw the UI
            terminal.draw(|f| ui::draw(f, &app))
                .map_err(|e| HttpDiffError::general(format!("Failed to draw: {}", e)))?;

            // Handle events
            if let Some(result) = events::handle_events(&mut app)? {
                match result {
                    events::AppResult::Quit => {
                        // Cleanup any running execution
                        if let Some(handle) = execution_handle.take() {
                            // Let the thread finish in the background
                            std::thread::spawn(move || {
                                let _ = handle.join();
                            });
                        }
                        break;
                    }
                    events::AppResult::Continue => continue,
                }
            }
        }
        Ok(())
    }
    
    /// Execute HTTP tests asynchronously and send progress updates
    async fn execute_http_tests_async(
        tx: std::sync::mpsc::Sender<ExecutionMessage>,
        config_path: String,
        users_file: String,
        selected_environments: Vec<String>,
        selected_routes: Vec<String>,
        _include_headers: bool,
        _include_errors: bool,
    ) {
        use crate::{
            config::{load_user_data, HttpDiffConfig},
            create_default_test_runner,
            ProgressCallback,
            TestRunner,
        };
        
        // Send initial progress
        let _ = tx.send(ExecutionMessage::Progress {
            completed: 0,
            total: 0, // Will be updated when HTTP runner calculates actual total
            successful: 0,
            failed: 0,
            percentage: 0.0,
            operation: "Loading configuration...".to_string(),
        });
        
        // Load configuration
        let config_path = std::path::Path::new(&config_path);
        let config = match HttpDiffConfig::load_from_file(config_path) {
            Ok(config) => config,
            Err(e) => {
                let _ = tx.send(ExecutionMessage::Failed(format!("Failed to load configuration: {}", e)));
                return;
            }
        };
        
        // Load user data
        let users_path = std::path::Path::new(&users_file);
        let user_data = match load_user_data(users_path) {
            Ok(data) => data,
            Err(e) => {
                let _ = tx.send(ExecutionMessage::Failed(format!("Failed to load user data: {}", e)));
                return;
            }
        };
        
        // Send progress update
        let _ = tx.send(ExecutionMessage::Progress {
            completed: 0,
            total: 0, // Will be updated when HTTP runner calculates actual total
            successful: 0,
            failed: 0,
            percentage: 0.0,
            operation: "Creating test runner...".to_string(),
        });
        
        // Create test runner
        let runner = match create_default_test_runner(config) {
            Ok(runner) => runner,
            Err(e) => {
                let _ = tx.send(ExecutionMessage::Failed(format!("Failed to create test runner: {}", e)));
                return;
            }
        };
        
        // Set up progress callback using ProgressTracker as single source of truth
        let tx_clone = tx.clone();
        
        let progress_callback: ProgressCallback = Box::new(move |progress_tracker| {
            let operation = format!("Completed {}/{} requests ({} successful, {} failed)", 
                                  progress_tracker.completed_requests, 
                                  progress_tracker.total_requests,
                                  progress_tracker.successful_requests,
                                  progress_tracker.failed_requests);
            
            let _ = tx_clone.send(ExecutionMessage::Progress {
                completed: progress_tracker.completed_requests,
                total: progress_tracker.total_requests,
                successful: progress_tracker.successful_requests,
                failed: progress_tracker.failed_requests,
                percentage: progress_tracker.progress_percentage(),
                operation,
            });
        });
        
        // Execute tests
        let _ = tx.send(ExecutionMessage::Progress {
            completed: 0,
            total: 0, // Will be updated when HTTP runner calculates actual total
            successful: 0,
            failed: 0,
            percentage: 0.0,
            operation: "Starting HTTP tests...".to_string(),
        });
        
        match runner.execute_with_data(
            &user_data,
            Some(selected_environments),
            Some(selected_routes),
            None, // No error collector for now
            Some(progress_callback),
        ).await {
            Ok(execution_result) => {
                let _ = tx.send(ExecutionMessage::Completed(execution_result.comparisons));
            }
            Err(e) => {
                let _ = tx.send(ExecutionMessage::Failed(format!("Test execution failed: {}", e)));
            }
        }
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
            "TUI feature not compiled. Use --no-tui to use CLI renderer."
        ))
    }

    fn run_workflow(&self, _args: impl std::fmt::Debug) -> Result<()> {
        Err(HttpDiffError::general(
            "TUI feature not compiled. Use --no-tui to use CLI renderer."
        ))
    }
}