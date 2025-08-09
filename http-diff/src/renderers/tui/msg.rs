use crate::types::ComparisonResult;

/// Top-level application messages (unidirectional flow)
#[derive(Debug, Clone)]
pub enum Msg {
    Quit,
    ToggleHelp,
    ToggleExpanded(super::app::PanelFocus),
    FocusNextPane,
    FocusPrevPane,
    ToggleDiffStyle,
    ToggleHeaders,
    ToggleErrors,

    // Panel-specific messages
    Config(ConfigMsg),
    Results(ResultsMsg),
    Details(DetailsMsg),

    // Execution lifecycle
    StartExecution,
    Exec(ExecMsg),

    // Report generation
    SaveReport,
}

#[derive(Debug, Clone)]
pub enum ConfigMsg {
    Load,
    ToggleEnv(usize),
    ToggleRoute(usize),
    SelectAll,
    ClearAll,
    MoveUp,
    MoveDown,
    FocusNext,
    FocusPrev,
}

#[derive(Debug, Clone)]
pub enum ResultsMsg {
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Home,
    End,
    SetFilterTab(usize),
    ToggleFilterPanel,
    ClearFilters,
}

#[derive(Debug, Clone)]
pub enum DetailsMsg {
    TabNext,
    TabPrev,
    SetTab(usize),
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    ScrollTop,
    ScrollBottom,
    ToggleDiffStyle,
}

/// Execution messages coming from async runner
#[derive(Debug, Clone)]
pub enum ExecMsg {
    Progress { completed: usize, total: usize, op: String },
    Completed(Vec<ComparisonResult>),
    Failed(String),
}


