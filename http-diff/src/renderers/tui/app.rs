use crate::types::{ComparisonResult, DiffViewStyle};

/// Focus state for better UX navigation
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    /// Environments panel is focused
    Environments,
    /// Routes panel is focused
    Routes,
    /// Actions/buttons panel is focused
    Actions,
}

/// UI feedback for user actions
#[derive(Debug, Clone)]
pub struct ActionFeedback {
    pub message: String,
    pub feedback_type: FeedbackType,
    pub timestamp: std::time::Instant,
    pub is_brief: bool,
}

/// Type of feedback to show different colors/styles
#[derive(Debug, Clone)]
pub enum FeedbackType {
    Success,
    Warning,
    Error,
    Info,
}

/// Different viewing modes for the TUI
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    /// Configuration setup and environment/route selection
    Configuration,
    /// Real-time execution progress display
    Execution,
    /// Table view showing all results
    ResultsList,
    /// Detailed view of a single result
    ResultDetail,
    /// Full diff view showing response differences
    DiffView,
}

/// Main TUI application state
pub struct TuiApp {
    /// All comparison results to display
    pub results: Vec<ComparisonResult>,
    /// Currently selected result index
    pub selected_index: usize,
    /// Current viewing mode
    pub view_mode: ViewMode,
    /// Diff view style (unified or side-by-side)
    pub diff_style: DiffViewStyle,
    /// Whether to show headers in comparisons
    pub show_headers: bool,
    /// Whether to show error details
    pub show_errors: bool,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Scroll position for detailed views
    pub scroll_offset: usize,
    
    // Configuration state
    /// Available environments from config
    pub available_environments: Vec<String>,
    /// Selected environments for testing
    pub selected_environments: Vec<String>,
    /// Available routes from config
    pub available_routes: Vec<String>,
    /// Selected routes for testing
    pub selected_routes: Vec<String>,
    /// Configuration file path
    pub config_path: String,
    /// Users file path
    pub users_file: String,
    /// Error message to display
    pub error_message: Option<String>,
    
    // Execution state
    /// Total number of tests to execute
    pub total_tests: usize,
    /// Number of completed tests
    pub completed_tests: usize,
    /// Current operation description
    pub current_operation: String,
    /// Start time of execution
    pub execution_start_time: Option<std::time::Instant>,
    /// Whether execution has been requested
    pub execution_requested: bool,
    /// Whether execution is currently running
    pub execution_running: bool,
    /// Execution has been cancelled
    pub execution_cancelled: bool,
    
    // UI State for better UX
    /// Currently focused panel (for better navigation)
    pub focused_panel: FocusedPanel,
    /// Action feedback to show to user
    pub action_feedback: Option<ActionFeedback>,
    /// Whether to show help overlay
    pub show_help: bool,
    /// Selected environment index for keyboard navigation
    pub selected_env_index: usize,
    /// Selected route index for keyboard navigation
    pub selected_route_index: usize,
}

impl TuiApp {
    /// Create a new TUI application for results viewing
    pub fn new(
        results: Vec<ComparisonResult>,
        diff_style: DiffViewStyle,
        show_headers: bool,
        show_errors: bool,
    ) -> Self {
        Self {
            results,
            selected_index: 0,
            view_mode: ViewMode::ResultsList,
            diff_style,
            show_headers,
            show_errors,
            should_quit: false,
            scroll_offset: 0,
            available_environments: Vec::new(),
            selected_environments: Vec::new(),
            available_routes: Vec::new(),
            selected_routes: Vec::new(),
            config_path: "http-diff.toml".to_string(),
            users_file: "users.csv".to_string(),
            error_message: None,
            total_tests: 0,
            completed_tests: 0,
            current_operation: String::new(),
            execution_start_time: None,
            execution_requested: false,
            execution_running: false,
            execution_cancelled: false,
            focused_panel: FocusedPanel::Environments,
            action_feedback: None,
            show_help: false,
            selected_env_index: 0,
            selected_route_index: 0,
        }
    }

