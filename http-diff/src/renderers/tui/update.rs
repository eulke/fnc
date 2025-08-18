use super::app::{PanelFocus, TuiApp};
use super::msg::{ConfigMsg, DetailsMsg, ExecMsg, Msg, ResultsMsg};

/// Side effects produced by the reducer. The main loop should execute them.
#[derive(Debug, Clone)]
pub enum Effect {
    None,
    StartExec {
        config_path: String,
        users: String,
        envs: Vec<String>,
        routes: Vec<String>,
        include_headers: bool,
        include_errors: bool,
    },
    SaveReport,
    Quit,
}

impl Default for Effect {
    fn default() -> Self {
        Effect::None
    }
}

pub fn update(app: &mut TuiApp, msg: Msg) -> Effect {
    match msg {
        Msg::Quit => Effect::Quit,
        Msg::ToggleHelp => {
            app.toggle_help();
            Effect::None
        }
        Msg::ToggleExpanded(panel) => {
            app.toggle_panel_expansion(panel);
            Effect::None
        }
        Msg::FocusNextPane => {
            app.next_dashboard_panel();
            Effect::None
        }
        Msg::FocusPrevPane => {
            app.previous_dashboard_panel();
            Effect::None
        }
        Msg::ToggleDiffStyle => {
            app.toggle_diff_style();
            Effect::None
        }
        Msg::ToggleHeaders => {
            app.toggle_headers();
            Effect::None
        }
        Msg::ToggleErrors => {
            app.toggle_errors();
            Effect::None
        }

        Msg::Config(c) => handle_config(app, c),
        Msg::Results(r) => handle_results(app, r),
        Msg::Details(d) => handle_details(app, d),

        Msg::StartExecution => {
            if app.selected_environments.is_empty() || app.selected_routes.is_empty() {
                app.show_feedback(
                    "Select at least one environment and route",
                    super::app::FeedbackType::Warning,
                );
                Effect::None
            } else {
                app.start_execution();
                Effect::StartExec {
                    config_path: app.config_path.clone(),
                    users: app.users_file.clone(),
                    envs: app.selected_environments.clone(),
                    routes: app.selected_routes.clone(),
                    include_headers: app.show_headers,
                    include_errors: app.show_errors,
                }
            }
        }
        Msg::Exec(em) => {
            match em {
                ExecMsg::Progress {
                    tracker,
                    op,
                } => {
                    app.update_execution_progress(tracker, op);
                }
                ExecMsg::Completed(results) => {
                    app.complete_execution(results);
                }
                ExecMsg::Failed(err) => {
                    app.set_error(format!("Execution failed: {}", err));
                    app.panel_focus = PanelFocus::Configuration;
                    app.execution_running = false;
                    app.execution_requested = false;
                    app.execution_cancelled = false;
                    app.current_operation = "Execution failed".to_string();
                }
            }
            Effect::None
        }

        Msg::SaveReport => Effect::SaveReport,
    }
}

fn handle_config(app: &mut TuiApp, msg: ConfigMsg) -> Effect {
    match msg {
        ConfigMsg::Load => match app.load_configuration() {
            Ok(()) => app.clear_error(),
            Err(e) => app.set_error(e),
        },
        ConfigMsg::ToggleEnv(i) => app.toggle_environment(i),
        ConfigMsg::ToggleRoute(i) => app.toggle_route(i),
        ConfigMsg::SelectAll => app.select_all_focused(),
        ConfigMsg::ClearAll => app.clear_all_focused(),
        ConfigMsg::MoveUp => app.navigate_up(),
        ConfigMsg::MoveDown => app.navigate_down(),
        ConfigMsg::FocusNext => app.next_panel(),
        ConfigMsg::FocusPrev => app.previous_panel(),
    }
    Effect::None
}

fn handle_results(app: &mut TuiApp, msg: ResultsMsg) -> Effect {
    match msg {
        ResultsMsg::MoveUp => app.previous_result(),
        ResultsMsg::MoveDown => app.next_result(),
        ResultsMsg::PageUp => {
            app.selected_index = app.selected_index.saturating_sub(10);
            app.sync_results_table_state();
        }
        ResultsMsg::PageDown => {
            let max = app.filtered_results().len().saturating_sub(1);
            app.selected_index = (app.selected_index + 10).min(max);
            app.sync_results_table_state();
        }
        ResultsMsg::Home => {
            app.selected_index = 0;
            app.sync_results_table_state();
        }
        ResultsMsg::End => {
            let max = app.filtered_results().len().saturating_sub(1);
            app.selected_index = max;
            app.sync_results_table_state();
        }
        ResultsMsg::SetFilterTab(n) => app.switch_details_tab(n), // reuse existing for now
        ResultsMsg::ToggleFilterPanel => app.toggle_filter_panel(),
        ResultsMsg::ClearFilters => app.clear_filters(),
    }
    Effect::None
}

fn handle_details(app: &mut TuiApp, msg: DetailsMsg) -> Effect {
    match msg {
        DetailsMsg::TabNext => app.next_details_tab(),
        DetailsMsg::TabPrev => app.previous_details_tab(),
        DetailsMsg::SetTab(n) => app.switch_details_tab(n),
        DetailsMsg::ScrollUp => app.scroll_up(),
        DetailsMsg::ScrollDown => app.scroll_down(),
        DetailsMsg::PageUp => app.page_up(),
        DetailsMsg::PageDown => app.page_down(),
        DetailsMsg::ScrollTop => app.scroll_to_top(),
        DetailsMsg::ScrollBottom => app.scroll_to_bottom(100),
        DetailsMsg::ToggleDiffStyle => app.toggle_details_diff_style(),
    }
    Effect::None
}
