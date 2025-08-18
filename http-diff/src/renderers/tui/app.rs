use crate::renderers::report::{ReportMetadata, ReportRendererFactory};
use crate::types::{ComparisonResult, DiffViewStyle};
use crate::execution::progress::ProgressTracker;
use ratatui::widgets::{ListState, ScrollbarState, TableState};
use std::fs;

/// Dashboard panel focus for 4-panel layout
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PanelFocus {
    /// Configuration panel (top-left): Environment/route selection
    Configuration,
    /// Progress panel (top-right): Live execution status with charts
    Progress,
    /// Results panel (bottom-left): Test results table
    Results,
    /// Details panel (bottom-right): Selected result details/diffs
    Details,
}

/// Legacy focus state for backward compatibility during transition
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

/// Filter status for results overview
#[derive(Debug, Clone, PartialEq)]
pub enum StatusFilter {
    All,
    Identical,
    Different,
    ErrorsOnly,
}

/// Filter state for results view
#[derive(Debug, Clone)]
pub struct FilterState {
    /// Current status filter
    pub status_filter: StatusFilter,
    /// Environment filter (None = all environments)
    pub environment_filter: Option<String>,
    /// Route name pattern filter
    pub route_pattern: Option<String>,
    /// Whether filter panel is open
    pub show_filter_panel: bool,
    /// Current tab index for filter navigation
    pub current_tab: usize,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            status_filter: StatusFilter::All,
            environment_filter: None,
            route_pattern: None,
            show_filter_panel: false,
            current_tab: 0,
        }
    }
}

/// Dashboard-only viewing mode for the TUI
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    /// Dashboard mode with 4 simultaneous panels - the only supported view
    Dashboard,
}

// Removed PanelSize in favor of a single expanded panel option for simplicity

/// Details panel tab selection
#[derive(Debug, Clone, PartialEq)]
pub enum DetailsTab {
    /// Overview of the result
    Overview,
    /// Detailed differences
    Diffs,
    /// Error information
    Errors,
    /// Suggestions and recommendations
    Suggestions,
}

impl DetailsTab {
    /// Get the tab index for ratatui Tabs widget
    pub fn as_index(&self) -> usize {
        match self {
            DetailsTab::Overview => 0,
            DetailsTab::Diffs => 1,
            DetailsTab::Errors => 2,
            DetailsTab::Suggestions => 3,
        }
    }

    /// Create from tab index
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => DetailsTab::Overview,
            1 => DetailsTab::Diffs,
            2 => DetailsTab::Errors,
            3 => DetailsTab::Suggestions,
            _ => DetailsTab::Overview,
        }
    }

    /// Get tab title
    pub fn title(&self) -> &'static str {
        match self {
            DetailsTab::Overview => "Overview",
            DetailsTab::Diffs => "Diffs",
            DetailsTab::Errors => "Errors",
            DetailsTab::Suggestions => "Suggestions",
        }
    }
}

/// Main TUI application state transitioning to dashboard architecture
pub struct TuiApp {
    /// All comparison results to display
    pub results: Vec<ComparisonResult>,
    /// Currently selected result index
    pub selected_index: usize,
    /// Current viewing mode (legacy support)
    pub view_mode: ViewMode,
    /// Dashboard panel focus for 4-panel layout
    pub panel_focus: PanelFocus,
    /// Expanded panel, if any
    pub expanded_panel: Option<PanelFocus>,
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
    /// Filter state for results view
    pub filter_state: FilterState,

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
    /// Progress tracker from HTTP execution engine
    pub progress_tracker: Option<ProgressTracker>,
    /// Current operation description
    pub current_operation: String,
    /// Duration of the last completed execution
    pub last_execution_duration: Option<std::time::Duration>,
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

    // List state for proper cursor positioning
    /// ListState for environments list widget
    pub env_list_state: ListState,
    /// ListState for routes list widget  
    pub route_list_state: ListState,
    /// TableState for results table widget
    pub results_table_state: TableState,

    // Scrollbar states for visual scroll indicators
    /// ScrollbarState for environments list scrollbar
    pub env_scrollbar_state: ScrollbarState,
    /// ScrollbarState for routes list scrollbar
    pub route_scrollbar_state: ScrollbarState,
    /// ScrollbarState for results table scrollbar
    pub results_scrollbar_state: ScrollbarState,