    /// Create a new TUI application for complete workflow
    pub fn new_for_workflow(
        diff_style: DiffViewStyle,
        show_headers: bool,
        show_errors: bool,
    ) -> Self {
        Self {
            results: Vec::new(),
            selected_index: 0,
            view_mode: ViewMode::Configuration,
            diff_style,
            show_headers,
            show_errors,
            should_quit: false,
            scroll_offset: 0,
            available_environments: Vec::new(),
            selected_environments: Vec::new(),
            available_routes: Vec::new(),
            selected_routes: Vec::new(),
            config_path: "http-diff.toml".to_string(),
            users_file: "users.csv".to_string(),
            error_message: None,
            total_tests: 0,
            completed_tests: 0,
            current_operation: "Loading configuration...".to_string(),
            execution_start_time: None,
            execution_requested: false,
            execution_running: false,
            execution_cancelled: false,
            focused_panel: FocusedPanel::Environments,
            action_feedback: None,
            show_help: false,
            selected_env_index: 0,
            selected_route_index: 0,
        }
    }

    /// Get the currently selected result
    pub fn current_result(&self) -> Option<&ComparisonResult> {
        self.results.get(self.selected_index)
    }

    /// Move to the next result
    pub fn next_result(&mut self) {
        if self.selected_index < self.results.len().saturating_sub(1) {
            self.selected_index += 1;
            self.scroll_offset = 0; // Reset scroll when changing results
        }
    }

