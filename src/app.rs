use crate::dimension::{Dimension, DimensionConfig, Tab};
use crate::tmux::Tmux;
use anyhow::Result;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    CreatingDimension,
    AddingTab,
    DeletingDimension,
    Searching,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    DimensionOnly,   // Dimension name matched
    TabOnly,         // Tab name matched
    Both,            // Both matched
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub dimension_index: usize,
    pub dimension_name: String,
    pub tab_index: usize,
    pub tab_name: String,
    pub score: i64,
    pub match_type: MatchType,
}

pub struct App {
    pub config: DimensionConfig,
    pub selected_dimension: usize,
    pub selected_tab: Option<usize>, // None means dimension selected, Some(i) means tab i selected
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_selected_index: usize,
    pub last_computed_query: String,
    pub pre_search_dimension: usize,
    pub pre_search_tab: Option<usize>,
    pub message: Option<String>,
    pub should_quit: bool,
    pub should_attach: Option<String>, // Session name to attach to after quitting
    pub should_select_window: Option<usize>, // Window index to select after attaching
    pub should_detach: bool, // Whether to detach from tmux on quit
    pub current_session: Option<String>, // Current tmux session when app was opened
    pub current_window: Option<usize>, // Current tmux window index when app was opened
}

impl App {
    pub fn new() -> Result<Self> {
        let config = DimensionConfig::load()?;

        // Detect current tmux session and window if inside tmux
        let (current_session, current_window) = if Tmux::is_inside_session() {
            let session = Tmux::get_current_session().ok();
            let window = Tmux::get_current_window_index().ok();
            (session, window)
        } else {
            (None, None)
        };

        Ok(Self {
            config,
            selected_dimension: 0,
            selected_tab: None, // Start with dimension selected, not a tab
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected_index: 0,
            last_computed_query: String::new(),
            pre_search_dimension: 0,
            pre_search_tab: None,
            message: None,
            should_quit: false,
            should_attach: None,
            should_select_window: None,
            should_detach: false,
            current_session,
            current_window,
        })
    }

    pub fn save_config(&self) -> Result<()> {
        self.config.save()
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
        self.should_detach = true; // Quit means detach from tmux
    }

    pub fn quit_without_detach(&mut self) {
        self.should_quit = true;
        self.should_detach = false; // Used when switching dimensions
    }

    pub fn close_popup(&mut self) {
        self.should_quit = true;
        self.should_detach = false;
        // Don't set should_attach - just close and stay where we are
    }

    pub fn set_message(&mut self, msg: String) {
        self.message = Some(msg);
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }

    // Navigation
    pub fn next_dimension(&mut self) {
        if !self.config.dimensions.is_empty() {
            self.selected_dimension = (self.selected_dimension + 1) % self.config.dimensions.len();
            self.selected_tab = None; // Reset to dimension when switching dimensions
        }
    }

    pub fn previous_dimension(&mut self) {
        if !self.config.dimensions.is_empty() {
            if self.selected_dimension == 0 {
                self.selected_dimension = self.config.dimensions.len() - 1;
            } else {
                self.selected_dimension -= 1;
            }
            self.selected_tab = None; // Reset to dimension when switching dimensions
        }
    }

