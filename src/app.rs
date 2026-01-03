use crate::dimension::{Dimension, DimensionConfig, Tab};
use crate::tmux::Tmux;
use crate::update;
use anyhow::Result;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    CreatingDimension,
    CreatingDimensionDirectory,
    AddingTab,
    DeletingDimension,
    DeletingTab,
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
    // Index into the dimension's tab list / `list_windows()` result list.
    pub tab_index: usize,
    // Actual tmux window index (e.g. 0, 1, 2) when known.
    pub tmux_window_index: usize,
    pub tab_name: String,
    pub score: i64,
    pub match_type: MatchType,
}

pub struct App {
    pub config: DimensionConfig,
    pub selected_dimension: usize,
    // None means dimension selected.
    // Some(i) means:
    // - if the selected dimension's tmux session exists: tmux window index (#I)
    // - otherwise: configured tab list index
    pub selected_tab: Option<usize>,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_selected_index: usize,
    pub last_computed_query: String,
    pub pre_search_dimension: usize,
    pub pre_search_tab: Option<usize>,
    pub message: Option<String>,
    pub update_message: Option<String>,
    pub should_quit: bool,
    pub should_attach: Option<String>, // Session name to attach to after quitting
    pub should_select_window: Option<usize>, // Window index to select after attaching
    pub should_detach: bool, // Whether to detach from tmux on quit
    pub current_session: Option<String>, // Current tmux session when app was opened
    pub current_window: Option<usize>, // Current tmux window index when app was opened

    // Directory input completion state
    pub pending_dimension_name: Option<String>, // Cache dimension name between creation steps
    pub completion_candidates: Vec<String>, // Directory matches for tab completion
    pub completion_index: usize, // Current selection when cycling through completions
    pub completion_base: String, // Original input before cycling completions

    update_rx: Option<mpsc::Receiver<Option<String>>>,
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

        // Start selection on the current tmux session's dimension (useful for popup mode).
        let selected_dimension = current_session
            .as_ref()
            .and_then(|session| config.dimensions.iter().position(|d| d.name == *session))
            .unwrap_or(0);

        // Check for updates in the background (best-effort).
        let (update_tx, update_rx) = mpsc::channel();
        thread::spawn(move || {
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("dimensions");
            let msg = update::check_for_update_message(config_dir, env!("CARGO_PKG_VERSION"));
            let _ = update_tx.send(msg);
        });

        Ok(Self {
            config,
            selected_dimension,
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
            update_message: None,
            should_quit: false,
            should_attach: None,
            should_select_window: None,
            should_detach: false,
            current_session,
            current_window,
            pending_dimension_name: None,
            completion_candidates: Vec::new(),
            completion_index: 0,
            completion_base: String::new(),
            update_rx: Some(update_rx),
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

    pub fn poll_update(&mut self) {
        let Some(rx) = self.update_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(msg) => {
                self.update_message = msg;
                self.update_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.update_rx = None;
            }
        }
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
            if Tmux::session_exists(&dimension.name) {
                // Live tmux windows: track selection by tmux window index for robustness.
                let windows = Tmux::list_windows(&dimension.name).unwrap_or_default();
                if windows.is_empty() {
                    self.selected_tab = None;
                    return;
                }

                let next_idx = match self.selected_tab {
                    None => windows[0].0,
                    Some(current_window_idx) => {
                        let pos = windows
                            .iter()
                            .position(|(idx, _)| *idx == current_window_idx)
                            .unwrap_or(0);
                        windows[(pos + 1) % windows.len()].0
                    }
                };
                self.selected_tab = Some(next_idx);
            } else {
                // Configured tabs: track selection by configured tab index.
                let tab_count = dimension.configured_tabs.len();
                if tab_count == 0 {
                    self.selected_tab = None;
                    return;
                }

                self.selected_tab = Some(match self.selected_tab {
                    None => 0, // First right arrow selects first tab
                    Some(i) => (i + 1) % tab_count,
                });
            }
        }
    }