    /// Move to the previous result
    pub fn previous_result(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.scroll_offset = 0; // Reset scroll when changing results
        }
    }

    /// Switch to the next view mode
    pub fn next_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Configuration => ViewMode::Configuration, // Stay in config
            ViewMode::Execution => ViewMode::Execution, // Stay in execution
            ViewMode::ResultsList => ViewMode::ResultDetail,
            ViewMode::ResultDetail => ViewMode::DiffView,
            ViewMode::DiffView => ViewMode::ResultsList,
        };
        self.scroll_offset = 0; // Reset scroll when changing views
    }

    /// Switch to the previous view mode
    pub fn previous_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Configuration => ViewMode::Configuration, // Stay in config
            ViewMode::Execution => ViewMode::Execution, // Stay in execution
            ViewMode::ResultsList => ViewMode::DiffView,
            ViewMode::ResultDetail => ViewMode::ResultsList,
            ViewMode::DiffView => ViewMode::ResultDetail,
        };
        self.scroll_offset = 0; // Reset scroll when changing views
    }

    /// Toggle diff view style between unified and side-by-side
    pub fn toggle_diff_style(&mut self) {
        self.diff_style = match self.diff_style {
            DiffViewStyle::Unified => DiffViewStyle::SideBySide,
            DiffViewStyle::SideBySide => DiffViewStyle::Unified,
        };
    }

    /// Toggle headers display
    pub fn toggle_headers(&mut self) {
        self.show_headers = !self.show_headers;
    }

    /// Toggle errors display
    pub fn toggle_errors(&mut self) {
        self.show_errors = !self.show_errors;
    }

    /// Scroll down in the current view
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scroll up in the current view
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Page down (scroll by multiple lines)
    pub fn page_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(10);
    }

    /// Page up (scroll by multiple lines)
    pub fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    /// Jump to the top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Jump to the bottom (this would need the content height to be accurate)
    pub fn scroll_to_bottom(&mut self, content_height: usize) {
        self.scroll_offset = content_height.saturating_sub(1);
    }

    /// Mark the app to quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Get summary statistics for display
    pub fn get_summary(&self) -> (usize, usize, usize) {
        let total = self.results.len();
        let identical = self.results.iter().filter(|r| r.is_identical).count();
        let different = total - identical;
        (total, identical, different)
    }

    /// Get the title for the current view
    pub fn get_view_title(&self) -> String {
        match self.view_mode {
            ViewMode::Configuration => "Configuration & Setup".to_string(),
            ViewMode::Execution => "Test Execution".to_string(),
            ViewMode::ResultsList => "Results Overview".to_string(),
            ViewMode::ResultDetail => {
                if let Some(result) = self.current_result() {
                    format!("Result Detail - {}", result.route_name)
                } else {
                    "Result Detail".to_string()
                }
            }
            ViewMode::DiffView => {
                if let Some(result) = self.current_result() {
                    format!("Diff View - {} ({})", result.route_name, 
                           match self.diff_style {
                               DiffViewStyle::Unified => "Unified",
                               DiffViewStyle::SideBySide => "Side-by-Side",
                           })
                } else {
                    "Diff View".to_string()
                }
            }
        }
    }

    /// Check if the current result has differences
    pub fn current_result_has_differences(&self) -> bool {
        self.current_result()
            .map(|r| !r.is_identical)
            .unwrap_or(false)
    }

    /// Get help text for the current view
    pub fn get_help_text(&self) -> &'static str {
        match self.view_mode {
            ViewMode::Configuration => {
                "Space: Toggle selection | a: Select all | n: Clear all | Enter: Start tests | i: Initialize config | q: Quit"
            }
            ViewMode::Execution => {
                "Ctrl+C: Cancel execution | q: Quit (after completion)"
            }
            ViewMode::ResultsList => {
                "↑↓: Navigate | Enter/→: Detail | Tab: Cycle views | d: Toggle diff style | h: Toggle headers | e: Toggle errors | q: Quit"
            }
            ViewMode::ResultDetail => {
                "↑↓: Navigate results | ←: Back to list | →: Diff view | Tab: Cycle views | PgUp/PgDn: Scroll | q: Quit"
            }
            ViewMode::DiffView => {
                "↑↓: Navigate results | ←: Back to detail | Tab: Cycle views | d: Toggle diff style | PgUp/PgDn: Scroll | q: Quit"
            }
        }
    }

    /// Load configuration and populate available environments/routes
    pub fn load_configuration(&mut self) -> Result<(), String> {
        use crate::HttpDiffConfig;
        
        let config_path = std::path::Path::new(&self.config_path);
        
        if !config_path.exists() {
            return Err("Configuration file not found. Press 'i' to initialize.".to_string());
        }

        let config = HttpDiffConfig::load_from_file(config_path)
            .map_err(|e| format!("Failed to load configuration: {}", e))?;

        self.available_environments = config.environments.keys().cloned().collect();
        self.available_routes = config.routes.iter().map(|r| r.name.clone()).collect();
        
        // Select all by default
        self.selected_environments = self.available_environments.clone();
        self.selected_routes = self.available_routes.clone();
        
        Ok(())
    }

    /// Toggle environment selection
    pub fn toggle_environment(&mut self, index: usize) {
        if let Some(env_name) = self.available_environments.get(index) {
            if let Some(pos) = self.selected_environments.iter().position(|x| x == env_name) {
                self.selected_environments.remove(pos);
            } else {
                self.selected_environments.push(env_name.clone());
            }
        }
    }

    /// Toggle route selection
    pub fn toggle_route(&mut self, index: usize) {
        if let Some(route_name) = self.available_routes.get(index) {
            if let Some(pos) = self.selected_routes.iter().position(|x| x == route_name) {
                self.selected_routes.remove(pos);
            } else {
                self.selected_routes.push(route_name.clone());
            }
        }
    }

    /// Check if environment is selected
    pub fn is_environment_selected(&self, index: usize) -> bool {
        if let Some(env_name) = self.available_environments.get(index) {
            self.selected_environments.contains(env_name)
        } else {
            false
        }
    }

    /// Check if route is selected
    pub fn is_route_selected(&self, index: usize) -> bool {
        if let Some(route_name) = self.available_routes.get(index) {
            self.selected_routes.contains(route_name)
        } else {
            false
        }
    }

    /// Request HTTP test execution (doesn't start immediately, just requests it)
    pub fn request_execution(&mut self) {
        if self.selected_environments.is_empty() || self.selected_routes.is_empty() {
            self.error_message = Some("Please select at least one environment and route".to_string());
            return;
        }

        self.execution_requested = true;
        self.execution_running = false;
        self.execution_cancelled = false;
        // Don't calculate total_tests here - let HTTP runner determine correct total
        self.total_tests = 0; // Will be updated by ProgressTracker
        self.completed_tests = 0;
        self.current_operation = "Preparing to start HTTP tests...".to_string();
        self.error_message = None;
    }

    /// Start the actual execution (called by TUI runner when async task starts)
    pub fn start_execution(&mut self) {
        self.view_mode = ViewMode::Execution;
        self.execution_requested = false;
        self.execution_running = true;
        self.execution_cancelled = false;
        self.current_operation = "Starting HTTP tests...".to_string();
        self.execution_start_time = Some(std::time::Instant::now());
    }

    /// Cancel execution
    pub fn cancel_execution(&mut self) {
        self.execution_cancelled = true;
        self.execution_running = false;
        self.current_operation = "Cancelling execution...".to_string();
    }

    /// Update execution progress
    pub fn update_execution_progress(&mut self, completed: usize, operation: String) {
        self.completed_tests = completed;
        self.current_operation = operation;
    }

    /// Complete execution and move to results
    pub fn complete_execution(&mut self, results: Vec<ComparisonResult>) {
        self.results = results;
        self.view_mode = ViewMode::ResultsList;
        self.selected_index = 0;
        self.execution_start_time = None;
        self.execution_running = false;
        self.execution_requested = false;
        self.execution_cancelled = false;
        self.current_operation = "Execution completed".to_string();
    }

    /// Set error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Switch focus to next panel
    pub fn next_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Environments => FocusedPanel::Routes,
            FocusedPanel::Routes => FocusedPanel::Actions,
            FocusedPanel::Actions => FocusedPanel::Environments,
        };
        self.selected_env_index = 0;
        self.selected_route_index = 0;
    }

    /// Switch focus to previous panel
    pub fn previous_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Environments => FocusedPanel::Actions,
            FocusedPanel::Routes => FocusedPanel::Environments,
            FocusedPanel::Actions => FocusedPanel::Routes,
        };
        self.selected_env_index = 0;
        self.selected_route_index = 0;
    }

    /// Navigate up in the current panel
    pub fn navigate_up(&mut self) {
        match self.focused_panel {
            FocusedPanel::Environments => {
                if self.selected_env_index > 0 {
                    self.selected_env_index -= 1;
                }
            }
            FocusedPanel::Routes => {
                if self.selected_route_index > 0 {
                    self.selected_route_index -= 1;
                }
            }
            FocusedPanel::Actions => {
                // Actions panel doesn't have navigation
            }
        }
    }

    /// Navigate down in the current panel
    pub fn navigate_down(&mut self) {
        match self.focused_panel {
            FocusedPanel::Environments => {
                if self.selected_env_index < self.available_environments.len().saturating_sub(1) {
                    self.selected_env_index += 1;
                }
            }
            FocusedPanel::Routes => {
                if self.selected_route_index < self.available_routes.len().saturating_sub(1) {
                    self.selected_route_index += 1;
                }
            }
            FocusedPanel::Actions => {
                // Actions panel doesn't have navigation
            }
        }
    }

    /// Toggle selection of currently focused item
    pub fn toggle_focused_item(&mut self) {
        match self.focused_panel {
            FocusedPanel::Environments => {
                self.toggle_environment(self.selected_env_index);
                // No intrusive feedback for basic selection toggles
            }
            FocusedPanel::Routes => {
                self.toggle_route(self.selected_route_index);
                // No intrusive feedback for basic selection toggles
            }
            FocusedPanel::Actions => {
                // Handle action panel interactions
                self.request_execution();
            }
        }
    }

    /// Show feedback message to user
    pub fn show_feedback(&mut self, message: &str, feedback_type: FeedbackType) {
        self.action_feedback = Some(ActionFeedback {
            message: message.to_string(),
            feedback_type,
            timestamp: std::time::Instant::now(),
            is_brief: false,
        });
    }
    
    /// Show brief feedback message (1.5 seconds instead of 3)
    pub fn show_brief_feedback(&mut self, message: &str, feedback_type: FeedbackType) {
        self.action_feedback = Some(ActionFeedback {
            message: message.to_string(),
            feedback_type,
            timestamp: std::time::Instant::now(),
            is_brief: true,
        });
    }

    /// Clear old feedback messages
    pub fn clear_old_feedback(&mut self) {
        if let Some(ref feedback) = self.action_feedback {
            let duration_limit = if feedback.is_brief { 1 } else { 3 };
            if feedback.timestamp.elapsed().as_secs() > duration_limit {
                self.action_feedback = None;
            }
        }
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Select all items in the current focused panel
    pub fn select_all_focused(&mut self) {
        match self.focused_panel {
            FocusedPanel::Environments => {
                self.selected_environments = self.available_environments.clone();
                // Brief feedback for bulk operations
                self.show_brief_feedback("All environments selected", FeedbackType::Success);
            }
            FocusedPanel::Routes => {
                self.selected_routes = self.available_routes.clone();
                // Brief feedback for bulk operations
                self.show_brief_feedback("All routes selected", FeedbackType::Success);
            }
            FocusedPanel::Actions => {
                // No select all for actions
            }
        }
    }

    /// Clear all selections in the current focused panel
    pub fn clear_all_focused(&mut self) {
        match self.focused_panel {
            FocusedPanel::Environments => {
                self.selected_environments.clear();
                // Brief feedback for bulk operations
                self.show_brief_feedback("All environments cleared", FeedbackType::Warning);
            }
            FocusedPanel::Routes => {
                self.selected_routes.clear();
                // Brief feedback for bulk operations
                self.show_brief_feedback("All routes cleared", FeedbackType::Warning);
            }
            FocusedPanel::Actions => {
                // No clear all for actions
            }
        }
    }
}