    pub fn next_tab(&mut self) {
        if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
            // Get actual window count from tmux if session exists
            let tab_count = if Tmux::session_exists(&dimension.name) {
                Tmux::get_window_count(&dimension.name).unwrap_or(dimension.tabs.len())
            } else {
                dimension.tabs.len()
            };

            if tab_count > 0 {
                match self.selected_tab {
                    None => self.selected_tab = Some(0), // First right arrow selects first tab
                    Some(i) => self.selected_tab = Some((i + 1) % tab_count),
                }
            }
        }
    }

    pub fn previous_tab(&mut self) {
        if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
            // Get actual window count from tmux if session exists
            let tab_count = if Tmux::session_exists(&dimension.name) {
                Tmux::get_window_count(&dimension.name).unwrap_or(dimension.tabs.len())
            } else {
                dimension.tabs.len()
            };

            if tab_count > 0 {
                match self.selected_tab {
                    None => self.selected_tab = Some(tab_count - 1), // Left arrow selects last tab
                    Some(0) => self.selected_tab = None, // Wrap back to dimension
                    Some(i) => self.selected_tab = Some(i - 1),
                }
            }
        }
    }

    // Dimension operations
    pub fn create_dimension(&mut self, name: String) -> Result<()> {
        // Check if dimension already exists
        if self.config.get_dimension(&name).is_some() {
            anyhow::bail!("Dimension '{}' already exists", name);
        }

        // Add to config only - tmux session will be created when switching to it
        let dimension = Dimension::new(name.clone());
        self.config.add_dimension(dimension);
        self.save_config()?;

        self.set_message(format!("Created dimension: {}", name));
        Ok(())
    }

    pub fn delete_dimension(&mut self, name: &str) -> Result<()> {
        // Remove from config
        if self.config.remove_dimension(name).is_none() {
            anyhow::bail!("Dimension '{}' not found", name);
        }

        // Kill tmux session if it exists
        if Tmux::session_exists(name) {
            Tmux::kill_session(name)?;
        }

        self.save_config()?;
        self.set_message(format!("Deleted dimension: {}", name));

        // Adjust selection
        if self.selected_dimension >= self.config.dimensions.len() && self.selected_dimension > 0 {
            self.selected_dimension -= 1;
        }
        self.selected_tab = None;

        Ok(())
    }

    pub fn toggle_collapse_dimension(&mut self) {
        if let Some(dimension) = self.config.dimensions.get_mut(self.selected_dimension) {
            dimension.collapsed = !dimension.collapsed;
            let _ = self.save_config();
        }
    }

    pub fn switch_to_dimension(&mut self) -> Result<()> {
        if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
            let name = dimension.name.clone();
            let has_tabs = !dimension.tabs.is_empty();
            let tabs = dimension.tabs.clone();

            // Ensure tmux session exists
            if !Tmux::session_exists(&name) {
                Tmux::create_session(&name, true)?;

                // Configure minimal status bar
                let _ = Tmux::set_minimal_status_bar();

                // If there are configured tabs, create windows for them
                if has_tabs {
                    for (i, tab) in tabs.iter().enumerate() {
                        if i == 0 {
                            // First window is created with the session, rename it to match first tab
                            Tmux::rename_window(&name, 0, &tab.name)?;

                            // If the first tab has a command, send it
                            if let Some(cmd) = &tab.command {
                                Tmux::send_keys(&name, 0, cmd)?;
                            }
                        } else {
                            Tmux::new_window(&name, &tab.name, tab.command.as_deref())?;
                        }
                    }
                } else {
                    // No configured tabs, create and save ad-hoc tab
                    let ad_hoc_name = format!("{}-1", name);
                    Tmux::rename_window(&name, 0, &ad_hoc_name)?;

                    // Add ad-hoc tab to config
                    let tab = Tab::new(ad_hoc_name, None);
                    if let Some(dim) = self.config.dimensions.get_mut(self.selected_dimension) {
                        dim.add_tab(tab);
                        self.save_config()?;
                    }
                }
            }

            // Determine which window to select
            let window_index = if let Some(selected_tab) = self.selected_tab {
                // Get the actual window index from tmux
                if Tmux::session_exists(&name) {
                    let windows = Tmux::list_windows(&name).unwrap_or_default();
                    windows.get(selected_tab).map(|(idx, _)| *idx).unwrap_or(0)
                } else {
                    selected_tab
                }
            } else {
                0
            };

            // Set the session and window to attach to after exiting TUI
            self.should_attach = Some(name);
            self.should_select_window = Some(window_index);

            // Quit the TUI without detaching (we're switching/attaching to a session)
            self.quit_without_detach();
        }

        Ok(())
    }

    // Tab operations
    pub fn add_tab_to_current_dimension(&mut self, name: String, command: Option<String>) -> Result<()> {
        if let Some(dimension) = self.config.dimensions.get_mut(self.selected_dimension) {
            let tab = Tab::new(name.clone(), command.clone());
            dimension.add_tab(tab);

            // Create window in tmux if session exists
            if Tmux::session_exists(&dimension.name) {
                Tmux::new_window(&dimension.name, &name, command.as_deref())?;
            }

            self.save_config()?;
            self.set_message(format!("Added tab: {}", name));
        }

        Ok(())
    }

    pub fn remove_tab_from_current_dimension(&mut self) -> Result<()> {
        if let Some(tab_index) = self.selected_tab {
            let session_name = {
                if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
                    dimension.name.clone()
                } else {
                    return Ok(());
                }
            };

            // Get the actual window index and name from tmux
            if Tmux::session_exists(&session_name) {
                let windows = Tmux::list_windows(&session_name)?;

                if let Some((window_idx, window_name)) = windows.get(tab_index) {
                    let window_idx = *window_idx;
                    let window_name = window_name.clone();

                    // Kill the tmux window
                    Tmux::kill_window(&session_name, window_idx)?;

                    // Remove from config if it exists there
                    if let Some(dimension) = self.config.dimensions.get_mut(self.selected_dimension) {
                        if let Some(config_index) = dimension.tabs.iter().position(|t| t.name == window_name) {
                            dimension.remove_tab(config_index);
                        }
                    }
                    self.save_config()?;
                    self.set_message(format!("Removed tab: {}", window_name));

                    // Adjust selection based on remaining window count
                    let new_window_count = Tmux::get_window_count(&session_name).unwrap_or(0);
                    if tab_index >= new_window_count && new_window_count > 0 {
                        self.selected_tab = Some(new_window_count - 1);
                    } else if new_window_count == 0 {
                        self.selected_tab = None;
                    }
                }
            } else {
                // Session doesn't exist, just remove from config
                let (removed_name, new_tab_count) = {
                    if let Some(dimension) = self.config.dimensions.get_mut(self.selected_dimension) {
                        if let Some(tab) = dimension.remove_tab(tab_index) {
                            (Some(tab.name), dimension.tabs.len())
                        } else {
                            (None, dimension.tabs.len())
                        }
                    } else {
                        (None, 0)
                    }
                };

                if let Some(name) = removed_name {
                    self.save_config()?;
                    self.set_message(format!("Removed tab: {}", name));

                    if tab_index >= new_tab_count && new_tab_count > 0 {
                        self.selected_tab = Some(new_tab_count - 1);
                    } else if new_tab_count == 0 {
                        self.selected_tab = None;
                    }
                }
            }
        }

        Ok(())
    }

    // Input mode handling
    pub fn start_create_dimension(&mut self) {
        self.input_mode = InputMode::CreatingDimension;
        self.input_buffer.clear();
        self.clear_message();
    }

    pub fn start_add_tab(&mut self) {
        self.input_mode = InputMode::AddingTab;
        self.input_buffer.clear();
        self.clear_message();
    }

    pub fn start_delete_dimension(&mut self) {
        self.input_mode = InputMode::DeletingDimension;
        self.clear_message();
    }

    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Searching;
        self.input_buffer.clear();
        self.search_query.clear();
        self.last_computed_query.clear();
        self.search_results.clear();
        self.search_selected_index = 0;

        // Save current selection
        self.pre_search_dimension = self.selected_dimension;
        self.pre_search_tab = self.selected_tab;

        self.clear_message();
    }

    pub fn cancel_input(&mut self) {
        let was_searching = self.input_mode == InputMode::Searching;
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        if was_searching {
            self.search_query.clear();
            self.search_results.clear();
            self.last_computed_query.clear();

            // Restore pre-search selection
            self.selected_dimension = self.pre_search_dimension;
            self.selected_tab = self.pre_search_tab;
        }
        self.clear_message();
    }

    pub fn handle_input_char(&mut self, c: char) {
        self.input_buffer.push(c);
        // Live search: update search query as user types
        if self.input_mode == InputMode::Searching {
            self.search_query = self.input_buffer.clone();
        }
    }

    pub fn handle_input_backspace(&mut self) {
        self.input_buffer.pop();
        // Live search: update search query as user types
        if self.input_mode == InputMode::Searching {
            self.search_query = self.input_buffer.clone();
        }
    }

    pub fn submit_input(&mut self) -> Result<()> {
        match self.input_mode {
            InputMode::CreatingDimension => {
                let name = self.input_buffer.trim().to_string();
                if !name.is_empty() {
                    self.create_dimension(name)?;
                }
            }
            InputMode::AddingTab => {
                let input = self.input_buffer.trim();
                if !input.is_empty() {
                    // Parse: "name" or "name:command"
                    let parts: Vec<&str> = input.splitn(2, ':').collect();
                    let name = parts[0].to_string();
                    let command = parts.get(1).map(|s| s.to_string());
                    self.add_tab_to_current_dimension(name, command)?;
                }
            }
            InputMode::DeletingDimension => {
                if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
                    self.delete_dimension(&dimension.name.clone())?;
                }
            }
            InputMode::Searching => {
                // Live search updates query as user types, so nothing to do here
                // Enter with results is handled in handle_input_mode -> select_search_result
                return Ok(());
            }
            InputMode::Normal => {}
        }

        self.cancel_input();
        Ok(())
    }

    pub fn get_current_dimension(&self) -> Option<&Dimension> {
        self.config.dimensions.get(self.selected_dimension)
    }

    pub fn compute_search_results(&mut self) {
        // Only recompute if query changed
        if self.search_query == self.last_computed_query {
            return;
        }

        self.last_computed_query = self.search_query.clone();
        self.search_results.clear();
        self.search_selected_index = 0;

        if self.search_query.is_empty() {
            return;
        }

        let matcher = SkimMatcherV2::default();

        for (dim_idx, dimension) in self.config.dimensions.iter().enumerate() {
            let dim_score = matcher.fuzzy_match(&dimension.name, &self.search_query);

            // Get tabs from tmux if session exists, otherwise from config
            let tabs: Vec<(usize, String)> = if Tmux::session_exists(&dimension.name) {
                Tmux::list_windows(&dimension.name).unwrap_or_default()
            } else {
                dimension.tabs.iter()
                    .enumerate()
                    .map(|(i, t)| (i, t.name.clone()))
                    .collect()
            };

            if tabs.is_empty() && dim_score.is_some() {
                // Dimension matches but has no tabs - add dimension-only result
                self.search_results.push(SearchResult {
                    dimension_index: dim_idx,
                    dimension_name: dimension.name.clone(),
                    tab_index: 0,
                    tab_name: String::from("(no tabs)"),
                    score: dim_score.unwrap(),
                    match_type: MatchType::DimensionOnly,
                });
            } else {
                // Check each tab
                for (tab_idx, tab_name) in tabs.iter() {
                    let tab_score = matcher.fuzzy_match(tab_name, &self.search_query);

                    // Include if dimension OR tab matches
                    let (final_score, match_type) = match (dim_score, tab_score) {
                        (Some(ds), Some(ts)) => {
                            // Both match - use sum for better ranking
                            (ds + ts, MatchType::Both)
                        },
                        (Some(ds), None) => {
                            // Only dimension matches - include all its tabs
                            (ds, MatchType::DimensionOnly)
                        },
                        (None, Some(ts)) => {
                            // Only tab matches
                            (ts, MatchType::TabOnly)
                        },
                        (None, None) => continue, // No match
                    };

                    self.search_results.push(SearchResult {
                        dimension_index: dim_idx,
                        dimension_name: dimension.name.clone(),
                        tab_index: *tab_idx,
                        tab_name: tab_name.clone(),
                        score: final_score,
                        match_type,
                    });
                }
            }
        }

        // Sort by score descending (highest match first)
        self.search_results.sort_by(|a, b| b.score.cmp(&a.score));
    }

    pub fn next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.search_selected_index = (self.search_selected_index + 1) % self.search_results.len();
        }
    }

    pub fn previous_search_result(&mut self) {
        if !self.search_results.is_empty() {
            if self.search_selected_index == 0 {
                self.search_selected_index = self.search_results.len() - 1;
            } else {
                self.search_selected_index -= 1;
            }
        }
    }

    pub fn select_search_result(&mut self) -> Result<()> {
        if let Some(result) = self.search_results.get(self.search_selected_index) {
            // Update selection based on search result
            self.selected_dimension = result.dimension_index;
            self.selected_tab = Some(result.tab_index);

            // Clear search and return to normal mode
            self.input_mode = InputMode::Normal;
            self.search_query.clear();
            self.search_results.clear();
            self.last_computed_query.clear();

            // Immediately switch to the dimension
            self.switch_to_dimension()?;
        }
        Ok(())
    }
}
