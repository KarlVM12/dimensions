use anyhow::{Context, Result};
use std::process::Command;

/// Wrapper for tmux operations
pub struct Tmux;

impl Tmux {
    /// Check if tmux is installed
    pub fn is_installed() -> bool {
        Command::new("tmux")
            .arg("-V")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if we're currently inside a tmux session
    pub fn is_inside_session() -> bool {
        std::env::var("TMUX").is_ok()
    }

    /// Get the current tmux session name
    pub fn get_current_session() -> Result<String> {
        let output = Command::new("tmux")
            .args(["display-message", "-p", "#S"])
            .output()
            .context("Failed to get current tmux session")?;

        if !output.status.success() {
            anyhow::bail!("Not in a tmux session");
        }

        let session = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        Ok(session)
    }

    /// List all tmux sessions
    pub fn list_sessions() -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}"])
            .output()
            .context("Failed to list tmux sessions")?;

        if !output.status.success() {
            // No sessions exist yet
            return Ok(vec![]);
        }

        let sessions = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();

        Ok(sessions)
    }

    /// Create a new tmux session
    pub fn create_session(name: &str, detached: bool) -> Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.args(["new-session", "-s", name]);

        if detached {
            cmd.arg("-d");
        }

        let output = cmd.output().context("Failed to create tmux session")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to create session '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Kill a tmux session
    pub fn kill_session(name: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args(["kill-session", "-t", name])
            .output()
            .context("Failed to kill tmux session")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to kill session '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Attach to a tmux session
    pub fn attach_session(name: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["attach-session", "-t", name])
            .status()
            .context("Failed to attach to tmux session")?;

        if !status.success() {
            anyhow::bail!("Failed to attach to session '{}'", name);
        }

        Ok(())
    }

    /// Switch to a tmux session (when inside tmux)
    pub fn switch_session(name: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args(["switch-client", "-t", name])
            .output()
            .context("Failed to switch tmux session")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to switch to session '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Create a new window in a session
    pub fn new_window(session: &str, name: &str, command: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.args(["new-window", "-t", session, "-n", name]);

        if let Some(command) = command {
            cmd.arg(command);
        }

        let output = cmd.output().context("Failed to create tmux window")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to create window '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// List windows in a session
    pub fn list_windows(session: &str) -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                session,
                "-F",
                "#{window_name}",
            ])
            .output()
            .context("Failed to list tmux windows")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to list windows for session '{}': {}",
                session,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let windows = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();

        Ok(windows)
    }

    /// Select a specific window in a session by index (0-based)
    pub fn select_window(session: &str, window_index: usize) -> Result<()> {
        let output = Command::new("tmux")
            .args(["select-window", "-t", &format!("{}:{}", session, window_index)])
            .output()
            .context("Failed to select tmux window")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to select window {} in session '{}': {}",
                window_index,
                session,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Rename a window in a session
    pub fn rename_window(session: &str, window_index: usize, new_name: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args([
                "rename-window",
                "-t",
                &format!("{}:{}", session, window_index),
                new_name,
            ])
            .output()
            .context("Failed to rename tmux window")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to rename window {} in session '{}': {}",
                window_index,
                session,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Send keys (command) to a window in a session
    pub fn send_keys(session: &str, window_index: usize, keys: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args([
                "send-keys",
                "-t",
                &format!("{}:{}", session, window_index),
                keys,
                "C-m", // Enter key
            ])
            .output()
            .context("Failed to send keys to tmux window")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to send keys to window {} in session '{}': {}",
                window_index,
                session,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get the number of windows in a session
    pub fn get_window_count(session: &str) -> Result<usize> {
        let windows = Self::list_windows(session)?;
        Ok(windows.len())
    }

    /// Detach from the current tmux session
    pub fn detach() -> Result<()> {
        let output = Command::new("tmux")
            .arg("detach")
            .output()
            .context("Failed to detach from tmux")?;

        if !output.status.success() {
            anyhow::bail!("Failed to detach from tmux");
        }

        Ok(())
    }

    /// Check if a session exists
    pub fn session_exists(name: &str) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Rename a session
    pub fn rename_session(old_name: &str, new_name: &str) -> Result<()> {
        let output = Command::new("tmux")
            .args(["rename-session", "-t", old_name, new_name])
            .output()
            .context("Failed to rename tmux session")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to rename session '{}' to '{}': {}",
                old_name,
                new_name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}
