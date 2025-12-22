use crate::dimension::{Dimension, DimensionConfig, Tab};
use crate::tmux::Tmux;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    CreatingDimension,
    AddingTab,
    DeletingDimension,
}

pub struct App {
    pub config: DimensionConfig,
    pub selected_dimension: usize,
    pub selected_tab: Option<usize>, // None means dimension selected, Some(i) means tab i selected
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub message: Option<String>,
    pub should_quit: bool,
    pub should_attach: Option<String>, // Session name to attach to after quitting
    pub should_detach: bool, // Whether to detach from tmux on quit
}

impl App {
    pub fn new() -> Result<Self> {
        let config = DimensionConfig::load()?;

        Ok(Self {
            config,
            selected_dimension: 0,
            selected_tab: None, // Start with dimension selected, not a tab
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            message: None,
            should_quit: false,
            should_attach: None,
            should_detach: false,
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
            if !dimension.tabs.is_empty() {
                match self.selected_tab {
                    None => self.selected_tab = Some(0), // First right arrow selects first tab
                    Some(i) => self.selected_tab = Some((i + 1) % dimension.tabs.len()),
                }
            }
        }
    }

    pub fn previous_tab(&mut self) {
        if let Some(dimension) = self.config.dimensions.get(self.selected_dimension) {
            if !dimension.tabs.is_empty() {
                match self.selected_tab {
                    None => self.selected_tab = Some(dimension.tabs.len() - 1), // Left arrow selects last tab
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

        // Create tmux session
        Tmux::create_session(&name, true)?;

        // Add to config
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

            // Ensure tmux session exists
            if !Tmux::session_exists(&name) {
                Tmux::create_session(&name, true)?;

                // If there are configured tabs, create windows for them
                if !dimension.tabs.is_empty() {
                    for (i, tab) in dimension.tabs.iter().enumerate() {
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
                    // No configured tabs, rename the default window to dimension name
                    Tmux::rename_window(&name, 0, &format!("{}-1", name))?;
                }
            }

            // Check if we're already in this session
            let already_in_session = Tmux::get_current_session()
                .ok()
                .map(|current| current == name)
                .unwrap_or(false);

            match self.selected_tab {
                None => {
                    // No tab selected - create ad-hoc window
                    if already_in_session {
                        // Get next available window number
                        let window_count = Tmux::get_window_count(&name)?;
                        let ad_hoc_name = format!("{}-{}", name, window_count + 1);
                        Tmux::new_window(&name, &ad_hoc_name, None)?;
                        Tmux::select_window(&name, window_count)?;
                    }
                }
                Some(tab_index) => {
                    // Specific tab selected - go to that tab
                    if already_in_session {
                        Tmux::select_window(&name, tab_index)?;
                    }
                }
            }

            // Set the session to attach to after exiting TUI
            self.should_attach = Some(name.clone());

            // Update config
            self.config.set_active(Some(name));
            self.save_config()?;

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
            let (tab_name, new_tab_count) = {
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

            if let Some(name) = tab_name {
                self.save_config()?;
                self.set_message(format!("Removed tab: {}", name));

                // Adjust selection
                if tab_index >= new_tab_count && new_tab_count > 0 {
                    self.selected_tab = Some(new_tab_count - 1);
                } else if new_tab_count == 0 {
                    self.selected_tab = None;
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

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.clear_message();
    }

    pub fn handle_input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn handle_input_backspace(&mut self) {
        self.input_buffer.pop();
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
            InputMode::Normal => {}
        }

        self.cancel_input();
        Ok(())
    }

    pub fn get_current_dimension(&self) -> Option<&Dimension> {
        self.config.dimensions.get(self.selected_dimension)
    }
}
