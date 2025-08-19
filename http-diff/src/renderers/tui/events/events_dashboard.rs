use crate::renderers::tui::app::{PanelFocus, TuiApp};
use crate::renderers::tui::msg::{DetailsMsg, Msg, ResultsMsg};
use crossterm::event::{KeyCode, KeyEvent};

/// Map keys for dashboard to top-level Msg
pub fn map_dashboard_keys_to_msg(app: &TuiApp, key: KeyEvent) -> Option<Msg> {
    match key.code {
        // Tab navigation between panels
        KeyCode::Tab => return Some(Msg::FocusNextPane),
        KeyCode::BackTab => return Some(Msg::FocusPrevPane),
        // Panel-specific navigation and actions
        KeyCode::Up | KeyCode::Char('k') => {
            return Some(match app.panel_focus {
                PanelFocus::Configuration => {
                    Msg::Config(crate::renderers::tui::msg::ConfigMsg::MoveUp)
                }
                PanelFocus::Progress => return None,
                PanelFocus::Results => Msg::Results(ResultsMsg::MoveUp),
                PanelFocus::Details => Msg::Details(DetailsMsg::ScrollUp),
            });
        }
        KeyCode::Down | KeyCode::Char('j') => {
            return Some(match app.panel_focus {
                PanelFocus::Configuration => {
                    Msg::Config(crate::renderers::tui::msg::ConfigMsg::MoveDown)
                }
                PanelFocus::Progress => return None,
                PanelFocus::Results => Msg::Results(ResultsMsg::MoveDown),
                PanelFocus::Details => Msg::Details(DetailsMsg::ScrollDown),
            });
        }
        KeyCode::Left | KeyCode::Char('h') => {
            return Some(match app.panel_focus {
                PanelFocus::Configuration => {
                    Msg::Config(crate::renderers::tui::msg::ConfigMsg::FocusPrev)
                }
                _ => return None,
            });
        }
        KeyCode::Right | KeyCode::Char('l') => {
            return Some(match app.panel_focus {
                PanelFocus::Configuration => {
                    Msg::Config(crate::renderers::tui::msg::ConfigMsg::FocusNext)
                }
                _ => return None,
            });
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            return Some(match app.panel_focus {
                PanelFocus::Configuration => {
                    // If config not loaded, load it; otherwise toggle currently focused item
                    if app.available_environments.is_empty() {
                        Msg::Config(crate::renderers::tui::msg::ConfigMsg::Load)
                    } else {
                        match app.focused_panel {
                            crate::renderers::tui::app::FocusedPanel::Environments => {
                                Msg::Config(crate::renderers::tui::msg::ConfigMsg::ToggleEnv(
                                    app.selected_env_index,
                                ))
                            }
                            crate::renderers::tui::app::FocusedPanel::Routes => {
                                Msg::Config(crate::renderers::tui::msg::ConfigMsg::ToggleRoute(
                                    app.selected_route_index,
                                ))
                            }
                            crate::renderers::tui::app::FocusedPanel::Actions => {
                                // No activation on actions; keep behavior simple
                                return None;
                            }
                        }
                    }
                }
                PanelFocus::Results => Msg::Details(DetailsMsg::ScrollTop),
                _ => return None,
            });
        }
        KeyCode::PageUp => {
            return Some(match app.panel_focus {
                PanelFocus::Details => Msg::Details(DetailsMsg::PageUp),
                PanelFocus::Results => Msg::Results(ResultsMsg::PageUp),
                _ => return None,
            });
        }
        KeyCode::PageDown => {
            return Some(match app.panel_focus {
                PanelFocus::Details => Msg::Details(DetailsMsg::PageDown),
                PanelFocus::Results => Msg::Results(ResultsMsg::PageDown),
                _ => return None,
            });
        }
        KeyCode::Home => {
            return Some(match app.panel_focus {
                PanelFocus::Details => Msg::Details(DetailsMsg::ScrollTop),
                PanelFocus::Results => Msg::Results(ResultsMsg::Home),
                _ => return None,
            });
        }
        KeyCode::End => {
            return Some(match app.panel_focus {
                PanelFocus::Details => Msg::Details(DetailsMsg::ScrollBottom),
                PanelFocus::Results => Msg::Results(ResultsMsg::End),
                _ => return None,
            });
        }
        // Panel-specific shortcuts
        KeyCode::Char('a') => {
            if app.panel_focus == PanelFocus::Configuration {
                return Some(Msg::Config(
                    crate::renderers::tui::msg::ConfigMsg::SelectAll,
                ));
            }
        }
        KeyCode::Char('n') => {
            if app.panel_focus == PanelFocus::Configuration {
                return Some(Msg::Config(crate::renderers::tui::msg::ConfigMsg::ClearAll));
            }
        }
        KeyCode::Char('f') => {
            if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::ToggleFilterPanel));
            }
        }
        KeyCode::Char('c') => {
            if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::ClearFilters));
            }
        }
        KeyCode::Char('[') => {
            if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::PrevFilterTab));
            }
        }
        KeyCode::Char(']') => {
            if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::NextFilterTab));
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Execute tests from any panel (main execution trigger)
            if !app.selected_environments.is_empty() && !app.selected_routes.is_empty() {
                return Some(Msg::StartExecution);
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            return Some(Msg::SaveReport);
        }
        // Details panel specific keys
        KeyCode::Char('1') => {
            if app.panel_focus == PanelFocus::Details {
                return Some(Msg::Details(DetailsMsg::SetTab(1)));
            } else if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::SetFilterTab(1)));
            }
        }
        KeyCode::Char('2') => {
            if app.panel_focus == PanelFocus::Details {
                return Some(Msg::Details(DetailsMsg::SetTab(2)));
            } else if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::SetFilterTab(2)));
            }
        }
        KeyCode::Char('3') => {
            if app.panel_focus == PanelFocus::Details {
                return Some(Msg::Details(DetailsMsg::SetTab(3)));
            } else if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::SetFilterTab(3)));
            }
        }
        KeyCode::Char('4') => {
            if app.panel_focus == PanelFocus::Details {
                return Some(Msg::Details(DetailsMsg::SetTab(4)));
            } else if app.panel_focus == PanelFocus::Results {
                return Some(Msg::Results(ResultsMsg::SetFilterTab(4)));
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if app.panel_focus == PanelFocus::Details {
                // Toggle diff style in details panel
                return Some(Msg::Details(DetailsMsg::ToggleDiffStyle));
            } else {
                // Global diff style toggle (existing behavior)
                return Some(Msg::ToggleDiffStyle);
            }
        }
        _ => {}
    }
    None
}
// Implementation intentionally moved to message mapping above; mutations happen in reducer