    pub fn previous_tab(&mut self) {
        if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
            if Tmux::session_exists(&dimension.name) {
                let windows = Tmux::list_windows(&dimension.name).unwrap_or_default();
                if windows.is_empty() {
                    self.selected_tab = None;
                    return;
                }

                self.selected_tab = match self.selected_tab {
                    None => Some(windows[windows.len() - 1].0), // Left arrow selects last tab
                    Some(current_window_idx) => {
                        let pos = windows
                            .iter()
                            .position(|(idx, _)| *idx == current_window_idx)
                            .unwrap_or(0);
                        if pos == 0 {
                            None // Wrap back to dimension
                        } else {
                            Some(windows[pos - 1].0)
                        }
                    }
                };
            } else {
                let tab_count = dimension.configured_tabs.len();
                if tab_count == 0 {
                    self.selected_tab = None;
                    return;
                }

                self.selected_tab = match self.selected_tab {
                    None => Some(tab_count - 1), // Left arrow selects last tab
                    Some(0) => None, // Wrap back to dimension
                    Some(i) => Some(i - 1),
                };
            }
        }
    }

    // Dimension operations
    pub fn create_dimension(&mut self, name: String, base_dir: Option<std::path::PathBuf>) -> Result<()> {
        // Check if dimension already exists
        if self.config.get_dimension(&name).is_some() {
            anyhow::bail!("Dimension '{}' already exists", name);
        }

        // Add to config only - tmux session will be created when switching to it
        let dimension = Dimension::new_with_base_dir(name.clone(), base_dir);
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

    pub fn switch_to_dimension(&mut self) -> Result<()> {
        if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
            let name = dimension.name.clone();
            let base_dir = dimension.base_dir.clone();
            let has_tabs = !dimension.configured_tabs.is_empty();
            let tabs = dimension.configured_tabs.clone();
            let session_preexisted = Tmux::session_exists(&name);

            // Ensure tmux session exists
            if !session_preexisted {
                // Create session in base_dir if available
                if let Some(dir) = base_dir.as_ref() {
                    Tmux::create_session_with_dir(&name, true, dir.to_str().unwrap_or("."))?;
                } else {
                    Tmux::create_session(&name, true)?;
                }

                // If there are configured tabs, create windows for them
                if has_tabs {
                    for (i, tab) in tabs.iter().enumerate() {
                        if i == 0 {
                            // First window is created with the session, rename it to match first tab
                            Tmux::rename_window(&name, 0, &tab.name)?;

                            // Build command for first tab (with working dir if needed)
                            let full_command = match (&tab.working_dir, &tab.command) {
                                (Some(dir), Some(cmd)) => {
                                    // Both working_dir and command: cd then run command
                                    format!("cd {:?} && {}", dir, cmd)
                                }
                                (Some(dir), None) => {
                                    // Only working_dir: just cd
                                    format!("cd {:?}", dir)
                                }
                                (None, Some(cmd)) => {
                                    // Only command: just run it
                                    cmd.clone()
                                }
                                (None, None) => String::new(),
                            };

                            // Send command if we have one
                            if !full_command.is_empty() {
                                Tmux::send_keys(&name, 0, &full_command)?;
                            }
                        } else {
                            Tmux::new_window(&name, &tab.name, tab.command.as_deref(), tab.working_dir.as_deref())?;
                        }
                    }
                } else {
                    // No configured tabs: create and save an initial tab
                    let initial_tab_name = format!("{}-1", name);
                    Tmux::rename_window(&name, 0, &initial_tab_name)?;

                    // Save this initial tab to config so it persists across restarts
                    let initial_tab = Tab::new(initial_tab_name, None, base_dir.clone());
                    if let Some(dim) = self.config.dimensions.get_mut(self.selected_dimension) {
                        dim.add_tab(initial_tab);
                        self.save_config()?;
                    }
                }
            }

            // Determine which window to select
            let window_index = match self.selected_tab {
                None => 0,
                Some(selected) => {
                    if session_preexisted {
                        // Selected is already a tmux window index; validate it still exists.
                        let windows = Tmux::list_windows(&name).unwrap_or_default();
                        if windows.iter().any(|(idx, _)| *idx == selected) {
                            selected
                        } else {
                            0
                        }
                    } else {
                        // Selected is a configured tab index; map to tmux window index after creation.
                        let windows = Tmux::list_windows(&name).unwrap_or_default();
                        windows.get(selected).map(|(idx, _)| *idx).unwrap_or(0)
                    }
                }
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
            // Inherit working_dir from dimension's base_dir, or use current_dir as fallback
            let working_dir = dimension.base_dir.clone()
                .or_else(|| std::env::current_dir().ok());

            let tab = Tab::new(name.clone(), command.clone(), working_dir.clone());
            dimension.add_tab(tab);

            // Create window in tmux if session exists
            if Tmux::session_exists(&dimension.name) {
                Tmux::new_window(&dimension.name, &name, command.as_deref(), working_dir.as_deref())?;
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
                if let Some((window_idx, window_name)) =
                    windows.iter().find(|(idx, _)| *idx == tab_index)
                {
                    let window_idx = *window_idx;
                    let window_name = window_name.clone();

                    // Kill the tmux window
                    Tmux::kill_window(&session_name, window_idx)?;

                    // Remove from config if it exists there
                    if let Some(dimension) = self.config.dimensions.get_mut(self.selected_dimension) {
                        if let Some(config_index) = dimension
                            .configured_tabs
                            .iter()
                            .position(|t| t.name == window_name)
                        {
                            dimension.remove_tab(config_index);
                        }
                    }
                    self.save_config()?;
                    self.set_message(format!("Removed tab: {}", window_name));

                    // If we just killed the active window in the current session, tmux will
                    // switch the client to another window. Keep our selection in sync.
                    if self.current_session.as_ref() == Some(&session_name) && Tmux::is_inside_session() {
                        if let Ok(current_idx) = Tmux::get_current_window_index() {
                            self.current_window = Some(current_idx);
                            self.selected_tab = Some(current_idx);
                            return Ok(());
                        }
                    }

                    // Otherwise, adjust selection based on remaining windows (track by tmux window index).
                    let remaining = Tmux::list_windows(&session_name).unwrap_or_default();
                    self.selected_tab = remaining.first().map(|(idx, _)| *idx);
                }
            } else {
                // Session doesn't exist, just remove from config
                let (removed_name, new_tab_count) = {
                    if let Some(dimension) = self.config.dimensions.get_mut(self.selected_dimension) {
                        if let Some(tab) = dimension.remove_tab(tab_index) {
                            (Some(tab.name), dimension.configured_tabs.len())
                        } else {
                            (None, dimension.configured_tabs.len())
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

    pub fn start_delete_tab(&mut self) {
        self.input_mode = InputMode::DeletingTab;
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
        self.pending_dimension_name = None;
        self.clear_completion_state();
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
        self.clear_completion_state();
        // Live search: update search query as user types
        if self.input_mode == InputMode::Searching {
            self.search_query = self.input_buffer.clone();
        }
    }

    pub fn handle_input_backspace(&mut self) {
        self.input_buffer.pop();
        self.clear_completion_state();
        // Live search: update search query as user types
        if self.input_mode == InputMode::Searching {
            self.search_query = self.input_buffer.clone();
        }
    }

    pub fn clear_completion_state(&mut self) {
        self.completion_candidates.clear();
        self.completion_base.clear();
        self.completion_index = 0;
    }

    pub fn handle_tab_completion(&mut self) {
        self.handle_tab_completion_direction(1);
    }

    pub fn handle_backtab_completion(&mut self) {
        self.handle_tab_completion_direction(-1);
    }

    fn handle_tab_completion_direction(&mut self, direction: i32) {
        use crate::path_completion::PathCompleter;

        // Only complete in directory input mode
        if self.input_mode != InputMode::CreatingDimensionDirectory {
            return;
        }

        // If we're cycling through existing candidates
        if !self.completion_candidates.is_empty() && !self.completion_base.is_empty() {
            // Move to next/previous candidate
            let len = self.completion_candidates.len() as i32;
            self.completion_index = ((self.completion_index as i32 + direction + len) % len) as usize;
            self.input_buffer = self.completion_candidates[self.completion_index].clone();
            return;
        }

        // Fresh completion request (only for forward tab)
        if direction < 0 {
            return;
        }

        let input = self.input_buffer.trim();
        let (candidates, common_prefix) = PathCompleter::complete_directory(input);

        match candidates.len() {
            0 => {
                // No matches - do nothing
            }
            1 => {
                // Single match - complete it fully and add trailing slash
                let completed = format!("{}/", &candidates[0]);
                self.input_buffer = completed;
                // Clear completion state
                self.completion_candidates.clear();
                self.completion_base.clear();
                self.completion_index = 0;
            }
            _ => {
                // Multiple matches
                if common_prefix.len() > input.len() {
                    // There's a common prefix we can complete to
                    self.input_buffer = common_prefix.clone();
                    // Save state for cycling
                    self.completion_base = common_prefix;
                    self.completion_candidates = candidates;
                    self.completion_index = 0;
                } else {
                    // No common prefix - start cycling through candidates
                    self.completion_base = input.to_string();
                    self.completion_candidates = candidates.clone();
                    self.completion_index = 0;
                    self.input_buffer = candidates[0].clone();
                }
            }
        }
    }

    pub fn submit_input(&mut self) -> Result<()> {
        match self.input_mode {
            InputMode::CreatingDimension => {
                let name = self.input_buffer.trim().to_string();
                if !name.is_empty() {
                    // Save the name and transition to directory input
                    self.pending_dimension_name = Some(name);
                    self.input_mode = InputMode::CreatingDimensionDirectory;
                    self.input_buffer.clear();
                    // Pre-fill with current directory as suggestion
                    if let Ok(cwd) = std::env::current_dir() {
                        if let Some(cwd_str) = cwd.to_str() {
                            self.input_buffer = cwd_str.to_string();
                        }
                    }
                    return Ok(());
                }
            }
            InputMode::CreatingDimensionDirectory => {
                use crate::path_completion::PathCompleter;

                let input = self.input_buffer.trim();

                // Allow empty input (no base directory)
                if input.is_empty() {
                    if let Some(name) = self.pending_dimension_name.take() {
                        self.create_dimension(name, None)?;
                    }
                } else {
                    // Validate the directory
                    match PathCompleter::validate_directory(input) {
                        Ok(path) => {
                            if let Some(name) = self.pending_dimension_name.take() {
                                self.create_dimension(name, Some(path))?;
                            }
                        }
                        Err(err) => {
                            self.set_message(err);
                            return Ok(()); // Stay in input mode to allow correction
                        }
                    }
                }

                self.cancel_input();
                return Ok(());
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
            InputMode::DeletingTab => {
                self.remove_tab_from_current_dimension()?;
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
                dimension
                    .configured_tabs
                    .iter()
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
                    tmux_window_index: 0,
                    tab_name: String::from("(no tabs)"),
                    score: dim_score.unwrap(),
                    match_type: MatchType::DimensionOnly,
                });
            } else {
                // Check each tab
                for (list_idx, (window_idx, tab_name)) in tabs.iter().enumerate() {
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
                        tab_index: list_idx,
                        tmux_window_index: *window_idx,
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
            self.selected_tab = if Tmux::session_exists(&result.dimension_name) {
                Some(result.tmux_window_index)
            } else {
                Some(result.tab_index)
            };

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
