# Dimensions 🌌


![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)


Dimensions is a TUI (Terminal User Interface) for managing tmux sessions and windows. It provides a visual interface to organize your terminal workflows into groups called "dimensions".  <br> <br>

<img width="1797" height="1048" alt="demo" src="https://github.com/user-attachments/assets/1e1b98ad-1e43-4156-9767-d1b2b212e1cf" />


Key features:

- **🔍 Fuzzy search** - Live search across all dimensions and tabs with instant results
- **🔄 Switch between dimensions** - All processes stay alive in the background
- **💾 Persistent configuration** - Dimension names, tabs, and commands saved to disk
- **⚡ Popup mode** - Quick access with Ctrl+G from anywhere (even inside vim/nvim)
- **🎨 Beautiful interface** - Clean TUI built with ratatui
- **🚀 Lightning fast** - Written in Rust, powered by tmux
- **🖥️ Works on macOS & Linux** - Any terminal emulator (iTerm2, Alacritty, Wezterm, Kitty, etc.)
- **📑 Tab Preview** - Running tabs will show a live preview


## Why Dimensions?

**Problem:** You have different workflows (dev, personal, work) that require different sets of terminals. Switching contexts means:
- Closing/reopening tabs manually
- Losing running processes
- Forgetting what you had open

**Solution:** Dimensions lets you:
- Group tabs into named dimensions
- Switch between dimensions instantly
- Never lose your running processes

## Installation

**Prerequisites:**
- **tmux**: `brew install tmux` (macOS) or `apt install tmux` (Linux)

**Install Dimensions:**

```bash
curl -fsSL https://raw.githubusercontent.com/KarlVM12/Dimensions/master/install.sh | sh
```

This installs to `~/.local/bin/dimensions`. Make sure `~/.local/bin` is in your `PATH`.

## Usage

### Recommended tmux Configuration

For the best experience, add this to your `~/.tmux.conf`:

```bash
# Bind Ctrl+G to open Dimensions in a popup (works even inside nvim/vim)
bind -n C-g display-popup -E -w 80% -h 80% "dimensions"

# Window configuration for Dimensions
set -g base-index 0           # Optional: start window numbering at 0
set -g renumber-windows on    # Optional: renumber windows when one is closed
set -g mouse on               # Enable mouse support

# Optional: Minimal status bar (avoids dimension name truncation)
set -g status-left "🌌 "
```

After adding this, reload your tmux config:
```bash
tmux source-file ~/.tmux.conf
```

Now press `Ctrl+G` from anywhere (even inside nvim, Claude, or other programs) to:
- Open Dimensions in a popup overlay
- Navigate and select a dimension/tab
- Press Enter to switch (popup closes and switches to selected tab)
- Press Esc to close popup without switching

**Alternative keybindings:**
```bash
# Use Ctrl+D instead
bind -n C-d display-popup -E -w 80% -h 80% "dimensions"

# Use Prefix + Space (Ctrl+B then Space)
bind Space display-popup -E -w 80% -h 80% "dimensions"
```

### Keyboard Shortcuts

#### Normal Mode
- `↑/k` - Previous dimension
- `↓/j` - Next dimension
- `→/l` - Navigate right to select a tab
- `←/h` - Navigate left (back to dimension)
- `Enter` - Switch to the selected dimension/tab
- `n` - Create new dimension
- `t` - Add new tab to current dimension (format: `name` or `name:command`)
- `d` - **Context-sensitive delete** (prompts for confirmation):
  - If tab is selected: Delete that tab
  - If on dimension: Delete entire dimension
  - If deleting your last tab or current dimension, automatically switches to the first available tab (or a scratch session as fallback)
- `r` - **Context-sensitive rename**:
  - If tab is selected: Rename that tab
  - If on dimension: Rename the dimension (also renames the live tmux session)
- `/` - **Fuzzy search** across all dimensions and tabs (live updates)
- `:` - Jump to a tab in the dimension you are hovering over
  - If you are hovering on a dimension typing `:2` will go to the third tab, finishing with `Enter` will bring you right in
- `Esc` - Close popup without switching
- `q` - Quit TUI and detach from tmux

#### Input Mode (when creating dimension/tab)
- `Enter` - Submit
- `Esc` - Cancel
- `Backspace` - Delete character
- **Adding Directory** - when creating a dimension, can specify which default directory for all the tabs to use as well
  - Starts new path based on where you original started the `dimensions` command from
  - Can accept env vars like `$HOME` or `$VAR` or relative pathing `..`
  - `Tab` will cycle through all dirs from what you have, `Shift+Tab` to go to prev tabbed over
  - `Enter` - Submit
  - `Esc` - Cancel

#### Search Mode (when searching with `/`)
- **Fuzzy matching** - Search updates live as you type (e.g., "edt" matches "Editor")
- Searches both **dimension names** and **tab names** across all dimensions
- Results shown as flat list: "dimension: tab_name"
- Sorted by fuzzy match score (best matches first)
- `↑/↓` - Navigate through search results
- `Enter` - Select result and switch to that dimension/tab immediately
- `Esc` - Cancel search and return to normal mode

### Tab Persistence

**Tabs created via Dimensions (`t` key)** are saved to the config file and will be recreated when you restart a dimension.

**Manual tmux windows** (created via `tmux new-window` or other tmux commands) are temporary and only exist until you kill the tmux session. They will appear in Dimensions while the session is active, but won't be recreated.

**If you start using raw tmux commands** to manage windows while also using Dimensions, we can't guarantee perfect parity between the two. Dimensions works best when you manage tabs through the TUI.

**Config location:**
- **macOS**: `~/Library/Application Support/dimensions/config.json`
- **Linux**: `~/.config/dimensions/config.json`

### Update Checks

Dimensions checks GitHub Releases once per day to show a "New version available" message. Disable with `DIMENSIONS_NO_UPDATE_CHECK=1`.

**Commands:**
- `dimensions` - Launch the TUI
- `dimensions --version` - Print current version
- `dimensions --update` - Check for updates and optionally install the latest release

## Contributing

PRs welcome! Some ideas:

- Support for saving/restoring working directories
- Mouse support
- Color themes
- Shell completions
- Export/import dimension configurations

## Credits

- Built with [ratatui](https://github.com/ratatui-org/ratatui) for the TUI
- Powered by [tmux](https://github.com/tmux/tmux) for session management
- Fuzzy search powered by [fuzzy-matcher](https://github.com/lotabout/fuzzy-matcher)