    // Details panel state
    /// Current tab in details panel
    pub details_current_tab: DetailsTab,
    /// Details panel specific diff style (independent of global)
    pub details_diff_style: DiffViewStyle,
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
            view_mode: ViewMode::Dashboard, // Start in dashboard mode
            panel_focus: PanelFocus::Results, // Focus on results when starting with data
            expanded_panel: None,
            diff_style: diff_style.clone(),
            show_headers,
            show_errors,
            should_quit: false,
            scroll_offset: 0,
            filter_state: FilterState::default(),
            available_environments: Vec::new(),
            selected_environments: Vec::new(),
            available_routes: Vec::new(),
            selected_routes: Vec::new(),
            config_path: "http-diff.toml".to_string(),
            users_file: "users.csv".to_string(),
            error_message: None,
            progress_tracker: None,
            current_operation: String::new(),
            last_execution_duration: None,
            execution_requested: false,
            execution_running: false,
            execution_cancelled: false,
            focused_panel: FocusedPanel::Environments,
            action_feedback: None,
            show_help: false,
            selected_env_index: 0,
            selected_route_index: 0,
            env_list_state: ListState::default(),
            route_list_state: ListState::default(),
            results_table_state: TableState::default(),
            env_scrollbar_state: ScrollbarState::default(),
            route_scrollbar_state: ScrollbarState::default(),
            results_scrollbar_state: ScrollbarState::default(),
            details_current_tab: DetailsTab::Overview,
            details_diff_style: diff_style,
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
            view_mode: ViewMode::Dashboard, // Start in dashboard mode for workflow
            panel_focus: PanelFocus::Configuration, // Focus on configuration when starting workflow
            expanded_panel: None,
            diff_style: diff_style.clone(),
            show_headers,
            show_errors,
            should_quit: false,
            scroll_offset: 0,
            filter_state: FilterState::default(),
            available_environments: Vec::new(),
            selected_environments: Vec::new(),
            available_routes: Vec::new(),
            selected_routes: Vec::new(),
            config_path: "http-diff.toml".to_string(),
            users_file: "users.csv".to_string(),
            error_message: None,
            progress_tracker: None,
            current_operation: "Loading configuration...".to_string(),
            last_execution_duration: None,
            execution_requested: false,
            execution_running: false,
            execution_cancelled: false,
            focused_panel: FocusedPanel::Environments,
            action_feedback: None,
            show_help: false,
            selected_env_index: 0,
            selected_route_index: 0,
            env_list_state: ListState::default(),
            route_list_state: ListState::default(),
            results_table_state: TableState::default(),
            env_scrollbar_state: ScrollbarState::default(),
            route_scrollbar_state: ScrollbarState::default(),
            results_scrollbar_state: ScrollbarState::default(),
            details_current_tab: DetailsTab::Overview,
            details_diff_style: diff_style,
        }
    }

    /// Get the currently selected result
    pub fn current_result(&self) -> Option<&ComparisonResult> {
        self.results.get(self.selected_index)
    }

    /// Move to the next result (now with inter-panel communication)
    pub fn next_result(&mut self) {
        let old_index = self.selected_index;
        let filtered_results = self.filtered_results();
        if self.selected_index < filtered_results.len().saturating_sub(1) {
            self.selected_index += 1;
            self.scroll_offset = 0; // Reset scroll when changing results
            if old_index != self.selected_index {
                self.on_result_selection_changed();
            }
        }
    }

    /// Move to the previous result (now with inter-panel communication)
    pub fn previous_result(&mut self) {
        let old_index = self.selected_index;
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.scroll_offset = 0; // Reset scroll when changing results
            if old_index != self.selected_index {
                self.on_result_selection_changed();
            }
        }
    }

    /// Switch to the next view mode (legacy method - now only handles panel navigation)
    pub fn next_view(&mut self) {
        // Only dashboard mode is supported - use panel navigation instead
        self.next_dashboard_panel();
    }

    /// Switch to the previous view mode (legacy method - now only handles panel navigation)
    pub fn previous_view(&mut self) {
        // Only dashboard mode is supported - use panel navigation instead
        self.previous_dashboard_panel();
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

    /// Get the title for the current view (always Dashboard mode)
    pub fn get_view_title(&self) -> String {
        "HTTP API Testing Dashboard".to_string()
    }

    /// Get the title for a specific dashboard panel
    pub fn get_panel_title(&self, panel: &PanelFocus) -> String {
        match panel {
            PanelFocus::Configuration => "Configuration".to_string(),
            PanelFocus::Progress => {
                if self.execution_running {
                    if let Some(ref tracker) = self.progress_tracker {
                        format!(
                            "Execution Progress ({}/{})",
                            tracker.completed_requests, tracker.total_requests
                        )
                    } else {
                        "Execution Starting...".to_string()
                    }
                } else if self.results.is_empty() {
                    "Execution Ready".to_string()
                } else {
                    "Execution Complete".to_string()
                }
            }
            PanelFocus::Results => {
                let (total, _identical, _different, errors) = self.get_filter_counts();
                format!("Results ({} total, {} errors)", total, errors)
            }
            PanelFocus::Details => {
                if let Some(result) = self.current_filtered_result() {
                    format!("Details - {}", result.route_name)
                } else {
                    "Details".to_string()
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

    /// Get help text for the current view (always Dashboard mode)
    pub fn get_help_text(&self) -> &'static str {
        "Tab: Switch panels | ↑↓←→: Navigate | R: Run tests | S: Save HTML report | 1-4: Tabs (Details) | D: Toggle diff | x: Expand | q: Quit"
    }

    // === Dashboard Panel Navigation ===

    /// Switch to the next panel in dashboard mode
    pub fn next_dashboard_panel(&mut self) {
        self.panel_focus = match self.panel_focus {
            PanelFocus::Configuration => PanelFocus::Progress,
            PanelFocus::Progress => PanelFocus::Results,
            PanelFocus::Results => PanelFocus::Details,
            PanelFocus::Details => PanelFocus::Configuration,
        };
        self.scroll_offset = 0; // Reset scroll when changing panels
    }

    /// Switch to the previous panel in dashboard mode
    pub fn previous_dashboard_panel(&mut self) {
        self.panel_focus = match self.panel_focus {
            PanelFocus::Configuration => PanelFocus::Details,
            PanelFocus::Progress => PanelFocus::Configuration,
            PanelFocus::Results => PanelFocus::Progress,
            PanelFocus::Details => PanelFocus::Results,
        };
        self.scroll_offset = 0; // Reset scroll when changing panels
    }

    /// Check if a panel is currently focused in dashboard mode
    pub fn is_panel_focused(&self, panel: &PanelFocus) -> bool {
        self.panel_focus == *panel
    }

    /// Toggle panel expansion (only one panel can be expanded)
    pub fn toggle_panel_expansion(&mut self, panel: PanelFocus) {
        self.expanded_panel = match self.expanded_panel {
            Some(ref p) if *p == panel => None,
            _ => Some(panel),
        };
    }

    /// Switch to the next tab in details panel
    pub fn next_details_tab(&mut self) {
        self.details_current_tab = match self.details_current_tab {
            DetailsTab::Overview => DetailsTab::Diffs,
            DetailsTab::Diffs => DetailsTab::Errors,
            DetailsTab::Errors => DetailsTab::Suggestions,
            DetailsTab::Suggestions => DetailsTab::Overview,
        };
    }

    /// Switch to the previous tab in details panel
    pub fn previous_details_tab(&mut self) {
        self.details_current_tab = match self.details_current_tab {
            DetailsTab::Overview => DetailsTab::Suggestions,
            DetailsTab::Diffs => DetailsTab::Overview,
            DetailsTab::Errors => DetailsTab::Diffs,
            DetailsTab::Suggestions => DetailsTab::Errors,
        };
    }

    /// Switch to specific details tab by number (1-4)
    pub fn switch_details_tab(&mut self, tab_number: usize) {
        if (1..=4).contains(&tab_number) {
            self.details_current_tab = DetailsTab::from_index(tab_number - 1);
        }
    }

    /// Toggle details panel specific diff style
    pub fn toggle_details_diff_style(&mut self) {
        self.details_diff_style = match self.details_diff_style {
            DiffViewStyle::Unified => DiffViewStyle::SideBySide,
            DiffViewStyle::SideBySide => DiffViewStyle::Unified,
        };
    }

    // === Inter-Panel Communication ===

    /// Update reactive state between panels when data changes
    pub fn update_panel_reactive_state(&mut self) {
        // If no results and not executing, ensure configuration panel is accessible
        if self.results.is_empty()
            && !self.execution_running
            && self.panel_focus == PanelFocus::Results
        {
            self.panel_focus = PanelFocus::Configuration;
        }

        // If results available and focused on progress, switch to results
        if !self.results.is_empty()
            && self.panel_focus == PanelFocus::Progress
            && !self.execution_running
        {
            self.panel_focus = PanelFocus::Results;
        }
    }

    /// React to result selection changes by updating dependent panels
    pub fn on_result_selection_changed(&mut self) {
        // When result selection changes, the details panel should update
        // This is handled automatically by the rendering system,
        // but we could add specific reactions here if needed

        // Reset scroll when changing selection
        self.scroll_offset = 0;

        // Sync table state with new selection
        self.sync_results_table_state();
    }

    /// Handle configuration changes and update dependent panels
    pub fn on_configuration_changed(&mut self) {
        // Clear results when configuration changes
        if self.execution_running {
            // Don't clear during execution
            return;
        }

        // Reset execution state when config changes
        self.execution_requested = false;
        self.execution_cancelled = false;
        self.current_operation = "Ready to execute".to_string();

        // Update reactive state
        self.update_panel_reactive_state();
    }

    /// Enhanced result navigation that updates inter-panel communication
    pub fn navigate_to_result(&mut self, index: usize) {
        let filtered_results = self.filtered_results();
        if index < filtered_results.len() {
            self.selected_index = index;
            self.on_result_selection_changed();
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

        // Sync ListState after loading configuration
        self.sync_env_list_state();
        self.sync_route_list_state();

        // Trigger inter-panel communication
        self.on_configuration_changed();

        Ok(())
    }

    /// Toggle environment selection
    pub fn toggle_environment(&mut self, index: usize) {
        if let Some(env_name) = self.available_environments.get(index) {
            if let Some(pos) = self
                .selected_environments
                .iter()
                .position(|x| x == env_name)
            {
                self.selected_environments.remove(pos);
            } else {
                self.selected_environments.push(env_name.clone());
            }
            // Trigger inter-panel communication
            self.on_configuration_changed();
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
            // Trigger inter-panel communication
            self.on_configuration_changed();
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
            self.error_message =
                Some("Please select at least one environment and route".to_string());
            return;
        }

        self.execution_requested = true;
        self.execution_running = false;
        self.execution_cancelled = false;
        self.progress_tracker = None; // Will be updated by progress callbacks
        self.current_operation = "Preparing to start HTTP tests...".to_string();
        self.error_message = None;
    }

    /// Start the actual execution (called by TUI runner when async task starts)
    pub fn start_execution(&mut self) {
        // In dashboard mode, keep the current view but focus on progress panel
        self.panel_focus = PanelFocus::Progress;
        self.execution_requested = false;
        self.execution_running = true;
        self.execution_cancelled = false;
        self.current_operation = "Starting HTTP tests...".to_string();
        self.last_execution_duration = None;
        
        // Create initial progress tracker to show immediate feedback
        // The total will be updated when the first real progress update arrives
        self.progress_tracker = Some(ProgressTracker::new(1));
    }

    /// Cancel execution
    pub fn cancel_execution(&mut self) {
        self.execution_cancelled = true;
        self.execution_running = false;
        self.current_operation = "Cancelling execution...".to_string();
    }

    /// Update execution progress
    pub fn update_execution_progress(&mut self, tracker: ProgressTracker, operation: String) {
        self.progress_tracker = Some(tracker);
        self.current_operation = operation;
    }

    /// Complete execution and move to results
    pub fn complete_execution(&mut self, results: Vec<ComparisonResult>) {
        self.results = results;
        // Focus on results panel after execution completes
        self.panel_focus = PanelFocus::Results;
        // Auto-select first result if available
        if !self.results.is_empty() {
            self.selected_index = 0;
        }
        self.selected_index = 0;
        // Calculate and store duration if available from progress tracker
        if let Some(ref tracker) = self.progress_tracker {
            self.last_execution_duration = Some(tracker.elapsed_time());
        }
        self.execution_running = false;
        self.execution_requested = false;
        // Sync table state with new results
        self.sync_results_table_state();
        self.execution_cancelled = false;
        self.current_operation = "Execution completed".to_string();

        // Trigger inter-panel update
        self.update_panel_reactive_state();
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
        self.sync_env_list_state();
        self.sync_route_list_state();
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
        self.sync_env_list_state();
        self.sync_route_list_state();
    }

    /// Navigate up in the current panel
    pub fn navigate_up(&mut self) {
        match self.focused_panel {
            FocusedPanel::Environments => {
                if self.selected_env_index > 0 {
                    self.selected_env_index -= 1;
                    self.sync_env_list_state();
                }
            }
            FocusedPanel::Routes => {
                if self.selected_route_index > 0 {
                    self.selected_route_index -= 1;
                    self.sync_route_list_state();
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
                    self.sync_env_list_state();
                }
            }
            FocusedPanel::Routes => {
                if self.selected_route_index < self.available_routes.len().saturating_sub(1) {
                    self.selected_route_index += 1;
                    self.sync_route_list_state();
                }
            }
            FocusedPanel::Actions => {
                // Actions panel doesn't have navigation
            }
        }
    }

    /// Sync environment ListState with current index
    pub fn sync_env_list_state(&mut self) {
        if !self.available_environments.is_empty() {
            self.env_list_state.select(Some(self.selected_env_index));
        } else {
            self.env_list_state.select(None);
        }
        // Update scrollbar to reflect current state
        self.env_scrollbar_state = self
            .env_scrollbar_state
            .content_length(self.available_environments.len())
            .position(self.selected_env_index);
    }

    /// Sync route ListState with current index  
    pub fn sync_route_list_state(&mut self) {
        if !self.available_routes.is_empty() {
            self.route_list_state
                .select(Some(self.selected_route_index));
        } else {
            self.route_list_state.select(None);
        }
        // Update scrollbar to reflect current state
        self.route_scrollbar_state = self
            .route_scrollbar_state
            .content_length(self.available_routes.len())
            .position(self.selected_route_index);
    }

    /// Sync results TableState with current index
    pub fn sync_results_table_state(&mut self) {
        let filtered_count = self.filtered_results().len();
        if filtered_count > 0 && self.selected_index < filtered_count {
            self.results_table_state.select(Some(self.selected_index));
        } else {
            self.results_table_state.select(None);
        }
        // Update scrollbar to reflect current state
        self.results_scrollbar_state = self
            .results_scrollbar_state
            .content_length(filtered_count)
            .position(self.selected_index);
    }

    /// Update scrollbar states based on current content and selection
    pub fn update_scrollbar_states(&mut self) {
        // Update environment scrollbar state
        self.env_scrollbar_state = self
            .env_scrollbar_state
            .content_length(self.available_environments.len())
            .position(self.selected_env_index);

        // Update routes scrollbar state
        self.route_scrollbar_state = self
            .route_scrollbar_state
            .content_length(self.available_routes.len())
            .position(self.selected_route_index);

        // Update results scrollbar state
        let filtered_results = self.filtered_results();
        self.results_scrollbar_state = self
            .results_scrollbar_state
            .content_length(filtered_results.len())
            .position(self.selected_index);
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

    // === Filter Management ===

    /// Get filtered results based on current filter state
    pub fn filtered_results(&self) -> Vec<&ComparisonResult> {
        self.results
            .iter()
            .filter(|result| {
                // Status filter
                match self.filter_state.status_filter {
                    StatusFilter::All => true,
                    StatusFilter::Identical => result.is_identical && !result.has_errors,
                    StatusFilter::Different => !result.is_identical && !result.has_errors,
                    StatusFilter::ErrorsOnly => result.has_errors,
                }
            })
            .filter(|result| {
                // Environment filter
                if let Some(ref env_filter) = self.filter_state.environment_filter {
                    result.responses.contains_key(env_filter)
                } else {
                    true
                }
            })
            .filter(|result| {
                // Route pattern filter
                if let Some(ref pattern) = self.filter_state.route_pattern {
                    result
                        .route_name
                        .to_lowercase()
                        .contains(&pattern.to_lowercase())
                } else {
                    true
                }
            })
            .collect()
    }

    /// Get count for each filter tab
    pub fn get_filter_counts(&self) -> (usize, usize, usize, usize) {
        let total = self.results.len();
        let identical = self
            .results
            .iter()
            .filter(|r| r.is_identical && !r.has_errors)
            .count();
        let different = self
            .results
            .iter()
            .filter(|r| !r.is_identical && !r.has_errors)
            .count();
        let errors = self.results.iter().filter(|r| r.has_errors).count();
        (total, identical, different, errors)
    }

    /// Switch to next filter tab
    pub fn next_filter_tab(&mut self) {
        self.filter_state.current_tab = (self.filter_state.current_tab + 1) % 4;
        self.update_filter_from_tab();
    }

    /// Switch to previous filter tab
    pub fn previous_filter_tab(&mut self) {
        self.filter_state.current_tab = if self.filter_state.current_tab == 0 {
            3
        } else {
            self.filter_state.current_tab - 1
        };
        self.update_filter_from_tab();
    }

    /// Update filter based on current tab
    fn update_filter_from_tab(&mut self) {
        self.filter_state.status_filter = match self.filter_state.current_tab {
            0 => StatusFilter::All,
            1 => StatusFilter::Identical,
            2 => StatusFilter::Different,
            3 => StatusFilter::ErrorsOnly,
            _ => StatusFilter::All,
        };
        // Reset selection when filter changes
        self.selected_index = 0;
    }

    /// Toggle filter panel visibility
    pub fn toggle_filter_panel(&mut self) {
        self.filter_state.show_filter_panel = !self.filter_state.show_filter_panel;
    }

    /// Set route pattern filter
    pub fn set_route_pattern(&mut self, pattern: Option<String>) {
        self.filter_state.route_pattern = pattern;
        self.selected_index = 0; // Reset selection when filter changes
    }

    /// Set environment filter
    pub fn set_environment_filter(&mut self, env: Option<String>) {
        self.filter_state.environment_filter = env;
        self.selected_index = 0; // Reset selection when filter changes
    }

    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.filter_state = FilterState::default();
        self.selected_index = 0;
    }

    /// Get current result accounting for filters
    pub fn current_filtered_result(&self) -> Option<&ComparisonResult> {
        let filtered = self.filtered_results();
        filtered.get(self.selected_index).copied()
    }

    /// Get position info for current result
    pub fn get_filter_position_info(&self) -> (usize, usize) {
        let filtered = self.filtered_results();
        let current_pos = if filtered.is_empty() {
            0
        } else {
            self.selected_index + 1
        };
        (current_pos, filtered.len())
    }

    /// Generate and save HTML report from current results
    pub fn generate_html_report(&mut self) -> Result<String, String> {
        if self.results.is_empty() {
            return Err("No results available to generate report".to_string());
        }

        // Always use HTML format - generate filename with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let report_filename = format!("http-diff-report-{}.html", timestamp);

        // Create report renderer (always HTML)
        let report_renderer = ReportRendererFactory::create_renderer(&report_filename);

        // Create metadata using selected or detected environments from results
        let env_names: Vec<String> = if !self.selected_environments.is_empty() {
            self.selected_environments.clone()
        } else {
            use std::collections::BTreeSet;
            let mut set: BTreeSet<String> = BTreeSet::new();
            for result in &self.results {
                for env in result.responses.keys() {
                    set.insert(env.clone());
                }
            }
            set.into_iter().collect()
        };

        let duration = self
            .last_execution_duration
            .unwrap_or_else(|| std::time::Duration::from_secs(0));

        let metadata = ReportMetadata::new(env_names, self.results.len())
            .with_duration(duration)
            .with_context("source", "TUI")
            .with_context(
                "diff_view",
                match self.diff_style {
                    DiffViewStyle::Unified => "unified",
                    DiffViewStyle::SideBySide => "side-by-side",
                },
            )
            .with_context("headers_included", self.show_headers.to_string())
            .with_context("errors_included", self.show_errors.to_string());

        // Generate report content
        let report_content = report_renderer.render_report(&self.results, &metadata);

        // Write to file
        fs::write(&report_filename, report_content)
            .map_err(|e| format!("Failed to write report file: {}", e))?;

        // Show success feedback
        self.show_feedback(
            &format!("HTML report saved to {}", report_filename),
            FeedbackType::Success,
        );

        Ok(report_filename)
    }
}
