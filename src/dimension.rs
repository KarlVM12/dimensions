use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents a single tab (tmux window) in a dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub name: String,
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
}

impl Tab {
    pub fn new(name: String, command: Option<String>, working_dir: Option<PathBuf>) -> Self {
        Self { name, command, working_dir }
    }
}

/// Represents a dimension (tmux session with multiple tabs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    // Tabs persisted in config (used as a template when creating a tmux session).
    #[serde(rename = "tabs", default)]
    pub configured_tabs: Vec<Tab>,
}

impl Dimension {
    pub fn new(name: String) -> Self {
        Self {
            name,
            configured_tabs: vec![],
        }
    }

    pub fn add_tab(&mut self, tab: Tab) {
        self.configured_tabs.push(tab);
    }

    pub fn remove_tab(&mut self, index: usize) -> Option<Tab> {
        if index < self.configured_tabs.len() {
            Some(self.configured_tabs.remove(index))
        } else {
            None
        }
    }
}

/// Configuration for all dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionConfig {
    pub dimensions: Vec<Dimension>,
}

impl Default for DimensionConfig {
    fn default() -> Self {
        Self {
            dimensions: vec![],
        }
    }
}

impl DimensionConfig {
    /// Get the config file path
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dimensions");

        fs::create_dir_all(&config_dir).ok();
        config_dir.join("config.json")
    }

    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let config: DimensionConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Add a new dimension
    pub fn add_dimension(&mut self, dimension: Dimension) {
        self.dimensions.push(dimension);
    }

    /// Remove a dimension by name
    pub fn remove_dimension(&mut self, name: &str) -> Option<Dimension> {
        if let Some(pos) = self.dimensions.iter().position(|d| d.name == name) {
            Some(self.dimensions.remove(pos))
        } else {
            None
        }
    }

    /// Get a dimension by name
    pub fn get_dimension(&self, name: &str) -> Option<&Dimension> {
        self.dimensions.iter().find(|d| d.name == name)
    }

}
